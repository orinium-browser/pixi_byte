use pixi_byte::value::{JSArray, JSValue};
#[test]
fn test_array_create() {
    let arr = JSArray::new();
    assert_eq!(arr.length(), 0);
}
#[test]
fn test_array_push_pop() {
    let mut arr = JSArray::new();
    arr.push(JSValue::Number(1.0));
    arr.push(JSValue::Number(2.0));
    arr.push(JSValue::Number(3.0));
    assert_eq!(arr.length(), 3);
    assert_eq!(arr.pop(), JSValue::Number(3.0));
    assert_eq!(arr.length(), 2);
}
#[test]
fn test_array_get_set() {
    let mut arr = JSArray::new();
    arr.set(0, JSValue::String("first".to_string()));
    arr.set(2, JSValue::String("third".to_string()));
    assert_eq!(arr.get(0), JSValue::String("first".to_string()));
    assert_eq!(arr.get(1), JSValue::Undefined);
    assert_eq!(arr.get(2), JSValue::String("third".to_string()));
}
#[test]
fn test_array_shift_unshift() {
    let mut arr = JSArray::from_vec(vec![JSValue::Number(2.0), JSValue::Number(3.0)]);
    arr.unshift(JSValue::Number(1.0));
    assert_eq!(arr.length(), 3);
    let first = arr.shift();
    assert_eq!(first, JSValue::Number(1.0));
}
#[test]
fn test_array_from_vec() {
    let arr = JSArray::from_vec(vec![JSValue::String("a".to_string()), JSValue::String("b".to_string())]);
    assert_eq!(arr.length(), 2);
}
