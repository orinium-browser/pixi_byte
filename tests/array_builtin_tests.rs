use pixi_byte::value::JSValue;
use pixi_byte::vm::VM;

#[test]
fn test_array_prototype_push_pop_native() {
    let mut vm = VM::new();
    let global = vm.global_object.clone();

    // Array が登録されている
    let array_ctor = global.borrow().get("Array");
    match array_ctor {
        JSValue::Object(ref ctor_ref) => {
            // prototype がある
            let proto = ctor_ref.borrow().get("prototype");
            match proto {
                JSValue::Object(proto_ref) => {
                    // push/pop がネイティブで登録されている
                    let push = proto_ref.borrow().get("push");
                    let pop = proto_ref.borrow().get("pop");
                    match push {
                        JSValue::NativeFunction(_) => {}
                        _ => panic!("push not native"),
                    }
                    match pop {
                        JSValue::NativeFunction(_) => {}
                        _ => panic!("pop not native"),
                    }

                    // 簡単な配列オブジェクトを作り、push/pop を呼び出す
                    let mut arr = pixi_byte::value::jsobject::JSObject::new();
                    arr.set("length".to_string(), JSValue::Number(0.0));
                    let arr_obj = JSValue::Object(std::rc::Rc::new(std::cell::RefCell::new(arr)));

                    // CallMethod semantics expect stack: ..., object, property, arg1,arg2..., and opcode will inject receiver.
                    // We'll invoke the native function directly via CallMethod path simulation: fetch method and call with receiver.
                    if let JSValue::NativeFunction(f) = proto_ref.borrow().get("push") {
                        let res = f(&mut vm, vec![arr_obj.clone(), JSValue::Number(10.0)]).unwrap();
                        assert_eq!(res, JSValue::Number(1.0));
                    } else {
                        panic!("push not callable");
                    }

                    if let JSValue::NativeFunction(f) = proto_ref.borrow().get("pop") {
                        let res = f(&mut vm, vec![arr_obj.clone()]).unwrap();
                        assert_eq!(res, JSValue::Number(10.0));
                    } else {
                        panic!("pop not callable");
                    }
                }
                _ => panic!("Array.prototype not object"),
            }
        }
        _ => panic!("Array global not object"),
    }
}
