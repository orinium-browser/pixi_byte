use pixi_byte::{JSEngine, JSValue};

#[test]
fn template_literal_interpolates_expressions() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const value = 3;
                `value=${value + 1}!`;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::String("value=4!".to_string()));
}

#[test]
fn template_literal_supports_object_literals_in_interpolation() {
    let mut engine = JSEngine::new();
    let result = engine.eval("`${({ value: 2 }).value}`;").unwrap();

    assert_eq!(result, JSValue::String("2".to_string()));
}

#[test]
fn string_concatenation_uses_custom_object_to_string() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const selector = {
                    toString() { return ".py-2"; }
                };
                `${selector}{padding:0.5rem}` + ":" + ("rule=" + selector);
            "#,
        )
        .unwrap();

    assert_eq!(
        result,
        JSValue::String(".py-2{padding:0.5rem}:rule=.py-2".to_string())
    );
}

#[test]
fn addition_uses_custom_object_value_of() {
    let mut engine = JSEngine::new();
    let result = engine.eval("({ valueOf() { return 40; } }) + 2").unwrap();

    assert_eq!(result, JSValue::Number(42.0));
}

#[test]
fn string_replace_uses_custom_object_to_string() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const selector = { toString() { return ".py-2"; } };
                "&{padding:0.5rem}".replace("&", selector) + ":" +
                    "&{padding:0.5rem}".replace("&", () => selector);
            "#,
        )
        .unwrap();

    assert_eq!(
        result,
        JSValue::String(".py-2{padding:0.5rem}:.py-2{padding:0.5rem}".to_string())
    );
}

#[test]
fn string_constructor_uses_custom_object_to_string() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const selector = { toString() { return ".py-2"; } };
                String(selector);
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::String(".py-2".to_string()));
}

#[test]
fn tagged_template_receives_strings_and_values() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                function tag(strings, value) {
                    return strings[0] + value + strings[1];
                }
                tag`a${2}b`;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::String("a2b".to_string()));
}

#[test]
fn string_raw_can_be_used_as_a_template_tag() {
    let mut engine = JSEngine::new();
    let result = engine.eval(r#"String.raw`\b${2}`"#).unwrap();
    assert_eq!(result, JSValue::String("\\b2".to_string()));
}
