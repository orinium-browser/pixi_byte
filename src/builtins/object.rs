//! Minimal Object builtin implementation
//!
//! Provides `Object` constructor-like object with methods such as `create`, `getPrototypeOf`,
//! `setPrototypeOf`, and helpers like `defineProperty` and `getOwnPropertyDescriptor`.

use crate::error::{JSError, JSResult};
use crate::value::JSValue;
use crate::value::jsobject::JSObject;
use crate::value::jsvalue::JsValueKind;
use std::cell::RefCell;
use std::rc::Rc;

fn object_constructor(vm: &mut crate::vm::VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    if let Some(value) = args.get(1)
        && matches!(
            value.clone().kind(),
            JsValueKind::Object | JsValueKind::Function | JsValueKind::ArrowFunction
        )
    {
        return Ok(value.clone());
    }
    if let Some(obj) = args.first().and_then(|v| v.as_object()) {
        return Ok(JSValue::from_object(obj));
    }
    let prototype = vm
        .global_object
        .borrow()
        .get("Object")
        .as_object()
        .and_then(|constructor| constructor.borrow().get("prototype").as_object());
    Ok(JSValue::from_object(Rc::new(RefCell::new(
        JSObject::with_prototype(prototype),
    ))))
}

fn object_static_arguments(vm: &crate::vm::VM, mut args: Vec<JSValue>) -> Vec<JSValue> {
    let constructor = vm.global_object.borrow().get("Object");
    let has_receiver = match args.first() {
        Some(receiver) if receiver.clone().kind() == JsValueKind::Object => {
            let rc = receiver.as_object().unwrap();
            if let Some(ctor_rc) = constructor.as_object() {
                Rc::ptr_eq(&rc, &vm.global_object) || Rc::ptr_eq(&rc, &ctor_rc)
            } else {
                Rc::ptr_eq(&rc, &vm.global_object)
            }
        }
        Some(receiver) => {
            let kind = receiver.clone().kind();
            kind == JsValueKind::Undefined || kind == JsValueKind::Null
        }
        _ => false,
    };
    if has_receiver {
        args.remove(0);
    }
    args
}

/// Object.create(proto[, properties])
/// - `proto` must be object or null
/// - If second arg present, it is treated as property descriptor object
fn object_create(vm: &mut crate::vm::VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let mut args = object_static_arguments(vm, args);
    if args.is_empty() {
        return Err(JSError::TypeError(
            "Object.create: missing prototype argument".to_string(),
        ));
    }

    let proto = args.remove(0);

    let new_obj_rc = match proto.kind() {
        JsValueKind::Object => {
            let obj_ref = proto.as_object().unwrap();
            let new_obj = JSObject::with_prototype(Some(obj_ref.clone()));
            Rc::new(RefCell::new(new_obj))
        }
        JsValueKind::Null => Rc::new(RefCell::new(JSObject::with_prototype(None))),
        _ => {
            return Err(JSError::TypeError(
                "Object.create: prototype must be an object or null".to_string(),
            ));
        }
    };

    // If a second argument is provided, treat it as property descriptors
    if !args.is_empty() {
        let props = args.remove(0);
        if let Some(desc_ref) = props.as_object() {
            // iterate enumerable keys of descriptor object
            for key in desc_ref.borrow().keys() {
                let desc_val = desc_ref.borrow().get(&key);
                // descriptor should be an object
                if let Some(dobj_ref) = desc_val.as_object() {
                    // Use define_property_descriptor to respect defaults/partial descriptors
                    new_obj_rc
                        .borrow_mut()
                        .define_property_descriptor(key, dobj_ref.clone());
                } else {
                    // If descriptor is not object, treat it as value shorthand
                    let prop = crate::value::jsobject::Property::data(desc_val);
                    new_obj_rc.borrow_mut().define_property(key, prop);
                }
            }
        } else {
            return Err(JSError::TypeError(
                "Object.create: properties argument must be an object".to_string(),
            ));
        }
    }

    Ok(JSValue::from_object(new_obj_rc))
}

/// Object.getPrototypeOf(obj)
/// - returns the prototype object or null
fn object_get_prototype_of(vm: &mut crate::vm::VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let mut args = object_static_arguments(vm, args);
    // args: [obj]
    if args.is_empty() {
        return Err(JSError::TypeError(
            "Object.getPrototypeOf: missing object argument".to_string(),
        ));
    }

    let obj = args.remove(0);

    match obj.clone().kind() {
        JsValueKind::Object => {
            let obj_ref = obj.as_object().unwrap();
            if let Some(proto) = obj_ref.borrow().get_prototype() {
                Ok(JSValue::from_object(proto.clone()))
            } else if !obj_ref.borrow().has_explicit_prototype()
                && !Rc::ptr_eq(&obj_ref, &vm.object_prototype)
            {
                Ok(JSValue::from_object(Rc::clone(&vm.object_prototype)))
            } else {
                Ok(JSValue::null())
            }
        }
        JsValueKind::Function | JsValueKind::ArrowFunction => {
            let prototype = vm
                .user_function_object(&obj)
                .and_then(|object| object.borrow().get_prototype())
                .unwrap_or_else(|| vm.function_prototype.clone());
            Ok(JSValue::from_object(prototype))
        }
        JsValueKind::NativeFunction | JsValueKind::BoundFunction => {
            Ok(JSValue::from_object(vm.function_prototype.clone()))
        }
        JsValueKind::String => Ok(JSValue::from_object(vm.string_prototype.clone())),
        JsValueKind::Number => Ok(JSValue::from_object(vm.number_prototype.clone())),
        JsValueKind::Boolean | JsValueKind::BigInt => {
            Ok(JSValue::from_object(vm.object_prototype.clone()))
        }
        JsValueKind::Null | JsValueKind::Undefined => Err(JSError::TypeError(format!(
            "Object.getPrototypeOf: cannot convert null or undefined to object (JS stack: {})",
            vm.formatted_js_stack()
        ))),
    }
}

/// getter for Object.prototype.__proto__
fn object_proto_get(_vm: &mut crate::vm::VM, mut args: Vec<JSValue>) -> JSResult<JSValue> {
    if args.is_empty() {
        return Ok(JSValue::undefined());
    }
    let receiver = args.remove(0);
    if let Some(obj_ref) = receiver.as_object() {
        if let Some(proto) = obj_ref.borrow().get_prototype() {
            Ok(JSValue::from_object(proto))
        } else {
            Ok(JSValue::null())
        }
    } else {
        Ok(JSValue::undefined())
    }
}

/// setter for Object.prototype.__proto__
fn object_proto_set(_vm: &mut crate::vm::VM, mut args: Vec<JSValue>) -> JSResult<JSValue> {
    if args.is_empty() {
        return Ok(JSValue::undefined());
    }
    let receiver = args.remove(0);
    let new_proto = if !args.is_empty() {
        args.remove(0)
    } else {
        JSValue::undefined()
    };

    if let Some(obj_ref) = receiver.as_object() {
        if let Some(o) = new_proto.as_object() {
            // Prevent prototype cycles: ensure `obj_ref` is not in the prototype chain of `o`
            let mut current = Some(o.clone());
            while let Some(p) = current {
                if Rc::ptr_eq(&p, &obj_ref) {
                    return Err(JSError::TypeError(
                        "__proto__ setter: setting prototype would create a cycle".to_string(),
                    ));
                }
                current = p.borrow().get_prototype();
            }

            obj_ref.borrow_mut().set_prototype(Some(o));
            Ok(JSValue::undefined())
        } else if new_proto.is_null() {
            obj_ref.borrow_mut().set_prototype(None);
            Ok(JSValue::undefined())
        } else {
            Err(JSError::TypeError(
                "__proto__ setter: value must be an object or null".to_string(),
            ))
        }
    } else {
        Err(JSError::TypeError(
            "__proto__ setter: receiver is not an object".to_string(),
        ))
    }
}

/// Object.setPrototypeOf(obj, proto)
fn object_set_prototype_of(vm: &mut crate::vm::VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let mut args = object_static_arguments(vm, args);
    if args.len() < 2 {
        return Err(JSError::TypeError(
            "Object.setPrototypeOf: missing arguments".to_string(),
        ));
    }

    let obj = args.remove(0);
    let proto = args.remove(0);

    let obj_ref = match obj.kind() {
        JsValueKind::Object => Some(obj.as_object().unwrap()),
        JsValueKind::Function | JsValueKind::ArrowFunction => vm.user_function_object(&obj),
        _ => None,
    };
    let Some(obj_ref) = obj_ref else {
        return Err(JSError::TypeError(format!(
            "Object.setPrototypeOf: first argument must be an object (found {}: {})",
            obj.type_of(),
            obj.to_console_string()
        )));
    };

    match proto.kind() {
        JsValueKind::Object => {
            let o = proto.as_object().unwrap();
            // Prevent prototype cycles: ensure obj_ref is not reachable from o
            let mut current = Some(o.clone());
            while let Some(p) = current {
                if Rc::ptr_eq(&p, &obj_ref) {
                    return Err(JSError::TypeError(
                        "Object.setPrototypeOf: setting prototype would create a cycle".to_string(),
                    ));
                }
                current = p.borrow().get_prototype();
            }

            obj_ref.borrow_mut().set_prototype(Some(o));
            Ok(obj)
        }
        JsValueKind::Function | JsValueKind::ArrowFunction => {
            let Some(prototype) = vm.user_function_object(&proto) else {
                return Err(JSError::TypeError(
                    "Object.setPrototypeOf: prototype must be an object or null".to_string(),
                ));
            };
            obj_ref.borrow_mut().set_prototype(Some(prototype));
            Ok(obj)
        }
        JsValueKind::Null => {
            obj_ref.borrow_mut().set_prototype(None);
            Ok(obj)
        }
        _ => Err(JSError::TypeError(
            "Object.setPrototypeOf: prototype must be an object or null".to_string(),
        )),
    }
}

/// Object.prototype.hasOwnProperty のネイティブ実装
fn object_has_own_property(vm: &mut crate::vm::VM, mut args: Vec<JSValue>) -> JSResult<JSValue> {
    if args.is_empty() {
        return Err(JSError::TypeError(
            "hasOwnProperty: missing receiver".to_string(),
        ));
    }
    let receiver = args.remove(0);
    if args.is_empty() {
        return Err(JSError::TypeError(
            "hasOwnProperty: missing property name".to_string(),
        ));
    }
    let prop = args.remove(0);
    let key = prop.to_string();

    match receiver.kind() {
        JsValueKind::Object => {
            let obj_ref = receiver.as_object().unwrap();
            Ok(JSValue::from_bool(obj_ref.borrow().has_own_property(&key)))
        }
        JsValueKind::Function | JsValueKind::ArrowFunction => Ok(JSValue::from_bool(
            vm.user_function_object(&receiver)
                .is_some_and(|object| object.borrow().has_own_property(&key)),
        )),
        JsValueKind::String => {
            let value = receiver.as_string().unwrap();
            let index = key.parse::<usize>().ok();
            Ok(JSValue::from_bool(
                key == "length" || index.is_some_and(|index| index < value.encode_utf16().count()),
            ))
        }
        JsValueKind::Number | JsValueKind::Boolean => Ok(JSValue::from_bool(false)),
        _ => Err(JSError::TypeError(format!(
            "hasOwnProperty: receiver is not an object ({})",
            receiver.type_of()
        ))),
    }
}

/// Object.prototype.isPrototypeOf のネイティブ実装
fn object_is_prototype_of(_vm: &mut crate::vm::VM, mut args: Vec<JSValue>) -> JSResult<JSValue> {
    if args.len() < 2 {
        return Err(JSError::TypeError(
            "isPrototypeOf: missing arguments".to_string(),
        ));
    }
    let receiver = args.remove(0);
    let obj = args.remove(0);

    match (receiver.as_object(), obj.as_object()) {
        (Some(proto_ref), Some(target_ref)) => {
            let mut current = target_ref.borrow().get_prototype();
            while let Some(p) = current {
                if Rc::ptr_eq(&p, &proto_ref) {
                    return Ok(JSValue::from_bool(true));
                }
                current = p.borrow().get_prototype();
            }
            Ok(JSValue::from_bool(false))
        }
        _ => Err(JSError::TypeError(
            "isPrototypeOf: receiver and argument must be objects".to_string(),
        )),
    }
}

/// Object.prototype.toString のネイティブ実装
pub(crate) fn object_to_string(
    _vm: &mut crate::vm::VM,
    mut args: Vec<JSValue>,
) -> JSResult<JSValue> {
    if args.is_empty() {
        return Ok(JSValue::from_str("[object Object]"));
    }
    let receiver = args.remove(0);
    match receiver.kind() {
        JsValueKind::Object => {
            let object = receiver.as_object().unwrap();
            if object.borrow().has_own_property("__pixi_array__") {
                Ok(JSValue::from_str("[object Array]"))
            } else {
                Ok(JSValue::from_str("[object Object]"))
            }
        }
        JsValueKind::Function
        | JsValueKind::ArrowFunction
        | JsValueKind::NativeFunction
        | JsValueKind::BoundFunction => Ok(JSValue::from_str("[object Function]")),
        JsValueKind::String => Ok(JSValue::from_str("[object String]")),
        JsValueKind::Number => Ok(JSValue::from_str("[object Number]")),
        JsValueKind::BigInt => Ok(JSValue::from_str("[object BigInt]")),
        JsValueKind::Boolean => Ok(JSValue::from_str("[object Boolean]")),
        JsValueKind::Null => Ok(JSValue::from_str("[object Null]")),
        JsValueKind::Undefined => Ok(JSValue::from_str("[object Undefined]")),
    }
}

fn object_value_of(_vm: &mut crate::vm::VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let value = args.first().cloned().unwrap_or(JSValue::undefined());
    if value.is_null() || value.is_undefined() {
        Err(JSError::TypeError(
            "Object.prototype.valueOf: invalid receiver".to_string(),
        ))
    } else {
        Ok(value)
    }
}

/// Object.defineProperty(obj, prop, descriptor)
fn object_define_property(vm: &mut crate::vm::VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let mut args = object_static_arguments(vm, args);
    if args.len() < 3 {
        return Err(JSError::TypeError(
            "Object.defineProperty: missing arguments".to_string(),
        ));
    }
    let obj = args.remove(0);
    let prop_name = args.remove(0);
    let desc = args.remove(0);

    let key = prop_name.to_string();

    let object = match obj.kind() {
        JsValueKind::Object => Some(obj.as_object().unwrap()),
        JsValueKind::Function | JsValueKind::ArrowFunction => vm.user_function_object(&obj),
        _ => None,
    };

    match object {
        Some(obj_ref) => {
            if let Some(desc_ref) = desc.as_object() {
                // use define_property_descriptor to apply descriptor semantics
                obj_ref
                    .borrow_mut()
                    .define_property_descriptor(key, desc_ref.clone());
                Ok(obj.clone())
            } else {
                Err(JSError::TypeError(
                    "Object.defineProperty: descriptor must be an object".to_string(),
                ))
            }
        }
        None => Err(JSError::TypeError(
            "Object.defineProperty: first argument must be an object".to_string(),
        )),
    }
}

/// Object.preventExtensions(obj)
fn object_prevent_extensions(vm: &mut crate::vm::VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let mut args = object_static_arguments(vm, args);
    if args.is_empty() {
        return Err(JSError::TypeError(
            "Object.preventExtensions: missing object".to_string(),
        ));
    }
    let obj = args.remove(0);
    if let Some(obj_ref) = obj.as_object() {
        obj_ref.borrow_mut().prevent_extensions();
        Ok(JSValue::from_object(obj_ref.clone()))
    } else {
        Err(JSError::TypeError(
            "Object.preventExtensions: argument must be an object".to_string(),
        ))
    }
}

/// Object.isExtensible(obj)
fn object_is_extensible(vm: &mut crate::vm::VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let mut args = object_static_arguments(vm, args);
    if args.is_empty() {
        return Err(JSError::TypeError(
            "Object.isExtensible: missing object".to_string(),
        ));
    }
    let obj = args.remove(0);
    if let Some(obj_ref) = obj.as_object() {
        Ok(JSValue::from_bool(obj_ref.borrow().is_extensible()))
    } else {
        Err(JSError::TypeError(
            "Object.isExtensible: argument must be an object".to_string(),
        ))
    }
}

/// Object.seal(obj): make all properties non-configurable and prevent extensions
fn object_seal(vm: &mut crate::vm::VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let mut args = object_static_arguments(vm, args);
    if args.is_empty() {
        return Err(JSError::TypeError(
            "Object.seal: missing object".to_string(),
        ));
    }
    let obj = args.remove(0);
    if let Some(obj_ref) = obj.as_object() {
        // make all existing properties non-configurable
        let mut props = obj_ref.borrow_mut();
        let keys = props.keys();
        for k in keys {
            if let Some(mut p) = props.get_property_descriptor(&k) {
                p.configurable = false;
                props.define_property(k, p);
            }
        }
        props.prevent_extensions();
        Ok(JSValue::from_object(obj_ref.clone()))
    } else {
        Err(JSError::TypeError(
            "Object.seal: argument must be an object".to_string(),
        ))
    }
}

/// Object.freeze(obj): make all properties non-configurable and non-writable and prevent extensions
fn object_freeze(vm: &mut crate::vm::VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let mut args = object_static_arguments(vm, args);
    if args.is_empty() {
        return Err(JSError::TypeError(
            "Object.freeze: missing object".to_string(),
        ));
    }
    let obj = args.remove(0);
    if let Some(obj_ref) = obj.as_object() {
        // make all existing properties non-configurable and non-writable
        let mut props = obj_ref.borrow_mut();
        let keys = props.keys();
        for k in keys {
            if let Some(mut p) = props.get_property_descriptor(&k) {
                p.configurable = false;
                p.writable = false;
                props.define_property(k, p);
            }
        }
        props.prevent_extensions();
        Ok(JSValue::from_object(obj_ref.clone()))
    } else {
        Err(JSError::TypeError(
            "Object.freeze: argument must be an object".to_string(),
        ))
    }
}

/// Object.getOwnPropertyDescriptor(obj, prop)
fn object_get_own_property_descriptor(
    vm: &mut crate::vm::VM,
    args: Vec<JSValue>,
) -> JSResult<JSValue> {
    let mut args = object_static_arguments(vm, args);
    if args.len() < 2 {
        return Err(JSError::TypeError(
            "Object.getOwnPropertyDescriptor: missing arguments".to_string(),
        ));
    }
    let obj = args.remove(0);
    let prop_name = args.remove(0);
    let key = prop_name.to_string();

    let object = match obj.kind() {
        JsValueKind::Object => Some(obj.as_object().unwrap()),
        JsValueKind::Function | JsValueKind::ArrowFunction => vm.user_function_object(&obj),
        _ => None,
    };

    match object {
        Some(obj_ref) => {
            if let Some(prop) = obj_ref.borrow().get_property_descriptor(&key) {
                // build descriptor object
                let mut desc = JSObject::new();
                // if accessor
                if prop.getter.is_some() || prop.setter.is_some() {
                    if let Some(getv) = prop.getter {
                        desc.set("get".to_string(), getv);
                    } else {
                        desc.set("get".to_string(), JSValue::undefined());
                    }
                    if let Some(setv) = prop.setter {
                        desc.set("set".to_string(), setv);
                    } else {
                        desc.set("set".to_string(), JSValue::undefined());
                    }
                } else {
                    desc.set("value".to_string(), prop.value.clone());
                    desc.set("writable".to_string(), JSValue::from_bool(prop.writable));
                }
                desc.set(
                    "enumerable".to_string(),
                    JSValue::from_bool(prop.enumerable),
                );
                desc.set(
                    "configurable".to_string(),
                    JSValue::from_bool(prop.configurable),
                );

                Ok(JSValue::from_object(Rc::new(RefCell::new(desc))))
            } else {
                Ok(JSValue::undefined())
            }
        }
        None => Err(JSError::TypeError(
            "Object.getOwnPropertyDescriptor: first argument must be an object".to_string(),
        )),
    }
}

/// Object.keys(obj)
fn object_keys(vm: &mut crate::vm::VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let args = object_static_arguments(vm, args);
    let Some(value) = args.into_iter().next() else {
        return Err(JSError::TypeError(
            "Object.keys: missing object argument".to_string(),
        ));
    };
    let keys = if let Some(object) = enumerable_object(vm, &value) {
        object
            .borrow()
            .keys()
            .into_iter()
            .map(JSValue::from_string)
            .collect()
    } else {
        match value.kind() {
            JsValueKind::Null | JsValueKind::Undefined => {
                return Err(JSError::TypeError(format!(
                    "Object.keys: cannot convert {} to object (JS stack: {})",
                    value.type_of(),
                    vm.formatted_js_stack()
                )));
            }
            JsValueKind::String => {
                let s = value.as_string().unwrap();
                (0..s.encode_utf16().count())
                    .map(|index| JSValue::from_str(&index.to_string()))
                    .collect()
            }
            JsValueKind::NativeFunction
            | JsValueKind::BoundFunction
            | JsValueKind::Number
            | JsValueKind::BigInt
            | JsValueKind::Boolean => Vec::new(),
            JsValueKind::Object | JsValueKind::Function | JsValueKind::ArrowFunction => {
                unreachable!("object-like values are handled above")
            }
        }
    };
    Ok(vm.array_from_values(keys))
}

fn object_get_own_property_names(vm: &mut crate::vm::VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let args = object_static_arguments(vm, args);
    let Some(value) = args.into_iter().next() else {
        return Err(JSError::TypeError(
            "Object.getOwnPropertyNames: missing object argument".to_string(),
        ));
    };
    let object = match value.kind() {
        JsValueKind::Object => Some(value.as_object().unwrap()),
        JsValueKind::Function | JsValueKind::ArrowFunction => vm.user_function_object(&value),
        _ => None,
    };
    let Some(object) = object else {
        return Err(JSError::TypeError(
            "Object.getOwnPropertyNames: argument must be an object".to_string(),
        ));
    };
    let names = object
        .borrow()
        .own_property_names()
        .into_iter()
        .filter(|name| {
            !matches!(
                name.as_str(),
                "__call__"
                    | "__construct__"
                    | "__host_get_property__"
                    | "__host_set_property__"
                    | "__host_has_instance__"
            )
        })
        .map(JSValue::from_string)
        .collect();
    Ok(vm.array_from_values(names))
}

fn object_get_own_property_symbols(
    vm: &mut crate::vm::VM,
    args: Vec<JSValue>,
) -> JSResult<JSValue> {
    let args = object_static_arguments(vm, args);
    if args.is_empty() {
        return Err(JSError::TypeError(
            "Object.getOwnPropertySymbols: missing object argument".to_string(),
        ));
    }
    Ok(vm.array_from_values(Vec::new()))
}

fn enumerable_object(vm: &crate::vm::VM, value: &JSValue) -> Option<Rc<RefCell<JSObject>>> {
    match value.kind() {
        JsValueKind::Object => Some(value.as_object().unwrap()),
        JsValueKind::Function | JsValueKind::ArrowFunction => vm.user_function_object(value),
        _ => None,
    }
}

fn object_entries(vm: &mut crate::vm::VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let args = object_static_arguments(vm, args);
    let Some(value) = args.first() else {
        return Err(JSError::TypeError(
            "Object.entries: missing object argument".to_string(),
        ));
    };
    let Some(object) = enumerable_object(vm, value) else {
        return Err(JSError::TypeError(
            "Object.entries: argument must be an object".to_string(),
        ));
    };
    let entries = object
        .borrow()
        .keys()
        .into_iter()
        .map(|key| {
            let value = object.borrow().get(&key);
            vm.array_from_values(vec![JSValue::from_str(&key), value])
        })
        .collect();
    Ok(vm.array_from_values(entries))
}

fn object_values(vm: &mut crate::vm::VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let args = object_static_arguments(vm, args);
    let Some(value) = args.first() else {
        return Err(JSError::TypeError(
            "Object.values: missing object argument".to_string(),
        ));
    };
    let Some(object) = enumerable_object(vm, value) else {
        return Err(JSError::TypeError(
            "Object.values: argument must be an object".to_string(),
        ));
    };
    let values = object
        .borrow()
        .keys()
        .into_iter()
        .map(|key| object.borrow().get(&key))
        .collect();
    Ok(vm.array_from_values(values))
}

fn object_from_entries(vm: &mut crate::vm::VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let args = object_static_arguments(vm, args);
    let source = args.first().cloned().unwrap_or(JSValue::undefined());
    let array_constructor = vm.global_object.borrow().get("Array");
    let Some(array_object) = array_constructor.as_object() else {
        return Err(JSError::InternalError(
            "Array constructor is missing".into(),
        ));
    };
    let array_from = array_object.borrow().get("from");
    let entries = vm.call(array_from, array_constructor, vec![source])?;
    let Some(entries) = entries.as_object() else {
        return Err(JSError::TypeError(
            "Object.fromEntries requires an iterable".to_string(),
        ));
    };
    let length = entries.borrow().get("length").to_number() as usize;
    let target = Rc::new(RefCell::new(JSObject::new()));
    for index in 0..length {
        let Some(entry) = entries.borrow().get_index(index).as_object() else {
            return Err(JSError::TypeError(
                "Object.fromEntries iterator value must be an object".to_string(),
            ));
        };
        let key = entry.borrow().get("0").to_string();
        let value = entry.borrow().get("1");
        target.borrow_mut().set(key, value);
    }
    Ok(JSValue::from_object(target))
}

/// Object.assign(target, ...sources)
fn object_assign(vm: &mut crate::vm::VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let mut args = object_static_arguments(vm, args).into_iter();
    let Some(target_value) = args.next() else {
        return Err(JSError::TypeError(
            "Object.assign: target must be an object".to_string(),
        ));
    };
    let target = match target_value.kind() {
        JsValueKind::Object => Some(target_value.as_object().unwrap()),
        JsValueKind::Function | JsValueKind::ArrowFunction => {
            vm.user_function_object(&target_value)
        }
        _ => None,
    };
    let Some(target) = target else {
        return Err(JSError::TypeError(
            "Object.assign: target must be an object".to_string(),
        ));
    };

    for source in args {
        let source = match source.kind() {
            JsValueKind::Object => Some(source.as_object().unwrap()),
            JsValueKind::Function | JsValueKind::ArrowFunction => vm.user_function_object(&source),
            JsValueKind::Null | JsValueKind::Undefined => None,
            _ => None,
        };
        let Some(source) = source else { continue };
        let keys = source.borrow().keys();
        for key in keys {
            let value = source.borrow().get(&key);
            target.borrow_mut().set(key, value);
        }
    }
    Ok(target_value)
}

/// Object.is(value1, value2)
fn object_is(_vm: &mut crate::vm::VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let left = args.get(1).cloned().unwrap_or(JSValue::undefined());
    let right = args.get(2).cloned().unwrap_or(JSValue::undefined());
    let same = match (left.as_number(), right.as_number()) {
        (Some(left), Some(right)) => {
            (left.is_nan() && right.is_nan())
                || (left == right
                    && (left != 0.0 || left.is_sign_positive() == right.is_sign_positive()))
        }
        _ => left.strict_equals(&right),
    };
    Ok(JSValue::from_bool(same))
}

/// Object.prototype.propertyIsEnumerable(prop)
fn object_property_is_enumerable(
    vm: &mut crate::vm::VM,
    mut args: Vec<JSValue>,
) -> JSResult<JSValue> {
    if args.is_empty() {
        return Err(JSError::TypeError(
            "propertyIsEnumerable: missing receiver".to_string(),
        ));
    }
    let receiver = args.remove(0);
    if args.is_empty() {
        return Err(JSError::TypeError(
            "propertyIsEnumerable: missing property name".to_string(),
        ));
    }
    let prop = args.remove(0);
    let key = prop.to_string();

    match receiver.kind() {
        JsValueKind::Object => {
            let obj_ref = receiver.as_object().unwrap();
            if let Some(descr) = obj_ref.borrow().get_property_descriptor(&key) {
                Ok(JSValue::from_bool(descr.enumerable))
            } else {
                Ok(JSValue::from_bool(false))
            }
        }
        JsValueKind::Function | JsValueKind::ArrowFunction => {
            let Some(obj_ref) = vm.user_function_object(&receiver) else {
                return Ok(JSValue::from_bool(false));
            };
            let enumerable = obj_ref
                .borrow()
                .get_property_descriptor(&key)
                .is_some_and(|descriptor| descriptor.enumerable);
            Ok(JSValue::from_bool(enumerable))
        }
        JsValueKind::String => {
            let value = receiver.as_string().unwrap();
            let enumerable = key
                .parse::<usize>()
                .ok()
                .is_some_and(|index| index < value.encode_utf16().count());
            Ok(JSValue::from_bool(enumerable))
        }
        JsValueKind::Number | JsValueKind::Boolean => Ok(JSValue::from_bool(false)),
        _ => Err(JSError::TypeError(
            "propertyIsEnumerable: receiver is not an object".to_string(),
        )),
    }
}

/// グローバルオブジェクトに Object 組み込みをインストールする
pub fn install(global: &Rc<RefCell<JSObject>>) {
    let mut obj = JSObject::new();
    obj.set(
        "__call__".to_string(),
        JSValue::from_native_function(object_constructor),
    );
    obj.set(
        "__construct__".to_string(),
        JSValue::from_native_function(object_constructor),
    );

    // Create Object.prototype and attach methods (e.g., hasOwnProperty)
    let mut proto = JSObject::new();
    proto.define_property(
        "hasOwnProperty".to_string(),
        builtin_method(object_has_own_property),
    );
    proto.define_property(
        "isPrototypeOf".to_string(),
        builtin_method(object_is_prototype_of),
    );
    proto.define_property("toString".to_string(), builtin_method(object_to_string));
    proto.define_property("valueOf".to_string(), builtin_method(object_value_of));
    proto.define_property(
        "propertyIsEnumerable".to_string(),
        builtin_method(object_property_is_enumerable),
    );

    // Define __proto__ accessor on Object.prototype
    let proto_accessor = crate::value::jsobject::Property {
        value: JSValue::undefined(),
        enumerable: false,
        writable: false,
        configurable: true,
        getter: Some(JSValue::from_native_function(object_proto_get)),
        setter: Some(JSValue::from_native_function(object_proto_set)),
    };
    proto.define_property("__proto__".to_string(), proto_accessor);

    // `create` と `getPrototypeOf` をネイティブ関数として登録
    obj.set(
        "create".to_string(),
        JSValue::from_native_function(object_create),
    );
    obj.set(
        "getPrototypeOf".to_string(),
        JSValue::from_native_function(object_get_prototype_of),
    );
    // register setPrototypeOf
    obj.set(
        "setPrototypeOf".to_string(),
        JSValue::from_native_function(object_set_prototype_of),
    );

    // Object.preventExtensions / isExtensible / seal / freeze
    obj.set(
        "preventExtensions".to_string(),
        JSValue::from_native_function(object_prevent_extensions),
    );
    obj.set(
        "isExtensible".to_string(),
        JSValue::from_native_function(object_is_extensible),
    );
    obj.set(
        "seal".to_string(),
        JSValue::from_native_function(object_seal),
    );
    obj.set(
        "freeze".to_string(),
        JSValue::from_native_function(object_freeze),
    );

    // register defineProperty and getOwnPropertyDescriptor
    obj.set(
        "defineProperty".to_string(),
        JSValue::from_native_function(object_define_property),
    );
    obj.set(
        "getOwnPropertyDescriptor".to_string(),
        JSValue::from_native_function(object_get_own_property_descriptor),
    );
    obj.set(
        "keys".to_string(),
        JSValue::from_native_function(object_keys),
    );
    obj.set(
        "entries".to_string(),
        JSValue::from_native_function(object_entries),
    );
    obj.set(
        "values".to_string(),
        JSValue::from_native_function(object_values),
    );
    obj.set(
        "fromEntries".to_string(),
        JSValue::from_native_function(object_from_entries),
    );
    obj.set(
        "getOwnPropertyNames".to_string(),
        JSValue::from_native_function(object_get_own_property_names),
    );
    obj.set(
        "getOwnPropertySymbols".to_string(),
        JSValue::from_native_function(object_get_own_property_symbols),
    );
    obj.set(
        "assign".to_string(),
        JSValue::from_native_function(object_assign),
    );
    obj.set("is".to_string(), JSValue::from_native_function(object_is));

    // Set prototype property on constructor-like object
    let proto = Rc::new(RefCell::new(proto));
    obj.set(
        "prototype".to_string(),
        JSValue::from_object(Rc::clone(&proto)),
    );
    let obj = Rc::new(RefCell::new(obj));
    proto.borrow_mut().define_property(
        "constructor".to_string(),
        crate::value::jsobject::Property {
            value: JSValue::from_object(Rc::clone(&obj)),
            enumerable: false,
            writable: true,
            configurable: true,
            getter: None,
            setter: None,
        },
    );
    global
        .borrow_mut()
        .set("Object".to_string(), JSValue::from_object(obj));
}

fn builtin_method(function: crate::NativeFunctionType) -> crate::value::jsobject::Property {
    crate::value::jsobject::Property {
        value: JSValue::from_native_function(function),
        enumerable: false,
        writable: true,
        configurable: true,
        getter: None,
        setter: None,
    }
}
