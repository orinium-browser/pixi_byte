//! Minimal Number constructor and prototype.

use std::cell::RefCell;
use std::rc::Rc;

use crate::error::{JSError, JSResult};
use crate::value::jsobject::JSObject;
use crate::value::JSValue;
use crate::vm::VM;

fn receiver(args: &[JSValue], method: &str) -> JSResult<f64> {
    match args.first() {
        Some(JSValue::Number(value)) => Ok(*value),
        _ => Err(JSError::TypeError(format!(
            "Number.prototype.{method}: invalid receiver"
        ))),
    }
}

fn number_constructor(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(JSValue::Number(
        args.get(1).map(JSValue::to_number).unwrap_or(0.0),
    ))
}

fn number_to_string(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let value = receiver(&args, "toString")?;
    let radix = args.get(1).map(JSValue::to_number).unwrap_or(10.0) as u32;
    if !(2..=36).contains(&radix) {
        return Err(JSError::RangeError(
            "Number.toString radix must be between 2 and 36".to_string(),
        ));
    }
    if radix == 10 || !value.is_finite() || value.fract() != 0.0 {
        return Ok(JSValue::String(JSValue::Number(value).to_string()));
    }

    let negative = value.is_sign_negative();
    let mut integer = value.abs() as u64;
    let mut digits = Vec::new();
    loop {
        let digit = (integer % radix as u64) as u8;
        digits.push(if digit < 10 {
            (b'0' + digit) as char
        } else {
            (b'a' + digit - 10) as char
        });
        integer /= radix as u64;
        if integer == 0 {
            break;
        }
    }
    if negative {
        digits.push('-');
    }
    digits.reverse();
    Ok(JSValue::String(digits.into_iter().collect()))
}

fn number_value_of(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(JSValue::Number(receiver(&args, "valueOf")?))
}

/// Installs Number and its primitive prototype.
pub fn install(global: &Rc<RefCell<JSObject>>) {
    let mut prototype = JSObject::new();
    prototype.set(
        "toString".to_string(),
        JSValue::NativeFunction(number_to_string),
    );
    prototype.set(
        "valueOf".to_string(),
        JSValue::NativeFunction(number_value_of),
    );

    let mut constructor = JSObject::new();
    constructor.set(
        "__call__".to_string(),
        JSValue::NativeFunction(number_constructor),
    );
    constructor.set(
        "prototype".to_string(),
        JSValue::Object(Rc::new(RefCell::new(prototype))),
    );
    global.borrow_mut().set(
        "Number".to_string(),
        JSValue::Object(Rc::new(RefCell::new(constructor))),
    );
}
