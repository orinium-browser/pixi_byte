use crate::error::JSResult;
use crate::value::JSValue;
use crate::value::jsobject::JSObject;
use crate::vm::VM;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

fn date_now(_vm: &mut VM, _args: Vec<JSValue>) -> JSResult<JSValue> {
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
        * 1_000.0;
    Ok(JSValue::Number(milliseconds.floor()))
}

pub fn install(global: &Rc<RefCell<JSObject>>) {
    let mut date = JSObject::new();
    date.set("now".to_string(), JSValue::NativeFunction(date_now));
    global.borrow_mut().set(
        "Date".to_string(),
        JSValue::Object(Rc::new(RefCell::new(date))),
    );
}
