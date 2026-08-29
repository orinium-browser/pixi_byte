use pixi_byte::JSValue;
#[test]
fn test_jsvalue_to_string() {
    assert_eq!(JSValue::undefined().to_string(), "undefined");
    assert_eq!(JSValue::null().to_string(), "null");
    assert_eq!(JSValue::from_bool(true).to_string(), "true");
}
#[test]
fn test_jsvalue_to_number() {
    assert!(JSValue::undefined().to_number().is_nan());
    assert_eq!(JSValue::null().to_number(), 0.0);
    assert_eq!(JSValue::from_bool(true).to_number(), 1.0);
}
#[test]
fn test_jsvalue_to_boolean() {
    assert!(!JSValue::undefined().to_boolean());
    assert!(!JSValue::null().to_boolean());
    assert!(JSValue::from_bool(true).to_boolean());
}
#[test]
fn test_jsvalue_strict_equals() {
    assert!(JSValue::undefined().strict_equals(&JSValue::undefined()));
    assert!(JSValue::null().strict_equals(&JSValue::null()));
}
#[test]
fn test_jsvalue_abstract_equals() {
    assert!(JSValue::null().abstract_equals(&JSValue::undefined()));
    assert!(JSValue::from_number(42.0).abstract_equals(&JSValue::from_string("42".to_string())));
}
