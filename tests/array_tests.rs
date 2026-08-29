use pixi_byte::value::{JSArray, JSValue};
#[test]
fn test_array_create() {
    let arr = JSArray::new();
    assert_eq!(arr.length(), 0);
}
#[test]
fn test_array_push_pop() {
    let mut arr = JSArray::new();
    arr.push(JSValue::from_number(1.0));
    arr.push(JSValue::from_number(2.0));
    arr.push(JSValue::from_number(3.0));
    assert_eq!(arr.length(), 3);
    assert_eq!(arr.pop(), JSValue::from_number(3.0));
    assert_eq!(arr.length(), 2);
}
#[test]
fn test_array_get_set() {
    let mut arr = JSArray::new();
    arr.set(0, JSValue::from_string("first".to_string()));
    arr.set(2, JSValue::from_string("third".to_string()));
    assert_eq!(arr.get(0), JSValue::from_string("first".to_string()));
    assert_eq!(arr.get(1), JSValue::undefined());
    assert_eq!(arr.get(2), JSValue::from_string("third".to_string()));
}
#[test]
fn test_array_shift_unshift() {
    let mut arr = JSArray::from_vec(vec![JSValue::from_number(2.0), JSValue::from_number(3.0)]);
    arr.unshift(JSValue::from_number(1.0));
    assert_eq!(arr.length(), 3);
    let first = arr.shift();
    assert_eq!(first, JSValue::from_number(1.0));
}
#[test]
fn test_array_from_vec() {
    let arr = JSArray::from_vec(vec![
        JSValue::from_string("a".to_string()),
        JSValue::from_string("b".to_string()),
    ]);
    assert_eq!(arr.length(), 2);
}
