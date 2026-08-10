use crate::error::{JSError, JSResult};
use crate::value::JSValue;
use crate::value::jsobject::{HOST_GET_PROPERTY, HOST_SET_PROPERTY, JSObject};
use crate::vm::VM;
use std::cell::RefCell;
use std::rc::Rc;

const PROXY_TARGET: &str = "__proxy_target__";
const PROXY_HANDLER: &str = "__proxy_handler__";

fn proxy_constructor(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let target = args.get(1).cloned().unwrap_or(JSValue::Undefined);
    let handler = args.get(2).cloned().unwrap_or(JSValue::Undefined);
    if !matches!(
        &target,
        JSValue::Object(..) | JSValue::Function(..) | JSValue::ArrowFunction(..)
    ) || !matches!(&handler, JSValue::Object(..))
    {
        return Err(JSError::TypeError(
            "Proxy target and handler must be objects".to_string(),
        ));
    }

    let mut proxy = JSObject::new();
    proxy.set(PROXY_TARGET.to_string(), target);
    proxy.set(PROXY_HANDLER.to_string(), handler);
    proxy.set(
        HOST_GET_PROPERTY.to_string(),
        JSValue::NativeFunction(proxy_get),
    );
    proxy.set(
        HOST_SET_PROPERTY.to_string(),
        JSValue::NativeFunction(proxy_set),
    );
    Ok(JSValue::Object(Rc::new(RefCell::new(proxy))))
}

fn proxy_get(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let receiver = args.first().cloned().unwrap_or(JSValue::Undefined);
    let key = args.get(1).cloned().unwrap_or(JSValue::Undefined);
    let JSValue::Object(proxy) = &receiver else {
        return Ok(JSValue::Undefined);
    };
    let target = proxy.borrow().get(PROXY_TARGET);
    let handler = proxy.borrow().get(PROXY_HANDLER);
    if let JSValue::Object(handler_object) = &handler {
        let trap = handler_object.borrow().get("get");
        if !matches!(&trap, JSValue::Undefined | JSValue::Null) {
            return vm.call(trap, handler, vec![target, key, receiver]);
        }
    }
    match target {
        JSValue::Object(object) => Ok(object.borrow().get(&key.to_string())),
        _ => Ok(JSValue::Undefined),
    }
}

fn proxy_set(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let receiver = args.first().cloned().unwrap_or(JSValue::Undefined);
    let key = args.get(1).cloned().unwrap_or(JSValue::Undefined);
    let value = args.get(2).cloned().unwrap_or(JSValue::Undefined);
    let JSValue::Object(proxy) = &receiver else {
        return Ok(JSValue::Boolean(false));
    };
    let target = proxy.borrow().get(PROXY_TARGET);
    let handler = proxy.borrow().get(PROXY_HANDLER);
    if let JSValue::Object(handler_object) = &handler {
        let trap = handler_object.borrow().get("set");
        if !matches!(&trap, JSValue::Undefined | JSValue::Null) {
            return vm.call(trap, handler, vec![target, key, value, receiver]);
        }
    }
    if let JSValue::Object(object) = target {
        object.borrow_mut().set(key.to_string(), value);
        Ok(JSValue::Boolean(true))
    } else {
        Ok(JSValue::Boolean(false))
    }
}

pub fn install(global: &Rc<RefCell<JSObject>>) {
    let mut constructor = JSObject::new();
    constructor.set(
        "__construct__".to_string(),
        JSValue::NativeFunction(proxy_constructor),
    );
    global.borrow_mut().set(
        "Proxy".to_string(),
        JSValue::Object(Rc::new(RefCell::new(constructor))),
    );
}
