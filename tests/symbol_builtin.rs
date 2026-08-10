use pixi_byte::{JSEngine, JSValue};

#[test]
fn symbol_for_returns_stable_registry_values() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            Symbol.for("react.element") === Symbol.for("react.element") &&
            Symbol.for("react.element") !== Symbol.for("react.portal") &&
            Symbol.iterator === "@@iterator";
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn symbol_call_returns_unique_values_and_well_known_symbols() {
    let mut engine = pixi_byte::JSEngine::new();
    let result = engine
        .eval(
            r#"
            const first = Symbol("key");
            const second = Symbol("key");
            first !== second && Symbol.toStringTag === "@@toStringTag";
            "#,
        )
        .unwrap();
    assert_eq!(result, pixi_byte::JSValue::Boolean(true));
}
