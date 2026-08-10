use pixi_byte::{JSEngine, JSValue};

#[test]
fn regexp_literals_support_flags_and_test() {
    let mut engine = JSEngine::new();
    assert_eq!(
        engine.eval(r#"/^hello/i.test("Hello world")"#).unwrap(),
        JSValue::Boolean(true)
    );
}

#[test]
fn regexp_exec_returns_captures_and_match_index() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const match = /(a)(b)/.exec("--ab--");
            match[0] === "ab" && match[1] === "a" && match.index === 2;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn division_is_not_tokenized_as_a_regexp() {
    let mut engine = JSEngine::new();
    assert_eq!(engine.eval("12 / 3 / 2").unwrap(), JSValue::Number(2.0));
}

#[test]
fn javascript_unicode_escapes_are_supported_in_regexp_literals() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            /^[\u00C0-\u00D6]+$/.test("ÀÖ")
                && "a\0b�c".replace(/\u0000|\uFFFD/g, "-") === "a-b-c";
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn regexp_constructor_exposes_source_flags_and_test() {
    let mut engine = pixi_byte::JSEngine::new();
    let result = engine
        .eval("const r = new RegExp('^a+$', 'i'); [r.source, r.flags, r.ignoreCase, r.test('AAA')].join('|')")
        .unwrap();
    assert_eq!(result.to_string(), "^a+$|i|true|true");
}

#[test]
fn regexp_is_callable_without_new() {
    let mut engine = pixi_byte::JSEngine::new();
    let result = engine.eval("RegExp('^ok$').test('ok')").unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn javascript_character_classes_allow_a_literal_open_bracket() {
    let mut engine = pixi_byte::JSEngine::new();
    let result = engine.eval(r#"/[\\^$.*+?()[\]{}|]/g.test("[")"#).unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn unsupported_lookarounds_do_not_prevent_regexp_construction() {
    let mut engine = pixi_byte::JSEngine::new();
    let result = engine
        .eval(r#"new RegExp("\\s+(add|subtract)\\b(?!\\))\\s*(?=[,])").test(" add ,")"#)
        .unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}
