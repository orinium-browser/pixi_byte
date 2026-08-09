use pixi_byte::{JSEngine, JSValue};

#[test]
fn encode_uri_component_uses_utf8_percent_encoding() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(r#"encodeURIComponent("a b/日本!~*'()");"#)
        .unwrap();

    assert_eq!(
        result,
        JSValue::String("a%20b%2F%E6%97%A5%E6%9C%AC!~*'()".to_string())
    );
}
