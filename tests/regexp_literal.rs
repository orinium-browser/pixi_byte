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
fn regexp_values_support_instanceof() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(r#"/ok/ instanceof RegExp && new RegExp("ok") instanceof RegExp && !({} instanceof RegExp)"#)
        .unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn javascript_character_classes_allow_a_literal_open_bracket() {
    let mut engine = pixi_byte::JSEngine::new();
    let result = engine.eval(r#"/[\\^$.*+?()[\]{}|]/g.test("[")"#).unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn legacy_character_class_ranges_allow_class_escapes_as_endpoints() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(r#"/^[\w-_]+$/.test("min-h-") && /[a-\s]/.test(" ")"#)
        .unwrap();
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

#[test]
fn invalid_quantifier_braces_are_treated_as_literals() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(r#"/{([^,]*?)}/.test("{value}") && /a{2,3}/.test("aaa")"#)
        .unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn global_regexp_test_and_exec_update_last_index() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const testPattern = /[; ]/g;
            testPattern.lastIndex = 1;
            const tested = testPattern.test("@tailwind base;");
            const afterTest = testPattern.lastIndex;
            const execPattern = /x/g;
            execPattern.lastIndex = 2;
            const match = execPattern.exec("abxxy");
            tested && afterTest === 10 && match.index === 2 &&
                execPattern.lastIndex === 3;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn sticky_regexp_requires_a_match_at_last_index_and_uses_utf16_offsets() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const pattern = /a/y;
            pattern.lastIndex = 2;
            const matched = pattern.test("😀a");
            const end = pattern.lastIndex;
            pattern.lastIndex = 2;
            matched && end === 3 && !pattern.test("😀ba") &&
                pattern.lastIndex === 0;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}
