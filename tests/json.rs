use pixi_byte::{JSEngine, JSValue};

#[test]
fn json_stringify_quotes_strings_for_selectors() {
    let mut engine = JSEngine::new();
    let result = engine.eval(r#"JSON.stringify("a\"b")"#).unwrap();
    assert_eq!(result, JSValue::from_string(r#""a\"b""#.to_string()));
}

#[test]
fn json_stringify_serializes_arrays_and_objects() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(r#"JSON.stringify({ value: 1, items: [true, null, "x"] })"#)
        .unwrap();
    let Some(value) = result.as_string() else {
        panic!("JSON.stringify must return a string");
    };
    assert!(value.starts_with('{'));
    assert!(value.ends_with('}'));
    assert!(value.contains(r#""value":1"#));
    assert!(value.contains(r#""items":[true,null,"x"]"#));
}

#[test]
fn json_parse_builds_nested_values_and_decodes_escapes() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const value = JSON.parse('{"items":[1,true,null,{"name":"Scratch\\n日本"}]}');
            value.items.length + ":" + value.items[0] + ":" +
                value.items[1] + ":" + (value.items[2] === null) + ":" +
                value.items[3].name;
            "#,
        )
        .unwrap();
    assert_eq!(
        result,
        JSValue::from_string("4:1:true:true:Scratch\n日本".to_string())
    );
}

#[test]
fn json_parse_handles_unicode_pairs_and_strict_number_grammar() {
    let mut engine = JSEngine::new();
    assert_eq!(
        engine.eval(r#"JSON.parse('"\\ud83d\\ude00"')"#).unwrap(),
        JSValue::from_string("😀".to_string())
    );
    assert!(engine.eval("JSON.parse('01')").is_err());
    assert!(engine.eval("JSON.parse('1.')").is_err());
}
