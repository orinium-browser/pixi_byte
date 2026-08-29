use pixi_byte::value::JSValue;
use pixi_byte::vm::VM;

#[test]
fn test_prevent_extensions_and_is_extensible() {
    let mut vm = VM::new();
    let global = vm.global_object.clone();

    let obj = pixi_byte::value::jsobject::JSObject::new();
    let rc = std::rc::Rc::new(std::cell::RefCell::new(obj));
    let val = JSValue::from_object(rc.clone());

    let obj_global = global.borrow().get("Object");
    if let Some(obj_ref) = obj_global.as_object() {
        if let Some(prevent) = obj_ref.borrow().get("preventExtensions").as_native_function() {
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

#[test]
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
    let added = rc.borrow_mut().set("b".to_string(), JSValue::from_number(2.0));
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
    let changed = rc.borrow_mut().set("x".to_string(), JSValue::from_number(20.0));
    assert_eq!(changed, false);

    // Property should be non-configurable
    let desc = rc.borrow().get_property_descriptor("x");
    assert!(desc.is_some());
    if let Some(d) = desc {
        assert_eq!(d.configurable, false);
        assert_eq!(d.writable, false);
    }
}
