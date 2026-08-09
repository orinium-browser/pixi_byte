use pixi_byte::{JSEngine, JSValue};

#[test]
fn member_updates_preserve_prefix_and_postfix_results() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const target = { count: 1 };
            const old = target.count++;
            const current = ++target.count;
            old === 1 && current === 3 && target.count === 3;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn member_assignment_returns_the_assigned_value() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const target = {};
            const assigned = (target.value = 42);
            assigned === 42 && target.value === 42;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Boolean(true));
}
