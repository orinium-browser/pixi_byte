use pixi_byte::{JSEngine, JSValue};

#[test]
fn error_is_callable_and_exposes_standard_properties() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const error = Error("failed");
            error.name === "Error" &&
            error.message === "failed" &&
            error.toString() === "Error: failed" &&
            error.stack.includes("failed");
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn thrown_errors_are_preserved_by_catch() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let message = "";
            try {
                throw Error("boom");
            } catch (error) {
                message = error.message;
            }
            message;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::String("boom".to_string()));
}
