use crate::error::JSResult;
use crate::value::JSValue;
use crate::value::jsobject::JSObject;
use crate::vm::VM;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

const DATE_VALUE: &str = "__date_value__";

fn epoch_milliseconds() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
        * 1_000.0
}

fn date_now(_vm: &mut VM, _args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(JSValue::Number(epoch_milliseconds().floor()))
}

fn date_parse(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let value = args
        .get(1)
        .map(JSValue::to_string)
        .unwrap_or_default()
        .trim()
        .parse::<f64>()
        .unwrap_or(f64::NAN);
    Ok(JSValue::Number(value))
}

fn date_constructor(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let milliseconds = match args.get(1) {
        None | Some(JSValue::Undefined) => epoch_milliseconds().floor(),
        Some(value) => vm.to_number_value(value.clone())?,
    };
    let Some(JSValue::Object(this)) = args.first() else {
        return Ok(JSValue::Undefined);
    };
    this.borrow_mut()
        .set(DATE_VALUE.to_string(), JSValue::Number(milliseconds));
    Ok(JSValue::Undefined)
}

fn date_call(_vm: &mut VM, _args: Vec<JSValue>) -> JSResult<JSValue> {
    // A full locale-sensitive Date string is not available yet. Returning a
    // stable timestamp string preserves the required callable shape while the
    // constructor and numeric methods remain standards-compatible.
    Ok(JSValue::String(epoch_milliseconds().floor().to_string()))
}

fn date_value(args: &[JSValue], method: &str) -> JSResult<f64> {
    let Some(JSValue::Object(this)) = args.first() else {
        return Err(crate::error::JSError::TypeError(format!(
            "Date.prototype.{method}: invalid receiver"
        )));
    };
    match this.borrow().get(DATE_VALUE) {
        JSValue::Number(value) => Ok(value),
        _ => Err(crate::error::JSError::TypeError(format!(
            "Date.prototype.{method}: invalid receiver"
        ))),
    }
}

fn date_get_time(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(JSValue::Number(date_value(&args, "getTime")?))
}

fn date_value_of(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(JSValue::Number(date_value(&args, "valueOf")?))
}

pub fn install(global: &Rc<RefCell<JSObject>>) {
    let mut prototype = JSObject::new();
    prototype.set(
        "getTime".to_string(),
        JSValue::NativeFunction(date_get_time),
    );
    prototype.set(
        "valueOf".to_string(),
        JSValue::NativeFunction(date_value_of),
    );

    let mut date = JSObject::new();
    date.set(
        "__call__".to_string(),
        JSValue::NativeFunction(date_call),
    );
    date.set(
        "__construct__".to_string(),
        JSValue::NativeFunction(date_constructor),
    );
    date.set(
        "prototype".to_string(),
        JSValue::Object(Rc::new(RefCell::new(prototype))),
    );
    date.set("now".to_string(), JSValue::NativeFunction(date_now));
    date.set("parse".to_string(), JSValue::NativeFunction(date_parse));
    global.borrow_mut().set(
        "Date".to_string(),
        JSValue::Object(Rc::new(RefCell::new(date))),
    );
}
