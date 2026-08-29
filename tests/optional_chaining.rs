use pixi_byte::{JSEngine, JSValue};

#[test]
fn optional_member_access_returns_undefined_for_nullish_receiver() {
    let mut engine = JSEngine::new();
    assert_eq!(engine.eval("null?.value;").unwrap(), JSValue::undefined());
    assert_eq!(
        engine
            .eval("const value = { answer: 42 }; value?.answer;")
            .unwrap(),
        JSValue::from_number(42.0)
    );
}

#[test]
fn optional_method_call_preserves_receiver() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const value = {
                    answer: 40,
                    read(extra) { return this.answer + extra; }
                };
                value?.read?.(2);
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(42.0));
}

#[test]
fn optional_call_returns_undefined_for_missing_function() {
    let mut engine = JSEngine::new();
    assert_eq!(
        engine.eval("const value = {}; value.missing?.();").unwrap(),
        JSValue::undefined()
    );
}

#[test]
fn nullish_coalescing_only_evaluates_fallback_for_nullish_values() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                let calls = 0;
                const zero = 0 ?? (calls = calls + 1);
                const missing = null ?? (calls = calls + 2);
                zero + missing + calls;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(4.0));
}
