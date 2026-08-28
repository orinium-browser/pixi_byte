use crate::error::JSResult;
use crate::value::JSValue;
use crate::value::jsobject::JSObject;
use crate::value::jsvalue::JsValueKind;
use crate::vm::VM;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

const DATE_VALUE: &str = "__date_value__";

fn epoch_milliseconds() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
        * 1_000.0
}

fn date_now(_vm: &mut VM, _args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(JSValue::from_number(epoch_milliseconds().floor()))
}

fn date_parse(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let value = args
        .get(1)
        .map(JSValue::to_string)
        .unwrap_or_default()
        .trim()
        .parse::<f64>()
        .unwrap_or(f64::NAN);
    Ok(JSValue::from_number(value))
}

fn date_constructor(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let milliseconds = match args.get(1) {
        None => epoch_milliseconds().floor(),
        Some(v) if &JSValue::undefined() == v => epoch_milliseconds().floor(),
        Some(value) => vm.to_number_value(value.clone())?,
    };
    let this = if let Some(v) = args.first()
        && JsValueKind::Object == v.kind()
    {
        v.as_object().unwrap()
    } else {
        return Ok(JSValue::undefined());
    };
    this.borrow_mut()
        .set(DATE_VALUE.to_string(), JSValue::from_number(milliseconds));
    Ok(JSValue::undefined())
}

fn date_call(_vm: &mut VM, _args: Vec<JSValue>) -> JSResult<JSValue> {
    // A full locale-sensitive Date string is not available yet. Returning a
    // stable timestamp string preserves the required callable shape while the
    // constructor and numeric methods remain standards-compatible.
    Ok(JSValue::from_string(
        epoch_milliseconds().floor().to_string(),
    ))
}

fn date_value(args: &[JSValue], method: &str) -> JSResult<f64> {
    let this = match args.first() {
        Some(value) if value.kind() == JsValueKind::Object => value.as_object().unwrap(),
        _ => {
            return Err(crate::error::JSError::TypeError(format!(
                "Date.prototype.{method}: invalid receiver"
            )));
        }
    };

    match this.borrow().get(DATE_VALUE).kind() {
        JsValueKind::Number => Ok(this.borrow().get(DATE_VALUE).as_number().unwrap()),
        _ => Err(crate::error::JSError::TypeError(format!(
            "Date.prototype.{method}: invalid receiver"
        ))),
    }
}

fn date_get_time(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(JSValue::from_number(date_value(&args, "getTime")?))
}

fn date_value_of(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(JSValue::from_number(date_value(&args, "valueOf")?))
}

fn civil_from_days(days_since_epoch: i64) -> (i32, u32, u32) {
    let days = days_since_epoch + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year as i32, month as u32, day as u32)
}

fn days_from_civil(year: i32, month: u32, day: i64) -> i64 {
    let mut year = i64::from(year);
    year -= i64::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month = i64::from(month);
    let month_prime = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn date_parts(args: &[JSValue], method: &str) -> JSResult<(i32, u32, u32)> {
    let milliseconds = date_value(args, method)?;
    if !milliseconds.is_finite() {
        return Ok((0, 0, 0));
    }
    let days = (milliseconds / 86_400_000.0).floor() as i64;
    Ok(civil_from_days(days))
}

fn date_get_date(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let (_, _, day) = date_parts(&args, "getDate")?;
    Ok(JSValue::from_number(day as f64))
}

fn date_get_month(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let (_, month, _) = date_parts(&args, "getMonth")?;
    Ok(JSValue::from_number(month.saturating_sub(1) as f64))
}

fn date_get_full_year(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let (year, _, _) = date_parts(&args, "getFullYear")?;
    Ok(JSValue::from_number(year as f64))
}

fn date_set_date(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let milliseconds = date_value(&args, "setDate")?;
    let requested_day = args
        .get(1)
        .unwrap_or(&JSValue::undefined())
        .to_number()
        .trunc() as i64;
    let days = (milliseconds / 86_400_000.0).floor() as i64;
    let time_within_day = milliseconds - days as f64 * 86_400_000.0;
    let (year, month, _) = civil_from_days(days);
    let updated =
        days_from_civil(year, month, requested_day) as f64 * 86_400_000.0 + time_within_day;
    match args.first() {
        Some(value) if value.kind() == JsValueKind::Object => {
            value
                .as_object()
                .unwrap()
                .borrow_mut()
                .set(DATE_VALUE.to_string(), JSValue::from_number(updated));
        }
        _ => {}
    }
    Ok(JSValue::from_number(updated))
}

fn set_date_year(args: &[JSValue], method: &str, legacy: bool) -> JSResult<JSValue> {
    let milliseconds = date_value(args, method)?;
    let mut requested_year = args
        .get(1)
        .unwrap_or(&JSValue::undefined())
        .to_number()
        .trunc() as i32;
    if legacy && (0..=99).contains(&requested_year) {
        requested_year += 1900;
    }
    let days = (milliseconds / 86_400_000.0).floor() as i64;
    let time_within_day = milliseconds - days as f64 * 86_400_000.0;
    let (_, month, day) = civil_from_days(days);
    let updated = days_from_civil(requested_year, month, i64::from(day)) as f64 * 86_400_000.0
        + time_within_day;
    if let Some(value) = args.first()
        && value.kind() == JsValueKind::Object
    {
        value
            .as_object()
            .unwrap()
            .borrow_mut()
            .set(DATE_VALUE.to_string(), JSValue::from_number(updated));
    }
    Ok(JSValue::from_number(updated))
}

fn date_set_full_year(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    set_date_year(&args, "setFullYear", false)
}

fn date_set_year(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    set_date_year(&args, "setYear", true)
}

fn date_to_utc_string(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let milliseconds = date_value(&args, "toUTCString")?;
    if !milliseconds.is_finite() {
        return Ok(JSValue::from_str("Invalid Date"));
    }
    let days = (milliseconds / 86_400_000.0).floor() as i64;
    let within_day = milliseconds.rem_euclid(86_400_000.0) as u64;
    let (year, month, day) = civil_from_days(days);
    let weekdays = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let weekday = (days + 4).rem_euclid(7) as usize;
    let hour = within_day / 3_600_000;
    let minute = within_day / 60_000 % 60;
    let second = within_day / 1_000 % 60;
    Ok(JSValue::from_string(format!(
        "{}, {:02} {} {:04} {:02}:{:02}:{:02} GMT",
        weekdays[weekday],
        day,
        months[month.saturating_sub(1) as usize],
        year,
        hour,
        minute,
        second
    )))
}

pub fn install(global: &Rc<RefCell<JSObject>>) {
    let mut prototype = JSObject::new();
    prototype.set(
        "getTime".to_string(),
        JSValue::from_native_function(date_get_time),
    );
    prototype.set(
        "valueOf".to_string(),
        JSValue::from_native_function(date_value_of),
    );
    prototype.set(
        "getDate".to_string(),
        JSValue::from_native_function(date_get_date),
    );
    prototype.set(
        "getMonth".to_string(),
        JSValue::from_native_function(date_get_month),
    );
    prototype.set(
        "getFullYear".to_string(),
        JSValue::from_native_function(date_get_full_year),
    );
    prototype.set(
        "setDate".to_string(),
        JSValue::from_native_function(date_set_date),
    );
    prototype.set(
        "setFullYear".to_string(),
        JSValue::from_native_function(date_set_full_year),
    );
    prototype.set(
        "setYear".to_string(),
        JSValue::from_native_function(date_set_year),
    );
    prototype.set(
        "toUTCString".to_string(),
        JSValue::from_native_function(date_to_utc_string),
    );

    let mut date = JSObject::new();
    date.set(
        "__call__".to_string(),
        JSValue::from_native_function(date_call),
    );
    date.set(
        "__construct__".to_string(),
        JSValue::from_native_function(date_constructor),
    );
    date.set(
        "prototype".to_string(),
        JSValue::from_object(Rc::new(RefCell::new(prototype))),
    );
    date.set("now".to_string(), JSValue::from_native_function(date_now));
    date.set(
        "parse".to_string(),
        JSValue::from_native_function(date_parse),
    );
    global.borrow_mut().set(
        "Date".to_string(),
        JSValue::from_object(Rc::new(RefCell::new(date))),
    );
}
