//! Symbol registry values required by React.

use std::cell::RefCell;
use std::rc::Rc;

use crate::error::JSResult;
use crate::value::JSValue;
use crate::value::jsobject::{JSObject, Property};
use crate::vm::VM;

fn symbol_for(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let key = args.get(1).map(JSValue::to_string).unwrap_or_default();
    Ok(JSValue::String(format!("@@symbol:{key}")))
}

/// Installs the global Symbol registry namespace.
pub fn install(global: &Rc<RefCell<JSObject>>) {
    let mut symbol = JSObject::new();
    symbol.set("for".to_string(), JSValue::NativeFunction(symbol_for));
    symbol.define_property(
        "iterator".to_string(),
        Property::read_only(JSValue::String("@@iterator".to_string())),
    );
    global.borrow_mut().set(
        "Symbol".to_string(),
        JSValue::Object(Rc::new(RefCell::new(symbol))),
    );
}
