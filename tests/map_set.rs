use pixi_byte::{JSEngine, JSValue};

#[test]
fn set_constructs_from_arrays_and_mutates_membership() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = new Set(["a", "b", "a"]);
            values.add("c");
            const deleted = values.delete("b");
            values.has("a") && values.has("c") && deleted && !values.has("b");
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn map_supports_identity_keys_and_chained_set() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const key = {};
            const other = {};
            const values = new Map;
            values.set(key, 1).set(other, 2);
            values.get(key) === 1 && values.get(other) === 2 && !values.has({});
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}
