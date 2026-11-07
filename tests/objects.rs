use pixi_byte::vm::VM;
use pixi_byte::{JSEngine, JSValue};
use std::cell::RefCell;
use std::rc::Rc;

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
            if let Some(getp) = obj_ref.borrow().get("getPrototypeOf").as_native_function() {
                let got = getp(&mut vm, vec![JSValue::from_object(created_ref.clone())]).unwrap();
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
                vec![
                    proto_obj.clone(),
                    JSValue::from_object(desc_outer_rc.clone()),
                ],
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
                    if let Some(proto_proto) =
                        proto_constructor.borrow().get("prototype").as_object()
                    {
                        // proto_proto is Object.prototype, but we want to call isPrototypeOf on proto_obj instance
                        if let Some(isp) = proto_proto
                            .borrow()
                            .get("isPrototypeOf")
                            .as_native_function()
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
                    vec![JSValue::from_native_function(|_, _| {
                        Ok(JSValue::undefined())
                    })],
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
        if let Some(def_fn) = obj_ref.borrow().get("defineProperty").as_native_function() {
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
        let getter_native =
            JSValue::from_native_function(|_vm, _args| Ok(JSValue::from_number(42.0)));
        let getter_val = getter_native.clone();

        let mut accessor_desc = pixi_byte::value::jsobject::JSObject::new();
        accessor_desc.set("get".to_string(), getter_val.clone());
        accessor_desc.set(
            "set".to_string(),
            JSValue::from_native_function(|_vm, _args| Ok(JSValue::undefined())),
        );
        let accessor_desc_rc = std::rc::Rc::new(std::cell::RefCell::new(accessor_desc));

        if let Some(def_fn2) = obj_ref.borrow().get("defineProperty").as_native_function() {
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

fn inherited_accessors_receive_the_original_receiver() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const prototype = {};
            Object.defineProperty(prototype, "value", {
                get: function () { return this._value; },
                set: function (next) { this._value = next; }
            });
            const target = Object.create(prototype);
            target.value = "tracked";
            target.value;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_string("tracked".to_string()));
}

fn getter_receives_this() {
    let mut vm = VM::new();

    // prepare target object with internal property 'x'
    let mut target = pixi_byte::value::jsobject::JSObject::new();
    target.set("x".to_string(), JSValue::from_string("hello".to_string()));
    let target_rc = std::rc::Rc::new(std::cell::RefCell::new(target));

    // getter native function returns receiver.x
    let getter_native = JSValue::from_native_function(|_vm, args| {
        if args.is_empty() {
            return Ok(JSValue::undefined());
        }
        match args[0].as_object() {
            Some(obj_ref) => Ok(obj_ref.borrow().get("x")),
            _ => Ok(JSValue::undefined()),
        }
    });

    // descriptor object { get: getter_native }
    let mut desc_inner = pixi_byte::value::jsobject::JSObject::new();
    desc_inner.set("get".to_string(), getter_native.clone());
    let desc_inner_rc = std::rc::Rc::new(std::cell::RefCell::new(desc_inner));

    // call Object.defineProperty(target, "a", desc)
    // use builtins directly
    let global = vm.global_object.clone();
    let obj_global = global.borrow().get("Object");
    if let Some(obj_ref) = obj_global.as_object() {
        let define_property = obj_ref.borrow().get("defineProperty");
        if define_property.is_native_function() {
            let _ = vm
                .call(
                    define_property,
                    JSValue::from_object(obj_ref),
                    vec![
                        JSValue::from_object(target_rc.clone()),
                        JSValue::from_string("a".to_string()),
                        JSValue::from_object(desc_inner_rc.clone()),
                    ],
                )
                .unwrap();
        } else {
            panic!("defineProperty not callable");
        }
    } else {
        panic!("Object constructor not found");
    }

    // Build bytecode chunk to perform GetProperty on the object
    let mut chunk = pixi_byte::compiler::BytecodeChunk::new();
    let idx_obj = chunk.add_constant(JSValue::from_object(target_rc.clone()));
    let idx_key = chunk.add_constant(JSValue::from_string("a".to_string()));
    chunk.emit(pixi_byte::compiler::Opcode::LoadConst(idx_obj));
    chunk.emit(pixi_byte::compiler::Opcode::LoadConst(idx_key));
    chunk.emit(pixi_byte::compiler::Opcode::GetProperty);
    chunk.emit(pixi_byte::compiler::Opcode::Return);

    let res = vm.execute(&chunk).unwrap();
    match res.as_string() {
        Some(s) => assert_eq!(s, "hello"),
        _ => panic!("getter did not return expected string"),
    }
}

fn setter_updates_internal_state() {
    let mut vm = VM::new();

    // prepare target object
    let target = pixi_byte::value::jsobject::JSObject::new();
    let target_rc = std::rc::Rc::new(std::cell::RefCell::new(target));

    // setter native function stores value into receiver.v
    let setter_native = JSValue::from_native_function(|_vm, args| {
        if args.len() < 2 {
            return Ok(JSValue::undefined());
        }
        match args[0].as_object() {
            Some(obj_ref) => {
                let val = args[1].clone();
                obj_ref.borrow_mut().set("v".to_string(), val);
                Ok(JSValue::undefined())
            }
            _ => Ok(JSValue::undefined()),
        }
    });

    // descriptor object { set: setter_native }
    let mut desc_inner = pixi_byte::value::jsobject::JSObject::new();
    desc_inner.set("set".to_string(), setter_native.clone());
    let desc_inner_rc = std::rc::Rc::new(std::cell::RefCell::new(desc_inner));

    // define property 'a' with setter
    let global = vm.global_object.clone();
    let obj_global = global.borrow().get("Object");
    if let Some(obj_ref) = obj_global.as_object() {
        let define_property = obj_ref.borrow().get("defineProperty");
        if define_property.is_native_function() {
            let _ = vm
                .call(
                    define_property,
                    JSValue::from_object(obj_ref),
                    vec![
                        JSValue::from_object(target_rc.clone()),
                        JSValue::from_string("a".to_string()),
                        JSValue::from_object(desc_inner_rc.clone()),
                    ],
                )
                .unwrap();
        } else {
            panic!("defineProperty not callable");
        }
    } else {
        panic!("Object constructor not found");
    }

    // Use VM to perform assignment target.a = 42 via opcodes: push obj, key, value, SetProperty
    let mut chunk = pixi_byte::compiler::BytecodeChunk::new();
    let idx_obj = chunk.add_constant(JSValue::from_object(target_rc.clone()));
    let idx_key = chunk.add_constant(JSValue::from_string("a".to_string()));
    let idx_val = chunk.add_constant(JSValue::from_number(42.0));
    chunk.emit(pixi_byte::compiler::Opcode::LoadConst(idx_obj));
    chunk.emit(pixi_byte::compiler::Opcode::LoadConst(idx_key));
    chunk.emit(pixi_byte::compiler::Opcode::LoadConst(idx_val));
    chunk.emit(pixi_byte::compiler::Opcode::SetProperty);
    chunk.emit(pixi_byte::compiler::Opcode::Return);

    let _ = vm.execute(&chunk).unwrap();

    // internal property 'v' should be set to 42
    let v = target_rc.borrow().get("v");
    match v.as_number() {
        Some(n) => assert_eq!(n, 42.0),
        _ => panic!("setter did not set internal property"),
    }
}

fn accessor_descriptor_enumeration() {
    // descriptor with enumerable true should appear in keys()
    let target = pixi_byte::value::jsobject::JSObject::new();
    let target_rc = std::rc::Rc::new(std::cell::RefCell::new(target));

    let getter_native = JSValue::from_native_function(|_vm, _args| Ok(JSValue::undefined()));
    let mut desc_inner = pixi_byte::value::jsobject::JSObject::new();
    desc_inner.set("get".to_string(), getter_native.clone());
    desc_inner.set("enumerable".to_string(), JSValue::from_bool(true));
    let desc_inner_rc = std::rc::Rc::new(std::cell::RefCell::new(desc_inner));

    // define property 'a' with enumerable accessor
    let mut target_mut = target_rc.borrow_mut();
    target_mut.define_property_descriptor("a".to_string(), desc_inner_rc.clone());
    drop(target_mut);

    let keys = target_rc.borrow().keys();
    assert!(keys.contains(&"a".to_string()));
}

fn host_property_hooks_handle_missing_properties() {
    let mut vm = VM::new();

    let mut target = pixi_byte::value::jsobject::JSObject::new();
    target.set(
        pixi_byte::value::jsobject::HOST_GET_PROPERTY.to_string(),
        JSValue::from_native_function(|_vm, args| {
            Ok(JSValue::from_string(format!(
                "read:{}",
                args[1].to_string()
            )))
        }),
    );
    target.set(
        pixi_byte::value::jsobject::HOST_SET_PROPERTY.to_string(),
        JSValue::from_native_function(|_vm, args| {
            if let Some(receiver) = args[0].as_object() {
                receiver
                    .borrow_mut()
                    .set("observed_key".to_string(), args[1].clone());
                receiver
                    .borrow_mut()
                    .set("observed_value".to_string(), args[2].clone());
            }
            Ok(JSValue::undefined())
        }),
    );
    let target = std::rc::Rc::new(std::cell::RefCell::new(target));

    let mut write = pixi_byte::compiler::BytecodeChunk::new();
    let object = write.add_constant(JSValue::from_object(target.clone()));
    let key = write.add_constant(JSValue::from_string("backgroundColor".to_string()));
    let value = write.add_constant(JSValue::from_string("red".to_string()));
    write.emit(pixi_byte::compiler::Opcode::LoadConst(object));
    write.emit(pixi_byte::compiler::Opcode::LoadConst(key));
    write.emit(pixi_byte::compiler::Opcode::LoadConst(value));
    write.emit(pixi_byte::compiler::Opcode::SetProperty);
    write.emit(pixi_byte::compiler::Opcode::Return);
    vm.execute(&write).unwrap();

    assert_eq!(
        target.borrow().get("observed_key").to_string(),
        "backgroundColor"
    );
    assert_eq!(target.borrow().get("observed_value").to_string(), "red");

    let mut read = pixi_byte::compiler::BytecodeChunk::new();
    let object = read.add_constant(JSValue::from_object(target));
    let key = read.add_constant(JSValue::from_string("color".to_string()));
    read.emit(pixi_byte::compiler::Opcode::LoadConst(object));
    read.emit(pixi_byte::compiler::Opcode::LoadConst(key));
    read.emit(pixi_byte::compiler::Opcode::GetProperty);
    read.emit(pixi_byte::compiler::Opcode::Return);

    assert_eq!(vm.execute(&read).unwrap().to_string(), "read:color");
}

fn method_call_resolves_an_accessor_getter() {
    let mut engine = pixi_byte::JSEngine::new();
    let result = engine
        .eval(
            r#"
            const object = {
                value: 7,
                get method() {
                    return function () { return this.value; };
                }
            };
            object.method();
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(7.0));
}

fn test_array_literal() {
    let mut engine = JSEngine::new();

    // 基本的な配列リテラル
    let result = engine.eval("[1, 2, 3]").unwrap();
    assert!(result.is_object(), "Expected object for array literal");
}

fn test_array_literal_empty() {
    let mut engine = JSEngine::new();
    let result = engine.eval("[]").unwrap();
    assert!(result.is_object(), "Expected object for empty array");
}

fn test_object_literal() {
    let mut engine = JSEngine::new();

    // 基本的なオブジェクトリテラル
    let result = engine.eval(r#"({ name: "Alice", age: 30 })"#).unwrap();
    assert!(result.is_object(), "Expected object for object literal");
}

fn test_object_literal_empty() {
    let mut engine = JSEngine::new();
    let result = engine.eval("({})").unwrap();
    assert!(result.is_object(), "Expected object for empty object");
}

fn test_member_access_dot() {
    let mut engine = JSEngine::new();

    // ドット記法でのプロパティアクセス
    let result = engine
        .eval(
            r#"
        let obj = { x: 10 };
        obj.x
    "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_number(10.0));
}

fn test_member_access_bracket() {
    let mut engine = JSEngine::new();

    // ブラケット記法でのプロパティアクセス
    let result = engine
        .eval(
            r#"
        let obj = { name: "Bob" };
        obj["name"]
    "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_string("Bob".to_string()));
}

fn test_member_assignment() {
    let mut engine = JSEngine::new();

    // プロパティへの代入
    let result = engine
        .eval(
            r#"
        let obj = {};
        obj.value = 42;
        obj.value
    "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_number(42.0));
}

fn test_array_index_access() {
    let mut engine = JSEngine::new();

    // 配列のインデックスアクセス
    let result = engine
        .eval(
            r#"
        let arr = [10, 20, 30];
        arr[1]
    "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_number(20.0));
}

fn test_nested_object() {
    let mut engine = JSEngine::new();

    // ネストされたオブジェクト
    let result = engine
        .eval(
            r#"
        let obj = { inner: { value: 100 } };
        obj.inner.value
    "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_number(100.0));
}

fn numeric_object_keys_use_their_canonical_string_value() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(r#"const modules = {93e3: "locale", 1.0: "one"}; modules[93000] + ":" + modules[1];"#)
        .unwrap();
    assert_eq!(result, JSValue::from_string("locale:one".to_string()));
}

fn object_literals_accept_string_number_and_keyword_keys() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = { "=": "equals", 1: "one", default: "fallback" };
            values["="] + values[1] + values.default;
            "#,
        )
        .unwrap();
    assert_eq!(
        result,
        JSValue::from_string("equalsonefallback".to_string())
    );
}

fn object_literals_accept_shorthand_properties() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const value = 42;
            const target = { value };
            target.value;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(42.0));
}

fn object_literals_support_getters_and_setters() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const target = {
                _value: 1,
                get value() { return this._value; },
                set value(next) { this._value = next * 2; }
            };
            target.value = 6;
            target.value;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(12.0));
}

fn object_literals_support_method_shorthand() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const target = {
                value: 4,
                read(extra = 1) { return this.value + extra; }
            };
            target.read(2);
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(6.0));
}

fn methods_named_get_and_set_are_not_parsed_as_accessors() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const target = {
                get(value) { return value + 1; },
                set(value) { return value + 2; }
            };
            target.get(2) + target.set(3);
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(8.0));
}

fn object_literals_support_spread_and_computed_keys() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const source = { first: 2 };
            const key = "second";
            const target = { ...source, [key]: 3 };
            target.first + target.second;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(5.0));
}

fn object_literals_support_computed_method_shorthand() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const method = "read";
            const target = {
                value: 4,
                [method](extra = 1) { return this.value + extra; }
            };
            target.read(2);
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(6.0));
}

fn computed_method_shorthand_accepts_symbol_keys() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const key = Symbol.iterator;
            const target = {
                [key]: function () { return "data"; },
                [key]() { return this; }
            };
            target[key]() === target;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn test_prevent_extensions_and_is_extensible() {
    let mut vm = VM::new();
    let global = vm.global_object.clone();

    let obj = pixi_byte::value::jsobject::JSObject::new();
    let rc = std::rc::Rc::new(std::cell::RefCell::new(obj));
    let val = JSValue::from_object(rc.clone());

    let obj_global = global.borrow().get("Object");
    if let Some(obj_ref) = obj_global.as_object() {
        if let Some(prevent) = obj_ref
            .borrow()
            .get("preventExtensions")
            .as_native_function()
        {
            let _ = prevent(&mut vm, vec![val.clone()]).unwrap();
        } else {
            panic!("preventExtensions not found");
        }
        if let Some(is_ext) = obj_ref.borrow().get("isExtensible").as_native_function() {
            let res = is_ext(&mut vm, vec![val.clone()]).unwrap();
            match res.as_boolean() {
                Some(b) => assert_eq!(b, false),
                _ => panic!("isExtensible returned non-boolean"),
            }
        } else {
            panic!("isExtensible not found");
        }
    } else {
        panic!("Object constructor missing");
    }
}

fn test_seal_prevents_adding_and_makes_non_configurable() {
    let mut vm = VM::new();
    let global = vm.global_object.clone();

    let mut obj = pixi_byte::value::jsobject::JSObject::new();
    obj.set("a".to_string(), JSValue::from_number(1.0));
    let rc = std::rc::Rc::new(std::cell::RefCell::new(obj));
    let val = JSValue::from_object(rc.clone());

    let obj_global = global.borrow().get("Object");
    if let Some(obj_ref) = obj_global.as_object() {
        if let Some(seal) = obj_ref.borrow().get("seal").as_native_function() {
            let _ = seal(&mut vm, vec![val.clone()]).unwrap();
        } else {
            panic!("seal not found");
        }
    } else {
        panic!("Object constructor missing");
    }

    // Attempt to add new property should fail due to non-extensible
    let added = rc
        .borrow_mut()
        .set("b".to_string(), JSValue::from_number(2.0));
    assert_eq!(added, false);

    // Existing property should become non-configurable
    let desc = rc.borrow().get_property_descriptor("a");
    assert!(desc.is_some());
    if let Some(d) = desc {
        assert_eq!(d.configurable, false);
    }
}

fn test_freeze_makes_non_writable_and_non_configurable() {
    let mut vm = VM::new();
    let global = vm.global_object.clone();

    let mut obj = pixi_byte::value::jsobject::JSObject::new();
    obj.set("x".to_string(), JSValue::from_number(10.0));
    let rc = std::rc::Rc::new(std::cell::RefCell::new(obj));
    let val = JSValue::from_object(rc.clone());

    let obj_global = global.borrow().get("Object");
    if let Some(obj_ref) = obj_global.as_object() {
        if let Some(freeze) = obj_ref.borrow().get("freeze").as_native_function() {
            let _ = freeze(&mut vm, vec![val.clone()]).unwrap();
        } else {
            panic!("freeze not found");
        }
    } else {
        panic!("Object constructor missing");
    }

    // Attempt to change existing property should fail (not writable)
    let changed = rc
        .borrow_mut()
        .set("x".to_string(), JSValue::from_number(20.0));
    assert_eq!(changed, false);

    // Property should be non-configurable
    let desc = rc.borrow().get_property_descriptor("x");
    assert!(desc.is_some());
    if let Some(d) = desc {
        assert_eq!(d.configurable, false);
        assert_eq!(d.writable, false);
    }
}

fn object_keys_returns_enumerable_own_keys() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const prototype = { inherited: true };
            const target = Object.create(prototype);
            target.first = 1;
            target.second = 2;
            const keys = Object.keys(target);
            keys.length === 2 && "0" in keys && "1" in keys && keys[0] !== "inherited";
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_bool(true));
}

fn object_keys_boxes_primitives_and_accepts_callable_values() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            Object.keys("abc").join("") + ":" +
                Object.keys(1).length + ":" +
                Object.keys(function () {}).length + ":" +
                Object.keys(Array.isArray).length;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("012:0:0:0".to_string()));
}

fn object_assign_copies_sources_from_left_to_right() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const target = { preserved: true };
            const returned = Object.assign(target, { value: 1 }, null, { value: 2 });
            returned === target && target.preserved && target.value === 2;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_bool(true));
}

fn extracted_object_assign_uses_its_first_argument_as_the_target() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const assign = Object.assign;
            const target = {};
            const returned = assign(target, { value: 42 });
            returned === target && target.value === 42 && typeof globalThis.value === "undefined";
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_bool(true));
}

fn object_is_uses_same_value_semantics() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            "Object.is(NaN, NaN) && !Object.is(0, -0) && Object.is(-0, -0) && Object.is('a', 'a');",
        )
        .unwrap();

    assert_eq!(result, JSValue::from_bool(true));
}

fn object_static_methods_do_not_treat_the_constructor_as_an_argument() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const prototype = { inherited: true };
            const target = Object.create(prototype);
            Object.defineProperty(target, "own", {
                value: 7,
                enumerable: true,
                configurable: true,
                writable: true
            });
            Object.getPrototypeOf(target) === prototype && target.own === 7;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_bool(true));
}

fn get_prototype_of_accepts_functions_and_boxable_primitives() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function value() {}
            Object.getPrototypeOf(value) === Function.prototype &&
                Object.getPrototypeOf("text") === String.prototype &&
                Object.getPrototypeOf(1) === Number.prototype;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn ordinary_and_explicit_null_prototypes_are_distinct() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const ordinary = {};
            const bare = Object.create(null);
            Object.getPrototypeOf(ordinary) === Object.prototype &&
                Object.getPrototypeOf(Object.getPrototypeOf(ordinary)) === null &&
                Object.getPrototypeOf(bare) === null &&
                bare.toString === undefined;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn get_own_property_names_includes_non_enumerable_properties() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const target = {};
            Object.defineProperty(target, "hidden", { value: 1 });
            const names = Object.getOwnPropertyNames(target);
            names.length + ":" + names[0] + ":" +
                Object.getOwnPropertySymbols(target).length;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("1:hidden:0".to_string()));
}

fn get_own_property_names_hides_engine_callable_slots() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function Component() {}
            const names = Object.getOwnPropertyNames(Component);
            names.includes("name") && names.includes("length") &&
                names.includes("prototype") && !names.includes("__call__") &&
                !names.includes("__construct__");
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn property_descriptor_methods_accept_functions() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function target() {}
            const returned = Object.defineProperty(target, "hidden", { value: 7 });
            returned === target &&
                Object.getOwnPropertyDescriptor(target, "hidden").value === 7;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn bound_set_prototype_of_forwards_arguments() {
    let mut engine = pixi_byte::JSEngine::new();
    let result = engine
        .eval(
            r#"
            function inherit(value, base) {
                return inherit = Object.setPrototypeOf.bind(), inherit(value, base);
            }
            function Base() {}
            function Derived() {}
            Base.marker = 42;
            inherit(Derived, Base);
            Derived.marker === 42;
            "#,
        )
        .unwrap();
    assert_eq!(result, pixi_byte::JSValue::from_bool(true));
}

fn object_assign_accepts_function_targets_and_sources() {
    let mut engine = pixi_byte::JSEngine::new();
    let result = engine
        .eval(
            r#"
            function target() {}
            function source() {}
            source.answer = 42;
            Object.assign(target, source) === target && target.answer === 42;
            "#,
        )
        .unwrap();
    assert_eq!(result, pixi_byte::JSValue::from_bool(true));
}

fn entries_values_and_from_entries_round_trip_enumerable_properties() {
    let mut engine = pixi_byte::JSEngine::new();
    let result = engine
        .eval(
            r#"
            const source = { first: 1, second: 2 };
            const entries = Object.entries(source);
            const rebuilt = Object.fromEntries(entries.map(([key, value]) => [key, value * 2]));
            Object.values(source).length === 2
                && Object.values(source).includes(1)
                && Object.values(source).includes(2)
                && rebuilt.first === 2
                && rebuilt.second === 4;
            "#,
        )
        .unwrap();
    assert_eq!(result, pixi_byte::JSValue::from_bool(true));
}

fn define_property_accepts_the_constructor_as_its_own_target() {
    let mut engine = pixi_byte::JSEngine::new();
    let result = engine
        .eval(
            r#"
            Object.defineProperty(Object, "ownSentinel", { value: 5, configurable: true });
            Object.ownSentinel;
            "#,
        )
        .unwrap();
    assert_eq!(result, pixi_byte::JSValue::from_number(5.0));
}

fn object_statics_drop_the_receiver_when_called_through_a_wrapper() {
    let mut engine = pixi_byte::JSEngine::new();
    let result = engine
        .eval(
            r#"
            const key = Symbol("state");
            const helper = { defineProperty: Object.defineProperty };
            const target = function () {};
            helper.defineProperty(target, key, { value: 7, configurable: true });
            target[key] === 7;
            "#,
        )
        .unwrap();
    assert_eq!(result, pixi_byte::JSValue::from_bool(true));
}

fn native_functions_carry_own_properties() {
    let mut engine = pixi_byte::JSEngine::new();
    let result = engine
        .eval(
            r#"
            Object.defineProperty(parseInt, "sentinel", { value: 3, configurable: true });
            const defined = parseInt.sentinel;
            parseInt.extra = 4;
            defined === 3
                && parseInt.extra === 4
                && Object.prototype.hasOwnProperty.call(parseInt, "extra");
            "#,
        )
        .unwrap();
    assert_eq!(result, pixi_byte::JSValue::from_bool(true));
}

fn eval(source: &str) -> JSValue {
    let mut engine = JSEngine::new();
    engine.eval(source).unwrap()
}

fn string_primitive_to_object_exposes_length_and_indices() {
    let result = eval(
        r#"
        const desc = Object.getOwnPropertyDescriptor("abc", "length");
        JSON.stringify(desc) + "|" +
            JSON.stringify(Object.getOwnPropertyDescriptor("abc", 1).value);
        "#,
    );
    assert_eq!(
        result.to_console_string(),
        r#"{"value":3,"writable":false,"enumerable":false,"configurable":false}|"b""#
    );
}

fn string_indices_and_length_are_not_enumerable() {
    let result = eval(
        r#"
        const wrapper = new Object("ab");
        Object.keys(wrapper).join(",");
        "#,
    );
    assert_eq!(result.to_console_string(), "");
}

fn number_and_boolean_primitives_have_empty_descriptors() {
    let result = eval(
        r#"
        JSON.stringify(Object.getOwnPropertyDescriptor(42, "x")) + "|" +
        JSON.stringify(Object.getOwnPropertyDescriptor(true, "x"));
        "#,
    );
    assert_eq!(result.to_console_string(), "undefined|undefined");
}

fn null_and_undefined_receivers_throw() {
    // このエンジンでは Rust ネイティブから送出された TypeError は
    // 文字列化された値として catch 節へ渡されるため、文字列内容で検証する
    let result = eval(
        r#"
        let message = "";
        try { Object.getOwnPropertyDescriptor(null, "x"); }
        catch (e) { message = String(e); }
        message;
        "#,
    );
    assert!(
        result
            .to_console_string()
            .contains("cannot convert null or undefined to object"),
        "unexpected error: {}",
        result.to_console_string()
    );
}

fn own_property_only_lookup_does_not_walk_prototype_chain() {
    let result = eval(
        r#"
        const proto = { x: 1 };
        const obj = Object.create(proto);
        Object.getOwnPropertyDescriptor(obj, "x");
        "#,
    );
    assert!(result.is_undefined());
}

fn inherited_property_is_not_reported_but_own_is() {
    let result = eval(
        r#"
        const proto = { x: 1 };
        const obj = Object.create(proto);
        obj.y = 2;
        JSON.stringify(Object.getOwnPropertyDescriptor(obj, "y"));
        "#,
    );
    assert_eq!(
        result.to_console_string(),
        r#"{"value":2,"writable":true,"enumerable":true,"configurable":true}"#
    );
}

fn accessor_descriptor_has_get_set_flags() {
    let result = eval(
        r#"
        const obj = {};
        Object.defineProperty(obj, "g", {
            get: function () { return 1; },
            enumerable: true,
            configurable: false
        });
        const desc = Object.getOwnPropertyDescriptor(obj, "g");
        typeof desc.get === "function" && desc.set === undefined &&
            desc.enumerable === true && desc.configurable === false &&
            desc.value === undefined && desc.writable === undefined;
        "#,
    );
    assert_eq!(result, JSValue::from_bool(true));
}

fn symbol_like_keys_resolve_through_to_property_key() {
    // このエンジンの Symbol 値は "@@..." 文字列キーで表現される
    let result = eval(
        r#"
        const obj = {};
        obj[Symbol.iterator] = 7;
        const desc = Object.getOwnPropertyDescriptor(obj, Symbol.iterator);
        desc.value;
        "#,
    );
    assert_eq!(result.to_number(), 7.0);
}

fn missing_property_argument_returns_descriptor_or_throws_consistently() {
    // 引数が足りない場合は TypeError（文字列化されて catch に届く）
    let result = eval(
        r#"
        let message = "";
        try { Object.getOwnPropertyDescriptor({}); }
        catch (e) { message = String(e); }
        message;
        "#,
    );
    assert!(
        result.to_console_string().contains("missing arguments"),
        "unexpected error: {}",
        result.to_console_string()
    );
}

fn function_receiver_reports_own_properties() {
    let result = eval(
        r#"
        function f() {}
        f.custom = 1;
        const desc = Object.getOwnPropertyDescriptor(f, "custom");
        desc.value + "|" + JSON.stringify(Object.getOwnPropertyDescriptor(f, "prototype"));
        "#,
    );
    assert!(result.to_console_string().starts_with("1|"));
}

fn test_set_prototype_of_basic() {
    let mut vm = VM::new();
    let global = vm.global_object.clone();

    // prepare proto and target
    let proto = pixi_byte::value::jsobject::JSObject::new();
    let proto_rc = std::rc::Rc::new(std::cell::RefCell::new(proto));
    let proto_val = JSValue::from_object(proto_rc.clone());

    let target = pixi_byte::value::jsobject::JSObject::new();
    let target_rc = std::rc::Rc::new(std::cell::RefCell::new(target));
    let target_val = JSValue::from_object(target_rc.clone());

    // call Object.setPrototypeOf(target, proto)
    let obj_global = global.borrow().get("Object");
    if let Some(obj_ref) = obj_global.as_object() {
        if let Some(setp) = obj_ref.borrow().get("setPrototypeOf").as_native_function() {
            let res = setp(&mut vm, vec![target_val.clone(), proto_val.clone()]).unwrap();
            match res.as_object() {
                Some(ret_rc) => {
                    // returned object should be the same target
                    assert!(std::rc::Rc::ptr_eq(&ret_rc, &target_rc));
                    // prototype should be set
                    let got = ret_rc.borrow().get_prototype();
                    assert!(got.is_some());
                    if let Some(gp) = got {
                        assert!(std::rc::Rc::ptr_eq(&gp, &proto_rc));
                    }
                }
                _ => panic!("setPrototypeOf did not return object"),
            }
        } else {
            panic!("setPrototypeOf not found");
        }
    } else {
        panic!("Object constructor missing");
    }
}

fn test_set_prototype_of_null() {
    let mut vm = VM::new();
    let global = vm.global_object.clone();

    let target = pixi_byte::value::jsobject::JSObject::new();
    let target_rc = std::rc::Rc::new(std::cell::RefCell::new(target));
    let target_val = JSValue::from_object(target_rc.clone());

    let obj_global = global.borrow().get("Object");
    if let Some(obj_ref) = obj_global.as_object() {
        if let Some(setp) = obj_ref.borrow().get("setPrototypeOf").as_native_function() {
            let res = setp(&mut vm, vec![target_val.clone(), JSValue::null()]).unwrap();
            match res.as_object() {
                Some(ret_rc) => {
                    assert!(std::rc::Rc::ptr_eq(&ret_rc, &target_rc));
                    // prototype should be null
                    let got = ret_rc.borrow().get_prototype();
                    assert!(got.is_none());
                }
                _ => panic!("setPrototypeOf did not return object"),
            }
        } else {
            panic!("setPrototypeOf not found");
        }
    } else {
        panic!("Object constructor missing");
    }
}

fn test_set_prototype_of_errors() {
    let mut vm = VM::new();
    let global = vm.global_object.clone();

    let obj_global = global.borrow().get("Object");
    if let Some(obj_ref) = obj_global.as_object() {
        if let Some(setp) = obj_ref.borrow().get("setPrototypeOf").as_native_function() {
            // first arg not object
            let res_err = setp(&mut vm, vec![JSValue::from_number(1.0), JSValue::null()]);
            assert!(res_err.is_err());

            // second arg primitive
            let target = pixi_byte::value::jsobject::JSObject::new();
            let target_rc = std::rc::Rc::new(std::cell::RefCell::new(target));
            let target_val = JSValue::from_object(target_rc.clone());

            let res_err2 = setp(&mut vm, vec![target_val.clone(), JSValue::from_number(2.0)]);
            assert!(res_err2.is_err());
        } else {
            panic!("setPrototypeOf not found");
        }
    } else {
        panic!("Object constructor missing");
    }
}

fn test_set_prototype_of_cycle_self() {
    let mut vm = VM::new();
    let global = vm.global_object.clone();

    let target = pixi_byte::value::jsobject::JSObject::new();
    let target_rc = std::rc::Rc::new(std::cell::RefCell::new(target));
    let target_val = JSValue::from_object(target_rc.clone());

    let obj_global = global.borrow().get("Object");
    if let Some(obj_ref) = obj_global.as_object() {
        if let Some(setp) = obj_ref.borrow().get("setPrototypeOf").as_native_function() {
            // attempt to set prototype to self should error
            let res_err = setp(&mut vm, vec![target_val.clone(), target_val.clone()]);
            assert!(res_err.is_err());
        } else {
            panic!("setPrototypeOf not found");
        }
    } else {
        panic!("Object constructor missing");
    }
}

fn test_set_prototype_of_cycle_two_objects() {
    let mut vm = VM::new();
    let global = vm.global_object.clone();

    // create two objects A and B
    let a = pixi_byte::value::jsobject::JSObject::new();
    let a_rc = std::rc::Rc::new(std::cell::RefCell::new(a));
    let a_val = JSValue::from_object(a_rc.clone());

    let b = pixi_byte::value::jsobject::JSObject::new();
    let b_rc = std::rc::Rc::new(std::cell::RefCell::new(b));
    let b_val = JSValue::from_object(b_rc.clone());

    let obj_global = global.borrow().get("Object");
    if let Some(obj_ref) = obj_global.as_object() {
        if let Some(setp) = obj_ref.borrow().get("setPrototypeOf").as_native_function() {
            // set A.prototype = B
            let _ = setp(&mut vm, vec![a_val.clone(), b_val.clone()]).unwrap();
            // now attempting to set B.prototype = A should error (creates cycle)
            let res_err = setp(&mut vm, vec![b_val.clone(), a_val.clone()]);
            assert!(res_err.is_err());
        } else {
            panic!("setPrototypeOf not found");
        }
    } else {
        panic!("Object constructor missing");
    }
}

fn object_literals_inherit_object_prototype_methods() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const reserved = { key: true, ref: true };
            reserved.hasOwnProperty("key") &&
                !reserved.hasOwnProperty("missing") &&
                reserved.toString() === "[object Object]";
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn constructed_instances_inherit_object_prototype_methods() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function Value() { this.answer = 42; }
            const value = new Value();
            value.hasOwnProperty("answer") && value.toString() === "[object Object]";
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn object_prototype_exposes_a_constructible_constructor() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const value = {};
            const clone = new value.constructor();
            value.constructor === Object && clone.constructor === Object;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn object_constructor(
    _vm: &mut pixi_byte::vm::VM,
    _args: Vec<JSValue>,
) -> pixi_byte::JSResult<JSValue> {
    let mut result = pixi_byte::value::jsobject::JSObject::new();
    result.set("value".to_string(), JSValue::from_number(9.0));
    Ok(JSValue::from_object(Rc::new(RefCell::new(result))))
}

fn new_calls_constructor_with_a_fresh_this_object() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function Box(value) { this.value = value; }
            let box = new Box(42);
            box.value;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_number(42.0));
}

fn constructor_object_return_value_replaces_this() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function Factory() { return { value: 7 }; }
            new Factory().value;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_number(7.0));
}

fn object_can_expose_an_internal_constructor_entry_point() {
    let mut engine = JSEngine::new();
    let mut constructor = pixi_byte::value::jsobject::JSObject::new();
    constructor.set(
        "__construct__".to_string(),
        JSValue::from_native_function(object_constructor),
    );
    engine.global_mut().borrow_mut().set(
        "ObjectConstructor".to_string(),
        JSValue::from_object(Rc::new(RefCell::new(constructor))),
    );

    assert_eq!(
        engine.eval("new ObjectConstructor().value").unwrap(),
        JSValue::from_number(9.0)
    );
}

fn new_accepts_member_expression_constructors() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const namespace = {
                Constructor: function (value) { this.value = value; }
            };
            const instance = new namespace.Constructor(42);
            instance.value;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(42.0));
}

fn implicit_object_expression_does_not_replace_constructor_this() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function Wrapper(value) { this.value = value; }
            const internal = { marker: "internal" };
            const wrapper = new Wrapper(internal);
            wrapper === internal ? "replaced" : wrapper.value.marker;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("internal".to_string()));
}

fn arrow_function_is_not_a_constructor() {
    let mut engine = JSEngine::new();
    assert!(engine.eval("new (() => 1)()").is_err());
}

fn proxy_get_and_set_traps_are_invoked() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const target = { value: 2 };
                const proxy = new Proxy(target, {
                    get(object, key) { return object[key] + 1; },
                    set(object, key, value) { object[key] = value * 2; return true; }
                });
                const read = proxy.value;
                proxy.value = 3;
                read + target.value;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(9.0));
}

fn proxy_internal_properties_do_not_leak_through_enumeration() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const proxy = new Proxy({}, {});
                const copy = { ...proxy };
                Object.keys(proxy).length === 0 && Object.keys(copy).length === 0;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn callable_proxy_forwards_calls_and_apply_traps() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const target = (left, right) => left + right;
                const direct = new Proxy(target, {});
                const trapped = new Proxy(target, {
                    apply(fn, receiver, args) { return fn(args[0] * 2, args[1]); }
                });
                direct(2, 3) + ":" + trapped(2, 3);
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("5:7".to_string()));
}

fn callable_proxy_forwards_function_properties_and_class_construction() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                class Box {
                    constructor(value) { this.value = value; }
                    read() { return this.value; }
                }
                Box.prototype.isComponent = true;
                const WrappedBox = new Proxy(Box, {});
                const box = new WrappedBox(13);
                typeof WrappedBox + ":" + WrappedBox.prototype.isComponent + ":" +
                    (box instanceof Box) + ":" + box.read();
            "#,
        )
        .unwrap();

    assert_eq!(
        result,
        JSValue::from_string("function:true:true:13".to_string())
    );
}

#[test]
fn suite_object_tests_1() {
    test_object_builtins_registered_and_functional();
    test_object_create_with_descriptors();
    test_is_prototype_of();
    test_object_to_string();
}

#[test]
fn suite_object_tests_2() {
    test_define_property_and_get_own_property_descriptor();
    test_function_call_apply();
}

#[test]
fn suite_object_accessor_tests_1() {
    inherited_accessors_receive_the_original_receiver();
    getter_receives_this();
    setter_updates_internal_state();
    accessor_descriptor_enumeration();
}

#[test]
fn suite_object_accessor_tests_2() {
    host_property_hooks_handle_missing_properties();
    method_call_resolves_an_accessor_getter();
}

#[test]
fn suite_object_literal_tests_1() {
    test_array_literal();
    test_array_literal_empty();
    test_object_literal();
    test_object_literal_empty();
}

#[test]
fn suite_object_literal_tests_2() {
    test_member_access_dot();
    test_member_access_bracket();
    test_member_assignment();
    test_array_index_access();
}

#[test]
fn suite_object_literal_tests_3() {
    test_nested_object();
}

#[test]
fn suite_object_literal_keys_1() {
    numeric_object_keys_use_their_canonical_string_value();
    object_literals_accept_string_number_and_keyword_keys();
    object_literals_accept_shorthand_properties();
    object_literals_support_getters_and_setters();
}

#[test]
fn suite_object_literal_keys_2() {
    object_literals_support_method_shorthand();
    methods_named_get_and_set_are_not_parsed_as_accessors();
    object_literals_support_spread_and_computed_keys();
    object_literals_support_computed_method_shorthand();
}

#[test]
fn suite_object_literal_keys_3() {
    computed_method_shorthand_accepts_symbol_keys();
}

#[test]
fn suite_object_mutation_tests() {
    test_prevent_extensions_and_is_extensible();
    test_seal_prevents_adding_and_makes_non_configurable();
    test_freeze_makes_non_writable_and_non_configurable();
}

#[test]
fn suite_object_collection_methods() {
    object_keys_returns_enumerable_own_keys();
    object_keys_boxes_primitives_and_accepts_callable_values();
    object_assign_copies_sources_from_left_to_right();
    extracted_object_assign_uses_its_first_argument_as_the_target();
}

#[test]
fn suite_object_static_methods_1() {
    object_is_uses_same_value_semantics();
    object_static_methods_do_not_treat_the_constructor_as_an_argument();
    get_prototype_of_accepts_functions_and_boxable_primitives();
    ordinary_and_explicit_null_prototypes_are_distinct();
}

#[test]
fn suite_object_static_methods_2() {
    get_own_property_names_includes_non_enumerable_properties();
    get_own_property_names_hides_engine_callable_slots();
    property_descriptor_methods_accept_functions();
    bound_set_prototype_of_forwards_arguments();
}

#[test]
fn suite_object_static_methods_3() {
    object_assign_accepts_function_targets_and_sources();
    entries_values_and_from_entries_round_trip_enumerable_properties();
    define_property_accepts_the_constructor_as_its_own_target();
    object_statics_drop_the_receiver_when_called_through_a_wrapper();
}

#[test]
fn suite_object_static_methods_4() {
    native_functions_carry_own_properties();
}

#[test]
fn suite_object_get_own_property_descriptor_1() {
    string_primitive_to_object_exposes_length_and_indices();
    string_indices_and_length_are_not_enumerable();
    number_and_boolean_primitives_have_empty_descriptors();
    null_and_undefined_receivers_throw();
}

#[test]
fn suite_object_get_own_property_descriptor_2() {
    own_property_only_lookup_does_not_walk_prototype_chain();
    inherited_property_is_not_reported_but_own_is();
    accessor_descriptor_has_get_set_flags();
    symbol_like_keys_resolve_through_to_property_key();
}

#[test]
fn suite_object_get_own_property_descriptor_3() {
    missing_property_argument_returns_descriptor_or_throws_consistently();
    function_receiver_reports_own_properties();
}

#[test]
fn suite_object_set_prototype_tests_1() {
    test_set_prototype_of_basic();
    test_set_prototype_of_null();
    test_set_prototype_of_errors();
    test_set_prototype_of_cycle_self();
}

#[test]
fn suite_object_set_prototype_tests_2() {
    test_set_prototype_of_cycle_two_objects();
}

#[test]
fn suite_object_prototype() {
    object_literals_inherit_object_prototype_methods();
    constructed_instances_inherit_object_prototype_methods();
    object_prototype_exposes_a_constructible_constructor();
}

#[test]
fn suite_new_expression_1() {
    new_calls_constructor_with_a_fresh_this_object();
    constructor_object_return_value_replaces_this();
    object_can_expose_an_internal_constructor_entry_point();
    new_accepts_member_expression_constructors();
}

#[test]
fn suite_new_expression_2() {
    implicit_object_expression_does_not_replace_constructor_this();
    arrow_function_is_not_a_constructor();
}

#[test]
fn suite_proxy() {
    proxy_get_and_set_traps_are_invoked();
    proxy_internal_properties_do_not_leak_through_enumeration();
    callable_proxy_forwards_calls_and_apply_traps();
    callable_proxy_forwards_function_properties_and_class_construction();
}
