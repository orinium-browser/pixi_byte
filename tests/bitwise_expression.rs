use pixi_byte::{JSEngine, JSValue};

#[test]
fn bitwise_and_shift_expressions_follow_javascript_precedence() {
    let mut engine = JSEngine::new();
    for source in [
        "(12 & 10) === 8",
        "(12 | 3) === 15",
        "(12 ^ 10) === 6",
        "(1 << 4) === 16",
        "(16 >> 2) === 4",
        "(-1 >>> 1) === 2147483647",
        "(1 | 2 & 4) === 1",
    ] {
        assert_eq!(
            engine.eval(source).unwrap(),
            JSValue::from_bool(true),
            "failed expression: {source}"
        );
    }
}

#[test]
fn exponentiation_is_right_associative() {
    let mut engine = JSEngine::new();
    assert_eq!(engine.eval("2 ** 3 ** 2").unwrap(), JSValue::from_number(512.0));
}
