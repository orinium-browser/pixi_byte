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
