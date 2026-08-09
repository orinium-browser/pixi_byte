//! Minimal RegExp objects used by regular expression literals.

use std::cell::RefCell;
use std::rc::Rc;

use regex::RegexBuilder;

use crate::error::{JSError, JSResult};
use crate::value::jsobject::JSObject;
use crate::value::JSValue;
use crate::vm::VM;

const PATTERN: &str = "__regexp_pattern";
const FLAGS: &str = "__regexp_flags";

fn compile(object: &Rc<RefCell<JSObject>>) -> JSResult<regex::Regex> {
    let pattern = object.borrow().get(PATTERN).to_string();
    let flags = object.borrow().get(FLAGS).to_string();
    let mut builder = RegexBuilder::new(&pattern);
    builder
        .case_insensitive(flags.contains('i'))
        .multi_line(flags.contains('m'))
        .dot_matches_new_line(flags.contains('s'));
    builder
        .build()
        .map_err(|error| {
            JSError::SyntaxError(
                error.to_string(),
                crate::lexer::Span::new(0, 0, 0, 0),
            )
        })
}

fn regexp_test(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let Some(JSValue::Object(object)) = args.first() else {
        return Err(JSError::TypeError("RegExp.test: invalid receiver".to_string()));
    };
    let input = args.get(1).map(JSValue::to_string).unwrap_or_default();
    Ok(JSValue::Boolean(compile(object)?.is_match(&input)))
}

fn regexp_exec(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let Some(JSValue::Object(object)) = args.first() else {
        return Err(JSError::TypeError("RegExp.exec: invalid receiver".to_string()));
    };
    let input = args.get(1).map(JSValue::to_string).unwrap_or_default();
    let expression = compile(object)?;
    let Some(captures) = expression.captures(&input) else {
        return Ok(JSValue::Null);
    };
    let values = captures
        .iter()
        .map(|capture| {
            capture
                .map(|capture| JSValue::String(capture.as_str().to_string()))
                .unwrap_or(JSValue::Undefined)
        })
        .collect();
    let result = vm.array_from_values(values);
    if let JSValue::Object(result) = &result {
        let index = captures.get(0).map(|capture| capture.start()).unwrap_or(0);
        result
            .borrow_mut()
            .set("index".to_string(), JSValue::Number(index as f64));
        result
            .borrow_mut()
            .set("input".to_string(), JSValue::String(input));
    }
    Ok(result)
}

/// Creates a RegExp object for a parsed literal.
pub fn create(pattern: &str, flags: &str) -> JSValue {
    let mut object = JSObject::new();
    object.set(PATTERN.to_string(), JSValue::String(pattern.to_string()));
    object.set(FLAGS.to_string(), JSValue::String(flags.to_string()));
    object.set("test".to_string(), JSValue::NativeFunction(regexp_test));
    object.set("exec".to_string(), JSValue::NativeFunction(regexp_exec));
    JSValue::Object(Rc::new(RefCell::new(object)))
}

/// Installs the minimal RegExp constructor namespace.
pub fn install(global: &Rc<RefCell<JSObject>>) {
    let constructor = JSObject::new();
    global.borrow_mut().set(
        "RegExp".to_string(),
        JSValue::Object(Rc::new(RefCell::new(constructor))),
    );
}
