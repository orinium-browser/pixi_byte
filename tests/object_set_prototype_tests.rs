use pixi_byte::value::JSValue;
use pixi_byte::vm::VM;

#[test]
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

#[test]
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

#[test]
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

#[test]
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

#[test]
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
