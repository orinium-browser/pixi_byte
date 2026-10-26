//! Minimal Boolean constructor.

use std::cell::RefCell;
use std::rc::Rc;

use crate::error::{JSError, JSResult};
use crate::value::JSValue;
use crate::value::jsobject::JSObject;
use crate::value::jsvalue::JsValueKind;
use crate::vm::VM;

fn boolean_constructor(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(JSValue::from_bool(
        args.get(1).map(JSValue::to_boolean).unwrap_or(false),
    ))
}

fn boolean_receiver(args: &[JSValue], method: &str) -> JSResult<bool> {
    match args.first() {
        Some(value) if value.kind() == JsValueKind::Boolean => {
            value.as_boolean().ok_or_else(|| {
                JSError::TypeError(format!("Boolean.prototype.{method}: invalid receiver"))
            })
        }
        _ => Err(JSError::TypeError(format!(
            "Boolean.prototype.{method}: invalid receiver"
        ))),
    }
}

fn boolean_to_string(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let value = boolean_receiver(&args, "toString")?;
    Ok(JSValue::from_str(if value { "true" } else { "false" }))
}

fn boolean_value_of(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(JSValue::from_bool(boolean_receiver(&args, "valueOf")?))
}

pub fn install(global: &Rc<RefCell<JSObject>>) {
    let mut prototype = JSObject::new();
    prototype.set(
        "toString".to_string(),
        JSValue::from_native_function(boolean_to_string),
    );
    prototype.set(
        "valueOf".to_string(),
        JSValue::from_native_function(boolean_value_of),
    );

    let mut constructor = JSObject::new();
    constructor.set(
        "__call__".to_string(),
        JSValue::from_native_function(boolean_constructor),
    );
    constructor.set(
        "__construct__".to_string(),
        JSValue::from_native_function(boolean_constructor),
    );
    constructor.set(
        "prototype".to_string(),
        JSValue::from_object(Rc::new(RefCell::new(prototype))),
    );
    global.borrow_mut().set(
        "Boolean".to_string(),
        JSValue::from_object(Rc::new(RefCell::new(constructor))),
    );
}
