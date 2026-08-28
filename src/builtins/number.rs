//! Minimal Number constructor and prototype.

use std::cell::RefCell;
use std::rc::Rc;

use num_bigint::BigInt;
use num_traits::FromPrimitive;

use crate::error::{JSError, JSResult};
use crate::lexer::Span;
use crate::value::JSValue;
use crate::value::jsobject::{JSObject, Property};
use crate::value::jsvalue::JsValueKind;
use crate::vm::VM;

fn receiver(args: &[JSValue], method: &str) -> JSResult<f64> {
    match args.first() {
        Some(value) if value.kind() == JsValueKind::Number => Ok(value.as_number().unwrap()),
        _ => Err(JSError::TypeError(format!(
            "Number.prototype.{method}: invalid receiver"
        ))),
    }
}

fn number_constructor(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let value = args.get(1).cloned().unwrap_or(JSValue::from_number(0.0));
    Ok(JSValue::from_number(vm.to_number_value(value)?))
}

fn big_int_constructor(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let undefined = JSValue::undefined();
    let value = args.get(1).unwrap_or(&undefined);

    let integer = match value.kind() {
        JsValueKind::BigInt => value.as_bigint().unwrap().clone(),
        JsValueKind::Boolean => BigInt::from(u8::from(value.as_boolean().unwrap())),
        JsValueKind::Number => {
            let value = value.as_number().unwrap();

            if value.is_finite() && value.fract() == 0.0 {
                BigInt::from_f64(value).ok_or_else(|| {
                    JSError::RangeError("Cannot convert number to a BigInt".into())
                })?
            } else {
                return Err(JSError::RangeError(
                    "Cannot convert number to a BigInt".into(),
                ));
            }
        }
        JsValueKind::String => {
            let value = value.as_string().unwrap();

            value.trim().parse().map_err(|_| {
                JSError::SyntaxError(
                    format!("Cannot convert {value} to a BigInt"),
                    Span::new(0, 0, 0, 0),
                )
            })?
        }
        _ => {
            return Err(JSError::RangeError(
                "Cannot convert value to a BigInt".into(),
            ));
        }
    };

    Ok(JSValue::from_bigint(integer))
}

fn global_is_nan(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let value = args.get(1).map(JSValue::to_number).unwrap_or(f64::NAN);
    Ok(JSValue::from_bool(value.is_nan()))
}

fn parse_int(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let input = args.get(1).map(JSValue::to_string).unwrap_or_default();
    let mut input = input.trim_start();
    let negative = input.starts_with('-');
    if input.starts_with(['-', '+']) {
        input = &input[1..];
    }
    let requested_radix = args.get(2).map(JSValue::to_number).unwrap_or(0.0) as u32;
    let mut radix = requested_radix;
    if radix == 0 {
        radix = if input.starts_with("0x") || input.starts_with("0X") {
            16
        } else {
            10
        };
    }
    if !(2..=36).contains(&radix) {
        return Ok(JSValue::from_number(f64::NAN));
    }
    if radix == 16 && (input.starts_with("0x") || input.starts_with("0X")) {
        input = &input[2..];
    }
    let mut value = 0.0;
    let mut digits = 0;
    for character in input.chars() {
        let Some(digit) = character.to_digit(radix) else {
            break;
        };
        value = value * radix as f64 + digit as f64;
        digits += 1;
    }
    if digits == 0 {
        return Ok(JSValue::from_number(f64::NAN));
    }
    Ok(JSValue::from_number(if negative { -value } else { value }))
}

fn parse_float(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let input = args.get(1).map(JSValue::to_string).unwrap_or_default();
    let input = input.trim_start();
    if input.starts_with("Infinity") || input.starts_with("+Infinity") {
        return Ok(JSValue::from_number(f64::INFINITY));
    }
    if input.starts_with("-Infinity") {
        return Ok(JSValue::from_number(f64::NEG_INFINITY));
    }
    let mut parsed = None;
    for (end, _) in input.char_indices().skip(1) {
        if let Ok(value) = input[..end].parse::<f64>() {
            parsed = Some(value);
        }
    }
    if let Ok(value) = input.parse::<f64>() {
        parsed = Some(value);
    }
    Ok(JSValue::from_number(parsed.unwrap_or(f64::NAN)))
}

fn number_to_string(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let value = receiver(&args, "toString")?;
    let radix = args.get(1).map(JSValue::to_number).unwrap_or(10.0) as u32;
    if !(2..=36).contains(&radix) {
        return Err(JSError::RangeError(
            "Number.toString radix must be between 2 and 36".to_string(),
        ));
    }
    if radix == 10 || !value.is_finite() || value.fract() != 0.0 {
        return Ok(JSValue::from_string(
            JSValue::from_number(value).to_string(),
        ));
    }

    let negative = value.is_sign_negative();
    let mut integer = value.abs() as u64;
    let mut digits = Vec::new();
    loop {
        let digit = (integer % radix as u64) as u8;
        digits.push(if digit < 10 {
            (b'0' + digit) as char
        } else {
            (b'a' + digit - 10) as char
        });
        integer /= radix as u64;
        if integer == 0 {
            break;
        }
    }
    if negative {
        digits.push('-');
    }
    digits.reverse();
    Ok(JSValue::from_string(digits.into_iter().collect()))
}

fn number_value_of(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(JSValue::from_number(receiver(&args, "valueOf")?))
}

/// Installs Number and its primitive prototype.
pub fn install(global: &Rc<RefCell<JSObject>>) {
    {
        let mut global = global.borrow_mut();
        global.set(
            "isNaN".to_string(),
            JSValue::from_native_function(global_is_nan),
        );
        global.set(
            "parseInt".to_string(),
            JSValue::from_native_function(parse_int),
        );
        global.set(
            "parseFloat".to_string(),
            JSValue::from_native_function(parse_float),
        );
        global.set(
            "BigInt".to_string(),
            JSValue::from_native_function(big_int_constructor),
        );
        global.define_property(
            "Infinity".to_string(),
            Property::read_only(JSValue::from_number(f64::INFINITY)),
        );
        global.define_property(
            "NaN".to_string(),
            Property::read_only(JSValue::from_number(f64::NAN)),
        );
    }

    let mut prototype = JSObject::new();
    prototype.set(
        "toString".to_string(),
        JSValue::from_native_function(number_to_string),
    );
    prototype.set(
        "valueOf".to_string(),
        JSValue::from_native_function(number_value_of),
    );

    let mut constructor = JSObject::new();
    constructor.set(
        "__call__".to_string(),
        JSValue::from_native_function(number_constructor),
    );
    constructor.set(
        "prototype".to_string(),
        JSValue::from_object(Rc::new(RefCell::new(prototype))),
    );
    constructor.set(
        "parseInt".to_string(),
        JSValue::from_native_function(parse_int),
    );
    constructor.set(
        "parseFloat".to_string(),
        JSValue::from_native_function(parse_float),
    );
    global.borrow_mut().set(
        "Number".to_string(),
        JSValue::from_object(Rc::new(RefCell::new(constructor))),
    );
}
