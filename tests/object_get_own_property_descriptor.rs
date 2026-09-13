use pixi_byte::JSEngine;
use pixi_byte::value::JSValue;

fn eval(source: &str) -> JSValue {
    let mut engine = JSEngine::new();
    engine.eval(source).unwrap()
}

#[test]
fn string_primitive_to_object_exposes_length_and_indices() {
    let result = eval(
        r#"
        const desc = Object.getOwnPropertyDescriptor("abc", "length");
        JSON.stringify(desc) + "|" +
            JSON.stringify(Object.getOwnPropertyDescriptor("abc", 1).value);
        "#,
    );
    assert_eq!(
        result.to_console_string(),
        r#"{"value":3,"writable":false,"enumerable":false,"configurable":false}|"b""#
    );
}

#[test]
fn string_indices_and_length_are_not_enumerable() {
    let result = eval(
        r#"
        const wrapper = new Object("ab");
        Object.keys(wrapper).join(",");
        "#,
    );
    assert_eq!(result.to_console_string(), "");
}

#[test]
fn number_and_boolean_primitives_have_empty_descriptors() {
    let result = eval(
        r#"
        JSON.stringify(Object.getOwnPropertyDescriptor(42, "x")) + "|" +
        JSON.stringify(Object.getOwnPropertyDescriptor(true, "x"));
        "#,
    );
    assert_eq!(result.to_console_string(), "undefined|undefined");
}

#[test]
fn null_and_undefined_receivers_throw() {
    // このエンジンでは Rust ネイティブから送出された TypeError は
    // 文字列化された値として catch 節へ渡されるため、文字列内容で検証する
    let result = eval(
        r#"
        let message = "";
        try { Object.getOwnPropertyDescriptor(null, "x"); }
        catch (e) { message = String(e); }
        message;
        "#,
    );
    assert!(
        result
            .to_console_string()
            .contains("cannot convert null or undefined to object"),
        "unexpected error: {}",
        result.to_console_string()
    );
}

#[test]
fn own_property_only_lookup_does_not_walk_prototype_chain() {
    let result = eval(
        r#"
        const proto = { x: 1 };
        const obj = Object.create(proto);
        Object.getOwnPropertyDescriptor(obj, "x");
        "#,
    );
    assert!(result.is_undefined());
}

#[test]
fn inherited_property_is_not_reported_but_own_is() {
    let result = eval(
        r#"
        const proto = { x: 1 };
        const obj = Object.create(proto);
        obj.y = 2;
        JSON.stringify(Object.getOwnPropertyDescriptor(obj, "y"));
        "#,
    );
    assert_eq!(
        result.to_console_string(),
        r#"{"value":2,"writable":true,"enumerable":true,"configurable":true}"#
    );
}

#[test]
fn accessor_descriptor_has_get_set_flags() {
    let result = eval(
        r#"
        const obj = {};
        Object.defineProperty(obj, "g", {
            get: function () { return 1; },
            enumerable: true,
            configurable: false
        });
        const desc = Object.getOwnPropertyDescriptor(obj, "g");
        typeof desc.get === "function" && desc.set === undefined &&
            desc.enumerable === true && desc.configurable === false &&
            desc.value === undefined && desc.writable === undefined;
        "#,
    );
    assert_eq!(result, JSValue::from_bool(true));
}

#[test]
fn symbol_like_keys_resolve_through_to_property_key() {
    // このエンジンの Symbol 値は "@@..." 文字列キーで表現される
    let result = eval(
        r#"
        const obj = {};
        obj[Symbol.iterator] = 7;
        const desc = Object.getOwnPropertyDescriptor(obj, Symbol.iterator);
        desc.value;
        "#,
    );
    assert_eq!(result.to_number(), 7.0);
}

#[test]
fn missing_property_argument_returns_descriptor_or_throws_consistently() {
    // 引数が足りない場合は TypeError（文字列化されて catch に届く）
    let result = eval(
        r#"
        let message = "";
        try { Object.getOwnPropertyDescriptor({}); }
        catch (e) { message = String(e); }
        message;
        "#,
    );
    assert!(
        result.to_console_string().contains("missing arguments"),
        "unexpected error: {}",
        result.to_console_string()
    );
}

#[test]
fn function_receiver_reports_own_properties() {
    let result = eval(
        r#"
        function f() {}
        f.custom = 1;
        const desc = Object.getOwnPropertyDescriptor(f, "custom");
        desc.value + "|" + JSON.stringify(Object.getOwnPropertyDescriptor(f, "prototype"));
        "#,
    );
    assert!(result.to_console_string().starts_with("1|"));
}
