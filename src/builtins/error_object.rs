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
    let prototype = match vm.global_object.borrow().get(name) {
        JSValue::Object(constructor) => match constructor.borrow().get("prototype") {
            JSValue::Object(prototype) => Some(prototype),
            _ => None,
        },
        _ => None,
    };
    let mut error = JSObject::with_prototype(prototype);
    error.set("name".to_string(), JSValue::String(name.to_string()));
    error.set("message".to_string(), JSValue::String(message.clone()));
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
    error.set("stack".to_string(), JSValue::String(stack));
    Ok(JSValue::Object(Rc::new(RefCell::new(error))))
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
    let error_prototype = Rc::new(RefCell::new(prototype));
    constructor.set(
        "prototype".to_string(),
        JSValue::Object(Rc::clone(&error_prototype)),
    );
    global.borrow_mut().set(
        "Error".to_string(),
        JSValue::Object(Rc::new(RefCell::new(constructor))),
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
        subtype_prototype.set("name".to_string(), JSValue::String(name.to_string()));
        subtype_prototype.set("message".to_string(), JSValue::String(String::new()));
        let mut subtype_constructor = JSObject::new();
        subtype_constructor.set("__call__".to_string(), JSValue::NativeFunction(native));
        subtype_constructor.set("__construct__".to_string(), JSValue::NativeFunction(native));
        subtype_constructor.set(
            "prototype".to_string(),
            JSValue::Object(Rc::new(RefCell::new(subtype_prototype))),
        );
        global.borrow_mut().set(
            name.to_string(),
            JSValue::Object(Rc::new(RefCell::new(subtype_constructor))),
        );
    }
}
