use pixi_byte::{JSEngine, JSValue};

#[test]
fn boolean_converts_values_using_javascript_truthiness() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval("Boolean(0) === false && Boolean('') === false && Boolean('x') === true")
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}
