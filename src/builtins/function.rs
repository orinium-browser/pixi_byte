use crate::value::JSValue;
use crate::value::jsobject::JSObject;
use crate::value::jsvalue::{BoundFunctionData, JsValueKind};
use std::cell::RefCell;
use std::rc::Rc;

// Native functions: function.call / function.apply

fn dynamic_function_stub(
    _vm: &mut crate::vm::VM,
    _args: Vec<JSValue>,
) -> crate::error::JSResult<JSValue> {
    Ok(JSValue::undefined())
}

fn function_constructor(
    _vm: &mut crate::vm::VM,
    _args: Vec<JSValue>,
) -> crate::error::JSResult<JSValue> {
    Ok(JSValue::from_native_function(dynamic_function_stub))
}

fn function_call(
    vm: &mut crate::vm::VM,
    mut args: Vec<JSValue>,
) -> crate::error::JSResult<JSValue> {
    // args: [func, thisArg, ...args]
    if args.is_empty() {
        return Err(crate::error::JSError::TypeError(
            "Function.prototype.call: missing function receiver".to_string(),
        ));
    }
    let func = args.remove(0);
    let this_arg = if !args.is_empty() {
        args.remove(0)
    } else {
        JSValue::undefined()
    };

    vm.call(func, this_arg, args)
}

fn function_apply(
    vm: &mut crate::vm::VM,
    mut args: Vec<JSValue>,
) -> crate::error::JSResult<JSValue> {
    if args.is_empty() {
        return Err(crate::error::JSError::TypeError(
            "Function.prototype.apply: missing function receiver".to_string(),
        ));
    }
    let func = args.remove(0);
    let this_arg = if !args.is_empty() {
        args.remove(0)
    } else {
        JSValue::undefined()
    };
    let args_array = if !args.is_empty() {
        args.remove(0)
    } else {
        JSValue::undefined()
    };

    // build argument vector from args_array
    let mut call_args_vec: Vec<JSValue> = Vec::new();
    match args_array.kind() {
        JsValueKind::Object => {
            let arr_ref = args_array.as_object().unwrap();
            let len_val = arr_ref.borrow().get("length");
            let mut len = len_val.to_number();
            if len.is_nan() {
                len = 0.0;
            }
            let mut idx = 0usize;
            while (idx as f64) < len {
                let v = arr_ref.borrow().get(&idx.to_string());
                call_args_vec.push(v);
                idx += 1;
            }
        }
        JsValueKind::Undefined | JsValueKind::Null => {}
        _ => {
            return Err(crate::error::JSError::TypeError(
                "Function.prototype.apply: second argument must be an array or null/undefined"
                    .to_string(),
            ));
        }
    }

    vm.call(func, this_arg, call_args_vec)
}

fn function_bind(
    _vm: &mut crate::vm::VM,
    mut args: Vec<JSValue>,
) -> crate::error::JSResult<JSValue> {
    // args: [func, thisArg, ...boundArgs]
    if args.is_empty() {
        return Err(crate::error::JSError::TypeError(
            "Function.prototype.bind: missing function receiver".to_string(),
        ));
    }
    let func = args.remove(0);
    let this_arg = if !args.is_empty() {
        args.remove(0)
    } else {
        JSValue::undefined()
    };
    let bound_args = args; // remaining

    match func.kind() {
        JsValueKind::BoundFunction => {
            let boxed = func.as_bound_function().unwrap();
            // If already bound, preserve original target and bound_this, concatenate args
            let mut new_args = boxed.bound_args.clone();
            new_args.extend(bound_args);
            let bf =
                BoundFunctionData::new((*boxed.target).clone(), boxed.bound_this.clone(), new_args);
            Ok(JSValue::from_bound_function(bf))
        }
        JsValueKind::Function | JsValueKind::ArrowFunction | JsValueKind::NativeFunction => {
            // create bound function wrapper
            let bf = BoundFunctionData::new(func.clone(), this_arg, bound_args);
            Ok(JSValue::from_bound_function(bf))
        }
        JsValueKind::Object
            if !matches!(
                func.as_object().unwrap().borrow().get("__call__").kind(),
                JsValueKind::Undefined
            ) =>
        {
            let func = JSValue::from_object(func.as_object().unwrap());
            let bf = BoundFunctionData::new(func, this_arg, bound_args);
            Ok(JSValue::from_bound_function(bf))
        }
        _ => Err(crate::error::JSError::TypeError(
            "Function.prototype.bind: receiver is not a function".to_string(),
        )),
    }
}

pub fn install(global: &Rc<RefCell<JSObject>>) -> Rc<RefCell<JSObject>> {
    // Create Function constructor-like object (minimal)
    let mut fn_ctor = JSObject::new();
    // Function.prototype object
    let object_prototype = {
        let object = global.borrow().get("Object");
        match object.kind() {
            JsValueKind::Object => {
                let constructor = object.as_object().unwrap();
                let prototype = constructor.borrow().get("prototype");

                match prototype.kind() {
                    JsValueKind::Object => Some(prototype.as_object().unwrap()),
                    _ => None,
                }
            }
            _ => None,
        }
    };
    let mut proto = JSObject::with_prototype(object_prototype);
    proto.set(
        "call".to_string(),
        JSValue::from_native_function(function_call),
    );
    proto.set(
        "apply".to_string(),
        JSValue::from_native_function(function_apply),
    );
    proto.set(
        "bind".to_string(),
        JSValue::from_native_function(function_bind),
    );
    let fn_proto = Rc::new(RefCell::new(proto));
    fn_ctor.set(
        "__call__".to_string(),
        JSValue::from_native_function(function_constructor),
    );
    fn_ctor.set(
        "__construct__".to_string(),
        JSValue::from_native_function(function_constructor),
    );
    fn_ctor.set_prototype(Some(Rc::clone(&fn_proto)));
    fn_ctor.set(
        "prototype".to_string(),
        JSValue::from_object(fn_proto.clone()),
    );

    global.borrow_mut().set(
        "Function".to_string(),
        JSValue::from_object(Rc::new(RefCell::new(fn_ctor))),
    );

    fn_proto
}
