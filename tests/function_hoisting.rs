use pixi_byte::{JSEngine, JSValue};

#[test]
fn function_declarations_are_available_before_their_source_position() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const result = later(20, 22);
            function later(left, right) {
                return left + right;
            }
            result;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(42.0));
}

#[test]
fn function_body_declarations_are_hoisted_per_call() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function outer() {
                return inner();
                function inner() {
                    return "ready";
                }
            }
            outer();
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("ready".to_string()));
}
