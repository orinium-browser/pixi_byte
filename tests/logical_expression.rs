use pixi_byte::{JSEngine, JSValue};

#[test]
fn logical_and_skips_a_falsy_left_hand_side() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let calls = 0;
            function hit() {
                calls += 1;
                return true;
            }
            false && hit();
            calls;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_number(0.0));
}

#[test]
fn logical_or_skips_a_truthy_left_hand_side() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let calls = 0;
            function hit() {
                calls += 1;
                return false;
            }
            true || hit();
            calls;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_number(0.0));
}

#[test]
fn logical_expressions_preserve_operand_values() {
    let mut engine = JSEngine::new();

    assert_eq!(
        engine.eval(r#"0 || "fallback""#).unwrap(),
        JSValue::from_string("fallback".to_string())
    );
    assert_eq!(
        engine.eval(r#""left" && "right""#).unwrap(),
        JSValue::from_string("right".to_string())
    );
}
