use pixi_byte::{JSEngine, JSValue};

#[test]
fn string_primitives_expose_common_prototype_methods() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const parts = " Alpha,Beta ".trim().toLowerCase().split(",");
            parts[0] === "alpha" && parts[1] === "beta" &&
                "hello".substring(1, 4) === "ell" &&
                "tailwind".startsWith("wind", 4) &&
                "tailwind.css".endsWith("wind", 8);
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn replace_supports_global_regexes_templates_and_callbacks() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const escaped = "a=b:c".replace(/[=:]/g, "[$&]");
            const callback = "a1b2".replace(/[0-9]/g, function (match) {
                return "-" + match;
            });
            escaped === "a[=]b[:]c" && callback === "a-1b-2";
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn string_constructor_and_from_char_code_are_callable() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(r#"String(42) + String.fromCharCode(65, 66)"#)
        .unwrap();
    assert_eq!(result, JSValue::String("42AB".to_string()));
}

#[test]
fn char_code_at_uses_utf16_code_units() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(r#""A😀".charCodeAt(0) === 65 && "A😀".charCodeAt(1) === 55357"#)
        .unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn last_index_of_honors_the_search_position() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            "text-blue-600".lastIndexOf("-") === 9 &&
                "text-blue-600".lastIndexOf("-", 8) === 4 &&
                "A😀B😀".lastIndexOf("😀") === 4 &&
                "abc".lastIndexOf("", 2) === 2;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}
