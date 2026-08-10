//! Minimal Boolean constructor.

use std::cell::RefCell;
use std::rc::Rc;

use crate::error::JSResult;
use crate::value::JSValue;
use crate::value::jsobject::JSObject;
use crate::vm::VM;

fn boolean_constructor(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(JSValue::Boolean(
        args.get(1).map(JSValue::to_boolean).unwrap_or(false),
    ))
}

pub fn install(global: &Rc<RefCell<JSObject>>) {
    let mut constructor = JSObject::new();
    constructor.set(
        "__call__".to_string(),
        JSValue::NativeFunction(boolean_constructor),
    );
    constructor.set(
        "__construct__".to_string(),
        JSValue::NativeFunction(boolean_constructor),
    );
    global.borrow_mut().set(
        "Boolean".to_string(),
        JSValue::Object(Rc::new(RefCell::new(constructor))),
    );
}
