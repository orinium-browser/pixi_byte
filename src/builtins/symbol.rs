//! Symbol registry values required by React.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::JSResult;
use crate::value::JSValue;
use crate::value::jsobject::{JSObject, Property};
use crate::vm::VM;

static NEXT_SYMBOL_ID: AtomicU64 = AtomicU64::new(1);

fn symbol_constructor(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let description = match args.get(1) {
        None => String::new(),
        Some(value) if value.clone().is_undefined() => String::new(),
        Some(value) => value.to_string(),
    };
    let id = NEXT_SYMBOL_ID.fetch_add(1, Ordering::Relaxed);
    Ok(JSValue::from_string(format!("@@symbol:{id}:{description}")))
}

fn symbol_for(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let key = args.get(1).map(JSValue::to_string).unwrap_or_default();
    Ok(JSValue::from_string(format!("@@symbol:{key}")))
}

/// Installs the global Symbol registry namespace.
pub fn install(global: &Rc<RefCell<JSObject>>) {
    let mut symbol = JSObject::new();
    symbol.set(
        "__call__".to_string(),
        JSValue::from_native_function(symbol_constructor),
    );
    symbol.set("for".to_string(), JSValue::from_native_function(symbol_for));
    symbol.define_property(
        "iterator".to_string(),
        Property::read_only(JSValue::from_string("@@iterator".to_string())),
    );
    symbol.define_property(
        "toStringTag".to_string(),
        Property::read_only(JSValue::from_string("@@toStringTag".to_string())),
    );
    symbol.define_property(
        "toPrimitive".to_string(),
        Property::read_only(JSValue::from_string("@@toPrimitive".to_string())),
    );
    global.borrow_mut().set(
        "Symbol".to_string(),
        JSValue::from_object(Rc::new(RefCell::new(symbol))),
    );
}
