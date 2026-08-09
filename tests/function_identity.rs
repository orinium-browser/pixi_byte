use pixi_byte::{JSEngine, JSValue};

#[test]
fn function_clones_retain_reference_identity() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const original = function () { return 1; };
            const alias = original;
            const distinct = function () { return 1; };
            original === alias && original !== distinct;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn bound_function_clones_retain_reference_identity() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function target() {}
            const first = target.bind(null);
            const alias = first;
            const second = target.bind(null);
            first === alias && first !== second;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}
