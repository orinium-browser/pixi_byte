use crate::value::JSValue;
use crate::value::jsobject::JSObject;
use std::cell::RefCell;
use std::rc::Rc;

/// シンプルな Object 組み込みの最小実装
/// グローバルオブジェクトに `Object` を登録し、`create` と `getPrototypeOf` を提供する

pub fn install(global: &Rc<RefCell<JSObject>>) {
    // 作業: グローバルオブジェクトに `Object` をオブジェクトとしてセット
    let mut obj = JSObject::new();

    // `create` と `getPrototypeOf` は名称のみセット（実体は VM/host が理解するプリミティブ）
    obj.set("create".to_string(), JSValue::String("[native Object.create]".to_string()));
    obj.set(
        "getPrototypeOf".to_string(),
        JSValue::String("[native Object.getPrototypeOf]".to_string()),
    );

    global.borrow_mut().set("Object".to_string(), JSValue::Object(Rc::new(RefCell::new(obj))));
}

