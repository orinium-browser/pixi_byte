//! Minimal RegExp objects used by regular expression literals.

use std::cell::RefCell;
use std::rc::Rc;

use regex::RegexBuilder;

use crate::error::{JSError, JSResult};
use crate::value::JSValue;
use crate::value::jsobject::JSObject;
use crate::value::jsvalue::JsValueKind;
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
    normalize_braces(&normalize_legacy_character_class_ranges(
        &normalize_character_classes(&output),
    ))
}

fn normalize_legacy_character_class_ranges(pattern: &str) -> String {
    let characters = pattern.chars().collect::<Vec<_>>();
    let mut output = String::with_capacity(pattern.len());
    let mut index = 0;
    let mut in_class = false;
    let mut previous_was_class_escape = false;

    while index < characters.len() {
        let character = characters[index];
        if character == '\\' {
            output.push(character);
            if let Some(escaped) = characters.get(index + 1) {
                output.push(*escaped);
                previous_was_class_escape =
                    in_class && matches!(escaped, 'd' | 'D' | 's' | 'S' | 'w' | 'W');
                index += 2;
            } else {
                index += 1;
            }
            continue;
        }

        match character {
            '[' if !in_class => {
                in_class = true;
                previous_was_class_escape = false;
                output.push(character);
            }
            ']' if in_class => {
                in_class = false;
                previous_was_class_escape = false;
                output.push(character);
            }
            '-' if in_class => {
                let next_is_class_escape = characters.get(index + 1) == Some(&'\\')
                    && matches!(
                        characters.get(index + 2),
                        Some('d' | 'D' | 's' | 'S' | 'w' | 'W')
                    );
                if previous_was_class_escape || next_is_class_escape {
                    output.push_str("\\-");
                } else {
                    output.push(character);
                }
                previous_was_class_escape = false;
            }
            _ => {
                output.push(character);
                previous_was_class_escape = false;
            }
        }
        index += 1;
    }

    output
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

pub(crate) fn compile(object: &Rc<RefCell<JSObject>>) -> JSResult<CompiledRegex> {
    let pattern = object.borrow().get(PATTERN).to_string();
    let flags = object.borrow().get(FLAGS).to_string();
    let pattern = normalize_pattern(&pattern);

    // Fast path: the linear-time `regex` crate handles the vast majority of
    // patterns and is the engine's preferred matcher.
    if let Ok(regex) = build_native(&pattern, &flags) {
        return Ok(CompiledRegex::Native(regex));
    }
    // `regex` rejects `\x{D800}`-style surrogate escapes (code units are not
    // Unicode scalar values). Swap them for U+FFFD and retry.
    let sanitized = sanitize_surrogate_escapes(&pattern);
    if sanitized != pattern {
        if let Ok(regex) = build_native(&sanitized, &flags) {
            return Ok(CompiledRegex::Native(regex));
        }
    }
    // Slow path: look-around and backreferences are not supported by `regex`,
    // so fall back to the backtracking `fancy-regex` engine. Real-world bundles
    // (e.g. es-abstract/get-intrinsic) rely on these constructs.
    if let Some(regex) = build_fancy(&pattern, &flags) {
        return Ok(CompiledRegex::Fancy(regex));
    }
    if sanitized != pattern {
        if let Some(regex) = build_fancy(&sanitized, &flags) {
            return Ok(CompiledRegex::Fancy(regex));
        }
    }
    // Last resort: never-match regex so pattern failures degrade to "no match"
    // instead of throwing out of the program.
    Ok(CompiledRegex::Native(
        RegexBuilder::new(r"\b\B")
            .build()
            .expect("never-matching fallback always compiles"),
    ))
}

fn build_native(pattern: &str, flags: &str) -> Result<regex::Regex, regex::Error> {
    let mut builder = RegexBuilder::new(pattern);
    builder
        .case_insensitive(flags.contains('i'))
        .multi_line(flags.contains('m'))
        .dot_matches_new_line(flags.contains('s'));
    builder.build()
}

fn build_fancy(pattern: &str, flags: &str) -> Option<fancy_regex::Regex> {
    // `fancy_regex::RegexBuilder` does not expose the `m`/`s` options, so apply
    // them (and `i`, harmlessly) as inline flags at the front of the pattern.
    let mut inline = String::new();
    if flags.contains('i') {
        inline.push('i');
    }
    if flags.contains('m') {
        inline.push('m');
    }
    if flags.contains('s') {
        inline.push('s');
    }
    let pattern = if inline.is_empty() {
        pattern.to_string()
    } else {
        format!("(?{inline}){pattern}")
    };
    fancy_regex::Regex::new(&pattern).ok()
}

/// A compiled regular expression backed by either the linear-time `regex`
/// engine or the backtracking `fancy-regex` engine (for look-around and
/// backreferences). Byte ranges are exposed so callers do not depend on the
/// concrete engine's capture types.
pub(crate) enum CompiledRegex {
    Native(regex::Regex),
    Fancy(fancy_regex::Regex),
}

pub(crate) type ByteSpan = (usize, usize);

fn native_capture_spans(captures: &regex::Captures<'_>) -> Vec<Option<ByteSpan>> {
    captures
        .iter()
        .map(|capture| capture.map(|matched| (matched.start(), matched.end())))
        .collect()
}

fn fancy_capture_spans(captures: &fancy_regex::Captures<'_>) -> Vec<Option<ByteSpan>> {
    captures
        .iter()
        .map(|capture| capture.map(|matched| (matched.start(), matched.end())))
        .collect()
}

impl CompiledRegex {
    pub(crate) fn is_match(&self, input: &str) -> bool {
        match self {
            Self::Native(regex) => regex.is_match(input),
            Self::Fancy(regex) => regex.is_match(input).unwrap_or(false),
        }
    }

    pub(crate) fn find_at(&self, input: &str, start: usize) -> Option<ByteSpan> {
        match self {
            Self::Native(regex) => regex.find_at(input, start).map(|m| (m.start(), m.end())),
            Self::Fancy(regex) => regex
                .find_from_pos(input, start)
                .ok()
                .flatten()
                .map(|m| (m.start(), m.end())),
        }
    }

    pub(crate) fn captures_at(&self, input: &str, start: usize) -> Option<Vec<Option<ByteSpan>>> {
        match self {
            Self::Native(regex) => regex
                .captures_at(input, start)
                .map(|captures| native_capture_spans(&captures)),
            Self::Fancy(regex) => regex
                .captures_from_pos(input, start)
                .ok()
                .flatten()
                .map(|captures| fancy_capture_spans(&captures)),
        }
    }

    pub(crate) fn find_iter(&self, input: &str) -> Vec<ByteSpan> {
        match self {
            Self::Native(regex) => regex
                .find_iter(input)
                .map(|matched| (matched.start(), matched.end()))
                .collect(),
            Self::Fancy(regex) => regex
                .find_iter(input)
                .filter_map(Result::ok)
                .map(|matched| (matched.start(), matched.end()))
                .collect(),
        }
    }

    pub(crate) fn captures_iter(&self, input: &str) -> Vec<Vec<Option<ByteSpan>>> {
        match self {
            Self::Native(regex) => regex
                .captures_iter(input)
                .map(|captures| native_capture_spans(&captures))
                .collect(),
            Self::Fancy(regex) => regex
                .captures_iter(input)
                .filter_map(Result::ok)
                .map(|captures| fancy_capture_spans(&captures))
                .collect(),
        }
    }

    pub(crate) fn split(&self, input: &str) -> Vec<ByteSpan> {
        let offset = input.as_ptr() as usize;
        match self {
            Self::Native(regex) => regex
                .split(input)
                .map(|part| {
                    let start = part.as_ptr() as usize - offset;
                    (start, start + part.len())
                })
                .collect(),
            Self::Fancy(regex) => regex
                .split(input)
                .filter_map(Result::ok)
                .map(|part| {
                    let start = part.as_ptr() as usize - offset;
                    (start, start + part.len())
                })
                .collect(),
        }
    }
}

/// Replaces `\x{D…}` surrogate escapes (which are not Unicode scalar values)
/// with the replacement character so Rust `regex` can still build the pattern.
fn sanitize_surrogate_escapes(pattern: &str) -> String {
    let mut output = String::with_capacity(pattern.len());
    let characters = pattern.chars().collect::<Vec<_>>();
    let mut index = 0;
    while index < characters.len() {
        if characters[index] == '\\'
            && characters.get(index + 1) == Some(&'x')
            && characters.get(index + 2) == Some(&'{')
        {
            let open = index + 3;
            let mut end = open;
            while end < characters.len() && characters[end] != '}' {
                end += 1;
            }
            if end < characters.len()
                && open < end
                && characters[open..end]
                    .iter()
                    .all(|character| character.is_ascii_hexdigit())
            {
                let value =
                    u32::from_str_radix(&characters[open..end].iter().collect::<String>(), 16)
                        .unwrap_or(0);
                if (0xD800..=0xDFFF).contains(&value) {
                    output.push('\u{FFFD}');
                    index = end + 1;
                    continue;
                }
            }
        }
        output.push(characters[index]);
        index += 1;
    }
    output
}

pub(crate) fn flags(object: &Rc<RefCell<JSObject>>) -> String {
    object.borrow().get(FLAGS).to_string()
}

pub(crate) fn is_regexp(object: &Rc<RefCell<JSObject>>) -> bool {
    object.borrow().has_own_property(PATTERN)
}

fn regexp_test(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let object = match args.first() {
        Some(value) if value.kind() == JsValueKind::Object => value.as_object().unwrap(),
        _ => {
            return Err(JSError::TypeError(
                "RegExp.test: invalid receiver".to_string(),
            ));
        }
    };
    let input = args.get(1).map(JSValue::to_string).unwrap_or_default();
    let expression = compile(&object)?;
    let flags = flags(&object);
    let stateful = flags.contains('g') || flags.contains('y');
    let start = if stateful {
        let last_index = object.borrow().get("lastIndex").to_number().max(0.0) as usize;
        match byte_offset_for_utf16(&input, last_index) {
            Some(start) => start,
            None => {
                object
                    .borrow_mut()
                    .set("lastIndex".to_string(), JSValue::from_number(0.0));
                return Ok(JSValue::from_bool(false));
            }
        }
    } else {
        0
    };
    let matched = expression
        .find_at(&input, start)
        .filter(|(match_start, _)| !flags.contains('y') || *match_start == start);
    if stateful {
        let last_index = matched
            .map(|(_, end)| utf16_offset(&input, end) as f64)
            .unwrap_or(0.0);
        object
            .borrow_mut()
            .set("lastIndex".to_string(), JSValue::from_number(last_index));
    }
    Ok(JSValue::from_bool(matched.is_some()))
}

fn regexp_exec(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let object = match args.first() {
        Some(value) if value.kind() == JsValueKind::Object => value.as_object().unwrap(),
        _ => {
            return Err(JSError::TypeError(
                "RegExp.exec: invalid receiver".to_string(),
            ));
        }
    };
    let input = args.get(1).map(JSValue::to_string).unwrap_or_default();
    let expression = compile(&object)?;
    let flags = flags(&object);
    let stateful = flags.contains('g') || flags.contains('y');
    let start = if stateful {
        let last_index = object.borrow().get("lastIndex").to_number().max(0.0) as usize;
        let Some(start) = byte_offset_for_utf16(&input, last_index) else {
            object
                .borrow_mut()
                .set("lastIndex".to_string(), JSValue::from_number(0.0));
            return Ok(JSValue::null());
        };
        start
    } else {
        0
    };
    let captures = expression.captures_at(&input, start).filter(|captures| {
        !flags.contains('y')
            || captures
                .first()
                .and_then(|capture| *capture)
                .is_some_and(|(match_start, _)| match_start == start)
    });
    let Some(captures) = captures else {
        if stateful {
            object
                .borrow_mut()
                .set("lastIndex".to_string(), JSValue::from_number(0.0));
        }
        return Ok(JSValue::null());
    };
    if stateful {
        let last_index = captures
            .first()
            .and_then(|capture| *capture)
            .map(|(_, end)| utf16_offset(&input, end) as f64)
            .unwrap_or(0.0);
        object
            .borrow_mut()
            .set("lastIndex".to_string(), JSValue::from_number(last_index));
    }
    let values = captures
        .iter()
        .map(|capture| {
            capture
                .map(|(start, end)| JSValue::from_string(input[start..end].to_string()))
                .unwrap_or(JSValue::undefined())
        })
        .collect();
    let result = vm.array_from_values(values);
    if result.kind() == JsValueKind::Object {
        let result = result.as_object().unwrap();
        let index = captures
            .first()
            .and_then(|capture| *capture)
            .map(|(start, _)| start)
            .unwrap_or(0);
        result
            .borrow_mut()
            .set("index".to_string(), JSValue::from_number(index as f64));
        result
            .borrow_mut()
            .set("input".to_string(), JSValue::from_string(input));
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
    object.set(
        PATTERN.to_string(),
        JSValue::from_string(pattern.to_string()),
    );
    object.set(FLAGS.to_string(), JSValue::from_string(flags.to_string()));
    object.set(
        "source".to_string(),
        JSValue::from_string(pattern.to_string()),
    );
    object.set("flags".to_string(), JSValue::from_string(flags.to_string()));
    object.set(
        "global".to_string(),
        JSValue::from_bool(flags.contains('g')),
    );
    object.set(
        "ignoreCase".to_string(),
        JSValue::from_bool(flags.contains('i')),
    );
    object.set(
        "multiline".to_string(),
        JSValue::from_bool(flags.contains('m')),
    );
    object.set(
        "dotAll".to_string(),
        JSValue::from_bool(flags.contains('s')),
    );
    object.set(
        "unicode".to_string(),
        JSValue::from_bool(flags.contains('u')),
    );
    object.set(
        "sticky".to_string(),
        JSValue::from_bool(flags.contains('y')),
    );
    object.set("lastIndex".to_string(), JSValue::from_number(0.0));
    object.set(
        "test".to_string(),
        JSValue::from_native_function(regexp_test),
    );
    object.set(
        "exec".to_string(),
        JSValue::from_native_function(regexp_exec),
    );
    object.set("__pixi_regexp__".to_string(), JSValue::from_bool(true));
    JSValue::from_object(Rc::new(RefCell::new(object)))
}

fn regexp_constructor(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let pattern = match args.get(1) {
        Some(value) if value.kind() == JsValueKind::Object => {
            let object = value.as_object().unwrap();
            if is_regexp(&object) {
                object.borrow().get(PATTERN).to_string()
            } else {
                value.to_string()
            }
        }
        Some(value) if value.kind() == JsValueKind::Undefined => String::new(),
        None => String::new(),
        Some(value) => value.to_string(),
    };

    let flags = match args.get(2) {
        Some(value) if value.kind() == JsValueKind::Undefined => match args.get(1) {
            Some(value) if value.kind() == JsValueKind::Object => {
                let object = value.as_object().unwrap();
                if is_regexp(&object) {
                    object.borrow().get(FLAGS).to_string()
                } else {
                    String::new()
                }
            }
            _ => String::new(),
        },
        None => match args.get(1) {
            Some(value) if value.kind() == JsValueKind::Object => {
                let object = value.as_object().unwrap();
                if is_regexp(&object) {
                    object.borrow().get(FLAGS).to_string()
                } else {
                    String::new()
                }
            }
            _ => String::new(),
        },
        Some(value) => value.to_string(),
    };

    let value = create(&pattern, &flags);

    if value.kind() == JsValueKind::Object {
        compile(&value.as_object().unwrap())?;
    }

    Ok(value)
}

fn regexp_to_string(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let object = match args.first() {
        Some(value) if value.kind() == JsValueKind::Object => value.as_object().unwrap().clone(),
        _ => {
            return Err(JSError::TypeError(
                "RegExp.toString: invalid receiver".to_string(),
            ));
        }
    };
    let pattern = object.borrow().get(PATTERN).to_string();
    let flags = flags(&object);
    Ok(JSValue::from_str(&format!("/{pattern}/{flags}")))
}

/// Installs the minimal RegExp constructor namespace.
pub fn install(global: &Rc<RefCell<JSObject>>) {
    let mut constructor = JSObject::new();
    constructor.set(
        "__construct__".to_string(),
        JSValue::from_native_function(regexp_constructor),
    );
    constructor.set(
        "__call__".to_string(),
        JSValue::from_native_function(regexp_constructor),
    );
    let mut prototype = JSObject::new();
    prototype.set(
        "exec".to_string(),
        JSValue::from_native_function(regexp_exec),
    );
    prototype.set(
        "test".to_string(),
        JSValue::from_native_function(regexp_test),
    );
    prototype.set(
        "toString".to_string(),
        JSValue::from_native_function(regexp_to_string),
    );
    constructor.set(
        "prototype".to_string(),
        JSValue::from_object(Rc::new(RefCell::new(prototype))),
    );
    global.borrow_mut().set(
        "RegExp".to_string(),
        JSValue::from_object(Rc::new(RefCell::new(constructor))),
    );
}
