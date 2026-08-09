//! 組み込み Array objectの 実装

use crate::value::JSValue;
use crate::value::jsobject::JSObject;
use std::cell::RefCell;
use std::rc::Rc;

// NativeFunction シグネチャ: fn(&mut VM, Vec<JSValue>) -> JSResult<JSValue>

fn receiver(args: &[JSValue], method: &str) -> crate::error::JSResult<Rc<RefCell<JSObject>>> {
    match args.first() {
        Some(JSValue::Object(object)) => Ok(Rc::clone(object)),
        _ => Err(crate::error::JSError::TypeError(format!(
            "Array.prototype.{method}: invalid receiver"
        ))),
    }
}

fn length(object: &Rc<RefCell<JSObject>>) -> usize {
    object.borrow().get("length").to_number().max(0.0) as usize
}

fn set_length(object: &Rc<RefCell<JSObject>>, length: usize) {
    object
        .borrow_mut()
        .set("length".to_string(), JSValue::Number(length as f64));
}

fn normalized_index(value: Option<&JSValue>, length: usize, default: isize) -> usize {
    let index = value.map(JSValue::to_number).unwrap_or(default as f64) as isize;
    if index < 0 {
        (length as isize + index).max(0) as usize
    } else {
        (index as usize).min(length)
    }
}

fn array_push(_vm: &mut crate::vm::VM, mut args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    // args: [this, value1, value2, ...] or if called via CallFunction maybe only values
    if args.is_empty() {
        return Err(crate::error::JSError::TypeError(
            "Array.prototype.push: missing receiver".to_string(),
        ));
    }

    let receiver = args.remove(0);

    match receiver {
        JSValue::Object(obj_ref) => {
            // determine length
            let len_val = obj_ref.borrow().get("length");
            let mut len = len_val.to_number();
            if len.is_nan() {
                len = 0.0;
            }
            let mut idx = len as usize;
            // push all remaining args
            for v in args.into_iter() {
                obj_ref.borrow_mut().set(idx.to_string(), v);
                idx += 1;
            }
            // update length
            obj_ref
                .borrow_mut()
                .set("length".to_string(), JSValue::Number(idx as f64));
            Ok(JSValue::Number(idx as f64))
        }
        _ => Err(crate::error::JSError::TypeError(
            "Array.prototype.push: receiver is not an object".to_string(),
        )),
    }
}

fn array_pop(_vm: &mut crate::vm::VM, mut args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    if args.is_empty() {
        return Err(crate::error::JSError::TypeError(
            "Array.prototype.pop: missing receiver".to_string(),
        ));
    }

    let receiver = args.remove(0);

    match receiver {
        JSValue::Object(obj_ref) => {
            let len_val = obj_ref.borrow().get("length");
            let mut len = len_val.to_number();
            if len.is_nan() {
                len = 0.0;
            }
            if len == 0.0 {
                // nothing to pop
                obj_ref
                    .borrow_mut()
                    .set("length".to_string(), JSValue::Number(0.0));
                return Ok(JSValue::Undefined);
            }
            let idx = (len as usize).saturating_sub(1);
            let element = obj_ref.borrow().get(&idx.to_string());
            // delete property
            obj_ref.borrow_mut().delete(&idx.to_string());
            // update length
            obj_ref
                .borrow_mut()
                .set("length".to_string(), JSValue::Number(idx as f64));
            Ok(element)
        }
        _ => Err(crate::error::JSError::TypeError(
            "Array.prototype.pop: receiver is not an object".to_string(),
        )),
    }
}

fn array_shift(_vm: &mut crate::vm::VM, args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    let object = receiver(&args, "shift")?;
    let length = length(&object);
    if length == 0 {
        set_length(&object, 0);
        return Ok(JSValue::Undefined);
    }
    let first = object.borrow().get("0");
    for index in 1..length {
        let value = object.borrow().get(&index.to_string());
        object.borrow_mut().set((index - 1).to_string(), value);
    }
    object.borrow_mut().delete(&(length - 1).to_string());
    set_length(&object, length - 1);
    Ok(first)
}

fn array_unshift(_vm: &mut crate::vm::VM, args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    let object = receiver(&args, "unshift")?;
    let length = length(&object);
    let additions = args.len().saturating_sub(1);
    for index in (0..length).rev() {
        let value = object.borrow().get(&index.to_string());
        object
            .borrow_mut()
            .set((index + additions).to_string(), value);
    }
    for (index, value) in args.into_iter().skip(1).enumerate() {
        object.borrow_mut().set(index.to_string(), value);
    }
    let new_length = length + additions;
    set_length(&object, new_length);
    Ok(JSValue::Number(new_length as f64))
}

fn array_slice(vm: &mut crate::vm::VM, args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    let object = receiver(&args, "slice")?;
    let length = length(&object);
    let start = normalized_index(args.get(1), length, 0);
    let end = normalized_index(args.get(2), length, length as isize).max(start);
    let values = (start..end)
        .map(|index| object.borrow().get(&index.to_string()))
        .collect();
    Ok(vm.array_from_values(values))
}

fn array_splice(vm: &mut crate::vm::VM, args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    let object = receiver(&args, "splice")?;
    let old_length = length(&object);
    let start = normalized_index(args.get(1), old_length, 0);
    let delete_count = args
        .get(2)
        .map(JSValue::to_number)
        .unwrap_or((old_length - start) as f64)
        .max(0.0) as usize;
    let delete_count = delete_count.min(old_length - start);
    let mut values: Vec<_> = (0..old_length)
        .map(|index| object.borrow().get(&index.to_string()))
        .collect();
    let removed: Vec<_> = values
        .splice(start..start + delete_count, args.into_iter().skip(3))
        .collect();
    for (index, value) in values.iter().cloned().enumerate() {
        object.borrow_mut().set(index.to_string(), value);
    }
    for index in values.len()..old_length {
        object.borrow_mut().delete(&index.to_string());
    }
    set_length(&object, values.len());
    Ok(vm.array_from_values(removed))
}

fn array_concat(vm: &mut crate::vm::VM, args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    let object = receiver(&args, "concat")?;
    let mut values: Vec<_> = (0..length(&object))
        .map(|index| object.borrow().get(&index.to_string()))
        .collect();
    for value in args.into_iter().skip(1) {
        match value {
            JSValue::Object(array) if array.borrow().has_own_property("__pixi_array__") => {
                for index in 0..length(&array) {
                    values.push(array.borrow().get(&index.to_string()));
                }
            }
            value => values.push(value),
        }
    }
    Ok(vm.array_from_values(values))
}

fn array_for_each(vm: &mut crate::vm::VM, args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    let object = receiver(&args, "forEach")?;
    let callback = args.get(1).cloned().unwrap_or(JSValue::Undefined);
    let this_arg = args.get(2).cloned().unwrap_or(JSValue::Undefined);
    let array = JSValue::Object(Rc::clone(&object));
    for index in 0..length(&object) {
        if !object.borrow().has_property(&index.to_string()) {
            continue;
        }
        let value = object.borrow().get(&index.to_string());
        vm.call(
            callback.clone(),
            this_arg.clone(),
            vec![value, JSValue::Number(index as f64), array.clone()],
        )?;
    }
    Ok(JSValue::Undefined)
}

fn array_map(vm: &mut crate::vm::VM, args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    let object = receiver(&args, "map")?;
    let callback = args.get(1).cloned().unwrap_or(JSValue::Undefined);
    let this_arg = args.get(2).cloned().unwrap_or(JSValue::Undefined);
    let array = JSValue::Object(Rc::clone(&object));
    let length = length(&object);
    let mut values = Vec::with_capacity(length);
    for index in 0..length {
        if object.borrow().has_property(&index.to_string()) {
            let value = object.borrow().get(&index.to_string());
            values.push(vm.call(
                callback.clone(),
                this_arg.clone(),
                vec![value, JSValue::Number(index as f64), array.clone()],
            )?);
        } else {
            values.push(JSValue::Undefined);
        }
    }
    Ok(vm.array_from_values(values))
}

fn array_filter(vm: &mut crate::vm::VM, args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    let object = receiver(&args, "filter")?;
    let callback = args.get(1).cloned().unwrap_or(JSValue::Undefined);
    let this_arg = args.get(2).cloned().unwrap_or(JSValue::Undefined);
    let array = JSValue::Object(Rc::clone(&object));
    let mut values = Vec::new();
    for index in 0..length(&object) {
        if !object.borrow().has_property(&index.to_string()) {
            continue;
        }
        let value = object.borrow().get(&index.to_string());
        if vm
            .call(
                callback.clone(),
                this_arg.clone(),
                vec![value.clone(), JSValue::Number(index as f64), array.clone()],
            )?
            .to_boolean()
        {
            values.push(value);
        }
    }
    Ok(vm.array_from_values(values))
}

fn array_some(vm: &mut crate::vm::VM, args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    let object = receiver(&args, "some")?;
    let callback = args.get(1).cloned().unwrap_or(JSValue::Undefined);
    let this_arg = args.get(2).cloned().unwrap_or(JSValue::Undefined);
    let array = JSValue::Object(Rc::clone(&object));
    for index in 0..length(&object) {
        if !object.borrow().has_property(&index.to_string()) {
            continue;
        }
        let value = object.borrow().get(&index.to_string());
        if vm
            .call(
                callback.clone(),
                this_arg.clone(),
                vec![value, JSValue::Number(index as f64), array.clone()],
            )?
            .to_boolean()
        {
            return Ok(JSValue::Boolean(true));
        }
    }
    Ok(JSValue::Boolean(false))
}

fn array_every(vm: &mut crate::vm::VM, args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    let object = receiver(&args, "every")?;
    let callback = args.get(1).cloned().unwrap_or(JSValue::Undefined);
    let this_arg = args.get(2).cloned().unwrap_or(JSValue::Undefined);
    let array = JSValue::Object(Rc::clone(&object));
    for index in 0..length(&object) {
        if !object.borrow().has_property(&index.to_string()) {
            continue;
        }
        let value = object.borrow().get(&index.to_string());
        if !vm
            .call(
                callback.clone(),
                this_arg.clone(),
                vec![value, JSValue::Number(index as f64), array.clone()],
            )?
            .to_boolean()
        {
            return Ok(JSValue::Boolean(false));
        }
    }
    Ok(JSValue::Boolean(true))
}

fn array_reduce(vm: &mut crate::vm::VM, args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    let object = receiver(&args, "reduce")?;
    let callback = args.get(1).cloned().unwrap_or(JSValue::Undefined);
    let array = JSValue::Object(Rc::clone(&object));
    let length = length(&object);
    let mut index = 0;
    let mut accumulator = if let Some(initial) = args.get(2) {
        initial.clone()
    } else {
        loop {
            if index >= length {
                return Err(crate::error::JSError::TypeError(
                    "Reduce of empty array with no initial value".to_string(),
                ));
            }
            if object.borrow().has_property(&index.to_string()) {
                let value = object.borrow().get(&index.to_string());
                index += 1;
                break value;
            }
            index += 1;
        }
    };
    while index < length {
        if object.borrow().has_property(&index.to_string()) {
            let value = object.borrow().get(&index.to_string());
            accumulator = vm.call(
                callback.clone(),
                JSValue::Undefined,
                vec![
                    accumulator,
                    value,
                    JSValue::Number(index as f64),
                    array.clone(),
                ],
            )?;
        }
        index += 1;
    }
    Ok(accumulator)
}

fn array_index_of(_vm: &mut crate::vm::VM, args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    let object = receiver(&args, "indexOf")?;
    let needle = args.get(1).cloned().unwrap_or(JSValue::Undefined);
    let start = normalized_index(args.get(2), length(&object), 0);
    for index in start..length(&object) {
        if object
            .borrow()
            .get(&index.to_string())
            .strict_equals(&needle)
        {
            return Ok(JSValue::Number(index as f64));
        }
    }
    Ok(JSValue::Number(-1.0))
}

fn array_includes(_vm: &mut crate::vm::VM, args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    let object = receiver(&args, "includes")?;
    let needle = args.get(1).cloned().unwrap_or(JSValue::Undefined);
    let start = normalized_index(args.get(2), length(&object), 0);
    for index in start..length(&object) {
        let value = object.borrow().get(&index.to_string());
        let both_nan = matches!((&value, &needle), (JSValue::Number(a), JSValue::Number(b)) if a.is_nan() && b.is_nan());
        if both_nan || value.strict_equals(&needle) {
            return Ok(JSValue::Boolean(true));
        }
    }
    Ok(JSValue::Boolean(false))
}

fn array_join(_vm: &mut crate::vm::VM, args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    let object = receiver(&args, "join")?;
    let separator = args
        .get(1)
        .filter(|value| !matches!(value, JSValue::Undefined))
        .map(JSValue::to_string)
        .unwrap_or_else(|| ",".to_string());
    let values = (0..length(&object))
        .map(|index| match object.borrow().get(&index.to_string()) {
            JSValue::Null | JSValue::Undefined => String::new(),
            value => value.to_string(),
        })
        .collect::<Vec<_>>()
        .join(&separator);
    Ok(JSValue::String(values))
}

fn array_is_array(_vm: &mut crate::vm::VM, args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    let value = if args.len() > 1 {
        args.get(1)
    } else {
        args.first()
    };
    let is_array = matches!(
        value,
        Some(JSValue::Object(object))
            if object.borrow().has_own_property("__pixi_array__")
    );
    Ok(JSValue::Boolean(is_array))
}

fn array_constructor(
    vm: &mut crate::vm::VM,
    args: Vec<JSValue>,
) -> crate::error::JSResult<JSValue> {
    let values: Vec<_> = args.into_iter().skip(1).collect();
    if let [JSValue::Number(length)] = values.as_slice() {
        let length = if length.is_finite() && *length >= 0.0 && length.fract() == 0.0 {
            *length as usize
        } else {
            return Err(crate::error::JSError::RangeError(
                "Invalid array length".to_string(),
            ));
        };
        return Ok(vm.array_from_values(vec![JSValue::Undefined; length]));
    }
    Ok(vm.array_from_values(values))
}

pub fn install(global: &Rc<RefCell<JSObject>>) {
    // Array コンストラクタオブジェクト（最小実装）
    let mut array_ctor = JSObject::new();

    // Array.prototype オブジェクト
    let mut proto = JSObject::new();

    // push と pop をネイティブ関数として登録
    proto.set("push".to_string(), JSValue::NativeFunction(array_push));
    proto.set("pop".to_string(), JSValue::NativeFunction(array_pop));
    proto.set("shift".to_string(), JSValue::NativeFunction(array_shift));
    proto.set(
        "unshift".to_string(),
        JSValue::NativeFunction(array_unshift),
    );
    proto.set("slice".to_string(), JSValue::NativeFunction(array_slice));
    proto.set("splice".to_string(), JSValue::NativeFunction(array_splice));
    proto.set("concat".to_string(), JSValue::NativeFunction(array_concat));
    proto.set(
        "forEach".to_string(),
        JSValue::NativeFunction(array_for_each),
    );
    proto.set("map".to_string(), JSValue::NativeFunction(array_map));
    proto.set("filter".to_string(), JSValue::NativeFunction(array_filter));
    proto.set("some".to_string(), JSValue::NativeFunction(array_some));
    proto.set("every".to_string(), JSValue::NativeFunction(array_every));
    proto.set("reduce".to_string(), JSValue::NativeFunction(array_reduce));
    proto.set(
        "indexOf".to_string(),
        JSValue::NativeFunction(array_index_of),
    );
    proto.set(
        "includes".to_string(),
        JSValue::NativeFunction(array_includes),
    );
    proto.set("join".to_string(), JSValue::NativeFunction(array_join));

    array_ctor.set(
        "prototype".to_string(),
        JSValue::Object(Rc::new(RefCell::new(proto))),
    );
    array_ctor.set(
        "isArray".to_string(),
        JSValue::NativeFunction(array_is_array),
    );
    array_ctor.set(
        "__call__".to_string(),
        JSValue::NativeFunction(array_constructor),
    );
    array_ctor.set(
        "__construct__".to_string(),
        JSValue::NativeFunction(array_constructor),
    );

    global.borrow_mut().set(
        "Array".to_string(),
        JSValue::Object(Rc::new(RefCell::new(array_ctor))),
    );
}
