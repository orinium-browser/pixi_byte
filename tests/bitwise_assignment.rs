use pixi_byte::{JSEngine, JSValue};

#[test]
fn bitwise_compound_assignments_update_identifiers() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let value = 5;
            value |= 2;
            value &= 6;
            value ^= 3;
            value <<= 2;
            value >>= 1;
            value >>>= 1;
            value;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Number(5.0));
}

#[test]
fn bitwise_compound_assignments_update_members() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const state = { flags: 1 };
            state.flags |= 4;
            state.flags &= 5;
            state.flags;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Number(5.0));
}
