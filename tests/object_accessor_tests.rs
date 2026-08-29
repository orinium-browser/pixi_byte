use pixi_byte::JSEngine;
use pixi_byte::value::JSValue;
use pixi_byte::vm::VM;

#[test]
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

#[test]
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

#[test]
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

#[test]
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

#[test]
fn host_property_hooks_handle_missing_properties() {
    let mut vm = VM::new();

    let mut target = pixi_byte::value::jsobject::JSObject::new();
    target.set(
        pixi_byte::value::jsobject::HOST_GET_PROPERTY.to_string(),
        JSValue::from_native_function(|_vm, args| {
            Ok(JSValue::from_string(format!("read:{}", args[1].to_string())))
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

#[test]
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
