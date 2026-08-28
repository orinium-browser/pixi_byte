//! Math constants and functions used by React.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::JSResult;
use crate::value::JSValue;
use crate::value::jsobject::{JSObject, Property};
use crate::vm::VM;

static RANDOM_STATE: AtomicU64 = AtomicU64::new(0);

fn argument(args: &[JSValue], index: usize) -> f64 {
    args.get(index + 1)
        .map(JSValue::to_number)
        .unwrap_or(f64::NAN)
}

fn math_min(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let value = args
        .iter()
        .skip(1)
        .map(JSValue::to_number)
        .fold(f64::INFINITY, f64::min);
    Ok(JSValue::from_number(value))
}

fn math_max(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let value = args
        .iter()
        .skip(1)
        .map(JSValue::to_number)
        .fold(f64::NEG_INFINITY, f64::max);
    Ok(JSValue::from_number(value))
}

fn math_abs(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(JSValue::from_number(argument(&args, 0).abs()))
}

fn math_pow(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(JSValue::from_number(
        argument(&args, 0).powf(argument(&args, 1)),
    ))
}

fn math_clz32(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(JSValue::from_number(
        (argument(&args, 0) as u32).leading_zeros() as f64,
    ))
}

fn math_ceil(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(JSValue::from_number(argument(&args, 0).ceil()))
}

fn math_floor(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(JSValue::from_number(argument(&args, 0).floor()))
}

fn math_log(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(JSValue::from_number(argument(&args, 0).ln()))
}

fn math_random(_vm: &mut VM, _args: Vec<JSValue>) -> JSResult<JSValue> {
    let mut state = RANDOM_STATE.load(Ordering::Relaxed);
    if state == 0 {
        state = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(0x9e3779b97f4a7c15);
    }
    state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    RANDOM_STATE.store(state, Ordering::Relaxed);
    let value = (state >> 11) as f64 / (1_u64 << 53) as f64;
    Ok(JSValue::from_number(value))
}

/// Installs the Math namespace.
pub fn install(global: &Rc<RefCell<JSObject>>) {
    let mut math = JSObject::new();
    math.set("min".to_string(), JSValue::from_native_function(math_min));
    math.set("max".to_string(), JSValue::from_native_function(math_max));
    math.set("abs".to_string(), JSValue::from_native_function(math_abs));
    math.set("pow".to_string(), JSValue::from_native_function(math_pow));
    math.set(
        "clz32".to_string(),
        JSValue::from_native_function(math_clz32),
    );
    math.set("ceil".to_string(), JSValue::from_native_function(math_ceil));
    math.set(
        "floor".to_string(),
        JSValue::from_native_function(math_floor),
    );
    math.set("log".to_string(), JSValue::from_native_function(math_log));
    math.set(
        "random".to_string(),
        JSValue::from_native_function(math_random),
    );
    math.define_property(
        "LN2".to_string(),
        Property::read_only(JSValue::from_number(std::f64::consts::LN_2)),
    );
    math.define_property(
        "PI".to_string(),
        Property::read_only(JSValue::from_number(std::f64::consts::PI)),
    );
    global.borrow_mut().set(
        "Math".to_string(),
        JSValue::from_object(Rc::new(RefCell::new(math))),
    );
}
