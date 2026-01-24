use pixi_byte::vm::VM;
use pixi_byte::value::JSValue;

#[test]
fn getter_receives_this() {
    let mut vm = VM::new();

    // prepare target object with internal property 'x'
    let mut target = pixi_byte::value::jsobject::JSObject::new();
    target.set("x".to_string(), JSValue::String("hello".to_string()));
    let target_rc = std::rc::Rc::new(std::cell::RefCell::new(target));

    // getter native function returns receiver.x
    let getter_native = JSValue::NativeFunction(|_vm, args| {
        if args.is_empty() {
            return Ok(JSValue::Undefined);
        }
        match &args[0] {
            JSValue::Object(obj_ref) => Ok(obj_ref.borrow().get("x")),
            _ => Ok(JSValue::Undefined),
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
    if let JSValue::Object(obj_ref) = obj_global {
        if let JSValue::NativeFunction(def_fn) = obj_ref.borrow().get("defineProperty") {
            let _ = def_fn(&mut vm, vec![JSValue::Object(target_rc.clone()), JSValue::String("a".to_string()), JSValue::Object(desc_inner_rc.clone())]).unwrap();
        } else {
            panic!("defineProperty not callable");
        }
    } else {
        panic!("Object constructor not found");
    }

    // Build bytecode chunk to perform GetProperty on the object
    let mut chunk = pixi_byte::compiler::BytecodeChunk::new();
    let idx_obj = chunk.add_constant(JSValue::Object(target_rc.clone()));
    let idx_key = chunk.add_constant(JSValue::String("a".to_string()));
    chunk.emit(pixi_byte::compiler::Opcode::LoadConst(idx_obj));
    chunk.emit(pixi_byte::compiler::Opcode::LoadConst(idx_key));
    chunk.emit(pixi_byte::compiler::Opcode::GetProperty);
    chunk.emit(pixi_byte::compiler::Opcode::Return);

    let res = vm.execute(chunk).unwrap();
    match res {
        JSValue::String(s) => assert_eq!(s, "hello"),
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
    let setter_native = JSValue::NativeFunction(|_vm, args| {
        if args.len() < 2 {
            return Ok(JSValue::Undefined);
        }
        match &args[0] {
            JSValue::Object(obj_ref) => {
                let val = args[1].clone();
                obj_ref.borrow_mut().set("v".to_string(), val);
                Ok(JSValue::Undefined)
            }
            _ => Ok(JSValue::Undefined),
        }
    });

    // descriptor object { set: setter_native }
    let mut desc_inner = pixi_byte::value::jsobject::JSObject::new();
    desc_inner.set("set".to_string(), setter_native.clone());
    let desc_inner_rc = std::rc::Rc::new(std::cell::RefCell::new(desc_inner));

    // define property 'a' with setter
    let global = vm.global_object.clone();
    let obj_global = global.borrow().get("Object");
    if let JSValue::Object(obj_ref) = obj_global {
        if let JSValue::NativeFunction(def_fn) = obj_ref.borrow().get("defineProperty") {
            let _ = def_fn(&mut vm, vec![JSValue::Object(target_rc.clone()), JSValue::String("a".to_string()), JSValue::Object(desc_inner_rc.clone())]).unwrap();
        } else {
            panic!("defineProperty not callable");
        }
    } else {
        panic!("Object constructor not found");
    }

    // Use VM to perform assignment target.a = 42 via opcodes: push obj, key, value, SetProperty
    let mut chunk = pixi_byte::compiler::BytecodeChunk::new();
    let idx_obj = chunk.add_constant(JSValue::Object(target_rc.clone()));
    let idx_key = chunk.add_constant(JSValue::String("a".to_string()));
    let idx_val = chunk.add_constant(JSValue::Number(42.0));
    chunk.emit(pixi_byte::compiler::Opcode::LoadConst(idx_obj));
    chunk.emit(pixi_byte::compiler::Opcode::LoadConst(idx_key));
    chunk.emit(pixi_byte::compiler::Opcode::LoadConst(idx_val));
    chunk.emit(pixi_byte::compiler::Opcode::SetProperty);
    chunk.emit(pixi_byte::compiler::Opcode::Return);

    let _ = vm.execute(chunk).unwrap();

    // internal property 'v' should be set to 42
    let v = target_rc.borrow().get("v");
    match v {
        JSValue::Number(n) => assert_eq!(n, 42.0),
        _ => panic!("setter did not set internal property"),
    }
}

#[test]
fn accessor_descriptor_enumeration() {
    // descriptor with enumerable true should appear in keys()
    let target = pixi_byte::value::jsobject::JSObject::new();
    let target_rc = std::rc::Rc::new(std::cell::RefCell::new(target));

    let getter_native = JSValue::NativeFunction(|_vm, _args| Ok(JSValue::Undefined));
    let mut desc_inner = pixi_byte::value::jsobject::JSObject::new();
    desc_inner.set("get".to_string(), getter_native.clone());
    desc_inner.set("enumerable".to_string(), JSValue::Boolean(true));
    let desc_inner_rc = std::rc::Rc::new(std::cell::RefCell::new(desc_inner));

    // define property 'a' with enumerable accessor
    let mut target_mut = target_rc.borrow_mut();
    target_mut.define_property_descriptor("a".to_string(), desc_inner_rc.clone());
    drop(target_mut);

    let keys = target_rc.borrow().keys();
    assert!(keys.contains(&"a".to_string()));
}
