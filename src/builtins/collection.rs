//! Map and Set collections used by ReactDOM caches.

use std::cell::RefCell;
use std::rc::Rc;

use crate::error::{JSError, JSResult};
use crate::value::JSValue;
use crate::value::jsobject::{JSObject, Property};
use crate::vm::VM;

const COUNT: &str = "__collection_count";
const ITERATOR_COLLECTION: &str = "__collection_iterator_collection";
const ITERATOR_INDEX: &str = "__collection_iterator_index";
const ITERATOR_KIND: &str = "__collection_iterator_kind";

fn receiver(args: &[JSValue], method: &str) -> JSResult<Rc<RefCell<JSObject>>> {
    match args.first() {
        Some(value) if value.clone().is_object() => {
            let object = value.as_object().unwrap();
            if object.borrow().has_own_property(COUNT) {
                return Ok(object);
            }
            Err(JSError::TypeError(format!("{method}: invalid receiver")))
        }
        _ => Err(JSError::TypeError(format!("{method}: invalid receiver"))),
    }
}

fn count(object: &Rc<RefCell<JSObject>>) -> usize {
    object.borrow().get(COUNT).to_number() as usize
}

fn collection_size(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let collection = receiver(&args, "collection.size")?;
    let size = (0..count(&collection))
        .filter(|index| {
            collection
                .borrow()
                .get(&format!("__collection_present_{index}"))
                .as_boolean()
                == Some(true)
        })
        .count();
    Ok(JSValue::from_number(size as f64))
}

fn size_property() -> Property {
    Property {
        value: JSValue::undefined(),
        enumerable: false,
        writable: false,
        configurable: true,
        getter: Some(JSValue::from_native_function(collection_size)),
        setter: None,
    }
}

fn find(object: &Rc<RefCell<JSObject>>, key: &JSValue) -> Option<usize> {
    (0..count(object)).find(|index| {
        object
            .borrow()
            .get(&format!("__collection_present_{index}"))
            .as_boolean()
            == Some(true)
            && object
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
        JSValue::from_bool(true),
    );
    object.set(COUNT.to_string(), JSValue::from_number((index + 1) as f64));
}

fn create_collection(vm: &VM, constructor_name: &str) -> Rc<RefCell<JSObject>> {
    let prototype = vm
        .global_object
        .borrow()
        .get(constructor_name)
        .as_object()
        .and_then(|constructor| {
            let proto = constructor.borrow().get("prototype");
            proto.as_object()
        });
    let mut object = JSObject::with_prototype(prototype);
    object.set(COUNT.to_string(), JSValue::from_number(0.0));
    Rc::new(RefCell::new(object))
}

fn iterable_values(vm: &mut VM, value: &JSValue) -> JSResult<Vec<JSValue>> {
    let object = match value.as_object() {
        Some(o) => o,
        None => return Ok(Vec::new()),
    };
    let length = object.borrow().get("length").to_number();
    if length.is_finite() && length >= 0.0 {
        return Ok((0..length.floor() as usize)
            .map(|index| object.borrow().get_index(index))
            .collect());
    }
    let iterator_method = object.borrow().get("@@iterator");
    if !is_callable(&iterator_method) {
        return Ok(Vec::new());
    }
    let iterator = vm.call(iterator_method, value.clone(), Vec::new())?;
    let iterator_object = iterator.as_object().ok_or_else(|| {
        JSError::TypeError("collection iterator must return an object".to_string())
    })?;
    let next = iterator_object.borrow().get("next");
    let mut values = Vec::new();
    loop {
        let result = vm.call(next.clone(), iterator.clone(), Vec::new())?;
        let result_object = result.as_object().ok_or_else(|| {
            JSError::TypeError("collection iterator result must be an object".to_string())
        })?;
        if result_object.borrow().get("done").to_boolean() {
            break;
        }
        values.push(result_object.borrow().get("value"));
    }
    Ok(values)
}

fn is_callable(value: &JSValue) -> bool {
    value.is_callable()
}

fn set_constructor(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let set = create_collection(vm, "Set");
    if let Some(iterable) = args.get(1) {
        for value in iterable_values(vm, iterable)? {
            insert(&set, value.clone(), value);
        }
    }
    Ok(JSValue::from_object(set))
}

fn set_add(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let set = receiver(&args, "Set.add")?;
    let value = args.get(1).cloned().unwrap_or(JSValue::undefined());
    insert(&set, value.clone(), value);
    Ok(JSValue::from_object(set))
}

fn collection_has(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let collection = receiver(&args, "collection.has")?;
    let key = args.get(1).cloned().unwrap_or(JSValue::undefined());
    Ok(JSValue::from_bool(find(&collection, &key).is_some()))
}

fn collection_delete(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let collection = receiver(&args, "collection.delete")?;
    let key = args.get(1).cloned().unwrap_or(JSValue::undefined());
    let Some(index) = find(&collection, &key) else {
        return Ok(JSValue::from_bool(false));
    };
    collection.borrow_mut().set(
        format!("__collection_present_{index}"),
        JSValue::from_bool(false),
    );
    Ok(JSValue::from_bool(true))
}

fn set_for_each(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let set = receiver(&args, "Set.forEach")?;
    let callback = args.get(1).cloned().unwrap_or(JSValue::undefined());
    if callback.clone().is_undefined() || callback.clone().is_null() {
        return Err(crate::error::JSError::TypeError(
            "Set.prototype.forEach: callback is not callable".to_string(),
        ));
    }
    let this_arg = args.get(2).cloned().unwrap_or(JSValue::undefined());
    let set_value = JSValue::from_object(Rc::clone(&set));
    let mut index = 0;
    while index < count(&set) {
        if set
            .borrow()
            .get(&format!("__collection_present_{index}"))
            .as_boolean()
            == Some(true)
        {
            let value = set.borrow().get(&format!("__collection_key_{index}"));
            vm.call(
                callback.clone(),
                this_arg.clone(),
                vec![value.clone(), value, set_value.clone()],
            )?;
        }
        index += 1;
    }
    Ok(JSValue::undefined())
}

fn map_for_each(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let map = receiver(&args, "Map.forEach")?;
    let callback = args.get(1).cloned().unwrap_or(JSValue::undefined());
    if callback.clone().is_undefined() || callback.clone().is_null() {
        return Err(crate::error::JSError::TypeError(
            "Map.prototype.forEach: callback is not callable".to_string(),
        ));
    }
    let this_arg = args.get(2).cloned().unwrap_or(JSValue::undefined());
    let map_value = JSValue::from_object(Rc::clone(&map));
    let mut index = 0;
    while index < count(&map) {
        if map
            .borrow()
            .get(&format!("__collection_present_{index}"))
            .as_boolean()
            == Some(true)
        {
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
    Ok(JSValue::undefined())
}

fn make_iterator(collection: Rc<RefCell<JSObject>>, kind: &str) -> JSValue {
    let mut iterator = JSObject::new();
    iterator.set(
        ITERATOR_COLLECTION.to_string(),
        JSValue::from_object(collection),
    );
    iterator.set(ITERATOR_INDEX.to_string(), JSValue::from_number(0.0));
    iterator.set(
        ITERATOR_KIND.to_string(),
        JSValue::from_string(kind.to_string()),
    );
    iterator.set(
        "next".to_string(),
        JSValue::from_native_function(collection_iterator_next),
    );
    iterator.set(
        "@@iterator".to_string(),
        JSValue::from_native_function(iterator_identity),
    );
    JSValue::from_object(Rc::new(RefCell::new(iterator)))
}

fn iterator_identity(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(args.first().cloned().unwrap_or(JSValue::undefined()))
}

fn iterator_result(value: JSValue, done: bool) -> JSValue {
    let mut result = JSObject::new();
    result.set("value".to_string(), value);
    result.set("done".to_string(), JSValue::from_bool(done));
    JSValue::from_object(Rc::new(RefCell::new(result)))
}

fn collection_iterator_next(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let iterator = args.first().and_then(|v| v.as_object()).ok_or_else(|| {
        JSError::TypeError("collection iterator next: invalid receiver".to_string())
    })?;
    let value = iterator_step(vm, &iterator)?.ok_or_else(|| {
        JSError::TypeError("collection iterator next: invalid receiver".to_string())
    })?;
    Ok(match value {
        Some(value) => iterator_result(value, false),
        None => iterator_result(JSValue::undefined(), true),
    })
}

pub(crate) fn iterator_step(
    vm: &VM,
    iterator: &Rc<RefCell<JSObject>>,
) -> JSResult<Option<Option<JSValue>>> {
    let collection = match iterator.borrow().get(ITERATOR_COLLECTION).as_object() {
        Some(o) => o,
        None => return Ok(None),
    };
    let kind = iterator.borrow().get(ITERATOR_KIND).to_string();
    let mut index = iterator.borrow().get(ITERATOR_INDEX).to_number() as usize;
    while index < count(&collection) {
        iterator.borrow_mut().set(
            ITERATOR_INDEX.to_string(),
            JSValue::from_number((index + 1) as f64),
        );
        if collection
            .borrow()
            .get(&format!("__collection_present_{index}"))
            .as_boolean()
            == Some(true)
        {
            let key = collection
                .borrow()
                .get(&format!("__collection_key_{index}"));
            let value = collection
                .borrow()
                .get(&format!("__collection_value_{index}"));
            let value = match kind.as_str() {
                "map-key" => key,
                "map-value" | "set-value" => value,
                "map-entry" => vm.array_from_values(vec![key, value]),
                "set-entry" => vm.array_from_values(vec![key.clone(), key]),
                _ => JSValue::undefined(),
            };
            return Ok(Some(Some(value)));
        }
        index += 1;
    }
    Ok(Some(None))
}

fn set_values(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(make_iterator(receiver(&args, "Set.values")?, "set-value"))
}

fn set_entries(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(make_iterator(receiver(&args, "Set.entries")?, "set-entry"))
}

fn map_keys(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(make_iterator(receiver(&args, "Map.keys")?, "map-key"))
}

fn map_values(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(make_iterator(receiver(&args, "Map.values")?, "map-value"))
}

fn map_entries(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    Ok(make_iterator(receiver(&args, "Map.entries")?, "map-entry"))
}

fn map_constructor(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let map = create_collection(vm, "Map");
    if let Some(iterable) = args.get(1) {
        for entry in iterable_values(vm, iterable)? {
            let entry_object = entry.as_object().ok_or_else(|| {
                JSError::TypeError("Map constructor entry must be array-like".to_string())
            })?;
            insert(
                &map,
                entry_object.borrow().get("0"),
                entry_object.borrow().get("1"),
            );
        }
    }
    Ok(JSValue::from_object(map))
}

fn weak_map_constructor(vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let map = create_collection(vm, "WeakMap");
    if let Some(iterable) = args.get(1) {
        for entry in iterable_values(vm, iterable)? {
            let entry_object = entry.as_object().ok_or_else(|| {
                JSError::TypeError("WeakMap constructor entry must be array-like".to_string())
            })?;
            let key = entry_object.borrow().get("0");
            if !is_weak_key(&key) {
                return Err(JSError::TypeError(
                    "Invalid value used as weak map key".to_string(),
                ));
            }
            insert(&map, key, entry_object.borrow().get("1"));
        }
    }
    Ok(JSValue::from_object(map))
}

fn is_weak_key(value: &JSValue) -> bool {
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

fn weak_map_set(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let map = receiver(&args, "WeakMap.set")?;
    let key = args.get(1).cloned().unwrap_or(JSValue::undefined());
    if !is_weak_key(&key) {
        return Err(JSError::TypeError(
            "Invalid value used as weak map key".to_string(),
        ));
    }
    let value = args.get(2).cloned().unwrap_or(JSValue::undefined());
    insert(&map, key, value);
    Ok(JSValue::from_object(map))
}

fn map_set(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let map = receiver(&args, "Map.set")?;
    let key = args.get(1).cloned().unwrap_or(JSValue::undefined());
    let value = args.get(2).cloned().unwrap_or(JSValue::undefined());
    insert(&map, key, value);
    Ok(JSValue::from_object(map))
}

fn map_get(_vm: &mut VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    let map = receiver(&args, "Map.get")?;
    let key = args.get(1).cloned().unwrap_or(JSValue::undefined());
    Ok(find(&map, &key)
        .map(|index| map.borrow().get(&format!("__collection_value_{index}")))
        .unwrap_or(JSValue::undefined()))
}

fn constructor(
    prototype: JSObject,
    construct: crate::value::jsvalue::NativeFunctionType,
) -> JSObject {
    let mut constructor = JSObject::new();
    constructor.set(
        "__construct__".to_string(),
        JSValue::from_native_function(construct),
    );
    constructor.set(
        "prototype".to_string(),
        JSValue::from_object(Rc::new(RefCell::new(prototype))),
    );
    constructor
}

/// Installs Map and Set constructors.
pub fn install(global: &Rc<RefCell<JSObject>>) {
    let mut set_prototype = JSObject::new();
    set_prototype.define_property("size".to_string(), size_property());
    set_prototype.set("add".to_string(), JSValue::from_native_function(set_add));
    set_prototype.set(
        "has".to_string(),
        JSValue::from_native_function(collection_has),
    );
    set_prototype.set(
        "forEach".to_string(),
        JSValue::from_native_function(set_for_each),
    );
    set_prototype.set(
        "values".to_string(),
        JSValue::from_native_function(set_values),
    );
    set_prototype.set(
        "keys".to_string(),
        JSValue::from_native_function(set_values),
    );
    set_prototype.set(
        "entries".to_string(),
        JSValue::from_native_function(set_entries),
    );
    set_prototype.set(
        "@@iterator".to_string(),
        JSValue::from_native_function(set_values),
    );
    set_prototype.set(
        "delete".to_string(),
        JSValue::from_native_function(collection_delete),
    );
    let mut map_prototype = JSObject::new();
    map_prototype.define_property("size".to_string(), size_property());
    map_prototype.set("set".to_string(), JSValue::from_native_function(map_set));
    map_prototype.set("get".to_string(), JSValue::from_native_function(map_get));
    map_prototype.set(
        "has".to_string(),
        JSValue::from_native_function(collection_has),
    );
    map_prototype.set(
        "forEach".to_string(),
        JSValue::from_native_function(map_for_each),
    );
    map_prototype.set("keys".to_string(), JSValue::from_native_function(map_keys));
    map_prototype.set(
        "values".to_string(),
        JSValue::from_native_function(map_values),
    );
    map_prototype.set(
        "entries".to_string(),
        JSValue::from_native_function(map_entries),
    );
    map_prototype.set(
        "@@iterator".to_string(),
        JSValue::from_native_function(map_entries),
    );
    map_prototype.set(
        "delete".to_string(),
        JSValue::from_native_function(collection_delete),
    );
    let mut weak_map_prototype = JSObject::new();
    weak_map_prototype.set(
        "set".to_string(),
        JSValue::from_native_function(weak_map_set),
    );
    weak_map_prototype.set("get".to_string(), JSValue::from_native_function(map_get));
    weak_map_prototype.set(
        "has".to_string(),
        JSValue::from_native_function(collection_has),
    );
    weak_map_prototype.set(
        "delete".to_string(),
        JSValue::from_native_function(collection_delete),
    );

    global.borrow_mut().set(
        "Set".to_string(),
        JSValue::from_object(Rc::new(RefCell::new(constructor(
            set_prototype,
            set_constructor,
        )))),
    );
    global.borrow_mut().set(
        "Map".to_string(),
        JSValue::from_object(Rc::new(RefCell::new(constructor(
            map_prototype,
            map_constructor,
        )))),
    );
    global.borrow_mut().set(
        "WeakMap".to_string(),
        JSValue::from_object(Rc::new(RefCell::new(constructor(
            weak_map_prototype,
            weak_map_constructor,
        )))),
    );
}
