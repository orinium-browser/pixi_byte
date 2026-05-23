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

    match func {
        JSValue::NativeFunction(native_fn) => {
            // call native with thisArg as first arg, then provided args
            let mut call_args = Vec::new();
            call_args.push(this_arg);
            call_args.extend(args);
            let res = native_fn(vm, call_args)?;
            Ok(res)
        }
        JSValue::Function(func_chunk, params, captured_env_opt, name_opt) => {
            // Prepare outer environment (captured or current)
            let outer = match captured_env_opt {
                Some(env_rc) => env_rc,
                None => vm.env.clone(),
            };
            let new_env = Rc::new(RefCell::new(crate::runtime::Environment::with_outer(outer)));

            // Bind parameters from args (args currently contains the function args, not thisArg)
            for (i, param_name) in params.iter().enumerate() {
                if i < args.len() {
                    new_env.borrow().define(param_name.clone(), args[i].clone());
                } else {
                    // missing args -> undefined
                    new_env
                        .borrow()
                        .define(param_name.clone(), JSValue::Undefined);
                }
            }

            // If extra args exist, store as argN
            for i in params.len()..args.len() {
                new_env
                    .borrow()
                    .define(format!("arg{}", i), args[i].clone());
            }

            // Named function expression handling: define name in env if present
            if let Some(name) = name_opt.clone() {
                new_env.borrow().define(name, JSValue::Undefined);
            }

            // Bind this
            new_env
                .borrow()
                .define("this".to_string(), this_arg.clone());

            // Swap env and stack, execute
            let old_env = vm.env.clone();
            let old_stack = std::mem::take(&mut vm.stack);
            vm.env = new_env;

            let res = vm.execute(func_chunk)?;

            // restore
            let _inner_stack = std::mem::replace(&mut vm.stack, old_stack);
            vm.env = old_env;

            Ok(res)
        }
        _ => Err(crate::error::JSError::TypeError(
            "Function.prototype.call: receiver is not a function".to_string(),
        )),
    }
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

    match func {
        JSValue::NativeFunction(native_fn) => {
            let mut call_args = Vec::new();
            call_args.push(this_arg);
            for v in call_args_vec.into_iter() {
                call_args.push(v);
            }
            let res = native_fn(vm, call_args)?;
            Ok(res)
        }
        JSValue::Function(func_chunk, params, captured_env_opt, name_opt) => {
            // Prepare outer environment (captured or current)
            let outer = match captured_env_opt {
                Some(env_rc) => env_rc,
                None => vm.env.clone(),
            };
            let new_env = Rc::new(RefCell::new(crate::runtime::Environment::with_outer(outer)));

            // Bind parameters from call_args_vec
            for (i, param_name) in params.iter().enumerate() {
                if i < call_args_vec.len() {
                    new_env
                        .borrow()
                        .define(param_name.clone(), call_args_vec[i].clone());
                } else {
                    new_env
                        .borrow()
                        .define(param_name.clone(), JSValue::Undefined);
                }
            }

            // Extra args
            for (i, value) in call_args_vec.iter().enumerate().skip(params.len()) {
                new_env.borrow().define(format!("arg{}", i), value.clone());
            }

            // Named function expression handling
            if let Some(name) = name_opt.clone() {
                new_env.borrow().define(name, JSValue::Undefined);
            }

            // Bind this
            new_env
                .borrow()
                .define("this".to_string(), this_arg.clone());

            // Swap env and stack, execute
            let old_env = vm.env.clone();
            let old_stack = std::mem::take(&mut vm.stack);
            vm.env = new_env;

            let res = vm.execute(func_chunk)?;

            // restore
            let _inner_stack = std::mem::replace(&mut vm.stack, old_stack);
            vm.env = old_env;

            Ok(res)
        }
        _ => Err(crate::error::JSError::TypeError(
            "Function.prototype.apply: receiver is not a function".to_string(),
        )),
    }
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
            let bf = BoundFunctionData {
                target: boxed.target.clone(),
                bound_this: boxed.bound_this.clone(),
                bound_args: new_args,
            };
            Ok(JSValue::BoundFunction(Box::new(bf)))
        }
        JSValue::Function(_, _, _, _) | JSValue::NativeFunction(_) => {
            // create bound function wrapper
            let bf = BoundFunctionData {
                target: Box::new(func.clone()),
                bound_this: this_arg,
                bound_args,
            };
            Ok(JSValue::BoundFunction(Box::new(bf)))
        }
        _ => Err(crate::error::JSError::TypeError(
            "Function.prototype.bind: receiver is not a function".to_string(),
        )),
    }
}

pub fn install(global: &Rc<RefCell<JSObject>>) {
    // Create Function constructor-like object (minimal)
    let mut fn_ctor = JSObject::new();
    // Function.prototype object
    let mut proto = JSObject::new();
    proto.set("call".to_string(), JSValue::NativeFunction(function_call));
    proto.set("apply".to_string(), JSValue::NativeFunction(function_apply));
    proto.set("bind".to_string(), JSValue::NativeFunction(function_bind));
    fn_ctor.set(
        "prototype".to_string(),
        JSValue::Object(Rc::new(RefCell::new(proto))),
    );

    global.borrow_mut().set(
        "Function".to_string(),
        JSValue::Object(Rc::new(RefCell::new(fn_ctor))),
    );
}
