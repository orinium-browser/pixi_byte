use pixi_byte::value::{JSObject, JSValue, Property};
use std::rc::Rc;
use std::cell::RefCell;
#[test]
fn test_object_create() {
    let obj = JSObject::new();
    assert_eq!(obj.get("nonexistent"), JSValue::Undefined);
}
#[test]
fn test_object_set_get() {
    let mut obj = JSObject::new();
    obj.set("name".to_string(), JSValue::String("Alice".to_string()));
    assert_eq!(obj.get("name"), JSValue::String("Alice".to_string()));
}
#[test]
fn test_object_has_property() {
    let mut obj = JSObject::new();
    obj.set("x".to_string(), JSValue::Number(10.0));
    assert!(obj.has_own_property("x"));
}
#[test]
fn test_object_delete() {
    let mut obj = JSObject::new();
    obj.set("temp".to_string(), JSValue::Number(42.0));
    assert!(obj.delete("temp"));
}
#[test]
fn test_prototype_chain() {
    let mut parent = JSObject::new();
    parent.set("inherited".to_string(), JSValue::String("from parent".to_string()));
    let parent_rc = Rc::new(RefCell::new(parent));
    let child = JSObject::with_prototype(Some(parent_rc));
    assert_eq!(child.get("inherited"), JSValue::String("from parent".to_string()));
}
#[test]
fn test_read_only_property() {
    let mut obj = JSObject::new();
    obj.define_property("const".to_string(), Property::read_only(JSValue::Number(42.0)));
    assert_eq!(obj.get("const"), JSValue::Number(42.0));
}
