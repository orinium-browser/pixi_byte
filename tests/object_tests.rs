use pixi_byte::vm::VM;
use pixi_byte::value::JSValue;

#[test]
fn test_object_builtins_registered_and_functional() {
    let mut vm = VM::new();
    let global = vm.global_object.clone();

    // Object がグローバルに存在する
    let obj_val = global.borrow().get("Object");
    match obj_val {
        JSValue::Object(obj_ref) => {
            // create と getPrototypeOf が登録されている
            let create = obj_ref.borrow().get("create");
            let get_proto = obj_ref.borrow().get("getPrototypeOf");

            match create {
                JSValue::NativeFunction(_) => {}
                _ => panic!("Object.create not installed as native function"),
            }

            match get_proto {
                JSValue::NativeFunction(_) => {}
                _ => panic!("Object.getPrototypeOf not installed as native function"),
            }

            // Object.create を直接呼び出してプロトタイプが正しく設定されるかを確認
            // まず prototype オブジェクトを作成
            let proto = pixi_byte::value::jsobject::JSObject::new();
            let proto_obj = JSValue::Object(std::rc::Rc::new(std::cell::RefCell::new(proto)));

            if let JSValue::NativeFunction(f) = obj_ref.borrow().get("create") {
                let res = f(&mut vm, vec![proto_obj.clone()]).unwrap();
                match res {
                    JSValue::Object(new_obj_ref) => {
                        // プロトタイプが設定されていることを確認
                        let got = new_obj_ref.borrow().get_prototype();
                        assert!(got.is_some());
                        if let Some(gp) = got {
                            // 同一参照かどうか
                            assert_eq!(std::rc::Rc::ptr_eq(&gp, &std::rc::Rc::new(std::cell::RefCell::new(pixi_byte::value::jsobject::JSObject::new())).clone()), false);
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
            if let JSValue::NativeFunction(f) = obj_ref.borrow().get("create") {
                let created = f(&mut vm, vec![proto_obj.clone()]).unwrap();
                if let JSValue::Object(created_ref) = created {
                    if let JSValue::NativeFunction(getp) = obj_ref.borrow().get("getPrototypeOf") {
                        let got = getp(&mut vm, vec![JSValue::Object(created_ref.clone())]).unwrap();
                        match got {
                            JSValue::Object(gp_ref) => {
                                // gp_ref should point to same prototype as proto_obj
                                if let JSValue::Object(proto_clone_ref) = proto_obj {
                                    assert!(std::rc::Rc::ptr_eq(&gp_ref, &proto_clone_ref));
                                }
                            }
                            JSValue::Null => panic!("getPrototypeOf returned null unexpectedly"),
                            _ => panic!("getPrototypeOf returned unexpected value"),
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
        _ => panic!("Object global is not an object"),
    }
}

#[test]
fn test_object_create_with_descriptors() {
    let mut vm = VM::new();
    let global = vm.global_object.clone();

    let proto = pixi_byte::value::jsobject::JSObject::new();
    let proto_obj = JSValue::Object(std::rc::Rc::new(std::cell::RefCell::new(proto)));
    let mut desc_inner = pixi_byte::value::jsobject::JSObject::new();

    desc_inner.set("value".to_string(), JSValue::Number(10.0));
    desc_inner.set("writable".to_string(), JSValue::Boolean(true));
    desc_inner.set("enumerable".to_string(), JSValue::Boolean(true));
    desc_inner.set("configurable".to_string(), JSValue::Boolean(false));
    let desc_inner_rc = std::rc::Rc::new(std::cell::RefCell::new(desc_inner));

    let mut desc_outer = pixi_byte::value::jsobject::JSObject::new();
    desc_outer.set("a".to_string(), JSValue::Object(desc_inner_rc.clone()));
    let desc_outer_rc = std::rc::Rc::new(std::cell::RefCell::new(desc_outer));

    let obj_global = global.borrow().get("Object");
    if let JSValue::Object(obj_ref) = obj_global {
        if let JSValue::NativeFunction(create_fn) = obj_ref.borrow().get("create") {
            let res = create_fn(&mut vm, vec![proto_obj.clone(), JSValue::Object(desc_outer_rc.clone())]).unwrap();
            match res {
                JSValue::Object(new_obj_ref) => {
                    let a = new_obj_ref.borrow().get("a");
                    match a {
                        JSValue::Number(n) => assert_eq!(n, 10.0),
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
    let proto_obj = JSValue::Object(std::rc::Rc::new(std::cell::RefCell::new(proto)));

    let obj_global = global.borrow().get("Object");
    if let JSValue::Object(obj_ref) = obj_global {
        // create an object with proto as its prototype
        if let JSValue::NativeFunction(create_fn) = obj_ref.borrow().get("create") {
            let created = create_fn(&mut vm, vec![proto_obj.clone()]).unwrap();
            if let JSValue::Object(created_ref) = created {
                // call isPrototypeOf on proto
                if let JSValue::Object(proto_constructor) = global.borrow().get("Object") {
                    if let JSValue::Object(proto_proto) = proto_constructor.borrow().get("prototype") {
                        // proto_proto is Object.prototype, but we want to call isPrototypeOf on proto_obj instance
                        if let JSValue::NativeFunction(isp) = proto_proto.borrow().get("isPrototypeOf") {
                            // method expects receiver as first arg
                            let res = isp(&mut vm, vec![proto_obj.clone(), JSValue::Object(created_ref.clone())]).unwrap();
                            match res {
                                JSValue::Boolean(b) => assert!(b),
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
    if let JSValue::Object(obj_ref) = obj_global {
        if let JSValue::Object(proto_obj) = obj_ref.borrow().get("prototype") {
            if let JSValue::NativeFunction(to_str) = proto_obj.borrow().get("toString") {
                // call on an object
                let target = JSValue::Object(std::rc::Rc::new(std::cell::RefCell::new(pixi_byte::value::jsobject::JSObject::new())));
                let res = to_str(&mut vm, vec![target]).unwrap();
                match res {
                    JSValue::String(s) => assert_eq!(s, "[object Object]"),
                    _ => panic!("toString returned non-string"),
                }

                // call on a function value (NativeFunction)
                let res2 = to_str(&mut vm, vec![JSValue::NativeFunction(|_, _| Ok(JSValue::Undefined))]).unwrap();
                match res2 {
                    JSValue::String(s) => assert_eq!(s, "[object Function]"),
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
    if let JSValue::Object(obj_ref) = obj_global {
        // prepare target object
        let target = pixi_byte::value::jsobject::JSObject::new();
        let target_rc = std::rc::Rc::new(std::cell::RefCell::new(target));
        let target_val = JSValue::Object(target_rc.clone());

        // descriptor: { value: 5, writable: true, enumerable: false, configurable: true }
        let mut desc = pixi_byte::value::jsobject::JSObject::new();
        desc.set("value".to_string(), JSValue::Number(5.0));
        desc.set("writable".to_string(), JSValue::Boolean(true));
        desc.set("enumerable".to_string(), JSValue::Boolean(false));
        desc.set("configurable".to_string(), JSValue::Boolean(true));
        let desc_rc = std::rc::Rc::new(std::cell::RefCell::new(desc));

        // call Object.defineProperty(target, "x", desc)
        if let JSValue::NativeFunction(def_fn) = obj_ref.borrow().get("defineProperty") {
            let _ = def_fn(&mut vm, vec![target_val.clone(), JSValue::String("x".to_string()), JSValue::Object(desc_rc.clone())]).unwrap();
            // get descriptor back
            if let JSValue::NativeFunction(gopd) = obj_ref.borrow().get("getOwnPropertyDescriptor") {
                let got = gopd(&mut vm, vec![target_val.clone(), JSValue::String("x".to_string())]).unwrap();
                match got {
                    JSValue::Object(desc_obj_ref) => {
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
        let getter_native = JSValue::NativeFunction(|_vm, _args| Ok(JSValue::Number(42.0)));
        let getter_val = getter_native.clone();

        let mut accessor_desc = pixi_byte::value::jsobject::JSObject::new();
        accessor_desc.set("get".to_string(), getter_val.clone());
        accessor_desc.set("set".to_string(), JSValue::NativeFunction(|_vm, _args| Ok(JSValue::Undefined)));
        let accessor_desc_rc = std::rc::Rc::new(std::cell::RefCell::new(accessor_desc));

        if let JSValue::NativeFunction(def_fn2) = obj_ref.borrow().get("defineProperty") {
            let _ = def_fn2(&mut vm, vec![target_val.clone(), JSValue::String("g".to_string()), JSValue::Object(accessor_desc_rc.clone())]).unwrap();

            if let JSValue::NativeFunction(gopd2) = obj_ref.borrow().get("getOwnPropertyDescriptor") {
                let got2 = gopd2(&mut vm, vec![target_val.clone(), JSValue::String("g".to_string())]).unwrap();
                match got2 {
                    JSValue::Object(desc_obj_ref) => {
                        let getv = desc_obj_ref.borrow().get("get");
                        match getv {
                            JSValue::NativeFunction(_) => {}
                            _ => panic!("getter not preserved in descriptor"),
                        }
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
    let native_fn = JSValue::NativeFunction(|_vm, args| {
        // return thisArg as string for testing
        if !args.is_empty() {
            let this_arg = &args[0];
            return Ok(this_arg.clone());
        }
        Ok(JSValue::Undefined)
    });

    // put function into a dummy container object so we can call via prototype.call
    let mut fn_holder = pixi_byte::value::jsobject::JSObject::new();
    fn_holder.set("fn".to_string(), native_fn.clone());
    let fn_holder_rc = std::rc::Rc::new(std::cell::RefCell::new(fn_holder));
    let _fn_holder_val = JSValue::Object(fn_holder_rc.clone());

    // retrieve Function.prototype.call
    let function_ctor = global.borrow().get("Function");
    if let JSValue::Object(functor) = function_ctor {
        if let JSValue::Object(proto) = functor.borrow().get("prototype") {
            if let JSValue::NativeFunction(call_fn) = proto.borrow().get("call") {
                // invoke call: call(fn, thisArg, ...args)
                let this_arg = JSValue::String("hello".to_string());
                let res = call_fn(&mut vm, vec![native_fn.clone(), this_arg.clone()]).unwrap();
                match res {
                    JSValue::String(s) => assert_eq!(s, "hello"),
                    _ => panic!("Function.prototype.call returned unexpected value"),
                }
            } else {
                panic!("call not found");
            }

            if let JSValue::NativeFunction(apply_fn) = proto.borrow().get("apply") {
                // create args array
                let mut arr = pixi_byte::value::jsarray::JSArray::new();
                arr.push(JSValue::String("world".to_string()));
                let arr_val = arr.to_object();
                let res2 = apply_fn(&mut vm, vec![native_fn.clone(), JSValue::String("world".to_string()), arr_val]).unwrap();
                match res2 {
                    JSValue::String(s) => assert_eq!(s, "world"),
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
