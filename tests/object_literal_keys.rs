use pixi_byte::{JSEngine, JSValue};

#[test]
fn object_literals_accept_string_number_and_keyword_keys() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = { "=": "equals", 1: "one", default: "fallback" };
            values["="] + values[1] + values.default;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::String("equalsonefallback".to_string()));
}

#[test]
fn object_literals_accept_shorthand_properties() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const value = 42;
            const target = { value };
            target.value;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Number(42.0));
}

#[test]
fn object_literals_support_getters_and_setters() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const target = {
                _value: 1,
                get value() { return this._value; },
                set value(next) { this._value = next * 2; }
            };
            target.value = 6;
            target.value;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Number(12.0));
}

#[test]
fn object_literals_support_method_shorthand() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const target = {
                value: 4,
                read(extra = 1) { return this.value + extra; }
            };
            target.read(2);
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Number(6.0));
}

#[test]
fn methods_named_get_and_set_are_not_parsed_as_accessors() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const target = {
                get(value) { return value + 1; },
                set(value) { return value + 2; }
            };
            target.get(2) + target.set(3);
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Number(8.0));
}

#[test]
fn object_literals_support_spread_and_computed_keys() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const source = { first: 2 };
            const key = "second";
            const target = { ...source, [key]: 3 };
            target.first + target.second;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Number(5.0));
}
