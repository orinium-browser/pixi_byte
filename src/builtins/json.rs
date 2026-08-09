use crate::error::{JSError, JSResult};
use crate::value::JSValue;
use crate::value::jsobject::JSObject;
use crate::vm::VM;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

fn quote(value: &str) -> String {
    let mut result = String::with_capacity(value.len() + 2);
    result.push('"');
    for character in value.chars() {
        match character {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\u{08}' => result.push_str("\\b"),
            '\u{0c}' => result.push_str("\\f"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            character if character <= '\u{1f}' => {
                result.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => result.push(character),
        }
    }
    result.push('"');
    result
}

fn serialize(
    value: &JSValue,
    stack: &mut HashSet<usize>,
    in_array: bool,
) -> JSResult<Option<String>> {
    match value {
        JSValue::Undefined
        | JSValue::Function(..)
        | JSValue::ArrowFunction(..)
        | JSValue::NativeFunction(_)
        | JSValue::BoundFunction(..) => Ok(in_array.then(|| "null".to_string())),
        JSValue::Null => Ok(Some("null".to_string())),
        JSValue::Boolean(value) => Ok(Some(value.to_string())),
        JSValue::Number(value) => {
            if value.is_finite() {
                Ok(Some(JSValue::Number(*value).to_string()))
            } else {
                Ok(Some("null".to_string()))
            }
        }
        JSValue::String(value) => Ok(Some(quote(value))),
        JSValue::Object(object) => {
            let identity = Rc::as_ptr(object) as usize;
            if !stack.insert(identity) {
                return Err(JSError::TypeError(
                    "Converting circular structure to JSON".to_string(),
                ));
            }

            let (is_array, entries) = {
                let object = object.borrow();
                let is_array = matches!(object.get("__pixi_array__"), JSValue::Boolean(true));
                if is_array {
                    let length = object.get("length").to_number().max(0.0) as usize;
                    let entries = (0..length)
                        .map(|index| (String::new(), object.get(&index.to_string())))
                        .collect();
                    (true, entries)
                } else {
                    let entries = object
                        .keys()
                        .into_iter()
                        .map(|key| {
                            let value = object.get(&key);
                            (key, value)
                        })
                        .collect();
                    (false, entries)
                }
            };

            let result = if is_array {
                let values = entries
                    .iter()
                    .map(|(_, value)| serialize(value, stack, true))
                    .collect::<JSResult<Vec<_>>>()?
                    .into_iter()
                    .map(|value| value.unwrap_or_else(|| "null".to_string()))
                    .collect::<Vec<_>>();
                format!("[{}]", values.join(","))
            } else {
                let mut properties = Vec::new();
                for (key, value) in entries {
                    if let Some(value) = serialize(&value, stack, false)? {
                        properties.push(format!("{}:{}", quote(&key), value));
                    }
                }
                format!("{{{}}}", properties.join(","))
            };
            stack.remove(&identity);
            Ok(Some(result))
        }
    }
}

fn json_stringify(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let value = args.get(1).unwrap_or(&JSValue::Undefined);
    Ok(serialize(value, &mut HashSet::new(), false)?
        .map(JSValue::String)
        .unwrap_or(JSValue::Undefined))
}

pub fn install(global: &Rc<RefCell<JSObject>>) {
    let mut json = JSObject::new();
    json.set(
        "stringify".to_string(),
        JSValue::NativeFunction(json_stringify),
    );
    global.borrow_mut().set(
        "JSON".to_string(),
        JSValue::Object(Rc::new(RefCell::new(json))),
    );
}
