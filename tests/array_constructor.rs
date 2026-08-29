use pixi_byte::{JSEngine, JSValue};

#[test]
fn array_constructor_creates_a_requested_length() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = Array(3);
            Array.isArray(values) && values.length === 3;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

#[test]
fn array_constructor_accepts_elements_and_new() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const first = Array("a", "b");
            const second = new Array(1, 2);
            first.join("") === "ab" && second.join("") === "12";
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}
