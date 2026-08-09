//! Minimal String constructor and prototype methods.

use std::cell::RefCell;
use std::rc::Rc;

use crate::builtins::regexp;
use crate::error::{JSError, JSResult};
use crate::value::jsobject::JSObject;
use crate::value::JSValue;
use crate::vm::VM;

fn receiver(args: &[JSValue], method: &str) -> JSResult<String> {
    match args.first() {
        Some(JSValue::Null | JSValue::Undefined) | None => Err(JSError::TypeError(format!(
            "String.prototype.{method}: invalid receiver"
        ))),
        Some(value) => Ok(value.to_string()),
    }
}

fn is_callable(value: &JSValue) -> bool {
    matches!(
        value,
        JSValue::Function(..)
            | JSValue::ArrowFunction(..)
            | JSValue::NativeFunction(..)
            | JSValue::BoundFunction(..)
    )
}

fn byte_index(input: &str, character_index: usize) -> usize {
    input
        .char_indices()
        .nth(character_index)
        .map(|(index, _)| index)
        .unwrap_or(input.len())
}

fn string_constructor(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(JSValue::String(
        args.get(1).map(JSValue::to_string).unwrap_or_default(),
    ))
}

fn string_from_char_code(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let units: Vec<u16> = args
        .iter()
        .skip(1)
        .map(|value| value.to_number() as u16)
        .collect();
    Ok(JSValue::String(String::from_utf16_lossy(&units)))
}

fn string_replace(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let input = receiver(&args, "replace")?;
    let search = args.get(1).cloned().unwrap_or(JSValue::Undefined);
    let replacement = args.get(2).cloned().unwrap_or(JSValue::Undefined);

    if let JSValue::Object(expression) = &search
        && regexp::is_regexp(expression)
    {
        let regex = regexp::compile(expression)?;
        let global = regexp::flags(expression).contains('g');
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
            return Ok(JSValue::String(input));
        }
        output.push_str(&input[last_end..]);
        return Ok(JSValue::String(output));
    }

    let needle = search.to_string();
    let Some(start) = input.find(&needle) else {
        return Ok(JSValue::String(input));
    };
    let end = start + needle.len();
    let replacement = replacement_text(
        vm,
        &replacement,
        &input,
        start,
        end,
        vec![Some(needle)],
    )?;
    Ok(JSValue::String(format!(
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
                    .map(|capture| JSValue::String(capture.clone()))
                    .unwrap_or(JSValue::Undefined)
            })
            .collect();
        arguments.push(JSValue::Number(start as f64));
        arguments.push(JSValue::String(input.to_string()));
        return Ok(vm
            .call(replacement.clone(), JSValue::Undefined, arguments)?
            .to_string());
    }

    let template = replacement.to_string();
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
        .filter(|value| !matches!(value, JSValue::Undefined))
        .map(JSValue::to_number)
        .unwrap_or(u32::MAX as f64) as usize;
    let parts: Vec<String> = match args.get(1) {
        None | Some(JSValue::Undefined) => vec![input],
        Some(JSValue::Object(expression)) if regexp::is_regexp(expression) => {
            regexp::compile(expression)?
                .split(&input)
                .take(limit)
                .map(str::to_string)
                .collect()
        }
        Some(separator) => {
            let separator = separator.to_string();
            if separator.is_empty() {
                input
                    .chars()
                    .take(limit)
                    .map(|character| character.to_string())
                    .collect()
            } else {
                input.split(&separator).take(limit).map(str::to_string).collect()
            }
        }
    };
    Ok(vm.array_from_values(parts.into_iter().map(JSValue::String).collect()))
}

fn string_trim(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(JSValue::String(receiver(&args, "trim")?.trim().to_string()))
}

fn string_match(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let input = receiver(&args, "match")?;
    let expression = match args.get(1) {
        Some(JSValue::Object(expression)) if regexp::is_regexp(expression) => Rc::clone(expression),
        value => {
            let pattern = value.map(JSValue::to_string).unwrap_or_default();
            let JSValue::Object(expression) = regexp::create(&regex::escape(&pattern), "") else {
                unreachable!();
            };
            expression
        }
    };
    let regex = regexp::compile(&expression)?;
    if regexp::flags(&expression).contains('g') {
        let matches: Vec<_> = regex
            .find_iter(&input)
            .map(|matched| JSValue::String(matched.as_str().to_string()))
            .collect();
        return if matches.is_empty() {
            Ok(JSValue::Null)
        } else {
            Ok(vm.array_from_values(matches))
        };
    }
    let Some(captures) = regex.captures(&input) else {
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
    Ok(vm.array_from_values(values))
}

fn string_includes(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let input = receiver(&args, "includes")?;
    let needle = args.get(1).map(JSValue::to_string).unwrap_or_default();
    let start = args.get(2).map(JSValue::to_number).unwrap_or(0.0).max(0.0) as usize;
    let start = byte_index(&input, start);
    Ok(JSValue::Boolean(input[start..].contains(&needle)))
}

fn string_to_lower_case(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(JSValue::String(receiver(&args, "toLowerCase")?.to_lowercase()))
}

fn string_to_upper_case(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(JSValue::String(receiver(&args, "toUpperCase")?.to_uppercase()))
}

fn string_to_string(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(JSValue::String(receiver(&args, "toString")?))
}

fn string_char_at(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let input = receiver(&args, "charAt")?;
    let index = args.get(1).map(JSValue::to_number).unwrap_or(0.0) as usize;
    Ok(JSValue::String(
        input.chars().nth(index).map(|value| value.to_string()).unwrap_or_default(),
    ))
}

fn string_substring(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let input = receiver(&args, "substring")?;
    let length = input.chars().count();
    let mut start = args.get(1).map(JSValue::to_number).unwrap_or(0.0).max(0.0) as usize;
    let mut end = args.get(2).map(JSValue::to_number).unwrap_or(length as f64).max(0.0) as usize;
    start = start.min(length);
    end = end.min(length);
    if start > end {
        std::mem::swap(&mut start, &mut end);
    }
    Ok(JSValue::String(input.chars().skip(start).take(end - start).collect()))
}

fn string_slice(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let input = receiver(&args, "slice")?;
    let length = input.chars().count() as isize;
    let normalize = |value: Option<&JSValue>, default: isize| {
        let value = value.map(JSValue::to_number).unwrap_or(default as f64) as isize;
        if value < 0 { (length + value).max(0) } else { value.min(length) }
    };
    let start = normalize(args.get(1), 0);
    let end = normalize(args.get(2), length).max(start);
    Ok(JSValue::String(input.chars().skip(start as usize).take((end - start) as usize).collect()))
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
    Ok(JSValue::Number(index))
}

/// Installs String and the methods used by React's production bundle.
pub fn install(global: &Rc<RefCell<JSObject>>) {
    let mut prototype = JSObject::new();
    prototype.set("replace".to_string(), JSValue::NativeFunction(string_replace));
    prototype.set("split".to_string(), JSValue::NativeFunction(string_split));
    prototype.set("trim".to_string(), JSValue::NativeFunction(string_trim));
    prototype.set("match".to_string(), JSValue::NativeFunction(string_match));
    prototype.set("includes".to_string(), JSValue::NativeFunction(string_includes));
    prototype.set("toLowerCase".to_string(), JSValue::NativeFunction(string_to_lower_case));
    prototype.set("toUpperCase".to_string(), JSValue::NativeFunction(string_to_upper_case));
    prototype.set("toString".to_string(), JSValue::NativeFunction(string_to_string));
    prototype.set("valueOf".to_string(), JSValue::NativeFunction(string_to_string));
    prototype.set("charAt".to_string(), JSValue::NativeFunction(string_char_at));
    prototype.set("substring".to_string(), JSValue::NativeFunction(string_substring));
    prototype.set("slice".to_string(), JSValue::NativeFunction(string_slice));
    prototype.set("indexOf".to_string(), JSValue::NativeFunction(string_index_of));

    let mut constructor = JSObject::new();
    constructor.set("__call__".to_string(), JSValue::NativeFunction(string_constructor));
    constructor.set("__construct__".to_string(), JSValue::NativeFunction(string_constructor));
    constructor.set("fromCharCode".to_string(), JSValue::NativeFunction(string_from_char_code));
    constructor.set("prototype".to_string(), JSValue::Object(Rc::new(RefCell::new(prototype))));
    global.borrow_mut().set("String".to_string(), JSValue::Object(Rc::new(RefCell::new(constructor))));
}
