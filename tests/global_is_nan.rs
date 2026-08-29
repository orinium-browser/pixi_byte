use pixi_byte::{JSEngine, JSValue};

#[test]
fn global_is_nan_applies_number_conversion() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            isNaN("not a number") &&
            !isNaN("42") &&
            isNaN(undefined);
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}
