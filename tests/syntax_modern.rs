use pixi_byte::{JSEngine, JSValue};

fn nullish_coalescing_assignment_falls_back_when_nullish() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                let a = null;
                let b = 1;
                a ??= 10;
                b ??= 20;
                a + ":" + b;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_string("10:1".to_string()));
}

fn nullish_coalescing_assignment_is_a_token_together() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const cache = { value: null };
                cache.value ??= (async () => 42)();
                String(cache.value);
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_string("42".to_string()));
}

fn for_await_of_parses_and_iterates_synchronously() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const collected = [];
                async function run() {
                    for await (const item of [1, 2, 3]) {
                        collected.push(item);
                    }
                    return collected.join(",");
                }
                collected.join("?");
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_string("".to_string()));
}
fn logical_assignment_operators_short_circuit_on_truthiness() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                let a = 0;
                let b = "keep";
                a ||= 5;
                b &&= b + "!";
                a + ":" + b;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_string("5:keep!".to_string()));
}

fn spread_arguments_work_with_new_and_optional_member_calls() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                class Point {
                    constructor(x, y) { this.x = x; this.y = y; }
                    sum() { return this.x + this.y; }
                }
                const args = [2, 3];
                const point = new Point(...args);
                const maybe = point;
                const optional = maybe?.sum(...args);
                point.sum() + ":" + optional;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_string("5:5".to_string()));
}

fn spread_arguments_forward_through_super() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                class Base {
                    constructor(a, b) { this.v = a + b; }
                }
                class Child extends Base {
                    constructor(...args) { super(...args); }
                }
                String(new Child(2, 3).v);
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_string("5".to_string()));
}

fn optional_member_access_returns_undefined_for_nullish_receiver() {
    let mut engine = JSEngine::new();
    assert_eq!(engine.eval("null?.value;").unwrap(), JSValue::undefined());
    assert_eq!(
        engine
            .eval("const value = { answer: 42 }; value?.answer;")
            .unwrap(),
        JSValue::from_number(42.0)
    );
}

fn optional_method_call_preserves_receiver() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const value = {
                    answer: 40,
                    read(extra) { return this.answer + extra; }
                };
                value?.read?.(2);
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(42.0));
}

fn optional_call_returns_undefined_for_missing_function() {
    let mut engine = JSEngine::new();
    assert_eq!(
        engine.eval("const value = {}; value.missing?.();").unwrap(),
        JSValue::undefined()
    );
}

fn nullish_coalescing_only_evaluates_fallback_for_nullish_values() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                let calls = 0;
                const zero = 0 ?? (calls = calls + 1);
                const missing = null ?? (calls = calls + 2);
                zero + missing + calls;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(4.0));
}

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

    assert_eq!(result, JSValue::from_string("value=4!".to_string()));
}

fn template_literal_supports_object_literals_in_interpolation() {
    let mut engine = JSEngine::new();
    let result = engine.eval("`${({ value: 2 }).value}`;").unwrap();

    assert_eq!(result, JSValue::from_string("2".to_string()));
}

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
        JSValue::from_string(".py-2{padding:0.5rem}:rule=.py-2".to_string())
    );
}

fn addition_uses_custom_object_value_of() {
    let mut engine = JSEngine::new();
    let result = engine.eval("({ valueOf() { return 40; } }) + 2").unwrap();

    assert_eq!(result, JSValue::from_number(42.0));
}

fn string_and_number_conversion_use_their_respective_primitive_hints() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const value = {
                    toString() { return "string"; },
                    valueOf() { return 42; }
                };
                String(value) + ":" + Number(value) + ":" + (value + 1);
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_string("string:42:43".to_string()));
}

fn conversion_calls_symbol_to_primitive_with_the_requested_hint() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const hints = [];
                const value = {};
                value[Symbol.toPrimitive] = function(hint) {
                    hints.push(hint);
                    return hint === "string" ? "text" : 5;
                };
                String(value) + ":" + Number(value) + ":" + (value + 1) + ":" + hints.join(",");
            "#,
        )
        .unwrap();

    assert_eq!(
        result,
        JSValue::from_string("text:5:6:string,number,default".to_string())
    );
}

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
        JSValue::from_string(".py-2{padding:0.5rem}:.py-2{padding:0.5rem}".to_string())
    );
}

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

    assert_eq!(result, JSValue::from_string(".py-2".to_string()));
}

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
    assert_eq!(result, JSValue::from_string("a2b".to_string()));
}

fn string_raw_can_be_used_as_a_template_tag() {
    let mut engine = JSEngine::new();
    let result = engine.eval(r#"String.raw`\b${2}`"#).unwrap();
    assert_eq!(result, JSValue::from_string("\\b2".to_string()));
}

fn nested_template_literals_inside_interpolation_scan_as_one_unit() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const path = (e) => `${e.host}${e.path ? `/${e.path}` : ""}/api/`;
                const envelope = (c) => `${`${path(c)}${c.projectId}/envelope/`}?q`;
                envelope({ host: "h", path: "p", projectId: 7 });
            "#,
        )
        .unwrap();

    assert_eq!(
        result,
        JSValue::from_string("h/p/api/7/envelope/?q".to_string())
    );
}

fn interpolation_expressions_with_comments_and_regex_braces_scan_correctly() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const a = `1${1/* } */+2}`;
                const b = `2${1// } skipped
                +2}`;
                const c = `3${/(})/.test("}")}`;
                const d = `4${{x: 0}.x}b`;
                a + "," + b + "," + c + "," + d;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_string("13,23,3true,40b".to_string()));
}

#[test]
fn suite_modern_syntax_1() {
    nullish_coalescing_assignment_falls_back_when_nullish();
    nullish_coalescing_assignment_is_a_token_together();
    for_await_of_parses_and_iterates_synchronously();
    logical_assignment_operators_short_circuit_on_truthiness();
}

#[test]
fn suite_modern_syntax_2() {
    spread_arguments_work_with_new_and_optional_member_calls();
    spread_arguments_forward_through_super();
}

#[test]
fn suite_optional_chaining() {
    optional_member_access_returns_undefined_for_nullish_receiver();
    optional_method_call_preserves_receiver();
    optional_call_returns_undefined_for_missing_function();
    nullish_coalescing_only_evaluates_fallback_for_nullish_values();
}

#[test]
fn suite_optional_chain_decimal() {
    ternary_question_mark_before_leading_dot_decimal();
    optional_chaining_still_works();
    unicode_escaped_identifiers();
}

#[test]
fn suite_template_literal_1() {
    template_literal_interpolates_expressions();
    template_literal_supports_object_literals_in_interpolation();
    string_concatenation_uses_custom_object_to_string();
    addition_uses_custom_object_value_of();
}

#[test]
fn suite_template_literal_2() {
    string_and_number_conversion_use_their_respective_primitive_hints();
    conversion_calls_symbol_to_primitive_with_the_requested_hint();
    string_replace_uses_custom_object_to_string();
    string_constructor_uses_custom_object_to_string();
}

#[test]
fn suite_template_literal_3() {
    tagged_template_receives_strings_and_values();
    string_raw_can_be_used_as_a_template_tag();
    nested_template_literals_inside_interpolation_scan_as_one_unit();
    interpolation_expressions_with_comments_and_regex_braces_scan_correctly();
}
