//! Minimal ECMAScript Promise implementation backed by the VM job queue.

use std::cell::RefCell;
use std::rc::Rc;

use crate::error::{JSError, JSResult};
use crate::value::jsobject::JSObject;
use crate::value::jsvalue::BoundFunctionData;
use crate::value::{JSValue, Property};
use crate::vm::VM;

const STATE: &str = "__promise_state";
const RESULT: &str = "__promise_result";
const REACTION_COUNT: &str = "__promise_reaction_count";

fn is_callable(value: &JSValue) -> bool {
    value.is_callable()
}

fn new_promise(vm: &VM) -> Rc<RefCell<JSObject>> {
    let prototype = vm
        .global_object
        .borrow()
        .get("Promise")
        .as_object()
        .and_then(|constructor| {
            let proto = constructor.borrow().get("prototype");
            proto.as_object()
        });
    let mut promise = JSObject::with_prototype(prototype);
    promise.set(
        STATE.to_string(),
        JSValue::from_string("pending".to_string()),
    );
    promise.set(RESULT.to_string(), JSValue::undefined());
    promise.set(REACTION_COUNT.to_string(), JSValue::from_number(0.0));
    promise.set(
        "then".to_string(),
        JSValue::from_native_function(promise_then),
    );
    promise.set(
        "catch".to_string(),
        JSValue::from_native_function(promise_catch),
    );
    Rc::new(RefCell::new(promise))
}

fn bound_settler(
    function: fn(&mut VM, Vec<JSValue>) -> JSResult<JSValue>,
    promise: &Rc<RefCell<JSObject>>,
) -> JSValue {
    JSValue::from_bound_function(BoundFunctionData::new(
        JSValue::from_native_function(function),
        JSValue::from_object(Rc::clone(promise)),
        Vec::new(),
    ))
}

fn promise_constructor(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let executor = args.get(1).cloned().unwrap_or(JSValue::undefined());
    if !is_callable(&executor) {
        return Err(JSError::TypeError(
            "Promise executor must be callable".to_string(),
        ));
    }

    let promise = new_promise(vm);
    let resolve = bound_settler(promise_resolve, &promise);
    let reject = bound_settler(promise_reject, &promise);
    if let Err(err) = vm.call(executor, JSValue::undefined(), vec![resolve, reject]) {
        settle_promise(vm, &promise, true, JSValue::from_string(err.to_string()));
    }
    Ok(JSValue::from_object(promise))
}

fn promise_resolve(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let promise = promise_receiver(args.first())?;
    let value = args.get(1).cloned().unwrap_or(JSValue::undefined());
    resolve_promise(vm, &promise, value);
    Ok(JSValue::undefined())
}

fn promise_reject(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let promise = promise_receiver(args.first())?;
    let reason = args.get(1).cloned().unwrap_or(JSValue::undefined());
    settle_promise(vm, &promise, true, reason);
    Ok(JSValue::undefined())
}

fn promise_resolve_static(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let value = args.get(1).cloned().unwrap_or(JSValue::undefined());
    if let Some(object) = value.as_object() {
        if object.borrow().has_own_property(STATE) {
            return Ok(value);
        }
    }

    let promise = new_promise(vm);
    resolve_promise(vm, &promise, value);
    Ok(JSValue::from_object(promise))
}

fn promise_reject_static(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let reason = args.get(1).cloned().unwrap_or(JSValue::undefined());
    let promise = new_promise(vm);
    settle_promise(vm, &promise, true, reason);
    Ok(JSValue::from_object(promise))
}

fn promise_all(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let values = args.get(1).and_then(|v| v.as_object()).ok_or_else(|| {
        JSError::TypeError("Promise.all expects an array-like object".to_string())
    })?;
    let length = values.borrow().get("length").to_number() as usize;
    let result = new_promise(vm);
    if length == 0 {
        let values = vm.array_from_values(Vec::new());
        resolve_promise(vm, &result, values);
        return Ok(JSValue::from_object(result));
    }

    let mut tracker = JSObject::new();
    tracker.set("remaining".to_string(), JSValue::from_number(length as f64));
    tracker.set(
        "result".to_string(),
        JSValue::from_object(Rc::clone(&result)),
    );
    tracker.set(
        "values".to_string(),
        vm.array_from_values(vec![JSValue::undefined(); length]),
    );
    let tracker = Rc::new(RefCell::new(tracker));

    for index in 0..length {
        let value = values.borrow().get(&index.to_string());
        let promise = promise_resolve_static(vm, vec![JSValue::undefined(), value])?;
        let on_fulfilled = JSValue::from_bound_function(BoundFunctionData::new(
            JSValue::from_native_function(promise_all_fulfill),
            JSValue::from_object(Rc::clone(&tracker)),
            vec![JSValue::from_number(index as f64)],
        ));
        let on_rejected = bound_settler(promise_reject, &result);
        let _ = promise_then(vm, vec![promise, on_fulfilled, on_rejected])?;
    }

    Ok(JSValue::from_object(result))
}

fn promise_all_fulfill(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let tracker = args
        .first()
        .and_then(|v| v.as_object())
        .ok_or_else(|| JSError::TypeError("invalid Promise.all tracker".to_string()))?;
    let index = args.get(1).map(JSValue::to_number).unwrap_or(0.0) as usize;
    let value = args.get(2).cloned().unwrap_or(JSValue::undefined());
    let values = tracker.borrow().get("values");
    if let Some(values_object) = values.as_object() {
        values_object.borrow_mut().set(index.to_string(), value);
    }

    let remaining = tracker.borrow().get("remaining").to_number() as usize - 1;
    tracker.borrow_mut().set(
        "remaining".to_string(),
        JSValue::from_number(remaining as f64),
    );
    if remaining == 0 {
        let result = tracker.borrow().get("result");
        if let Some(result_object) = result.as_object() {
            resolve_promise(vm, &result_object, values);
        }
    }
    Ok(JSValue::undefined())
}

fn promise_then(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let promise = promise_receiver(args.first())?;
    let on_fulfilled = args.get(1).cloned().unwrap_or(JSValue::undefined());
    let on_rejected = args.get(2).cloned().unwrap_or(JSValue::undefined());
    let child = new_promise(vm);

    let (state, result) = {
        let promise = promise.borrow();
        (promise.get(STATE).to_string(), promise.get(RESULT))
    };
    if state == "pending" {
        let index = promise.borrow().get(REACTION_COUNT).to_number() as usize;
        let mut promise = promise.borrow_mut();
        promise.set(format!("__promise_fulfill_{index}"), on_fulfilled);
        promise.set(format!("__promise_reject_{index}"), on_rejected);
        promise.set(
            format!("__promise_child_{index}"),
            JSValue::from_object(Rc::clone(&child)),
        );
        promise.set(
            REACTION_COUNT.to_string(),
            JSValue::from_number((index + 1) as f64),
        );
    } else {
        enqueue_reaction(
            vm,
            child.clone(),
            if state == "rejected" {
                on_rejected
            } else {
                on_fulfilled
            },
            result,
            state == "rejected",
        );
    }

    Ok(JSValue::from_object(child))
}

fn promise_catch(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let promise = args.first().cloned().unwrap_or(JSValue::undefined());
    let on_rejected = args.get(1).cloned().unwrap_or(JSValue::undefined());
    promise_then(vm, vec![promise, JSValue::undefined(), on_rejected])
}

fn await_value(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let value = args.get(1).cloned().unwrap_or(JSValue::undefined());
    let object = match value.as_object() {
        Some(o) => o,
        None => return Ok(value),
    };
    if object.borrow().has_own_property(STATE) {
        let mut state = object.borrow().get(STATE).to_string();
        if state == "pending" {
            vm.run_jobs()?;
            state = object.borrow().get(STATE).to_string();
        }
        let result = object.borrow().get(RESULT);
        return match state.as_str() {
            "fulfilled" => Ok(result),
            "rejected" => Err(JSError::Thrown(result)),
            _ => Ok(value),
        };
    }
    let sync = object.borrow().get("sync");
    if is_callable(&sync) {
        let resolved = vm.call(sync, value.clone(), Vec::new())?;
        let to_string = object.borrow().get("toString");
        if is_callable(&to_string) {
            let css = vm.call(to_string, value, Vec::new())?;
            if let Some(result_object) = resolved.as_object() {
                if css.clone().is_string() {
                    result_object.borrow_mut().set("css".to_string(), css);
                }
            }
        }
        return Ok(resolved);
    }
    Ok(value)
}

fn promise_receiver(value: Option<&JSValue>) -> JSResult<Rc<RefCell<JSObject>>> {
    let promise = value.and_then(|v| v.as_object()).ok_or_else(|| {
        JSError::TypeError("Promise method called on incompatible receiver".to_string())
    })?;
    if !promise.borrow().has_own_property(STATE) {
        return Err(JSError::TypeError(
            "Promise method called on incompatible receiver".to_string(),
        ));
    }
    Ok(promise)
}

fn resolve_promise(vm: &mut VM, promise: &Rc<RefCell<JSObject>>, value: JSValue) {
    if let Some(other) = value.as_object() {
        if other.borrow().has_own_property(STATE) {
            if Rc::ptr_eq(promise, &other) {
                settle_promise(
                    vm,
                    promise,
                    true,
                    JSValue::from_string("Promise cannot resolve itself".to_string()),
                );
                return;
            }
            let resolve = bound_settler(promise_resolve, promise);
            let reject = bound_settler(promise_reject, promise);
            let _ = promise_then(vm, vec![JSValue::from_object(other), resolve, reject]);
            return;
        }
    }
    settle_promise(vm, promise, false, value);
}

fn settle_promise(vm: &mut VM, promise: &Rc<RefCell<JSObject>>, rejected: bool, result: JSValue) {
    if promise.borrow().get(STATE).to_string() != "pending" {
        return;
    }
    let reaction_count = promise.borrow().get(REACTION_COUNT).to_number() as usize;
    {
        let mut promise = promise.borrow_mut();
        promise.set(
            STATE.to_string(),
            JSValue::from_string(if rejected { "rejected" } else { "fulfilled" }.to_string()),
        );
        promise.set(RESULT.to_string(), result.clone());
    }

    for index in 0..reaction_count {
        let (handler, child) = {
            let promise = promise.borrow();
            let handler_key = if rejected {
                format!("__promise_reject_{index}")
            } else {
                format!("__promise_fulfill_{index}")
            };
            let handler = promise.get(&handler_key);
            let child = promise.get(&format!("__promise_child_{index}"));
            (handler, child)
        };
        if let Some(child_object) = child.as_object() {
            enqueue_reaction(vm, child_object, handler, result.clone(), rejected);
        }
    }
}

fn enqueue_reaction(
    vm: &mut VM,
    child: Rc<RefCell<JSObject>>,
    handler: JSValue,
    value: JSValue,
    rejected: bool,
) {
    if !is_callable(&handler) {
        if rejected {
            settle_promise(vm, &child, true, value);
        } else {
            resolve_promise(vm, &child, value);
        }
        return;
    }

    let job = JSValue::from_bound_function(BoundFunctionData::new(
        JSValue::from_native_function(promise_reaction_job),
        JSValue::from_object(child),
        vec![handler, value],
    ));
    vm.enqueue_job(job, JSValue::undefined(), Vec::new());
}

fn promise_reaction_job(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let child = promise_receiver(args.first())?;
    let handler = args.get(1).cloned().unwrap_or(JSValue::undefined());
    let value = args.get(2).cloned().unwrap_or(JSValue::undefined());
    match vm.call(handler, JSValue::undefined(), vec![value]) {
        Ok(value) => resolve_promise(vm, &child, value),
        Err(err) => settle_promise(vm, &child, true, JSValue::from_string(err.to_string())),
    }
    Ok(JSValue::undefined())
}

/// Installs the Promise constructor.
pub fn install(global: &Rc<RefCell<JSObject>>) {
    let mut prototype = JSObject::new();
    prototype.set(
        "then".to_string(),
        JSValue::from_native_function(promise_then),
    );
    prototype.set(
        "catch".to_string(),
        JSValue::from_native_function(promise_catch),
    );
    prototype.set(
        "@@toStringTag".to_string(),
        JSValue::from_string("Promise".to_string()),
    );
    let prototype = Rc::new(RefCell::new(prototype));
    let mut constructor = JSObject::new();
    constructor.set(
        "__construct__".to_string(),
        JSValue::from_native_function(promise_constructor),
    );
    constructor.set(
        "resolve".to_string(),
        JSValue::from_native_function(promise_resolve_static),
    );
    constructor.set(
        "reject".to_string(),
        JSValue::from_native_function(promise_reject_static),
    );
    constructor.set(
        "all".to_string(),
        JSValue::from_native_function(promise_all),
    );
    constructor.set(
        "prototype".to_string(),
        JSValue::from_object(Rc::clone(&prototype)),
    );
    let constructor = Rc::new(RefCell::new(constructor));
    prototype.borrow_mut().set(
        "constructor".to_string(),
        JSValue::from_object(Rc::clone(&constructor)),
    );
    constructor.borrow_mut().set(
        "@@species".to_string(),
        JSValue::from_object(Rc::clone(&constructor)),
    );
    global.borrow_mut().define_property(
        "Promise".to_string(),
        Property::read_only(JSValue::from_object(constructor)),
    );
    global.borrow_mut().set(
        "__pixi_await".to_string(),
        JSValue::from_native_function(await_value),
    );
}
