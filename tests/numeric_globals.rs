use pixi_byte::{JSEngine, JSValue};

#[test]
fn exposes_ecmascript_numeric_globals() {
    let mut engine = JSEngine::new();
    let result = engine.eval("Infinity === 1 / 0 && isNaN(NaN);").unwrap();

    assert_eq!(result, JSValue::Boolean(true));
}
