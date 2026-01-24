use pixi_byte::vm::VM;
use pixi_byte::value::JSValue;

#[test]
fn test_prevent_extensions_and_is_extensible() {
    let mut vm = VM::new();
    let global = vm.global_object.clone();

    let obj = pixi_byte::value::jsobject::JSObject::new();
    let rc = std::rc::Rc::new(std::cell::RefCell::new(obj));
    let val = JSValue::Object(rc.clone());

    let obj_global = global.borrow().get("Object");
    if let JSValue::Object(obj_ref) = obj_global {
        if let JSValue::NativeFunction(prevent) = obj_ref.borrow().get("preventExtensions") {
            let _ = prevent(&mut vm, vec![val.clone()]).unwrap();
        } else {
            panic!("preventExtensions not found");
        }
        if let JSValue::NativeFunction(is_ext) = obj_ref.borrow().get("isExtensible") {
            let res = is_ext(&mut vm, vec![val.clone()]).unwrap();
            match res {
                JSValue::Boolean(b) => assert_eq!(b, false),
                _ => panic!("isExtensible returned non-boolean"),
            }
        } else {
            panic!("isExtensible not found");
        }
    } else {
        panic!("Object constructor missing");
    }
}

#[test]
fn test_seal_prevents_adding_and_makes_non_configurable() {
    let mut vm = VM::new();
    let global = vm.global_object.clone();

    let mut obj = pixi_byte::value::jsobject::JSObject::new();
    obj.set("a".to_string(), JSValue::Number(1.0));
    let rc = std::rc::Rc::new(std::cell::RefCell::new(obj));
    let val = JSValue::Object(rc.clone());

    let obj_global = global.borrow().get("Object");
    if let JSValue::Object(obj_ref) = obj_global {
        if let JSValue::NativeFunction(seal) = obj_ref.borrow().get("seal") {
            let _ = seal(&mut vm, vec![val.clone()]).unwrap();
        } else {
            panic!("seal not found");
        }
    } else {
        panic!("Object constructor missing");
    }

    // Attempt to add new property should fail due to non-extensible
    let added = rc.borrow_mut().set("b".to_string(), JSValue::Number(2.0));
    assert_eq!(added, false);

    // Existing property should become non-configurable
    let desc = rc.borrow().get_property_descriptor("a");
    assert!(desc.is_some());
    if let Some(d) = desc {
        assert_eq!(d.configurable, false);
    }
}

#[test]
fn test_freeze_makes_non_writable_and_non_configurable() {
    let mut vm = VM::new();
    let global = vm.global_object.clone();

    let mut obj = pixi_byte::value::jsobject::JSObject::new();
    obj.set("x".to_string(), JSValue::Number(10.0));
    let rc = std::rc::Rc::new(std::cell::RefCell::new(obj));
    let val = JSValue::Object(rc.clone());

    let obj_global = global.borrow().get("Object");
    if let JSValue::Object(obj_ref) = obj_global {
        if let JSValue::NativeFunction(freeze) = obj_ref.borrow().get("freeze") {
            let _ = freeze(&mut vm, vec![val.clone()]).unwrap();
        } else {
            panic!("freeze not found");
        }
    } else {
        panic!("Object constructor missing");
    }

    // Attempt to change existing property should fail (not writable)
    let changed = rc.borrow_mut().set("x".to_string(), JSValue::Number(20.0));
    assert_eq!(changed, false);

    // Property should be non-configurable
    let desc = rc.borrow().get_property_descriptor("x");
    assert!(desc.is_some());
    if let Some(d) = desc {
        assert_eq!(d.configurable, false);
        assert_eq!(d.writable, false);
    }
}
