use pixi_byte::{JSEngine, JSValue};

#[test]
fn json_stringify_quotes_strings_for_selectors() {
    let mut engine = JSEngine::new();
    let result = engine.eval(r#"JSON.stringify("a\"b")"#).unwrap();
    assert_eq!(result, JSValue::String(r#""a\"b""#.to_string()));
}

#[test]
fn json_stringify_serializes_arrays_and_objects() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(r#"JSON.stringify({ value: 1, items: [true, null, "x"] })"#)
        .unwrap();
    let JSValue::String(result) = result else {
        panic!("JSON.stringify must return a string");
    };
    assert!(result.starts_with('{'));
    assert!(result.ends_with('}'));
    assert!(result.contains(r#""value":1"#));
    assert!(result.contains(r#""items":[true,null,"x"]"#));
}
