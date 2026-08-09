use pixi_byte::{JSEngine, JSValue};

#[test]
fn in_operator_checks_own_and_inherited_properties() {
    let mut engine = JSEngine::new();

    assert_eq!(
        engine.eval(r#""own" in { own: 1 }"#).unwrap(),
        JSValue::Boolean(true)
    );
    assert_eq!(
        engine.eval(r#""missing" in { own: 1 }"#).unwrap(),
        JSValue::Boolean(false)
    );
    assert_eq!(
        engine
            .eval(
                r#"
                const parent = { inherited: true };
                const child = Object.create(parent);
                "inherited" in child;
                "#,
            )
            .unwrap(),
        JSValue::Boolean(true)
    );
}

#[test]
fn in_operator_rejects_a_primitive_right_hand_side() {
    let mut engine = JSEngine::new();
    assert!(engine.eval(r#""length" in 1"#).is_err());
}
