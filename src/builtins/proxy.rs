use crate::error::{JSError, JSResult};
use crate::value::JSValue;
use crate::value::jsobject::{HOST_GET_PROPERTY, HOST_SET_PROPERTY, JSObject, Property};
use crate::value::jsvalue::BoundFunctionData;
use crate::vm::VM;
use std::cell::RefCell;
use std::rc::Rc;

const PROXY_TARGET: &str = "__proxy_target__";
const PROXY_HANDLER: &str = "__proxy_handler__";

fn proxy_constructor(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let target = args.get(1).cloned().unwrap_or(JSValue::Undefined);
    let handler = args.get(2).cloned().unwrap_or(JSValue::Undefined);
    if !matches!(
        &target,
        JSValue::Object(..)
            | JSValue::Function(..)
            | JSValue::ArrowFunction(..)
            | JSValue::NativeFunction(..)
            | JSValue::BoundFunction(..)
    ) || !matches!(&handler, JSValue::Object(..))
    {
        return Err(JSError::TypeError(
            "Proxy target and handler must be objects".to_string(),
        ));
    }

    let mut proxy = JSObject::new();
    proxy.define_property(PROXY_TARGET.to_string(), internal_property(target.clone()));
    proxy.define_property(PROXY_HANDLER.to_string(), internal_property(handler));
    proxy.define_property(
        HOST_GET_PROPERTY.to_string(),
        internal_property(JSValue::NativeFunction(proxy_get)),
    );
    proxy.define_property(
        HOST_SET_PROPERTY.to_string(),
        internal_property(JSValue::NativeFunction(proxy_set)),
    );
    let proxy = Rc::new(RefCell::new(proxy));
    if vm.is_callable(&target) {
        proxy
            .borrow_mut()
            .set_prototype(Some(Rc::clone(&vm.function_prototype)));
        proxy.borrow_mut().define_property(
            "__call__".to_string(),
            internal_property(JSValue::BoundFunction(Box::new(BoundFunctionData::new(
                JSValue::NativeFunction(proxy_call),
                JSValue::Object(Rc::clone(&proxy)),
                Vec::new(),
            )))),
        );
        let construct = match &target {
            JSValue::Function(..) | JSValue::BoundFunction(..) => Some(target.clone()),
            JSValue::Object(object) => {
                let construct = object.borrow().get("__construct__");
                (!matches!(construct, JSValue::Undefined)).then_some(construct)
            }
            _ => None,
        };
        if let Some(construct) = construct {
            proxy
                .borrow_mut()
                .define_property("__construct__".to_string(), internal_property(construct));
        }
    }
    Ok(JSValue::Object(proxy))
}

fn internal_property(value: JSValue) -> Property {
    Property {
        value,
        enumerable: false,
        writable: false,
        configurable: false,
        getter: None,
        setter: None,
    }
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
    match &target {
        JSValue::Object(object) => Ok(object.borrow().get(&key.to_string())),
        JSValue::Function(..) | JSValue::ArrowFunction(..) | JSValue::BoundFunction(..) => Ok(vm
            .user_function_object(&target)
            .map(|object| object.borrow().get(&key.to_string()))
            .unwrap_or_else(|| vm.function_prototype.borrow().get(&key.to_string()))),
        JSValue::NativeFunction(..) => Ok(vm.function_prototype.borrow().get(&key.to_string())),
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

fn proxy_call(vm: &mut VM, mut args: Vec<JSValue>) -> JSResult<JSValue> {
    let Some(JSValue::Object(proxy)) = args.first().cloned() else {
        return Err(JSError::TypeError("invalid callable Proxy".to_string()));
    };
    args.remove(0);
    let target = proxy.borrow().get(PROXY_TARGET);
    let handler = proxy.borrow().get(PROXY_HANDLER);
    // Object-call forwarding does not currently retain the dynamic receiver;
    // ordinary proxy-wrapped callbacks use undefined here, as in strict mode.
    let this_arg = JSValue::Undefined;
    let call_args = args;
    if let JSValue::Object(handler_object) = &handler {
        let trap = handler_object.borrow().get("apply");
        if vm.is_callable(&trap) {
            let arguments = vm.array_from_values(call_args);
            return vm.call(trap, handler, vec![target, this_arg, arguments]);
        }
    }
    vm.call(target, this_arg, call_args)
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
