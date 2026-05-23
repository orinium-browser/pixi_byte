use crate::value::JSValue;
use crate::value::jsobject::JSObject;
use std::cell::RefCell;
use std::rc::Rc;

/// シンプルな Array 組み込みの最小実装
/// グローバルオブジェクトに `Array` を登録し、`prototype.push` と `prototype.pop` を提供する

// NativeFunction シグネチャ: fn(&mut VM, Vec<JSValue>) -> JSResult<JSValue>

fn array_push(_vm: &mut crate::vm::VM, mut args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    // args: [this, value1, value2, ...] or if called via CallFunction maybe only values
    if args.is_empty() {
        return Err(crate::error::JSError::TypeError(
            "Array.prototype.push: missing receiver".to_string(),
        ));
    }

    let receiver = args.remove(0);

    match receiver {
        JSValue::Object(obj_ref) => {
            // determine length
            let len_val = obj_ref.borrow().get("length");
            let mut len = len_val.to_number();
            if len.is_nan() {
                len = 0.0;
            }
            let mut idx = len as usize;
            // push all remaining args
            for v in args.into_iter() {
                obj_ref.borrow_mut().set(idx.to_string(), v);
                idx += 1;
            }
            // update length
            obj_ref
                .borrow_mut()
                .set("length".to_string(), JSValue::Number(idx as f64));
            Ok(JSValue::Number(idx as f64))
        }
        _ => Err(crate::error::JSError::TypeError(
            "Array.prototype.push: receiver is not an object".to_string(),
        )),
    }
}

fn array_pop(_vm: &mut crate::vm::VM, mut args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    if args.is_empty() {
        return Err(crate::error::JSError::TypeError(
            "Array.prototype.pop: missing receiver".to_string(),
        ));
    }

    let receiver = args.remove(0);

    match receiver {
        JSValue::Object(obj_ref) => {
            let len_val = obj_ref.borrow().get("length");
            let mut len = len_val.to_number();
            if len.is_nan() {
                len = 0.0;
            }
            if len == 0.0 {
                // nothing to pop
                obj_ref
                    .borrow_mut()
                    .set("length".to_string(), JSValue::Number(0.0));
                return Ok(JSValue::Undefined);
            }
            let idx = (len as usize).saturating_sub(1);
            let element = obj_ref.borrow().get(&idx.to_string());
            // delete property
            obj_ref.borrow_mut().delete(&idx.to_string());
            // update length
            obj_ref
                .borrow_mut()
                .set("length".to_string(), JSValue::Number(idx as f64));
            Ok(element)
        }
        _ => Err(crate::error::JSError::TypeError(
            "Array.prototype.pop: receiver is not an object".to_string(),
        )),
    }
}

pub fn install(global: &Rc<RefCell<JSObject>>) {
    // Array コンストラクタオブジェクト（最小実装）
    let mut array_ctor = JSObject::new();

    // Array.prototype オブジェクト
    let mut proto = JSObject::new();

    // push と pop をネイティブ関数として登録
    proto.set("push".to_string(), JSValue::NativeFunction(array_push));
    proto.set("pop".to_string(), JSValue::NativeFunction(array_pop));

    array_ctor.set(
        "prototype".to_string(),
        JSValue::Object(Rc::new(RefCell::new(proto))),
    );

    global.borrow_mut().set(
        "Array".to_string(),
        JSValue::Object(Rc::new(RefCell::new(array_ctor))),
    );
}
