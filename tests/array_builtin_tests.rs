use pixi_byte::value::JSValue;
use pixi_byte::vm::VM;

#[test]
fn test_array_prototype_push_pop_native() {
    let mut vm = VM::new();
    let global = vm.global_object.clone();

    // Array が登録されている
    let array_ctor = global.borrow().get("Array");
    let ctor_ref = array_ctor
        .as_object()
        .unwrap_or_else(|| panic!("Array global not object"));
    // prototype がある
    let proto = ctor_ref.borrow().get("prototype");
    let proto_ref = proto
        .as_object()
        .unwrap_or_else(|| panic!("Array.prototype not object"));
    // push/pop がネイティブで登録されている
    let push = proto_ref.borrow().get("push");
    let pop = proto_ref.borrow().get("pop");
    assert!(push.is_native_function(), "push not native");
    assert!(pop.is_native_function(), "pop not native");

    // 簡単な配列オブジェクトを作り、push/pop を呼び出す
    let mut arr = pixi_byte::value::jsobject::JSObject::new();
    arr.set("length".to_string(), JSValue::from_number(0.0));
    let arr_obj = JSValue::from_object(std::rc::Rc::new(std::cell::RefCell::new(arr)));

    // CallMethod semantics expect stack: ..., object, property, arg1,arg2..., and opcode will inject receiver.
    // We'll invoke the native function directly via CallMethod path simulation: fetch method and call with receiver.
    if let Some(f) = proto_ref.borrow().get("push").as_native_function() {
        let res = f(&mut vm, vec![arr_obj.clone(), JSValue::from_number(10.0)]).unwrap();
        assert_eq!(res, JSValue::from_number(1.0));
    } else {
        panic!("push not callable");
    }

    if let Some(f) = proto_ref.borrow().get("pop").as_native_function() {
        let res = f(&mut vm, vec![arr_obj.clone()]).unwrap();
        assert_eq!(res, JSValue::from_number(10.0));
    } else {
        panic!("pop not callable");
    }
}
