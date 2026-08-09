//! Minimal ECMAScript Promise implementation backed by the VM job queue.

use std::cell::RefCell;
use std::rc::Rc;

use crate::error::{JSError, JSResult};
use crate::value::JSValue;
use crate::value::jsobject::JSObject;
use crate::value::jsvalue::BoundFunctionData;
use crate::vm::VM;

const STATE: &str = "__promise_state";
const RESULT: &str = "__promise_result";
const REACTION_COUNT: &str = "__promise_reaction_count";

fn is_callable(value: &JSValue) -> bool {
    matches!(
        value,
        JSValue::Function(..) | JSValue::NativeFunction(..) | JSValue::BoundFunction(..)
    )
}

fn new_promise() -> Rc<RefCell<JSObject>> {
    let mut promise = JSObject::new();
    promise.set(STATE.to_string(), JSValue::String("pending".to_string()));
    promise.set(RESULT.to_string(), JSValue::Undefined);
    promise.set(REACTION_COUNT.to_string(), JSValue::Number(0.0));
    promise.set("then".to_string(), JSValue::NativeFunction(promise_then));
    promise.set("catch".to_string(), JSValue::NativeFunction(promise_catch));
    Rc::new(RefCell::new(promise))
}

fn bound_settler(
    function: fn(&mut VM, Vec<JSValue>) -> JSResult<JSValue>,
    promise: &Rc<RefCell<JSObject>>,
) -> JSValue {
    JSValue::BoundFunction(Box::new(BoundFunctionData {
        target: Box::new(JSValue::NativeFunction(function)),
        bound_this: JSValue::Object(Rc::clone(promise)),
        bound_args: Vec::new(),
    }))
}

fn promise_constructor(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let executor = args.get(1).cloned().unwrap_or(JSValue::Undefined);
    if !is_callable(&executor) {
        return Err(JSError::TypeError(
            "Promise executor must be callable".to_string(),
        ));
    }

    let promise = new_promise();
    let resolve = bound_settler(promise_resolve, &promise);
    let reject = bound_settler(promise_reject, &promise);
    if let Err(err) = vm.call(executor, JSValue::Undefined, vec![resolve, reject]) {
        settle_promise(vm, &promise, true, JSValue::String(err.to_string()));
    }
    Ok(JSValue::Object(promise))
}

fn promise_resolve(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let promise = promise_receiver(args.first())?;
    let value = args.get(1).cloned().unwrap_or(JSValue::Undefined);
    resolve_promise(vm, &promise, value);
    Ok(JSValue::Undefined)
}

fn promise_reject(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let promise = promise_receiver(args.first())?;
    let reason = args.get(1).cloned().unwrap_or(JSValue::Undefined);
    settle_promise(vm, &promise, true, reason);
    Ok(JSValue::Undefined)
}

fn promise_then(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let promise = promise_receiver(args.first())?;
    let on_fulfilled = args.get(1).cloned().unwrap_or(JSValue::Undefined);
    let on_rejected = args.get(2).cloned().unwrap_or(JSValue::Undefined);
    let child = new_promise();

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
            JSValue::Object(Rc::clone(&child)),
        );
        promise.set(
            REACTION_COUNT.to_string(),
            JSValue::Number((index + 1) as f64),
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

    Ok(JSValue::Object(child))
}

fn promise_catch(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let promise = args.first().cloned().unwrap_or(JSValue::Undefined);
    let on_rejected = args.get(1).cloned().unwrap_or(JSValue::Undefined);
    promise_then(vm, vec![promise, JSValue::Undefined, on_rejected])
}

fn promise_receiver(value: Option<&JSValue>) -> JSResult<Rc<RefCell<JSObject>>> {
    let Some(JSValue::Object(promise)) = value else {
        return Err(JSError::TypeError(
            "Promise method called on incompatible receiver".to_string(),
        ));
    };
    if !promise.borrow().has_own_property(STATE) {
        return Err(JSError::TypeError(
            "Promise method called on incompatible receiver".to_string(),
        ));
    }
    Ok(Rc::clone(promise))
}

fn resolve_promise(vm: &mut VM, promise: &Rc<RefCell<JSObject>>, value: JSValue) {
    if let JSValue::Object(other) = &value
        && other.borrow().has_own_property(STATE)
    {
        if Rc::ptr_eq(promise, other) {
            settle_promise(
                vm,
                promise,
                true,
                JSValue::String("Promise cannot resolve itself".to_string()),
            );
            return;
        }
        let resolve = bound_settler(promise_resolve, promise);
        let reject = bound_settler(promise_reject, promise);
        let _ = promise_then(vm, vec![JSValue::Object(Rc::clone(other)), resolve, reject]);
        return;
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
            JSValue::String(if rejected { "rejected" } else { "fulfilled" }.to_string()),
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
        if let JSValue::Object(child) = child {
            enqueue_reaction(vm, child, handler, result.clone(), rejected);
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

    let job = JSValue::BoundFunction(Box::new(BoundFunctionData {
        target: Box::new(JSValue::NativeFunction(promise_reaction_job)),
        bound_this: JSValue::Object(child),
        bound_args: vec![handler, value],
    }));
    vm.enqueue_job(job, JSValue::Undefined, Vec::new());
}

fn promise_reaction_job(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let child = promise_receiver(args.first())?;
    let handler = args.get(1).cloned().unwrap_or(JSValue::Undefined);
    let value = args.get(2).cloned().unwrap_or(JSValue::Undefined);
    match vm.call(handler, JSValue::Undefined, vec![value]) {
        Ok(value) => resolve_promise(vm, &child, value),
        Err(err) => settle_promise(vm, &child, true, JSValue::String(err.to_string())),
    }
    Ok(JSValue::Undefined)
}

/// Installs the Promise constructor.
pub fn install(global: &Rc<RefCell<JSObject>>) {
    global.borrow_mut().set(
        "Promise".to_string(),
        JSValue::NativeFunction(promise_constructor),
    );
}
