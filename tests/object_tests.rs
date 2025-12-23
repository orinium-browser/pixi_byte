use pixi_byte::vm::VM;
use pixi_byte::value::JSValue;

#[test]
fn test_object_builtins_registered() {
    let vm = VM::new();
    let global = vm.global_object;

    // Object がグローバルに存在する
    let obj_val = global.borrow().get("Object");
    match obj_val {
        JSValue::Object(obj_ref) => {
            // create と getPrototypeOf が登録されている（最小実装では native のプレースホルダ文字列）
            let create = obj_ref.borrow().get("create");
            let get_proto = obj_ref.borrow().get("getPrototypeOf");

            match create {
                JSValue::String(s) => assert!(s.contains("native Object.create")),
                _ => panic!("Object.create not installed as expected"),
            }

            match get_proto {
                JSValue::String(s) => assert!(s.contains("native Object.getPrototypeOf")),
                _ => panic!("Object.getPrototypeOf not installed as expected"),
            }
        }
        _ => panic!("Object global is not an object"),
    }
}

