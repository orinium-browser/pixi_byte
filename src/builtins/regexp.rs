//! Minimal RegExp objects used by regular expression literals.

use std::cell::RefCell;
use std::rc::Rc;

use regex::RegexBuilder;

use crate::error::{JSError, JSResult};
use crate::value::JSValue;
use crate::value::jsobject::JSObject;
use crate::vm::VM;

const PATTERN: &str = "__regexp_pattern";
const FLAGS: &str = "__regexp_flags";

fn normalize_pattern(pattern: &str) -> String {
    let characters: Vec<char> = pattern.chars().collect();
    let mut output = String::with_capacity(pattern.len());
    let mut index = 0;
    while index < characters.len() {
        if characters[index] != '\\' {
            output.push(characters[index]);
            index += 1;
            continue;
        }
        if characters.get(index + 1) == Some(&'\\') {
            output.push('\\');
            output.push('\\');
            index += 2;
            continue;
        }
        if characters.get(index + 1) == Some(&'u')
            && index + 5 < characters.len()
            && characters[index + 2..=index + 5]
                .iter()
                .all(|character| character.is_ascii_hexdigit())
        {
            output.push_str("\\x{");
            output.extend(characters[index + 2..=index + 5].iter().copied());
            output.push('}');
            index += 6;
            continue;
        }
        output.push('\\');
        index += 1;
    }
    strip_unsupported_lookarounds(&normalize_braces(&normalize_character_classes(&output)))
}

fn normalize_braces(pattern: &str) -> String {
    let characters = pattern.chars().collect::<Vec<_>>();
    let mut output = String::with_capacity(pattern.len());
    let mut index = 0;
    let mut in_class = false;
    while index < characters.len() {
        match characters[index] {
            '\\' => {
                output.push('\\');
                index += 1;
                if let Some(character) = characters.get(index) {
                    output.push(*character);
                    index += 1;
                }
            }
            '[' => {
                in_class = true;
                output.push('[');
                index += 1;
            }
            ']' if in_class => {
                in_class = false;
                output.push(']');
                index += 1;
            }
            '{' if !in_class => {
                let closing = characters[index + 1..]
                    .iter()
                    .position(|character| *character == '}')
                    .map(|offset| index + 1 + offset);
                let valid_unicode_escape = output.ends_with("\\x")
                    && closing.is_some_and(|closing| {
                        let content = &characters[index + 1..closing];
                        !content.is_empty()
                            && content
                                .iter()
                                .all(|character| character.is_ascii_hexdigit())
                    });
                let valid_quantifier = closing.is_some_and(|closing| {
                    let content = characters[index + 1..closing].iter().collect::<String>();
                    let mut parts = content.split(',');
                    let minimum = parts.next().unwrap_or_default();
                    let maximum = parts.next();
                    !minimum.is_empty()
                        && minimum.chars().all(|character| character.is_ascii_digit())
                        && maximum.is_none_or(|maximum| {
                            maximum.chars().all(|character| character.is_ascii_digit())
                        })
                        && parts.next().is_none()
                });
                if valid_unicode_escape || valid_quantifier {
                    let closing = closing.unwrap();
                    output.extend(characters[index..=closing].iter());
                    index = closing + 1;
                } else {
                    output.push_str("\\{");
                    index += 1;
                }
            }
            '}' if !in_class => {
                output.push_str("\\}");
                index += 1;
            }
            character => {
                output.push(character);
                index += 1;
            }
        }
    }
    output
}

fn normalize_character_classes(pattern: &str) -> String {
    let mut output = String::with_capacity(pattern.len());
    let mut in_class = false;
    let mut escaped = false;
    for character in pattern.chars() {
        if escaped {
            output.push(character);
            escaped = false;
            continue;
        }
        if character == '\\' {
            output.push(character);
            escaped = true;
            continue;
        }
        match character {
            '[' if in_class => output.push_str("\\["),
            '[' => {
                in_class = true;
                output.push(character);
            }
            ']' if in_class => {
                in_class = false;
                output.push(character);
            }
            _ => output.push(character),
        }
    }
    output
}

fn strip_unsupported_lookarounds(pattern: &str) -> String {
    let characters: Vec<char> = pattern.chars().collect();
    let mut output = String::with_capacity(pattern.len());
    let mut index = 0;
    while index < characters.len() {
        let assertion_prefix = characters.get(index) == Some(&'(')
            && characters.get(index + 1) == Some(&'?')
            && (matches!(characters.get(index + 2), Some('=' | '!'))
                || (characters.get(index + 2) == Some(&'<')
                    && matches!(characters.get(index + 3), Some('=' | '!'))));
        if !assertion_prefix {
            output.push(characters[index]);
            index += 1;
            continue;
        }

        let mut depth = 1usize;
        let mut escaped = false;
        let mut in_class = false;
        index += if characters.get(index + 2) == Some(&'<') {
            4
        } else {
            3
        };
        while index < characters.len() && depth > 0 {
            let character = characters[index];
            index += 1;
            if escaped {
                escaped = false;
                continue;
            }
            if character == '\\' {
                escaped = true;
                continue;
            }
            if character == '[' && !in_class {
                in_class = true;
                continue;
            }
            if character == ']' && in_class {
                in_class = false;
                continue;
            }
            if in_class {
                continue;
            }
            if character == '(' {
                depth += 1;
            } else if character == ')' {
                depth -= 1;
            }
        }
    }
    output
}

pub(crate) fn compile(object: &Rc<RefCell<JSObject>>) -> JSResult<regex::Regex> {
    let pattern = object.borrow().get(PATTERN).to_string();
    let flags = object.borrow().get(FLAGS).to_string();
    let pattern = normalize_pattern(&pattern);
    let mut builder = RegexBuilder::new(&pattern);
    builder
        .case_insensitive(flags.contains('i'))
        .multi_line(flags.contains('m'))
        .dot_matches_new_line(flags.contains('s'));
    builder.build().map_err(|error| {
        JSError::SyntaxError(error.to_string(), crate::lexer::Span::new(0, 0, 0, 0))
    })
}

pub(crate) fn flags(object: &Rc<RefCell<JSObject>>) -> String {
    object.borrow().get(FLAGS).to_string()
}

pub(crate) fn is_regexp(object: &Rc<RefCell<JSObject>>) -> bool {
    object.borrow().has_own_property(PATTERN)
}

fn regexp_test(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let Some(JSValue::Object(object)) = args.first() else {
        return Err(JSError::TypeError(
            "RegExp.test: invalid receiver".to_string(),
        ));
    };
    let input = args.get(1).map(JSValue::to_string).unwrap_or_default();
    let expression = compile(object)?;
    let flags = flags(object);
    let stateful = flags.contains('g') || flags.contains('y');
    let start = if stateful {
        let last_index = object.borrow().get("lastIndex").to_number().max(0.0) as usize;
        match byte_offset_for_utf16(&input, last_index) {
            Some(start) => start,
            None => {
                object
                    .borrow_mut()
                    .set("lastIndex".to_string(), JSValue::Number(0.0));
                return Ok(JSValue::Boolean(false));
            }
        }
    } else {
        0
    };
    let matched = expression
        .find_at(&input, start)
        .filter(|matched| !flags.contains('y') || matched.start() == start);
    if stateful {
        let last_index = matched
            .map(|matched| utf16_offset(&input, matched.end()) as f64)
            .unwrap_or(0.0);
        object
            .borrow_mut()
            .set("lastIndex".to_string(), JSValue::Number(last_index));
    }
    Ok(JSValue::Boolean(matched.is_some()))
}

fn regexp_exec(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let Some(JSValue::Object(object)) = args.first() else {
        return Err(JSError::TypeError(
            "RegExp.exec: invalid receiver".to_string(),
        ));
    };
    let input = args.get(1).map(JSValue::to_string).unwrap_or_default();
    let expression = compile(object)?;
    let flags = flags(object);
    let stateful = flags.contains('g') || flags.contains('y');
    let start = if stateful {
        let last_index = object.borrow().get("lastIndex").to_number().max(0.0) as usize;
        let Some(start) = byte_offset_for_utf16(&input, last_index) else {
            object
                .borrow_mut()
                .set("lastIndex".to_string(), JSValue::Number(0.0));
            return Ok(JSValue::Null);
        };
        start
    } else {
        0
    };
    let captures = expression.captures_at(&input, start).filter(|captures| {
        !flags.contains('y') || captures.get(0).is_some_and(|m| m.start() == start)
    });
    let Some(captures) = captures else {
        if stateful {
            object
                .borrow_mut()
                .set("lastIndex".to_string(), JSValue::Number(0.0));
        }
        return Ok(JSValue::Null);
    };
    if stateful {
        let last_index = captures
            .get(0)
            .map(|matched| utf16_offset(&input, matched.end()) as f64)
            .unwrap_or(0.0);
        object
            .borrow_mut()
            .set("lastIndex".to_string(), JSValue::Number(last_index));
    }
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

fn byte_offset_for_utf16(input: &str, target: usize) -> Option<usize> {
    let mut offset = 0;
    for (byte, character) in input.char_indices() {
        if offset == target {
            return Some(byte);
        }
        offset += character.len_utf16();
        if offset > target {
            return None;
        }
    }
    (offset == target).then_some(input.len())
}

fn utf16_offset(input: &str, byte: usize) -> usize {
    input[..byte].encode_utf16().count()
}

/// Creates a RegExp object for a parsed literal.
pub fn create(pattern: &str, flags: &str) -> JSValue {
    let mut object = JSObject::new();
    object.set(PATTERN.to_string(), JSValue::String(pattern.to_string()));
    object.set(FLAGS.to_string(), JSValue::String(flags.to_string()));
    object.set("source".to_string(), JSValue::String(pattern.to_string()));
    object.set("flags".to_string(), JSValue::String(flags.to_string()));
    object.set("global".to_string(), JSValue::Boolean(flags.contains('g')));
    object.set(
        "ignoreCase".to_string(),
        JSValue::Boolean(flags.contains('i')),
    );
    object.set(
        "multiline".to_string(),
        JSValue::Boolean(flags.contains('m')),
    );
    object.set("dotAll".to_string(), JSValue::Boolean(flags.contains('s')));
    object.set("unicode".to_string(), JSValue::Boolean(flags.contains('u')));
    object.set("sticky".to_string(), JSValue::Boolean(flags.contains('y')));
    object.set("lastIndex".to_string(), JSValue::Number(0.0));
    object.set("test".to_string(), JSValue::NativeFunction(regexp_test));
    object.set("exec".to_string(), JSValue::NativeFunction(regexp_exec));
    JSValue::Object(Rc::new(RefCell::new(object)))
}

fn regexp_constructor(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let pattern = match args.get(1) {
        Some(JSValue::Object(object)) if is_regexp(object) => {
            object.borrow().get(PATTERN).to_string()
        }
        Some(JSValue::Undefined) | None => String::new(),
        Some(value) => value.to_string(),
    };
    let flags = match args.get(2) {
        Some(JSValue::Undefined) | None => match args.get(1) {
            Some(JSValue::Object(object)) if is_regexp(object) => {
                object.borrow().get(FLAGS).to_string()
            }
            _ => String::new(),
        },
        Some(value) => value.to_string(),
    };
    let value = create(&pattern, &flags);
    if let JSValue::Object(object) = &value {
        compile(object)?;
    }
    Ok(value)
}

/// Installs the minimal RegExp constructor namespace.
pub fn install(global: &Rc<RefCell<JSObject>>) {
    let mut constructor = JSObject::new();
    constructor.set(
        "__construct__".to_string(),
        JSValue::NativeFunction(regexp_constructor),
    );
    constructor.set(
        "__call__".to_string(),
        JSValue::NativeFunction(regexp_constructor),
    );
    global.borrow_mut().set(
        "RegExp".to_string(),
        JSValue::Object(Rc::new(RefCell::new(constructor))),
    );
}
