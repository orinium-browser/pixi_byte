//! JavaScript Error objects used by React invariants.

use std::cell::RefCell;
use std::rc::Rc;

use crate::error::{JSError, JSResult};
use crate::value::jsobject::JSObject;
use crate::value::JSValue;
use crate::vm::VM;

fn error_constructor(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let message = args.get(1).map(JSValue::to_string).unwrap_or_default();
    let prototype = match vm.global_object.borrow().get("Error") {
        JSValue::Object(constructor) => match constructor.borrow().get("prototype") {
            JSValue::Object(prototype) => Some(prototype),
            _ => None,
        },
        _ => None,
    };
    let mut error = JSObject::with_prototype(prototype);
    error.set("name".to_string(), JSValue::String("Error".to_string()));
    error.set("message".to_string(), JSValue::String(message.clone()));
    error.set(
        "stack".to_string(),
        JSValue::String(if message.is_empty() {
            "Error".to_string()
        } else {
            format!("Error: {message}")
        }),
    );
    Ok(JSValue::Object(Rc::new(RefCell::new(error))))
}

fn error_to_string(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let Some(JSValue::Object(error)) = args.first() else {
        return Err(JSError::TypeError(
            "Error.prototype.toString: invalid receiver".to_string(),
        ));
    };
    let name = match error.borrow().get("name") {
        JSValue::Undefined => "Error".to_string(),
        value => value.to_string(),
    };
    let message = match error.borrow().get("message") {
        JSValue::Undefined => String::new(),
        value => value.to_string(),
    };
    Ok(JSValue::String(if message.is_empty() {
        name
    } else if name.is_empty() {
        message
    } else {
        format!("{name}: {message}")
    }))
}

/// Installs the global Error constructor.
pub fn install(global: &Rc<RefCell<JSObject>>) {
    let mut prototype = JSObject::new();
    prototype.set(
        "toString".to_string(),
        JSValue::NativeFunction(error_to_string),
    );
    prototype.set("name".to_string(), JSValue::String("Error".to_string()));
    prototype.set("message".to_string(), JSValue::String(String::new()));

    let mut constructor = JSObject::new();
    constructor.set(
        "__call__".to_string(),
        JSValue::NativeFunction(error_constructor),
    );
    constructor.set(
        "__construct__".to_string(),
        JSValue::NativeFunction(error_constructor),
    );
    constructor.set(
        "prototype".to_string(),
        JSValue::Object(Rc::new(RefCell::new(prototype))),
    );
    global.borrow_mut().set(
        "Error".to_string(),
        JSValue::Object(Rc::new(RefCell::new(constructor))),
    );
}
