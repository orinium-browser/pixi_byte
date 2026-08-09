use pixi_byte::{JSEngine, JSValue};

#[test]
fn new_calls_constructor_with_a_fresh_this_object() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function Box(value) { this.value = value; }
            let box = new Box(42);
            box.value;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Number(42.0));
}

#[test]
fn constructor_object_return_value_replaces_this() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function Factory() { return { value: 7 }; }
            new Factory().value;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Number(7.0));
}
