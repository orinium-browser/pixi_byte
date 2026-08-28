//! JavaScript Error objects used by React invariants.

use std::cell::RefCell;
use std::rc::Rc;

use crate::error::{JSError, JSResult};
use crate::value::JSValue;
use crate::value::jsobject::JSObject;
use crate::vm::VM;

fn error_constructor(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    make_error(vm, args, "Error")
}

fn make_error(vm: &mut VM, args: Vec<JSValue>, name: &str) -> JSResult<JSValue> {
    let message = args.get(1).map(JSValue::to_string).unwrap_or_default();
    let prototype = vm
        .global_object
        .borrow()
        .get(name)
        .as_object()
        .and_then(|constructor| {
            let proto = constructor.borrow().get("prototype");
            proto.as_object()
        });
    let mut error = JSObject::with_prototype(prototype);
    error.set("name".to_string(), JSValue::from_string(name.to_string()));
    error.set("message".to_string(), JSValue::from_string(message.clone()));
    let mut stack = if message.is_empty() {
        name.to_string()
    } else {
        format!("{name}: {message}")
    };
    let frames = vm.formatted_js_stack();
    if !frames.is_empty() {
        stack.push_str("\nJS stack: ");
        stack.push_str(&frames);
    }
    error.set("stack".to_string(), JSValue::from_string(stack));
    Ok(JSValue::from_object(Rc::new(RefCell::new(error))))
}

macro_rules! error_constructor {
    ($function:ident, $name:literal) => {
        fn $function(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
            make_error(vm, args, $name)
        }
    };
}

error_constructor!(eval_error_constructor, "EvalError");
error_constructor!(range_error_constructor, "RangeError");
error_constructor!(reference_error_constructor, "ReferenceError");
error_constructor!(syntax_error_constructor, "SyntaxError");
error_constructor!(type_error_constructor, "TypeError");
error_constructor!(uri_error_constructor, "URIError");

fn error_to_string(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let error = args.first().and_then(|v| v.as_object()).ok_or_else(|| {
        JSError::TypeError("Error.prototype.toString: invalid receiver".to_string())
    })?;
    let name = if error.borrow().get("name").is_undefined() {
        "Error".to_string()
    } else {
        error.borrow().get("name").to_string()
    };
    let message = if error.borrow().get("message").is_undefined() {
        String::new()
    } else {
        error.borrow().get("message").to_string()
    };
    Ok(JSValue::from_string(if message.is_empty() {
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
        JSValue::from_native_function(error_to_string),
    );
    prototype.set(
        "name".to_string(),
        JSValue::from_string("Error".to_string()),
    );
    prototype.set("message".to_string(), JSValue::from_string(String::new()));

    let mut constructor = JSObject::new();
    constructor.set(
        "__call__".to_string(),
        JSValue::from_native_function(error_constructor),
    );
    constructor.set(
        "__construct__".to_string(),
        JSValue::from_native_function(error_constructor),
    );
    let error_prototype = Rc::new(RefCell::new(prototype));
    constructor.set(
        "prototype".to_string(),
        JSValue::from_object(Rc::clone(&error_prototype)),
    );
    global.borrow_mut().set(
        "Error".to_string(),
        JSValue::from_object(Rc::new(RefCell::new(constructor))),
    );

    let subtypes: [(&str, crate::NativeFunctionType); 6] = [
        ("EvalError", eval_error_constructor),
        ("RangeError", range_error_constructor),
        ("ReferenceError", reference_error_constructor),
        ("SyntaxError", syntax_error_constructor),
        ("TypeError", type_error_constructor),
        ("URIError", uri_error_constructor),
    ];
    for (name, native) in subtypes {
        let mut subtype_prototype = JSObject::with_prototype(Some(Rc::clone(&error_prototype)));
        subtype_prototype.set("name".to_string(), JSValue::from_string(name.to_string()));
        subtype_prototype.set("message".to_string(), JSValue::from_string(String::new()));
        let mut subtype_constructor = JSObject::new();
        subtype_constructor.set(
            "__call__".to_string(),
            JSValue::from_native_function(native),
        );
        subtype_constructor.set(
            "__construct__".to_string(),
            JSValue::from_native_function(native),
        );
        subtype_constructor.set(
            "prototype".to_string(),
            JSValue::from_object(Rc::new(RefCell::new(subtype_prototype))),
        );
        global.borrow_mut().set(
            name.to_string(),
            JSValue::from_object(Rc::new(RefCell::new(subtype_constructor))),
        );
    }
}
