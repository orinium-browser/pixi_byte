use pixi_byte::JSValue;
#[test]
fn test_jsvalue_to_string() {
    assert_eq!(JSValue::Undefined.to_string(), "undefined");
    assert_eq!(JSValue::Null.to_string(), "null");
    assert_eq!(JSValue::Boolean(true).to_string(), "true");
}
#[test]
fn test_jsvalue_to_number() {
    assert!(JSValue::Undefined.to_number().is_nan());
    assert_eq!(JSValue::Null.to_number(), 0.0);
    assert_eq!(JSValue::Boolean(true).to_number(), 1.0);
}
#[test]
fn test_jsvalue_to_boolean() {
    assert!(!JSValue::Undefined.to_boolean());
    assert!(!JSValue::Null.to_boolean());
    assert!(JSValue::Boolean(true).to_boolean());
}
#[test]
fn test_jsvalue_strict_equals() {
    assert!(JSValue::Undefined.strict_equals(&JSValue::Undefined));
    assert!(JSValue::Null.strict_equals(&JSValue::Null));
}
#[test]
fn test_jsvalue_abstract_equals() {
    assert!(JSValue::Null.abstract_equals(&JSValue::Undefined));
    assert!(JSValue::Number(42.0).abstract_equals(&JSValue::String("42".to_string())));
}
