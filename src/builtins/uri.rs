//! URI encoding functions used by browser-targeted bundles.

use std::cell::RefCell;
use std::fmt::Write;
use std::rc::Rc;

use crate::error::JSResult;
use crate::value::jsobject::JSObject;
use crate::value::JSValue;
use crate::vm::VM;

fn is_uri_component_unescaped(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(byte, b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')')
}

fn encode_uri_component(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let input = args.get(1).map(JSValue::to_string).unwrap_or_default();
    let mut output = String::with_capacity(input.len());
    for byte in input.bytes() {
        if is_uri_component_unescaped(byte) {
            output.push(byte as char);
        } else {
            let _ = write!(output, "%{byte:02X}");
        }
    }
    Ok(JSValue::String(output))
}

/// Installs URI encoding functions on the global object.
pub fn install(global: &Rc<RefCell<JSObject>>) {
    global.borrow_mut().set(
        "encodeURIComponent".to_string(),
        JSValue::NativeFunction(encode_uri_component),
    );
}
