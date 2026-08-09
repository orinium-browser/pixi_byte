//! Map and Set collections used by ReactDOM caches.

use std::cell::RefCell;
use std::rc::Rc;

use crate::error::{JSError, JSResult};
use crate::value::jsobject::JSObject;
use crate::value::JSValue;
use crate::vm::VM;

const COUNT: &str = "__collection_count";

fn receiver(args: &[JSValue], method: &str) -> JSResult<Rc<RefCell<JSObject>>> {
    match args.first() {
        Some(JSValue::Object(object)) if object.borrow().has_own_property(COUNT) => {
            Ok(Rc::clone(object))
        }
        _ => Err(JSError::TypeError(format!("{method}: invalid receiver"))),
    }
}

fn count(object: &Rc<RefCell<JSObject>>) -> usize {
    object.borrow().get(COUNT).to_number() as usize
}

fn find(object: &Rc<RefCell<JSObject>>, key: &JSValue) -> Option<usize> {
    (0..count(object)).find(|index| {
        matches!(
            object.borrow().get(&format!("__collection_present_{index}")),
            JSValue::Boolean(true)
        ) && object
            .borrow()
            .get(&format!("__collection_key_{index}"))
            .strict_equals(key)
    })
}

fn insert(object: &Rc<RefCell<JSObject>>, key: JSValue, value: JSValue) {
    if let Some(index) = find(object, &key) {
        object
            .borrow_mut()
            .set(format!("__collection_value_{index}"), value);
        return;
    }
    let index = count(object);
    let mut object = object.borrow_mut();
    object.set(format!("__collection_key_{index}"), key);
    object.set(format!("__collection_value_{index}"), value);
    object.set(
        format!("__collection_present_{index}"),
        JSValue::Boolean(true),
    );
    object.set(COUNT.to_string(), JSValue::Number((index + 1) as f64));
}

fn create_collection(vm: &VM, constructor_name: &str) -> Rc<RefCell<JSObject>> {
    let prototype = match vm.global_object.borrow().get(constructor_name) {
        JSValue::Object(constructor) => match constructor.borrow().get("prototype") {
            JSValue::Object(prototype) => Some(prototype),
            _ => None,
        },
        _ => None,
    };
    let mut object = JSObject::with_prototype(prototype);
    object.set(COUNT.to_string(), JSValue::Number(0.0));
    Rc::new(RefCell::new(object))
}

fn set_constructor(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let set = create_collection(vm, "Set");
    if let Some(JSValue::Object(iterable)) = args.get(1) {
        let length = iterable.borrow().get("length").to_number() as usize;
        for index in 0..length {
            let value = iterable.borrow().get(&index.to_string());
            insert(&set, value.clone(), value);
        }
    }
    Ok(JSValue::Object(set))
}

fn set_add(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let set = receiver(&args, "Set.add")?;
    let value = args.get(1).cloned().unwrap_or(JSValue::Undefined);
    insert(&set, value.clone(), value);
    Ok(JSValue::Object(set))
}

fn collection_has(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let collection = receiver(&args, "collection.has")?;
    let key = args.get(1).cloned().unwrap_or(JSValue::Undefined);
    Ok(JSValue::Boolean(find(&collection, &key).is_some()))
}

fn collection_delete(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let collection = receiver(&args, "collection.delete")?;
    let key = args.get(1).cloned().unwrap_or(JSValue::Undefined);
    let Some(index) = find(&collection, &key) else {
        return Ok(JSValue::Boolean(false));
    };
    collection.borrow_mut().set(
        format!("__collection_present_{index}"),
        JSValue::Boolean(false),
    );
    Ok(JSValue::Boolean(true))
}

fn set_for_each(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let set = receiver(&args, "Set.forEach")?;
    let callback = args.get(1).cloned().unwrap_or(JSValue::Undefined);
    let this_arg = args.get(2).cloned().unwrap_or(JSValue::Undefined);
    let set_value = JSValue::Object(Rc::clone(&set));
    let mut index = 0;
    while index < count(&set) {
        if matches!(
            set.borrow().get(&format!("__collection_present_{index}")),
            JSValue::Boolean(true)
        ) {
            let value = set.borrow().get(&format!("__collection_key_{index}"));
            vm.call(
                callback.clone(),
                this_arg.clone(),
                vec![value.clone(), value, set_value.clone()],
            )?;
        }
        index += 1;
    }
    Ok(JSValue::Undefined)
}

fn map_for_each(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let map = receiver(&args, "Map.forEach")?;
    let callback = args.get(1).cloned().unwrap_or(JSValue::Undefined);
    let this_arg = args.get(2).cloned().unwrap_or(JSValue::Undefined);
    let map_value = JSValue::Object(Rc::clone(&map));
    let mut index = 0;
    while index < count(&map) {
        if matches!(
            map.borrow().get(&format!("__collection_present_{index}")),
            JSValue::Boolean(true)
        ) {
            let key = map.borrow().get(&format!("__collection_key_{index}"));
            let value = map.borrow().get(&format!("__collection_value_{index}"));
            vm.call(
                callback.clone(),
                this_arg.clone(),
                vec![value, key, map_value.clone()],
            )?;
        }
        index += 1;
    }
    Ok(JSValue::Undefined)
}

fn map_constructor(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let map = create_collection(vm, "Map");
    if let Some(JSValue::Object(iterable)) = args.get(1) {
        let length = iterable.borrow().get("length").to_number() as usize;
        for index in 0..length {
            let JSValue::Object(entry) = iterable.borrow().get(&index.to_string()) else {
                return Err(JSError::TypeError(
                    "Map constructor entry must be array-like".to_string(),
                ));
            };
            insert(&map, entry.borrow().get("0"), entry.borrow().get("1"));
        }
    }
    Ok(JSValue::Object(map))
}

fn map_set(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let map = receiver(&args, "Map.set")?;
    let key = args.get(1).cloned().unwrap_or(JSValue::Undefined);
    let value = args.get(2).cloned().unwrap_or(JSValue::Undefined);
    insert(&map, key, value);
    Ok(JSValue::Object(map))
}

fn map_get(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let map = receiver(&args, "Map.get")?;
    let key = args.get(1).cloned().unwrap_or(JSValue::Undefined);
    Ok(find(&map, &key)
        .map(|index| map.borrow().get(&format!("__collection_value_{index}")))
        .unwrap_or(JSValue::Undefined))
}

fn constructor(
    prototype: JSObject,
    construct: crate::value::jsvalue::NativeFunctionType,
) -> JSObject {
    let mut constructor = JSObject::new();
    constructor.set("__construct__".to_string(), JSValue::NativeFunction(construct));
    constructor.set(
        "prototype".to_string(),
        JSValue::Object(Rc::new(RefCell::new(prototype))),
    );
    constructor
}

/// Installs Map and Set constructors.
pub fn install(global: &Rc<RefCell<JSObject>>) {
    let mut set_prototype = JSObject::new();
    set_prototype.set("add".to_string(), JSValue::NativeFunction(set_add));
    set_prototype.set("has".to_string(), JSValue::NativeFunction(collection_has));
    set_prototype.set("forEach".to_string(), JSValue::NativeFunction(set_for_each));
    set_prototype.set(
        "delete".to_string(),
        JSValue::NativeFunction(collection_delete),
    );
    let mut map_prototype = JSObject::new();
    map_prototype.set("set".to_string(), JSValue::NativeFunction(map_set));
    map_prototype.set("get".to_string(), JSValue::NativeFunction(map_get));
    map_prototype.set("has".to_string(), JSValue::NativeFunction(collection_has));
    map_prototype.set("forEach".to_string(), JSValue::NativeFunction(map_for_each));
    map_prototype.set(
        "delete".to_string(),
        JSValue::NativeFunction(collection_delete),
    );

    global.borrow_mut().set(
        "Set".to_string(),
        JSValue::Object(Rc::new(RefCell::new(constructor(
            set_prototype,
            set_constructor,
        )))),
    );
    global.borrow_mut().set(
        "Map".to_string(),
        JSValue::Object(Rc::new(RefCell::new(constructor(
            map_prototype,
            map_constructor,
        )))),
    );
}
