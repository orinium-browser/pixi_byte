use crate::error::{JSError, JSResult};
use crate::value::JSValue;
use crate::value::jsobject::JSObject;
use crate::value::jsvalue::JsValueKind;
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
    match value.kind() {
        JsValueKind::Undefined
        | JsValueKind::Function
        | JsValueKind::ArrowFunction
        | JsValueKind::NativeFunction
        | JsValueKind::BoundFunction => Ok(in_array.then(|| "null".to_string())),
        JsValueKind::Null => Ok(Some("null".to_string())),
        JsValueKind::Boolean => Ok(Some(value.as_boolean().unwrap().to_string())),
        JsValueKind::Number => {
            let value = value.as_number().unwrap();
            if value.is_finite() {
                Ok(Some(JSValue::from_number(value).to_string()))
            } else {
                Ok(Some("null".to_string()))
            }
        }
        JsValueKind::BigInt => Err(JSError::TypeError(
            "Do not know how to serialize a BigInt".to_string(),
        )),
        JsValueKind::String => Ok(Some(quote(value.as_string().unwrap()))),
        JsValueKind::Object => {
            let object = value.as_object().unwrap();
            let identity = Rc::as_ptr(&object) as usize;
            if !stack.insert(identity) {
                return Err(JSError::TypeError(
                    "Converting circular structure to JSON".to_string(),
                ));
            }

            let (is_array, entries): (bool, Vec<(String, JSValue)>) = {
                let object = object.borrow();
                let is_array = object.get("__pixi_array__") == JSValue::from_bool(true);
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
    let undefined = JSValue::undefined();
    let value = args.get(1).unwrap_or(&undefined);
    Ok(serialize(value, &mut HashSet::new(), false)?
        .map(JSValue::from_string)
        .unwrap_or(JSValue::undefined()))
}

struct JsonParser<'a> {
    source: &'a [u8],
    offset: usize,
}

impl JsonParser<'_> {
    fn skip_whitespace(&mut self) {
        while self
            .source
            .get(self.offset)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.offset += 1;
        }
    }

    fn parse_value(&mut self, vm: &mut VM) -> JSResult<JSValue> {
        self.skip_whitespace();
        match self.source.get(self.offset).copied() {
            Some(b'n') => self.keyword(b"null", JSValue::null()),
            Some(b't') => self.keyword(b"true", JSValue::from_bool(true)),
            Some(b'f') => self.keyword(b"false", JSValue::from_bool(false)),
            Some(b'"') => self.parse_string().map(JSValue::from_string),
            Some(b'[') => self.parse_array(vm),
            Some(b'{') => self.parse_object(vm),
            Some(b'-' | b'0'..=b'9') => self.parse_number(),
            _ => self.syntax_error(),
        }
    }

    fn keyword(&mut self, keyword: &[u8], value: JSValue) -> JSResult<JSValue> {
        if self.source.get(self.offset..self.offset + keyword.len()) == Some(keyword) {
            self.offset += keyword.len();
            Ok(value)
        } else {
            self.syntax_error()
        }
    }

    fn parse_string(&mut self) -> JSResult<String> {
        self.offset += 1;
        let mut output = String::new();
        while let Some(byte) = self.source.get(self.offset).copied() {
            self.offset += 1;
            match byte {
                b'"' => return Ok(output),
                b'\\' => {
                    let escape = self.source.get(self.offset).copied().ok_or_else(|| {
                        JSError::SyntaxError(
                            "Unterminated JSON string".to_string(),
                            crate::lexer::token::Span::new(self.offset, self.offset, 0, 0),
                        )
                    })?;
                    self.offset += 1;
                    match escape {
                        b'"' => output.push('"'),
                        b'\\' => output.push('\\'),
                        b'/' => output.push('/'),
                        b'b' => output.push('\u{8}'),
                        b'f' => output.push('\u{c}'),
                        b'n' => output.push('\n'),
                        b'r' => output.push('\r'),
                        b't' => output.push('\t'),
                        b'u' => {
                            let unit = self.parse_hex_unit()?;
                            let scalar = if (0xd800..=0xdbff).contains(&unit) {
                                if self.source.get(self.offset..self.offset + 2) != Some(b"\\u") {
                                    return self.syntax_error();
                                }
                                self.offset += 2;
                                let low = self.parse_hex_unit()?;
                                if !(0xdc00..=0xdfff).contains(&low) {
                                    return self.syntax_error();
                                }
                                0x10000
                                    + ((u32::from(unit) - 0xd800) << 10)
                                    + (u32::from(low) - 0xdc00)
                            } else if (0xdc00..=0xdfff).contains(&unit) {
                                return self.syntax_error();
                            } else {
                                u32::from(unit)
                            };
                            output.push(char::from_u32(scalar).expect("valid JSON scalar"));
                        }
                        _ => return self.syntax_error(),
                    }
                }
                0..=0x1f => return self.syntax_error(),
                _ => {
                    let start = self.offset - 1;
                    let character = std::str::from_utf8(&self.source[start..])
                        .ok()
                        .and_then(|text| text.chars().next())
                        .ok_or_else(|| {
                            JSError::SyntaxError(
                                "Invalid JSON UTF-8".to_string(),
                                crate::lexer::token::Span::new(self.offset, self.offset, 0, 0),
                            )
                        })?;
                    self.offset = start + character.len_utf8();
                    output.push(character);
                }
            }
        }
        self.syntax_error()
    }

    fn parse_hex_unit(&mut self) -> JSResult<u16> {
        let digits = self
            .source
            .get(self.offset..self.offset + 4)
            .and_then(|digits| std::str::from_utf8(digits).ok())
            .and_then(|digits| u16::from_str_radix(digits, 16).ok())
            .ok_or_else(|| {
                JSError::SyntaxError(
                    "Invalid JSON unicode escape".to_string(),
                    crate::lexer::token::Span::new(self.offset, self.offset, 0, 0),
                )
            })?;
        self.offset += 4;
        Ok(digits)
    }

    fn parse_number(&mut self) -> JSResult<JSValue> {
        let start = self.offset;
        if self.source.get(self.offset) == Some(&b'-') {
            self.offset += 1;
        }
        match self.source.get(self.offset) {
            Some(b'0') => self.offset += 1,
            Some(b'1'..=b'9') => {
                self.offset += 1;
                while self.source.get(self.offset).is_some_and(u8::is_ascii_digit) {
                    self.offset += 1;
                }
            }
            _ => return self.syntax_error(),
        }
        if self.source.get(self.offset) == Some(&b'.') {
            self.offset += 1;
            let fraction_start = self.offset;
            while self.source.get(self.offset).is_some_and(u8::is_ascii_digit) {
                self.offset += 1;
            }
            if self.offset == fraction_start {
                return self.syntax_error();
            }
        }
        if self
            .source
            .get(self.offset)
            .is_some_and(|byte| matches!(byte, b'e' | b'E'))
        {
            self.offset += 1;
            if self
                .source
                .get(self.offset)
                .is_some_and(|byte| matches!(byte, b'+' | b'-'))
            {
                self.offset += 1;
            }
            let exponent_start = self.offset;
            while self.source.get(self.offset).is_some_and(u8::is_ascii_digit) {
                self.offset += 1;
            }
            if self.offset == exponent_start {
                return self.syntax_error();
            }
        }
        let number = std::str::from_utf8(&self.source[start..self.offset])
            .ok()
            .and_then(|number| number.parse::<f64>().ok())
            .filter(|number| number.is_finite())
            .ok_or_else(|| {
                JSError::SyntaxError(
                    "Invalid JSON number".to_string(),
                    crate::lexer::token::Span::new(self.offset, self.offset, 0, 0),
                )
            })?;
        Ok(JSValue::from_number(number))
    }

    fn parse_array(&mut self, vm: &mut VM) -> JSResult<JSValue> {
        self.offset += 1;
        let mut values = Vec::new();
        loop {
            self.skip_whitespace();
            if self.source.get(self.offset) == Some(&b']') {
                self.offset += 1;
                return Ok(vm.array_from_values(values));
            }
            if !values.is_empty() {
                if self.source.get(self.offset) != Some(&b',') {
                    return self.syntax_error();
                }
                self.offset += 1;
            }
            values.push(self.parse_value(vm)?);
        }
    }

    fn parse_object(&mut self, vm: &mut VM) -> JSResult<JSValue> {
        self.offset += 1;
        let mut object = JSObject::with_prototype(Some(Rc::clone(&vm.object_prototype)));
        let mut first = true;
        loop {
            self.skip_whitespace();
            if self.source.get(self.offset) == Some(&b'}') {
                self.offset += 1;
                return Ok(JSValue::from_object(Rc::new(RefCell::new(object))));
            }
            if !first {
                if self.source.get(self.offset) != Some(&b',') {
                    return self.syntax_error();
                }
                self.offset += 1;
                self.skip_whitespace();
            }
            if self.source.get(self.offset) != Some(&b'"') {
                return self.syntax_error();
            }
            let key = self.parse_string()?;
            self.skip_whitespace();
            if self.source.get(self.offset) != Some(&b':') {
                return self.syntax_error();
            }
            self.offset += 1;
            let value = self.parse_value(vm)?;
            object.set(key, value);
            first = false;
        }
    }

    fn syntax_error<T>(&self) -> JSResult<T> {
        Err(JSError::SyntaxError(
            format!("Unexpected JSON token at position {}", self.offset),
            crate::lexer::token::Span::new(self.offset, self.offset, 0, 0),
        ))
    }
}

fn json_parse(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let undefined = JSValue::undefined();
    let source = args.get(1).unwrap_or(&undefined).to_string();
    let mut parser = JsonParser {
        source: source.as_bytes(),
        offset: 0,
    };
    let value = parser.parse_value(vm)?;
    parser.skip_whitespace();
    if parser.offset != parser.source.len() {
        return parser.syntax_error();
    }
    Ok(value)
}

pub fn install(global: &Rc<RefCell<JSObject>>) {
    let mut json = JSObject::new();
    json.set(
        "stringify".to_string(),
        JSValue::from_native_function(json_stringify),
    );
    json.set(
        "parse".to_string(),
        JSValue::from_native_function(json_parse),
    );
    global.borrow_mut().set(
        "JSON".to_string(),
        JSValue::from_object(Rc::new(RefCell::new(json))),
    );
}
