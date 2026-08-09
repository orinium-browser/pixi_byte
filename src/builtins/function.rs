use crate::value::JSValue;
use crate::value::jsobject::JSObject;
use crate::value::jsvalue::BoundFunctionData;
use std::cell::RefCell;
use std::rc::Rc;

// Native functions: function.call / function.apply

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
        JSValue::Undefined
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
        JSValue::Undefined
    };
    let args_array = if !args.is_empty() {
        args.remove(0)
    } else {
        JSValue::Undefined
    };

    // build argument vector from args_array
    let mut call_args_vec: Vec<JSValue> = Vec::new();
    match args_array {
        JSValue::Object(arr_ref) => {
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
        JSValue::Undefined | JSValue::Null => {}
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
        JSValue::Undefined
    };
    let bound_args = args; // remaining

    match func {
        JSValue::BoundFunction(boxed) => {
            // If already bound, preserve original target and bound_this, concatenate args
            let mut new_args = boxed.bound_args.clone();
            new_args.extend(bound_args);
            let bf = BoundFunctionData::new(
                (*boxed.target).clone(),
                boxed.bound_this.clone(),
                new_args,
            );
            Ok(JSValue::BoundFunction(Box::new(bf)))
        }
        JSValue::Function(_, _, _, _)
        | JSValue::ArrowFunction(_, _, _, _)
        | JSValue::NativeFunction(_) => {
            // create bound function wrapper
            let bf = BoundFunctionData::new(func.clone(), this_arg, bound_args);
            Ok(JSValue::BoundFunction(Box::new(bf)))
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
    let mut proto = JSObject::new();
    proto.set("call".to_string(), JSValue::NativeFunction(function_call));
    proto.set("apply".to_string(), JSValue::NativeFunction(function_apply));
    proto.set("bind".to_string(), JSValue::NativeFunction(function_bind));
    let fn_proto = Rc::new(RefCell::new(proto));
    fn_ctor.set("prototype".to_string(), JSValue::Object(fn_proto.clone()));

    global.borrow_mut().set(
        "Function".to_string(),
        JSValue::Object(Rc::new(RefCell::new(fn_ctor))),
    );

    fn_proto
}
