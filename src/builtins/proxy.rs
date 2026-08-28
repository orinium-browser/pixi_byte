use crate::error::{JSError, JSResult};
use crate::value::JSValue;
use crate::value::jsobject::{HOST_GET_PROPERTY, HOST_SET_PROPERTY, JSObject, Property};
use crate::value::jsvalue::BoundFunctionData;
use crate::vm::VM;
use std::cell::RefCell;
use std::rc::Rc;

const PROXY_TARGET: &str = "__proxy_target__";
const PROXY_HANDLER: &str = "__proxy_handler__";

fn is_object_or_callable(value: &JSValue) -> bool {
    use crate::value::jsvalue::JsValueKind;
    matches!(
        value.clone().kind(),
        JsValueKind::Object
            | JsValueKind::Function
            | JsValueKind::ArrowFunction
            | JsValueKind::NativeFunction
            | JsValueKind::BoundFunction
    )
}

fn proxy_constructor(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let target = args.get(1).cloned().unwrap_or(JSValue::undefined());
    let handler = args.get(2).cloned().unwrap_or(JSValue::undefined());
    if !is_object_or_callable(&target) || !handler.clone().is_object() {
        return Err(JSError::TypeError(
            "Proxy target and handler must be objects".to_string(),
        ));
    }

    let mut proxy = JSObject::new();
    proxy.define_property(PROXY_TARGET.to_string(), internal_property(target.clone()));
    proxy.define_property(PROXY_HANDLER.to_string(), internal_property(handler));
    proxy.define_property(
        HOST_GET_PROPERTY.to_string(),
        internal_property(JSValue::from_native_function(proxy_get)),
    );
    proxy.define_property(
        HOST_SET_PROPERTY.to_string(),
        internal_property(JSValue::from_native_function(proxy_set)),
    );
    let proxy = Rc::new(RefCell::new(proxy));
    if vm.is_callable(&target) {
        proxy
            .borrow_mut()
            .set_prototype(Some(Rc::clone(&vm.function_prototype)));
        proxy.borrow_mut().define_property(
            "__call__".to_string(),
            internal_property(JSValue::from_bound_function(BoundFunctionData::new(
                JSValue::from_native_function(proxy_call),
                JSValue::from_object(Rc::clone(&proxy)),
                Vec::new(),
            ))),
        );
        let construct = {
            use crate::value::jsvalue::JsValueKind;
            let k = target.clone().kind();
            if k == JsValueKind::Function || k == JsValueKind::BoundFunction {
                Some(target.clone())
            } else {
                target.as_object().and_then(|object| {
                    let c = object.borrow().get("__construct__");
                    (!c.clone().is_undefined()).then_some(c)
                })
            }
        };
        if let Some(construct) = construct {
            proxy
                .borrow_mut()
                .define_property("__construct__".to_string(), internal_property(construct));
        }
    }
    Ok(JSValue::from_object(proxy))
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
    let receiver = args.first().cloned().unwrap_or(JSValue::undefined());
    let key = args.get(1).cloned().unwrap_or(JSValue::undefined());
    let proxy = match receiver.as_object() {
        Some(o) => o,
        None => return Ok(JSValue::undefined()),
    };
    let target = proxy.borrow().get(PROXY_TARGET);
    let handler = proxy.borrow().get(PROXY_HANDLER);
    if let Some(handler_object) = handler.as_object() {
        let trap = handler_object.borrow().get("get");
        if !trap.clone().is_undefined() && !trap.clone().is_null() {
            return vm.call(trap, handler, vec![target, key, receiver]);
        }
    }
    if let Some(object) = target.as_object() {
        Ok(object.borrow().get(&key.to_string()))
    } else if is_object_or_callable(&target) {
        Ok(vm
            .user_function_object(&target)
            .map(|object| object.borrow().get(&key.to_string()))
            .unwrap_or_else(|| vm.function_prototype.borrow().get(&key.to_string())))
    } else {
        Ok(JSValue::undefined())
    }
}

fn proxy_set(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let receiver = args.first().cloned().unwrap_or(JSValue::undefined());
    let key = args.get(1).cloned().unwrap_or(JSValue::undefined());
    let value = args.get(2).cloned().unwrap_or(JSValue::undefined());
    let proxy = match receiver.as_object() {
        Some(o) => o,
        None => return Ok(JSValue::from_bool(false)),
    };
    let target = proxy.borrow().get(PROXY_TARGET);
    let handler = proxy.borrow().get(PROXY_HANDLER);
    if let Some(handler_object) = handler.as_object() {
        let trap = handler_object.borrow().get("set");
        if !trap.clone().is_undefined() && !trap.clone().is_null() {
            return vm.call(trap, handler, vec![target, key, value, receiver]);
        }
    }
    if let Some(object) = target.as_object() {
        object.borrow_mut().set(key.to_string(), value);
        Ok(JSValue::from_bool(true))
    } else {
        Ok(JSValue::from_bool(false))
    }
}

fn proxy_call(vm: &mut VM, mut args: Vec<JSValue>) -> JSResult<JSValue> {
    let proxy = args
        .first()
        .cloned()
        .and_then(|v| v.as_object())
        .ok_or_else(|| JSError::TypeError("invalid callable Proxy".to_string()))?;
    args.remove(0);
    let target = proxy.borrow().get(PROXY_TARGET);
    let handler = proxy.borrow().get(PROXY_HANDLER);
    // Object-call forwarding does not currently retain the dynamic receiver;
    // ordinary proxy-wrapped callbacks use undefined here, as in strict mode.
    let this_arg = JSValue::undefined();
    let call_args = args;
    if let Some(handler_object) = handler.as_object() {
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
        JSValue::from_native_function(proxy_constructor),
    );
    global.borrow_mut().set(
        "Proxy".to_string(),
        JSValue::from_object(Rc::new(RefCell::new(constructor))),
    );
}
