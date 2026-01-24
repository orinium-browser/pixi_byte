//! Minimal Object builtin implementation
//!
//! Provides `Object` constructor-like object with methods such as `create`, `getPrototypeOf`,
//! `setPrototypeOf`, and helpers like `defineProperty` and `getOwnPropertyDescriptor`.

use crate::value::JSValue;
use crate::value::jsobject::JSObject;
use std::cell::RefCell;
use std::rc::Rc;

/// Object.create(proto[, properties])
/// - `proto` must be object or null
/// - If second arg present, it is treated as property descriptor object
fn object_create(_vm: &mut crate::vm::VM, mut args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    if args.is_empty() {
        return Err(crate::error::JSError::TypeError(
            "Object.create: missing prototype argument".to_string(),
        ));
    }

    let proto = args.remove(0);

    let new_obj_rc = match proto {
        JSValue::Object(obj_ref) => {
            let new_obj = JSObject::with_prototype(Some(obj_ref.clone()));
            Rc::new(RefCell::new(new_obj))
        }
        JSValue::Null => Rc::new(RefCell::new(JSObject::with_prototype(None))),
        _ => {
            return Err(crate::error::JSError::TypeError(
                "Object.create: prototype must be an object or null".to_string(),
            ))
        }
    };

    // If a second argument is provided, treat it as property descriptors
    if !args.is_empty() {
        let props = args.remove(0);
        if let JSValue::Object(desc_ref) = props {
            // iterate enumerable keys of descriptor object
            for key in desc_ref.borrow().keys() {
                let desc_val = desc_ref.borrow().get(&key);
                // descriptor should be an object
                if let JSValue::Object(dobj_ref) = desc_val {
                    // Use define_property_descriptor to respect defaults/partial descriptors
                    new_obj_rc.borrow_mut().define_property_descriptor(key, dobj_ref.clone());
                } else {
                    // If descriptor is not object, treat it as value shorthand
                    let prop = crate::value::jsobject::Property::data(desc_val);
                    new_obj_rc.borrow_mut().define_property(key, prop);
                }
            }
        } else {
            return Err(crate::error::JSError::TypeError(
                "Object.create: properties argument must be an object".to_string(),
            ));
        }
    }

    Ok(JSValue::Object(new_obj_rc))
}

/// Object.getPrototypeOf(obj)
/// - returns the prototype object or null
fn object_get_prototype_of(_vm: &mut crate::vm::VM, mut args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    // args: [obj]
    if args.is_empty() {
        return Err(crate::error::JSError::TypeError(
            "Object.getPrototypeOf: missing object argument".to_string(),
        ));
    }

    let obj = args.remove(0);

    match obj {
        JSValue::Object(obj_ref) => {
            if let Some(proto) = obj_ref.borrow().get_prototype() {
                Ok(JSValue::Object(proto.clone()))
            } else {
                Ok(JSValue::Null)
            }
        }
        _ => Err(crate::error::JSError::TypeError(
            "Object.getPrototypeOf: argument is not an object".to_string(),
        )),
    }
}

/// getter for Object.prototype.__proto__
fn object_proto_get(_vm: &mut crate::vm::VM, mut args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    if args.is_empty() {
        return Ok(JSValue::Undefined);
    }
    let receiver = args.remove(0);
    match receiver {
        JSValue::Object(obj_ref) => {
            if let Some(proto) = obj_ref.borrow().get_prototype() {
                Ok(JSValue::Object(proto))
            } else {
                Ok(JSValue::Null)
            }
        }
        _ => Ok(JSValue::Undefined),
    }
}

/// setter for Object.prototype.__proto__
fn object_proto_set(_vm: &mut crate::vm::VM, mut args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    if args.is_empty() {
        return Ok(JSValue::Undefined);
    }
    let receiver = args.remove(0);
    let new_proto = if !args.is_empty() { args.remove(0) } else { JSValue::Undefined };

    match receiver {
        JSValue::Object(obj_ref) => match new_proto {
            JSValue::Object(o) => {
                // Prevent prototype cycles: ensure `obj_ref` is not in the prototype chain of `o`
                let mut current = Some(o.clone());
                while let Some(p) = current {
                    if Rc::ptr_eq(&p, &obj_ref) {
                        return Err(crate::error::JSError::TypeError(
                            "__proto__ setter: setting prototype would create a cycle".to_string(),
                        ));
                    }
                    current = p.borrow().get_prototype();
                }

                obj_ref.borrow_mut().set_prototype(Some(o));
                Ok(JSValue::Undefined)
            }
            JSValue::Null => {
                obj_ref.borrow_mut().set_prototype(None);
                Ok(JSValue::Undefined)
            }
            _ => Err(crate::error::JSError::TypeError(
                "__proto__ setter: value must be an object or null".to_string(),
            )),
        },
        _ => Err(crate::error::JSError::TypeError(
            "__proto__ setter: receiver is not an object".to_string(),
        )),
    }
}

/// Object.setPrototypeOf(obj, proto)
fn object_set_prototype_of(_vm: &mut crate::vm::VM, mut args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    if args.len() < 2 {
        return Err(crate::error::JSError::TypeError(
            "Object.setPrototypeOf: missing arguments".to_string(),
        ));
    }

    let obj = args.remove(0);
    let proto = args.remove(0);

    match obj {
        JSValue::Object(obj_ref) => match proto {
            JSValue::Object(o) => {
                // Prevent prototype cycles: ensure obj_ref is not reachable from o
                let mut current = Some(o.clone());
                while let Some(p) = current {
                    if Rc::ptr_eq(&p, &obj_ref) {
                        return Err(crate::error::JSError::TypeError(
                            "Object.setPrototypeOf: setting prototype would create a cycle".to_string(),
                        ));
                    }
                    current = p.borrow().get_prototype();
                }

                obj_ref.borrow_mut().set_prototype(Some(o));
                Ok(JSValue::Object(obj_ref.clone()))
            }
            JSValue::Null => {
                obj_ref.borrow_mut().set_prototype(None);
                Ok(JSValue::Object(obj_ref.clone()))
            }
            _ => Err(crate::error::JSError::TypeError(
                "Object.setPrototypeOf: prototype must be an object or null".to_string(),
            )),
        },
        _ => Err(crate::error::JSError::TypeError(
            "Object.setPrototypeOf: first argument must be an object".to_string(),
        )),
    }
}

/// Object.prototype.hasOwnProperty のネイティブ実装
fn object_has_own_property(_vm: &mut crate::vm::VM, mut args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    if args.is_empty() {
        return Err(crate::error::JSError::TypeError(
            "hasOwnProperty: missing receiver".to_string(),
        ));
    }
    let receiver = args.remove(0);
    if args.is_empty() {
        return Err(crate::error::JSError::TypeError(
            "hasOwnProperty: missing property name".to_string(),
        ));
    }
    let prop = args.remove(0);
    let key = prop.to_string();

    match receiver {
        JSValue::Object(obj_ref) => Ok(JSValue::Boolean(obj_ref.borrow().has_own_property(&key))),
        _ => Err(crate::error::JSError::TypeError(
            "hasOwnProperty: receiver is not an object".to_string(),
        )),
    }
}

/// Object.prototype.isPrototypeOf のネイティブ実装
fn object_is_prototype_of(_vm: &mut crate::vm::VM, mut args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    if args.len() < 2 {
        return Err(crate::error::JSError::TypeError(
            "isPrototypeOf: missing arguments".to_string(),
        ));
    }
    let receiver = args.remove(0);
    let obj = args.remove(0);

    match (receiver, obj) {
        (JSValue::Object(proto_ref), JSValue::Object(target_ref)) => {
            let mut current = target_ref.borrow().get_prototype();
            while let Some(p) = current {
                if Rc::ptr_eq(&p, &proto_ref) {
                    return Ok(JSValue::Boolean(true));
                }
                current = p.borrow().get_prototype();
            }
            Ok(JSValue::Boolean(false))
        }
        _ => Err(crate::error::JSError::TypeError(
            "isPrototypeOf: receiver and argument must be objects".to_string(),
        )),
    }
}

/// Object.prototype.toString のネイティブ実装
fn object_to_string(_vm: &mut crate::vm::VM, mut args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    if args.is_empty() {
        return Ok(JSValue::String("[object Object]".to_string()));
    }
    let receiver = args.remove(0);
    match receiver {
        JSValue::Object(_) => Ok(JSValue::String("[object Object]".to_string())),
        JSValue::Function(_, _, _, _) | JSValue::NativeFunction(_) => Ok(JSValue::String("[object Function]".to_string())),
        JSValue::String(_) => Ok(JSValue::String("[object String]".to_string())),
        JSValue::Number(_) => Ok(JSValue::String("[object Number]".to_string())),
        JSValue::Boolean(_) => Ok(JSValue::String("[object Boolean]".to_string())),
        JSValue::Null => Ok(JSValue::String("[object Null]".to_string())),
        JSValue::Undefined => Ok(JSValue::String("[object Undefined]".to_string())),
    }
}

/// グローバルオブジェクトに Object 組み込みをインストールする
pub fn install(global: &Rc<RefCell<JSObject>>) {
    // TODO: グローバルオブジェクトに `Object` をオブジェクトとしてセット
    let mut obj = JSObject::new();

    // Create Object.prototype and attach methods (e.g., hasOwnProperty)
    let mut proto = JSObject::new();
    proto.set("hasOwnProperty".to_string(), JSValue::NativeFunction(object_has_own_property));
    proto.set("isPrototypeOf".to_string(), JSValue::NativeFunction(object_is_prototype_of));
    proto.set("toString".to_string(), JSValue::NativeFunction(object_to_string));
    proto.set("propertyIsEnumerable".to_string(), JSValue::NativeFunction(object_property_is_enumerable));

    // Define __proto__ accessor on Object.prototype
    let proto_accessor = crate::value::jsobject::Property {
        value: JSValue::Undefined,
        enumerable: false,
        writable: false,
        configurable: true,
        getter: Some(JSValue::NativeFunction(object_proto_get)),
        setter: Some(JSValue::NativeFunction(object_proto_set)),
    };
    proto.define_property("__proto__".to_string(), proto_accessor);

    // `create` と `getPrototypeOf` をネイティブ関数として登録
    obj.set("create".to_string(), JSValue::NativeFunction(object_create));
    obj.set(
        "getPrototypeOf".to_string(),
        JSValue::NativeFunction(object_get_prototype_of),
    );
    // register setPrototypeOf
    obj.set("setPrototypeOf".to_string(), JSValue::NativeFunction(object_set_prototype_of));

    // Object.preventExtensions / isExtensible / seal / freeze
    obj.set("preventExtensions".to_string(), JSValue::NativeFunction(object_prevent_extensions));
    obj.set("isExtensible".to_string(), JSValue::NativeFunction(object_is_extensible));
    obj.set("seal".to_string(), JSValue::NativeFunction(object_seal));
    obj.set("freeze".to_string(), JSValue::NativeFunction(object_freeze));

    // register defineProperty and getOwnPropertyDescriptor
    obj.set("defineProperty".to_string(), JSValue::NativeFunction(object_define_property));
    obj.set(
        "getOwnPropertyDescriptor".to_string(),
        JSValue::NativeFunction(object_get_own_property_descriptor),
    );

    // Set prototype property on constructor-like object
    obj.set("prototype".to_string(), JSValue::Object(Rc::new(RefCell::new(proto))));

    global.borrow_mut().set("Object".to_string(), JSValue::Object(Rc::new(RefCell::new(obj))));
}

// New: Object.defineProperty(obj, prop, descriptor)
fn object_define_property(_vm: &mut crate::vm::VM, mut args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    if args.len() < 3 {
        return Err(crate::error::JSError::TypeError(
            "Object.defineProperty: missing arguments".to_string(),
        ));
    }
    let obj = args.remove(0);
    let prop_name = args.remove(0);
    let desc = args.remove(0);

    let key = prop_name.to_string();

    match obj {
        JSValue::Object(obj_ref) => {
            if let JSValue::Object(desc_ref) = desc {
                // use define_property_descriptor to apply descriptor semantics
                obj_ref.borrow_mut().define_property_descriptor(key, desc_ref.clone());
                Ok(JSValue::Object(obj_ref.clone()))
            } else {
                return Err(crate::error::JSError::TypeError(
                    "Object.defineProperty: descriptor must be an object".to_string(),
                ));
            }
        }
        _ => Err(crate::error::JSError::TypeError(
            "Object.defineProperty: first argument must be an object".to_string(),
        )),
    }
}

// Object.preventExtensions(obj)
fn object_prevent_extensions(_vm: &mut crate::vm::VM, mut args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    if args.is_empty() {
        return Err(crate::error::JSError::TypeError("Object.preventExtensions: missing object".to_string()));
    }
    let obj = args.remove(0);
    match obj {
        JSValue::Object(obj_ref) => {
            obj_ref.borrow_mut().prevent_extensions();
            Ok(JSValue::Object(obj_ref.clone()))
        }
        _ => Err(crate::error::JSError::TypeError("Object.preventExtensions: argument must be an object".to_string())),
    }
}

/// Object.isExtensible(obj)
fn object_is_extensible(_vm: &mut crate::vm::VM, mut args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    if args.is_empty() {
        return Err(crate::error::JSError::TypeError("Object.isExtensible: missing object".to_string()));
    }
    let obj = args.remove(0);
    match obj {
        JSValue::Object(obj_ref) => Ok(JSValue::Boolean(obj_ref.borrow().is_extensible())),
        _ => Err(crate::error::JSError::TypeError("Object.isExtensible: argument must be an object".to_string())),
    }
}

/// Object.seal(obj): make all properties non-configurable and prevent extensions
fn object_seal(_vm: &mut crate::vm::VM, mut args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    if args.is_empty() {
        return Err(crate::error::JSError::TypeError("Object.seal: missing object".to_string()));
    }
    let obj = args.remove(0);
    match obj {
        JSValue::Object(obj_ref) => {
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
            Ok(JSValue::Object(obj_ref.clone()))
        }
        _ => Err(crate::error::JSError::TypeError("Object.seal: argument must be an object".to_string())),
    }
}

/// Object.freeze(obj): make all properties non-configurable and non-writable and prevent extensions
fn object_freeze(_vm: &mut crate::vm::VM, mut args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    if args.is_empty() {
        return Err(crate::error::JSError::TypeError("Object.freeze: missing object".to_string()));
    }
    let obj = args.remove(0);
    match obj {
        JSValue::Object(obj_ref) => {
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
            Ok(JSValue::Object(obj_ref.clone()))
        }
        _ => Err(crate::error::JSError::TypeError("Object.freeze: argument must be an object".to_string())),
    }
}

// New: Object.getOwnPropertyDescriptor(obj, prop)
fn object_get_own_property_descriptor(_vm: &mut crate::vm::VM, mut args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    if args.len() < 2 {
        return Err(crate::error::JSError::TypeError(
            "Object.getOwnPropertyDescriptor: missing arguments".to_string(),
        ));
    }
    let obj = args.remove(0);
    let prop_name = args.remove(0);
    let key = prop_name.to_string();

    match obj {
        JSValue::Object(obj_ref) => {
            if let Some(prop) = obj_ref.borrow().get_property_descriptor(&key) {
                // build descriptor object
                let mut desc = JSObject::new();
                // if accessor
                if prop.getter.is_some() || prop.setter.is_some() {
                    if let Some(getv) = prop.getter {
                        desc.set("get".to_string(), getv);
                    } else {
                        desc.set("get".to_string(), JSValue::Undefined);
                    }
                    if let Some(setv) = prop.setter {
                        desc.set("set".to_string(), setv);
                    } else {
                        desc.set("set".to_string(), JSValue::Undefined);
                    }
                } else {
                    desc.set("value".to_string(), prop.value.clone());
                    desc.set("writable".to_string(), JSValue::Boolean(prop.writable));
                }
                desc.set("enumerable".to_string(), JSValue::Boolean(prop.enumerable));
                desc.set("configurable".to_string(), JSValue::Boolean(prop.configurable));

                Ok(JSValue::Object(Rc::new(RefCell::new(desc))))
            } else {
                Ok(JSValue::Undefined)
            }
        }
        _ => Err(crate::error::JSError::TypeError(
            "Object.getOwnPropertyDescriptor: first argument must be an object".to_string(),
        )),
    }
}

/// Object.prototype.propertyIsEnumerable(prop)
fn object_property_is_enumerable(_vm: &mut crate::vm::VM, mut args: Vec<JSValue>) -> crate::error::JSResult<JSValue> {
    if args.is_empty() {
        return Err(crate::error::JSError::TypeError("propertyIsEnumerable: missing receiver".to_string()));
    }
    let receiver = args.remove(0);
    if args.is_empty() {
        return Err(crate::error::JSError::TypeError("propertyIsEnumerable: missing property name".to_string()));
    }
    let prop = args.remove(0);
    let key = prop.to_string();

    match receiver {
        JSValue::Object(obj_ref) => {
            if let Some(descr) = obj_ref.borrow().get_property_descriptor(&key) {
                Ok(JSValue::Boolean(descr.enumerable))
            } else {
                Ok(JSValue::Boolean(false))
            }
        }
        _ => Err(crate::error::JSError::TypeError("propertyIsEnumerable: receiver is not an object".to_string())),
    }
}
