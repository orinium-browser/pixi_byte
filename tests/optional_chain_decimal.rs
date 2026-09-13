use pixi_byte::{JSEngine, JSValue};

#[test]
fn ternary_question_mark_before_leading_dot_decimal() {
    // `?.` must not be lexed as the optional-chain operator when it is
    // followed by a digit (`W === void 0 ? .01 : W`).
    let mut engine = JSEngine::new();
    for source in [
        "(true ? .01 : 0) === 0.01",
        "(false ? .01 : 7) === 7",
        "(undefined ? .5 : 2) === 2",
        "var W = 0; ((W === void 0) ? .01 : W) === 0",
    ] {
        assert_eq!(
            engine.eval(source).unwrap(),
            JSValue::from_bool(true),
            "failed expression: {source}"
        );
    }
}

#[test]
fn optional_chaining_still_works() {
    let mut engine = JSEngine::new();
    for source in [
        "null?.x === undefined",
        "({ a: 42 })?.x === undefined",
        "({ a: 42 })?.a === 42",
        "var o = {}; (o?.x ?? 'fallback') === 'fallback'",
    ] {
        assert_eq!(
            engine.eval(source).unwrap(),
            JSValue::from_bool(true),
            "failed expression: {source}"
        );
    }
}

#[test]
fn unicode_escaped_identifiers() {
    let mut engine = JSEngine::new();
    for source in [
        "var id = {}; id.\\u0275prov = 42; id.\\u0275prov === 42",
        "var \\u0275x = 5; \\u0275x * 2 === 10",
        "var a = {}; a.foo\\u0062ar = 1; a.foobar === 1",
    ] {
        assert_eq!(
            engine.eval(source).unwrap(),
            JSValue::from_bool(true),
            "failed expression: {source}"
        );
    }
}
