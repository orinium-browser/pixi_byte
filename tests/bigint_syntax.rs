use pixi_byte::{JSEngine, JSValue};

#[test]
fn bigint_literals_are_accepted_by_bitwise_expressions() {
    let mut engine = JSEngine::new();
    let result = engine.eval("((1n | 2n) & 3n) === 3n;").unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}
