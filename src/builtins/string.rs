//! Minimal String constructor and prototype methods.

use std::cell::RefCell;
use std::rc::Rc;

use crate::builtins::regexp;
use crate::error::{JSError, JSResult};
use crate::value::JSValue;
use crate::value::jsobject::JSObject;
use crate::value::jsvalue::JsValueKind;
use crate::vm::VM;

fn receiver(args: &[JSValue], method: &str) -> JSResult<String> {
    match args.first() {
        Some(value) if !matches!(value.kind(), JsValueKind::Null | JsValueKind::Undefined) => {
            Ok(value.to_string())
        }
        _ => Err(JSError::TypeError(format!(
            "String.prototype.{method}: invalid receiver"
        ))),
    }
}

fn is_callable(value: &JSValue) -> bool {
    matches!(
        value.kind(),
        JsValueKind::Function
            | JsValueKind::ArrowFunction
            | JsValueKind::NativeFunction
            | JsValueKind::BoundFunction
    )
}

fn byte_index(input: &str, character_index: usize) -> usize {
    input
        .char_indices()
        .nth(character_index)
        .map(|(index, _)| index)
        .unwrap_or(input.len())
}

fn string_constructor(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let value = match args.get(1) {
        Some(value) => vm.to_string_value(value.clone())?,
        None => String::new(),
    };
    Ok(JSValue::from_string(value))
}

fn string_from_char_code(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let units: Vec<u16> = args
        .iter()
        .skip(1)
        .map(|value| value.to_number() as u16)
        .collect();
    Ok(JSValue::from_string(String::from_utf16_lossy(&units)))
}

fn string_raw(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let template = match args.get(1) {
        Some(value) if value.kind() == JsValueKind::Object => value.as_object().unwrap(),
        _ => {
            return Err(JSError::TypeError(
                "String.raw requires a template object".to_string(),
            ));
        }
    };
    let raw = match template.borrow().get("raw") {
        value if value.kind() == JsValueKind::Object => value.as_object().unwrap().clone(),
        _ => Rc::clone(&template),
    };
    let length = raw.borrow().get("length").to_number().max(0.0) as usize;
    let mut output = String::new();
    for index in 0..length {
        output.push_str(&raw.borrow().get(&index.to_string()).to_string());
        if index + 1 < length {
            output.push_str(
                &args
                    .get(index + 2)
                    .cloned()
                    .unwrap_or(JSValue::undefined())
                    .to_string(),
            );
        }
    }
    Ok(JSValue::from_string(output))
}

fn string_concat(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let mut output = receiver(&args, "concat")?;
    for value in args.into_iter().skip(1) {
        output.push_str(&vm.to_string_value(value)?);
    }
    Ok(JSValue::from_string(output))
}

fn string_replace(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let input = receiver(&args, "replace")?;
    let search = args.get(1).cloned().unwrap_or(JSValue::undefined());
    let replacement = args.get(2).cloned().unwrap_or(JSValue::undefined());

    if JsValueKind::Object == search.kind() && regexp::is_regexp(&search.as_object().unwrap()) {
        let expression = search.as_object().unwrap();
        let regex = regexp::compile(&expression)?;
        let global = regexp::flags(&expression).contains('g');
        let mut output = String::new();
        let mut last_end = 0;
        for captures in regex.captures_iter(&input) {
            let matched = captures.get(0).expect("capture zero must exist");
            output.push_str(&input[last_end..matched.start()]);
            output.push_str(&replacement_text(
                vm,
                &replacement,
                &input,
                matched.start(),
                matched.end(),
                captures
                    .iter()
                    .map(|capture| capture.map(|capture| capture.as_str().to_string()))
                    .collect(),
            )?);
            last_end = matched.end();
            if !global {
                break;
            }
        }
        if last_end == 0 && !regex.is_match(&input) {
            return Ok(JSValue::from_string(input));
        }
        output.push_str(&input[last_end..]);
        return Ok(JSValue::from_string(output));
    }

    let needle = vm.to_string_value(search)?;
    let Some(start) = input.find(&needle) else {
        return Ok(JSValue::from_string(input));
    };
    let end = start + needle.len();
    let replacement = replacement_text(vm, &replacement, &input, start, end, vec![Some(needle)])?;
    Ok(JSValue::from_string(format!(
        "{}{}{}",
        &input[..start],
        replacement,
        &input[end..]
    )))
}

fn replacement_text(
    vm: &mut VM,
    replacement: &JSValue,
    input: &str,
    start: usize,
    end: usize,
    captures: Vec<Option<String>>,
) -> JSResult<String> {
    if is_callable(replacement) {
        let mut arguments: Vec<_> = captures
            .iter()
            .map(|capture| {
                capture
                    .as_ref()
                    .map(|capture| JSValue::from_string(capture.clone()))
                    .unwrap_or(JSValue::undefined())
            })
            .collect();
        arguments.push(JSValue::from_number(start as f64));
        arguments.push(JSValue::from_string(input.to_string()));
        let result = vm.call(replacement.clone(), JSValue::undefined(), arguments)?;
        return vm.to_string_value(result);
    }

    let template = vm.to_string_value(replacement.clone())?;
    let mut output = String::new();
    let mut characters = template.chars().peekable();
    while let Some(character) = characters.next() {
        if character != '$' {
            output.push(character);
            continue;
        }
        match characters.next() {
            Some('$') => output.push('$'),
            Some('&') => output.push_str(captures[0].as_deref().unwrap_or("")),
            Some('`') => output.push_str(&input[..start]),
            Some('\'') => output.push_str(&input[end..]),
            Some(digit @ '1'..='9') => {
                let index = digit.to_digit(10).unwrap_or(0) as usize;
                if let Some(Some(capture)) = captures.get(index) {
                    output.push_str(capture);
                }
            }
            Some(other) => {
                output.push('$');
                output.push(other);
            }
            None => output.push('$'),
        }
    }
    Ok(output)
}

fn string_split(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let input = receiver(&args, "split")?;
    let limit = args
        .get(2)
        .filter(|value| value.kind() != JsValueKind::Undefined)
        .map(JSValue::to_number)
        .unwrap_or(u32::MAX as f64) as usize;

    let parts: Vec<JSValue> = match args.get(1) {
        None => vec![JSValue::from_string(input)],
        Some(value) => match value.kind() {
            JsValueKind::Undefined => vec![JSValue::from_string(input)],
            JsValueKind::Object if regexp::is_regexp(&value.as_object().unwrap()) => {
                regexp::compile(&value.as_object().unwrap())?
                    .split(&input)
                    .take(limit)
                    .map(JSValue::from_str)
                    .collect()
            }
            _ => {
                let separator = value.to_string();

                if separator.is_empty() {
                    input.chars().take(limit).map(JSValue::from_char).collect()
                } else {
                    input
                        .split(&separator)
                        .take(limit)
                        .map(JSValue::from_str)
                        .collect()
                }
            }
        },
    };

    Ok(vm.array_from_values(parts))
}

fn string_trim(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(JSValue::from_string(
        receiver(&args, "trim")?.trim().to_string(),
    ))
}

fn string_match(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let input = receiver(&args, "match")?;
    let expression = match args.get(1) {
        Some(value)
            if value.kind() == JsValueKind::Object
                && regexp::is_regexp(&value.as_object().unwrap()) =>
        {
            Rc::clone(&value.as_object().unwrap())
        }
        value => {
            let pattern = value.map(JSValue::to_string).unwrap_or_default();
            let expression = regexp::create(&regex::escape(&pattern), "");

            match expression.kind() {
                JsValueKind::Object => expression.as_object().unwrap().clone(),
                _ => unreachable!(),
            }
        }
    };
    let regex = regexp::compile(&expression)?;
    if regexp::flags(&expression).contains('g') {
        let matches: Vec<_> = regex
            .find_iter(&input)
            .map(|matched| JSValue::from_str(matched.as_str()))
            .collect();
        return if matches.is_empty() {
            Ok(JSValue::null())
        } else {
            Ok(vm.array_from_values(matches))
        };
    }
    let Some(captures) = regex.captures(&input) else {
        return Ok(JSValue::null());
    };
    let values = captures
        .iter()
        .map(|capture| {
            capture
                .map(|capture| JSValue::from_string(capture.as_str().to_string()))
                .unwrap_or(JSValue::undefined())
        })
        .collect();
    Ok(vm.array_from_values(values))
}

fn string_includes(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let input = receiver(&args, "includes")?;
    let needle = args.get(1).map(JSValue::to_string).unwrap_or_default();
    let start = args.get(2).map(JSValue::to_number).unwrap_or(0.0).max(0.0) as usize;
    let start = byte_index(&input, start);
    Ok(JSValue::from_bool(input[start..].contains(&needle)))
}

fn string_starts_with(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let input = receiver(&args, "startsWith")?;
    let needle = args.get(1).map(JSValue::to_string).unwrap_or_default();
    let start = args.get(2).map(JSValue::to_number).unwrap_or(0.0).max(0.0) as usize;
    let start = byte_index(&input, start);
    Ok(JSValue::from_bool(input[start..].starts_with(&needle)))
}

fn string_ends_with(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let input = receiver(&args, "endsWith")?;
    let needle = args.get(1).map(JSValue::to_string).unwrap_or_default();
    let length = input.chars().count();
    let end = args
        .get(2)
        .map(JSValue::to_number)
        .unwrap_or(length as f64)
        .max(0.0) as usize;
    let end = byte_index(&input, end.min(length));
    Ok(JSValue::from_bool(input[..end].ends_with(&needle)))
}

fn string_to_lower_case(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(JSValue::from_string(
        receiver(&args, "toLowerCase")?.to_lowercase(),
    ))
}

fn string_to_upper_case(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(JSValue::from_string(
        receiver(&args, "toUpperCase")?.to_uppercase(),
    ))
}

fn string_to_string(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(JSValue::from_string(receiver(&args, "toString")?))
}

fn string_char_at(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let input = receiver(&args, "charAt")?;
    let index = args.get(1).map(JSValue::to_number).unwrap_or(0.0) as usize;
    Ok(JSValue::from_string(
        input
            .chars()
            .nth(index)
            .map(|value| value.to_string())
            .unwrap_or_default(),
    ))
}

fn string_char_code_at(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let input = receiver(&args, "charCodeAt")?;
    let index = args.get(1).map(JSValue::to_number).unwrap_or(0.0);
    if !index.is_finite() || index < 0.0 {
        return Ok(JSValue::from_number(f64::NAN));
    }
    Ok(JSValue::from_number(
        input
            .encode_utf16()
            .nth(index.trunc() as usize)
            .map(f64::from)
            .unwrap_or(f64::NAN),
    ))
}

fn string_substring(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let input = receiver(&args, "substring")?;
    let length = input.chars().count();
    let mut start = args.get(1).map(JSValue::to_number).unwrap_or(0.0).max(0.0) as usize;
    let mut end = args
        .get(2)
        .map(JSValue::to_number)
        .unwrap_or(length as f64)
        .max(0.0) as usize;
    start = start.min(length);
    end = end.min(length);
    if start > end {
        std::mem::swap(&mut start, &mut end);
    }
    Ok(JSValue::from_string(
        input.chars().skip(start).take(end - start).collect(),
    ))
}

fn string_slice(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let input = receiver(&args, "slice")?;
    let length = input.chars().count() as isize;
    let normalize = |value: Option<&JSValue>, default: isize| {
        let value = value.map(JSValue::to_number).unwrap_or(default as f64) as isize;
        if value < 0 {
            (length + value).max(0)
        } else {
            value.min(length)
        }
    };
    let start = normalize(args.get(1), 0);
    let end = normalize(args.get(2), length).max(start);
    Ok(JSValue::from_string(
        input
            .chars()
            .skip(start as usize)
            .take((end - start) as usize)
            .collect(),
    ))
}

fn string_index_of(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let input = receiver(&args, "indexOf")?;
    let needle = args.get(1).map(JSValue::to_string).unwrap_or_default();
    let start = args.get(2).map(JSValue::to_number).unwrap_or(0.0).max(0.0) as usize;
    let start = byte_index(&input, start);
    let index = input
        .get(start..)
        .and_then(|suffix| suffix.find(&needle).map(|index| index + start))
        .map(|index| input[..index].encode_utf16().count() as f64)
        .unwrap_or(-1.0);
    Ok(JSValue::from_number(index))
}

fn string_last_index_of(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let input = receiver(&args, "lastIndexOf")?;
    let input: Vec<u16> = input.encode_utf16().collect();
    let needle: Vec<u16> = args
        .get(1)
        .map(JSValue::to_string)
        .unwrap_or_default()
        .encode_utf16()
        .collect();
    let position = match args.get(2) {
        None => input.len(),
        Some(value) => {
            let value = value.to_number();
            if value.is_nan() || value <= 0.0 {
                0
            } else if value.is_infinite() {
                input.len()
            } else {
                (value.trunc() as usize).min(input.len())
            }
        }
    };
    if needle.is_empty() {
        return Ok(JSValue::from_number(position as f64));
    }
    if needle.len() > input.len() {
        return Ok(JSValue::from_number(-1.0));
    }
    let last_start = position.min(input.len() - needle.len());
    let index = (0..=last_start)
        .rev()
        .find(|start| input[*start..*start + needle.len()] == needle)
        .map(|index| index as f64)
        .unwrap_or(-1.0);
    Ok(JSValue::from_number(index))
}

/// Installs String and the methods used by React's production bundle.
pub fn install(global: &Rc<RefCell<JSObject>>) {
    let mut prototype = JSObject::new();
    prototype.set(
        "replace".to_string(),
        JSValue::from_native_function(string_replace),
    );
    prototype.set(
        "concat".to_string(),
        JSValue::from_native_function(string_concat),
    );
    prototype.set(
        "split".to_string(),
        JSValue::from_native_function(string_split),
    );
    prototype.set(
        "trim".to_string(),
        JSValue::from_native_function(string_trim),
    );
    prototype.set(
        "match".to_string(),
        JSValue::from_native_function(string_match),
    );
    prototype.set(
        "includes".to_string(),
        JSValue::from_native_function(string_includes),
    );
    prototype.set(
        "startsWith".to_string(),
        JSValue::from_native_function(string_starts_with),
    );
    prototype.set(
        "endsWith".to_string(),
        JSValue::from_native_function(string_ends_with),
    );
    prototype.set(
        "toLowerCase".to_string(),
        JSValue::from_native_function(string_to_lower_case),
    );
    prototype.set(
        "toUpperCase".to_string(),
        JSValue::from_native_function(string_to_upper_case),
    );
    prototype.set(
        "toString".to_string(),
        JSValue::from_native_function(string_to_string),
    );
    prototype.set(
        "valueOf".to_string(),
        JSValue::from_native_function(string_to_string),
    );
    prototype.set(
        "charAt".to_string(),
        JSValue::from_native_function(string_char_at),
    );
    prototype.set(
        "charCodeAt".to_string(),
        JSValue::from_native_function(string_char_code_at),
    );
    prototype.set(
        "substring".to_string(),
        JSValue::from_native_function(string_substring),
    );
    prototype.set(
        "slice".to_string(),
        JSValue::from_native_function(string_slice),
    );
    prototype.set(
        "indexOf".to_string(),
        JSValue::from_native_function(string_index_of),
    );
    prototype.set(
        "lastIndexOf".to_string(),
        JSValue::from_native_function(string_last_index_of),
    );

    let mut constructor = JSObject::new();
    constructor.set(
        "__call__".to_string(),
        JSValue::from_native_function(string_constructor),
    );
    constructor.set(
        "__construct__".to_string(),
        JSValue::from_native_function(string_constructor),
    );
    constructor.set(
        "fromCharCode".to_string(),
        JSValue::from_native_function(string_from_char_code),
    );
    constructor.set("raw".to_string(), JSValue::from_native_function(string_raw));
    constructor.set(
        "prototype".to_string(),
        JSValue::from_object(Rc::new(RefCell::new(prototype))),
    );
    global.borrow_mut().set(
        "String".to_string(),
        JSValue::from_object(Rc::new(RefCell::new(constructor))),
    );
}
