use pixi_byte::{JSEngine, JSValue};

#[test]
fn object_literals_inherit_object_prototype_methods() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const reserved = { key: true, ref: true };
            reserved.hasOwnProperty("key") &&
                !reserved.hasOwnProperty("missing") &&
                reserved.toString() === "[object Object]";
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn constructed_instances_inherit_object_prototype_methods() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function Value() { this.answer = 42; }
            const value = new Value();
            value.hasOwnProperty("answer") && value.toString() === "[object Object]";
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}
