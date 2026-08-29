use pixi_byte::value::JSValue;
use pixi_byte::vm::VM;

#[test]
fn test_object_builtins_registered_and_functional() {
    let mut vm = VM::new();
    let global = vm.global_object.clone();

    // Object がグローバルに存在する
    let obj_val = global.borrow().get("Object");
    let obj_ref = obj_val
        .as_object()
        .unwrap_or_else(|| panic!("Object global is not an object"));
    // create と getPrototypeOf が登録されている
    let create = obj_ref.borrow().get("create");
    let get_proto = obj_ref.borrow().get("getPrototypeOf");

    assert!(
        create.is_native_function(),
        "Object.create not installed as native function"
    );

    assert!(
        get_proto.is_native_function(),
        "Object.getPrototypeOf not installed as native function"
    );

            // Object.create を直接呼び出してプロトタイプが正しく設定されるかを確認
            // まず prototype オブジェクトを作成
            let proto = pixi_byte::value::jsobject::JSObject::new();
            let proto_obj = JSValue::from_object(std::rc::Rc::new(std::cell::RefCell::new(proto)));

            if let Some(f) = obj_ref.borrow().get("create").as_native_function() {
                let res = f(&mut vm, vec![proto_obj.clone()]).unwrap();
                match res.as_object() {
                    Some(new_obj_ref) => {
                        // プロトタイプが設定されていることを確認
                        let got = new_obj_ref.borrow().get_prototype();
                        assert!(got.is_some());
                        if let Some(gp) = got {
                            // 同一参照かどうか
                            assert_eq!(
                                std::rc::Rc::ptr_eq(
                                    &gp,
                                    &std::rc::Rc::new(std::cell::RefCell::new(
                                        pixi_byte::value::jsobject::JSObject::new()
                                    ))
                                    .clone()
                                ),
                                false
                            );
                            // ここでは proto_obj と同じプロトタイプが設定されていることを期待
                        }
                    }
                    _ => panic!("Object.create did not return an object"),
                }
            } else {
                panic!("create not callable");
            }

            // Object.getPrototypeOf を直接呼び出してプロトタイプを取得できることを確認
            // 先ほど作ったオブジェクトを再利用するため再度作成
            if let Some(f) = obj_ref.borrow().get("create").as_native_function() {
                let created = f(&mut vm, vec![proto_obj.clone()]).unwrap();
                if let Some(created_ref) = created.as_object() {
                    if let Some(getp) = obj_ref
                        .borrow()
                        .get("getPrototypeOf")
                        .as_native_function()
                    {
                        let got =
                            getp(&mut vm, vec![JSValue::from_object(created_ref.clone())]).unwrap();
                        if let Some(gp_ref) = got.as_object() {
                            // gp_ref should point to same prototype as proto_obj
                            if let Some(proto_clone_ref) = proto_obj.as_object() {
                                assert!(std::rc::Rc::ptr_eq(&gp_ref, &proto_clone_ref));
                            }
                        } else if got.is_null() {
                            panic!("getPrototypeOf returned null unexpectedly");
                        } else {
                            panic!("getPrototypeOf returned unexpected value");
                        }
                    } else {
                        panic!("getPrototypeOf not callable");
                    }
                } else {
                    panic!("create did not return object for second call");
                }
            } else {
                panic!("create not callable for second call");
            }
}

#[test]
fn test_object_create_with_descriptors() {
    let mut vm = VM::new();
    let global = vm.global_object.clone();

    let proto = pixi_byte::value::jsobject::JSObject::new();
    let proto_obj = JSValue::from_object(std::rc::Rc::new(std::cell::RefCell::new(proto)));
    let mut desc_inner = pixi_byte::value::jsobject::JSObject::new();

    desc_inner.set("value".to_string(), JSValue::from_number(10.0));
    desc_inner.set("writable".to_string(), JSValue::from_bool(true));
    desc_inner.set("enumerable".to_string(), JSValue::from_bool(true));
    desc_inner.set("configurable".to_string(), JSValue::from_bool(false));
    let desc_inner_rc = std::rc::Rc::new(std::cell::RefCell::new(desc_inner));

    let mut desc_outer = pixi_byte::value::jsobject::JSObject::new();
    desc_outer.set("a".to_string(), JSValue::from_object(desc_inner_rc.clone()));
    let desc_outer_rc = std::rc::Rc::new(std::cell::RefCell::new(desc_outer));

    let obj_global = global.borrow().get("Object");
    if let Some(obj_ref) = obj_global.as_object() {
        if let Some(create_fn) = obj_ref.borrow().get("create").as_native_function() {
            let res = create_fn(
                &mut vm,
                vec![proto_obj.clone(), JSValue::from_object(desc_outer_rc.clone())],
            )
            .unwrap();
            match res.as_object() {
                Some(new_obj_ref) => {
                    let a = new_obj_ref.borrow().get("a");
                    match a.as_number() {
                        Some(n) => assert_eq!(n, 10.0),
                        _ => panic!("property 'a' not defined correctly"),
                    }
                    let deleted = new_obj_ref.borrow_mut().delete("a");
                    assert_eq!(deleted, false);
                }
                _ => panic!("Object.create with descriptors did not return object"),
            }
        } else {
            panic!("create not callable");
        }
    } else {
        panic!("Object constructor not found");
    }
}

#[test]
fn test_is_prototype_of() {
    let mut vm = VM::new();
    let global = vm.global_object.clone();

    // create proto and child objects
    let proto = pixi_byte::value::jsobject::JSObject::new();
    let proto_obj = JSValue::from_object(std::rc::Rc::new(std::cell::RefCell::new(proto)));

    let obj_global = global.borrow().get("Object");
    if let Some(obj_ref) = obj_global.as_object() {
        // create an object with proto as its prototype
        if let Some(create_fn) = obj_ref.borrow().get("create").as_native_function() {
            let created = create_fn(&mut vm, vec![proto_obj.clone()]).unwrap();
            if let Some(created_ref) = created.as_object() {
                // call isPrototypeOf on proto
                if let Some(proto_constructor) = global.borrow().get("Object").as_object() {
                    if let Some(proto_proto) = proto_constructor.borrow().get("prototype").as_object()
                    {
                        // proto_proto is Object.prototype, but we want to call isPrototypeOf on proto_obj instance
                        if let Some(isp) = proto_proto.borrow().get("isPrototypeOf").as_native_function()
                        {
                            // method expects receiver as first arg
                            let res = isp(
                                &mut vm,
                                vec![proto_obj.clone(), JSValue::from_object(created_ref.clone())],
                            )
                            .unwrap();
                            match res.as_boolean() {
                                Some(b) => assert!(b),
                                _ => panic!("isPrototypeOf returned non-boolean"),
                            }
                        } else {
                            panic!("isPrototypeOf not found on Object.prototype");
                        }
                    } else {
                        panic!("Object.prototype not found");
                    }
                } else {
                    panic!("Object constructor not found");
                }
            } else {
                panic!("create did not return object");
            }
        } else {
            panic!("create not callable");
        }
    } else {
        panic!("Object constructor missing");
    }
}

#[test]
fn test_object_to_string() {
    let mut vm = VM::new();
    let global = vm.global_object.clone();

    // get Object.prototype.toString
    let obj_global = global.borrow().get("Object");
    if let Some(obj_ref) = obj_global.as_object() {
        if let Some(proto_obj) = obj_ref.borrow().get("prototype").as_object() {
            if let Some(to_str) = proto_obj.borrow().get("toString").as_native_function() {
                // call on an object
                let target = JSValue::from_object(std::rc::Rc::new(std::cell::RefCell::new(
                    pixi_byte::value::jsobject::JSObject::new(),
                )));
                let res = to_str(&mut vm, vec![target]).unwrap();
                match res.as_string() {
                    Some(s) => assert_eq!(s, "[object Object]"),
                    _ => panic!("toString returned non-string"),
                }

                // call on a function value (NativeFunction)
                let res2 = to_str(
                    &mut vm,
                    vec![JSValue::from_native_function(|_, _| Ok(JSValue::undefined()))],
                )
                .unwrap();
                match res2.as_string() {
                    Some(s) => assert_eq!(s, "[object Function]"),
                    _ => panic!("toString on function returned non-string"),
                }
            } else {
                panic!("toString not found on Object.prototype");
            }
        } else {
            panic!("Object.prototype missing");
        }
    } else {
        panic!("Object constructor missing");
    }
}

#[test]
fn test_define_property_and_get_own_property_descriptor() {
    let mut vm = VM::new();
    let global = vm.global_object.clone();

    let obj_global = global.borrow().get("Object");
    if let Some(obj_ref) = obj_global.as_object() {
        // prepare target object
        let target = pixi_byte::value::jsobject::JSObject::new();
        let target_rc = std::rc::Rc::new(std::cell::RefCell::new(target));
        let target_val = JSValue::from_object(target_rc.clone());

        // descriptor: { value: 5, writable: true, enumerable: false, configurable: true }
        let mut desc = pixi_byte::value::jsobject::JSObject::new();
        desc.set("value".to_string(), JSValue::from_number(5.0));
        desc.set("writable".to_string(), JSValue::from_bool(true));
        desc.set("enumerable".to_string(), JSValue::from_bool(false));
        desc.set("configurable".to_string(), JSValue::from_bool(true));
        let desc_rc = std::rc::Rc::new(std::cell::RefCell::new(desc));

        // call Object.defineProperty(target, "x", desc)
        if let Some(def_fn) = obj_ref
            .borrow()
            .get("defineProperty")
            .as_native_function()
        {
            let _ = def_fn(
                &mut vm,
                vec![
                    target_val.clone(),
                    JSValue::from_string("x".to_string()),
                    JSValue::from_object(desc_rc.clone()),
                ],
            )
            .unwrap();
            // get descriptor back
            if let Some(gopd) = obj_ref
                .borrow()
                .get("getOwnPropertyDescriptor")
                .as_native_function()
            {
                let got = gopd(
                    &mut vm,
                    vec![target_val.clone(), JSValue::from_string("x".to_string())],
                )
                .unwrap();
                match got.as_object() {
                    Some(desc_obj_ref) => {
                        let v = desc_obj_ref.borrow().get("value");
                        assert_eq!(v.to_number(), 5.0);
                        let writable = desc_obj_ref.borrow().get("writable");
                        assert!(writable.to_boolean());
                        let enumerable = desc_obj_ref.borrow().get("enumerable");
                        assert!(!enumerable.to_boolean());
                    }
                    _ => panic!("getOwnPropertyDescriptor returned unexpected value"),
                }
            } else {
                panic!("getOwnPropertyDescriptor not callable");
            }
        } else {
            panic!("defineProperty not callable");
        }

        // accessor property: { get: function() { return 42 }, set: function(v) { } }
        let _accessor_desc = pixi_byte::value::jsobject::JSObject::new();
        // represent getter as NativeFunction
        let getter_native = JSValue::from_native_function(|_vm, _args| Ok(JSValue::from_number(42.0)));
        let getter_val = getter_native.clone();

        let mut accessor_desc = pixi_byte::value::jsobject::JSObject::new();
        accessor_desc.set("get".to_string(), getter_val.clone());
        accessor_desc.set(
            "set".to_string(),
            JSValue::from_native_function(|_vm, _args| Ok(JSValue::undefined())),
        );
        let accessor_desc_rc = std::rc::Rc::new(std::cell::RefCell::new(accessor_desc));

        if let Some(def_fn2) = obj_ref
            .borrow()
            .get("defineProperty")
            .as_native_function()
        {
            let _ = def_fn2(
                &mut vm,
                vec![
                    target_val.clone(),
                    JSValue::from_string("g".to_string()),
                    JSValue::from_object(accessor_desc_rc.clone()),
                ],
            )
            .unwrap();

            if let Some(gopd2) = obj_ref
                .borrow()
                .get("getOwnPropertyDescriptor")
                .as_native_function()
            {
                let got2 = gopd2(
                    &mut vm,
                    vec![target_val.clone(), JSValue::from_string("g".to_string())],
                )
                .unwrap();
                match got2.as_object() {
                    Some(desc_obj_ref) => {
                        let getv = desc_obj_ref.borrow().get("get");
                        assert!(
                            getv.is_native_function(),
                            "getter not preserved in descriptor"
                        );
                    }
                    _ => panic!("getOwnPropertyDescriptor returned unexpected value for accessor"),
                }
            } else {
                panic!("getOwnPropertyDescriptor not callable (2)");
            }
        } else {
            panic!("defineProperty not callable (2)");
        }
    } else {
        panic!("Object constructor not found");
    }
}

#[test]
fn test_function_call_apply() {
    let mut vm = VM::new();
    let global = vm.global_object.clone();

    // prepare a native function to act as callable
    let native_fn = JSValue::from_native_function(|_vm, args| {
        // return thisArg as string for testing
        if !args.is_empty() {
            let this_arg = &args[0];
            return Ok(this_arg.clone());
        }
        Ok(JSValue::undefined())
    });

    // put function into a dummy container object so we can call via prototype.call
    let mut fn_holder = pixi_byte::value::jsobject::JSObject::new();
    fn_holder.set("fn".to_string(), native_fn.clone());
    let fn_holder_rc = std::rc::Rc::new(std::cell::RefCell::new(fn_holder));
    let _fn_holder_val = JSValue::from_object(fn_holder_rc.clone());

    // retrieve Function.prototype.call
    let function_ctor = global.borrow().get("Function");
    if let Some(functor) = function_ctor.as_object() {
        if let Some(proto) = functor.borrow().get("prototype").as_object() {
            if let Some(call_fn) = proto.borrow().get("call").as_native_function() {
                // invoke call: call(fn, thisArg, ...args)
                let this_arg = JSValue::from_string("hello".to_string());
                let res = call_fn(&mut vm, vec![native_fn.clone(), this_arg.clone()]).unwrap();
                match res.as_string() {
                    Some(s) => assert_eq!(s, "hello"),
                    _ => panic!("Function.prototype.call returned unexpected value"),
                }
            } else {
                panic!("call not found");
            }

            if let Some(apply_fn) = proto.borrow().get("apply").as_native_function() {
                // create args array
                let mut arr = pixi_byte::value::jsarray::JSArray::new();
                arr.push(JSValue::from_string("world".to_string()));
                let arr_val = arr.to_object();
                let res2 = apply_fn(
                    &mut vm,
                    vec![
                        native_fn.clone(),
                        JSValue::from_string("world".to_string()),
                        arr_val,
                    ],
                )
                .unwrap();
                match res2.as_string() {
                    Some(s) => assert_eq!(s, "world"),
                    _ => panic!("Function.prototype.apply returned unexpected value"),
                }
            } else {
                panic!("apply not found");
            }
        } else {
            panic!("Function.prototype missing");
        }
    } else {
        panic!("Function constructor missing");
    }
}
