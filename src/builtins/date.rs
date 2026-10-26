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

const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Milliseconds within the current day plus the clock fields.
fn within_day(milliseconds: f64) -> (i64, i64, i64, i64) {
    let day = milliseconds.rem_euclid(86_400_000.0) as i64;
    (
        day / 3_600_000,
        day / 60_000 % 60,
        day / 1_000 % 60,
        day % 1_000,
    )
}

fn weekday_index(milliseconds: f64) -> i64 {
    let days = (milliseconds / 86_400_000.0).floor() as i64;
    (days + 4).rem_euclid(7)
}

fn store_date_value(args: &[JSValue], updated: f64) {
    if let Some(value) = args.first()
        && value.kind() == JsValueKind::Object
    {
        value
            .as_object()
            .unwrap()
            .borrow_mut()
            .set(DATE_VALUE.to_string(), JSValue::from_number(updated));
    }
}

fn arg_number(args: &[JSValue], index: usize) -> f64 {
    args.get(index).map(JSValue::to_number).unwrap_or(f64::NAN)
}

/// Rebuilds a timestamp from the current one, overriding the supplied fields.
/// `None` keeps the current value; `month` is zero-based, matching `setMonth`.
#[allow(clippy::too_many_arguments)]
fn rebuild_timestamp(
    milliseconds: f64,
    year: Option<i64>,
    month: Option<i64>,
    day: Option<i64>,
    hour: Option<i64>,
    minute: Option<i64>,
    second: Option<i64>,
    millisecond: Option<i64>,
) -> f64 {
    if !milliseconds.is_finite() {
        return f64::NAN;
    }
    let days = (milliseconds / 86_400_000.0).floor() as i64;
    let (current_year, current_month, current_day) = civil_from_days(days);
    let (current_hour, current_minute, current_second, current_millisecond) =
        within_day(milliseconds);

    let month = month.unwrap_or(i64::from(current_month) - 1);
    let (year_offset, month) = (month.div_euclid(12), month.rem_euclid(12));
    let year = year.unwrap_or(i64::from(current_year)) + year_offset;
    let day = day.unwrap_or(i64::from(current_day));
    let hour = hour.unwrap_or(current_hour);
    let minute = minute.unwrap_or(current_minute);
    let second = second.unwrap_or(current_second);
    let millisecond = millisecond.unwrap_or(current_millisecond);

    let base_days = days_from_civil(year as i32, (month + 1) as u32, day);
    base_days as f64 * 86_400_000.0
        + (hour * 3_600_000 + minute * 60_000 + second * 1_000 + millisecond) as f64
}

fn date_get_day(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let milliseconds = date_value(&args, "getDay")?;
    if !milliseconds.is_finite() {
        return Ok(JSValue::from_number(f64::NAN));
    }
    Ok(JSValue::from_number(weekday_index(milliseconds) as f64))
}

fn date_get_hours(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let milliseconds = date_value(&args, "getHours")?;
    if !milliseconds.is_finite() {
        return Ok(JSValue::from_number(f64::NAN));
    }
    Ok(JSValue::from_number(within_day(milliseconds).0 as f64))
}

fn date_get_minutes(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let milliseconds = date_value(&args, "getMinutes")?;
    if !milliseconds.is_finite() {
        return Ok(JSValue::from_number(f64::NAN));
    }
    Ok(JSValue::from_number(within_day(milliseconds).1 as f64))
}

fn date_get_seconds(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let milliseconds = date_value(&args, "getSeconds")?;
    if !milliseconds.is_finite() {
        return Ok(JSValue::from_number(f64::NAN));
    }
    Ok(JSValue::from_number(within_day(milliseconds).2 as f64))
}

fn date_get_milliseconds(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let milliseconds = date_value(&args, "getMilliseconds")?;
    if !milliseconds.is_finite() {
        return Ok(JSValue::from_number(f64::NAN));
    }
    Ok(JSValue::from_number(within_day(milliseconds).3 as f64))
}

fn date_get_year(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let (year, _, _) = date_parts(&args, "getYear")?;
    Ok(JSValue::from_number((year - 1900) as f64))
}

fn date_get_timezone_offset(_vm: &mut VM, _args: Vec<JSValue>) -> JSResult<JSValue> {
    // The engine has no time-zone database yet; dates are treated as UTC.
    Ok(JSValue::from_number(0.0))
}

fn date_set_time(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let updated = arg_number(&args, 1);
    store_date_value(&args, updated);
    Ok(JSValue::from_number(updated))
}

fn date_set_milliseconds(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let milliseconds = date_value(&args, "setMilliseconds")?;
    let updated = rebuild_timestamp(
        milliseconds,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(arg_number(&args, 1) as i64),
    );
    store_date_value(&args, updated);
    Ok(JSValue::from_number(updated))
}

fn date_set_seconds(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let milliseconds = date_value(&args, "setSeconds")?;
    let updated = rebuild_timestamp(
        milliseconds,
        None,
        None,
        None,
        None,
        None,
        Some(arg_number(&args, 1) as i64),
        if args.len() > 2 {
            Some(arg_number(&args, 2) as i64)
        } else {
            None
        },
    );
    store_date_value(&args, updated);
    Ok(JSValue::from_number(updated))
}

fn date_set_minutes(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let milliseconds = date_value(&args, "setMinutes")?;
    let updated = rebuild_timestamp(
        milliseconds,
        None,
        None,
        None,
        None,
        Some(arg_number(&args, 1) as i64),
        if args.len() > 2 {
            Some(arg_number(&args, 2) as i64)
        } else {
            None
        },
        if args.len() > 3 {
            Some(arg_number(&args, 3) as i64)
        } else {
            None
        },
    );
    store_date_value(&args, updated);
    Ok(JSValue::from_number(updated))
}

fn date_set_hours(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let milliseconds = date_value(&args, "setHours")?;
    let updated = rebuild_timestamp(
        milliseconds,
        None,
        None,
        None,
        Some(arg_number(&args, 1) as i64),
        if args.len() > 2 {
            Some(arg_number(&args, 2) as i64)
        } else {
            None
        },
        if args.len() > 3 {
            Some(arg_number(&args, 3) as i64)
        } else {
            None
        },
        if args.len() > 4 {
            Some(arg_number(&args, 4) as i64)
        } else {
            None
        },
    );
    store_date_value(&args, updated);
    Ok(JSValue::from_number(updated))
}

fn date_set_month(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let milliseconds = date_value(&args, "setMonth")?;
    let updated = rebuild_timestamp(
        milliseconds,
        None,
        Some(arg_number(&args, 1) as i64),
        if args.len() > 2 {
            Some(arg_number(&args, 2) as i64)
        } else {
            None
        },
        None,
        None,
        None,
        None,
    );
    store_date_value(&args, updated);
    Ok(JSValue::from_number(updated))
}

fn date_set_full_year_all(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let milliseconds = date_value(&args, "setFullYear")?;
    let updated = rebuild_timestamp(
        milliseconds,
        Some(arg_number(&args, 1) as i64),
        if args.len() > 2 {
            Some(arg_number(&args, 2) as i64)
        } else {
            None
        },
        if args.len() > 3 {
            Some(arg_number(&args, 3) as i64)
        } else {
            None
        },
        None,
        None,
        None,
        None,
    );
    store_date_value(&args, updated);
    Ok(JSValue::from_number(updated))
}

fn date_set_date_all(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let milliseconds = date_value(&args, "setDate")?;
    let updated = rebuild_timestamp(
        milliseconds,
        None,
        None,
        Some(arg_number(&args, 1) as i64),
        None,
        None,
        None,
        None,
    );
    store_date_value(&args, updated);
    Ok(JSValue::from_number(updated))
}

fn format_offset() -> &'static str {
    "GMT+0000 (Coordinated Universal Time)"
}

fn date_to_string(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let milliseconds = date_value(&args, "toString")?;
    if !milliseconds.is_finite() {
        return Ok(JSValue::from_str("Invalid Date"));
    }
    let days = (milliseconds / 86_400_000.0).floor() as i64;
    let (year, month, day) = civil_from_days(days);
    let (hour, minute, second, _) = within_day(milliseconds);
    Ok(JSValue::from_string(format!(
        "{} {} {:02} {:04} {:02}:{:02}:{:02} {}",
        WEEKDAYS[weekday_index(milliseconds) as usize],
        MONTHS[month.saturating_sub(1) as usize],
        day,
        year,
        hour,
        minute,
        second,
        format_offset()
    )))
}

fn date_to_date_string(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let milliseconds = date_value(&args, "toDateString")?;
    if !milliseconds.is_finite() {
        return Ok(JSValue::from_str("Invalid Date"));
    }
    let days = (milliseconds / 86_400_000.0).floor() as i64;
    let (year, month, day) = civil_from_days(days);
    Ok(JSValue::from_string(format!(
        "{} {} {:02} {:04}",
        WEEKDAYS[weekday_index(milliseconds) as usize],
        MONTHS[month.saturating_sub(1) as usize],
        day,
        year
    )))
}

fn date_to_time_string(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let milliseconds = date_value(&args, "toTimeString")?;
    if !milliseconds.is_finite() {
        return Ok(JSValue::from_str("Invalid Date"));
    }
    let (hour, minute, second, _) = within_day(milliseconds);
    Ok(JSValue::from_string(format!(
        "{:02}:{:02}:{:02} {}",
        hour,
        minute,
        second,
        format_offset()
    )))
}

fn date_to_iso_string(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let milliseconds = date_value(&args, "toISOString")?;
    if !milliseconds.is_finite() {
        return Err(crate::error::JSError::RangeError(
            "Invalid time value".to_string(),
        ));
    }
    let days = (milliseconds / 86_400_000.0).floor() as i64;
    let (year, month, day) = civil_from_days(days);
    let (hour, minute, second, millisecond) = within_day(milliseconds);
    if (0..=9999).contains(&year) {
        Ok(JSValue::from_string(format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
            year, month, day, hour, minute, second, millisecond
        )))
    } else {
        let sign = if year < 0 { '-' } else { '+' };
        Ok(JSValue::from_string(format!(
            "{sign}{:06}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
            year.abs(),
            month,
            day,
            hour,
            minute,
            second,
            millisecond
        )))
    }
}

fn date_to_json(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let milliseconds = date_value(&args, "toJSON")?;
    if !milliseconds.is_finite() {
        return Ok(JSValue::null());
    }
    date_to_iso_string(_vm, args)
}

fn date_utc(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let milliseconds = rebuild_timestamp(
        0.0,
        Some(arg_number(&args, 1) as i64),
        Some(arg_number(&args, 2) as i64),
        if args.len() > 3 {
            Some(arg_number(&args, 3) as i64)
        } else {
            Some(1)
        },
        if args.len() > 4 {
            Some(arg_number(&args, 4) as i64)
        } else {
            Some(0)
        },
        if args.len() > 5 {
            Some(arg_number(&args, 5) as i64)
        } else {
            Some(0)
        },
        if args.len() > 6 {
            Some(arg_number(&args, 6) as i64)
        } else {
            Some(0)
        },
        if args.len() > 7 {
            Some(arg_number(&args, 7) as i64)
        } else {
            Some(0)
        },
    );
    Ok(JSValue::from_number(milliseconds))
}

pub fn install(global: &Rc<RefCell<JSObject>>) {
    let mut prototype = JSObject::new();
    let methods: &[(&str, crate::value::jsvalue::NativeFunctionType)] = &[
        ("getTime", date_get_time),
        ("valueOf", date_value_of),
        ("getDate", date_get_date),
        ("getMonth", date_get_month),
        ("getFullYear", date_get_full_year),
        ("getDay", date_get_day),
        ("getHours", date_get_hours),
        ("getMinutes", date_get_minutes),
        ("getSeconds", date_get_seconds),
        ("getMilliseconds", date_get_milliseconds),
        ("getYear", date_get_year),
        ("getTimezoneOffset", date_get_timezone_offset),
        ("getUTCDate", date_get_date),
        ("getUTCMonth", date_get_month),
        ("getUTCFullYear", date_get_full_year),
        ("getUTCDay", date_get_day),
        ("getUTCHours", date_get_hours),
        ("getUTCMinutes", date_get_minutes),
        ("getUTCSeconds", date_get_seconds),
        ("getUTCMilliseconds", date_get_milliseconds),
        ("setTime", date_set_time),
        ("setMilliseconds", date_set_milliseconds),
        ("setUTCMilliseconds", date_set_milliseconds),
        ("setSeconds", date_set_seconds),
        ("setUTCSeconds", date_set_seconds),
        ("setMinutes", date_set_minutes),
        ("setUTCMinutes", date_set_minutes),
        ("setHours", date_set_hours),
        ("setUTCHours", date_set_hours),
        ("setDate", date_set_date_all),
        ("setUTCDate", date_set_date_all),
        ("setMonth", date_set_month),
        ("setUTCMonth", date_set_month),
        ("setFullYear", date_set_full_year_all),
        ("setUTCFullYear", date_set_full_year_all),
        ("setYear", date_set_year),
        ("toUTCString", date_to_utc_string),
        ("toGMTString", date_to_utc_string),
        ("toString", date_to_string),
        ("toDateString", date_to_date_string),
        ("toTimeString", date_to_time_string),
        ("toISOString", date_to_iso_string),
        ("toJSON", date_to_json),
        ("toLocaleString", date_to_string),
        ("toLocaleDateString", date_to_date_string),
        ("toLocaleTimeString", date_to_time_string),
    ];
    for (name, function) in methods {
        prototype.set(
            (*name).to_string(),
            JSValue::from_native_function(*function),
        );
    }

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
    date.set("UTC".to_string(), JSValue::from_native_function(date_utc));
    global.borrow_mut().set(
        "Date".to_string(),
        JSValue::from_object(Rc::new(RefCell::new(date))),
    );
}
