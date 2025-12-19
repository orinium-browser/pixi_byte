use pixi_byte::{JSEngine, JSValue};

#[test]
fn test_array_literal() {
    let mut engine = JSEngine::new();

    // 基本的な配列リテラル
    let result = engine.eval("[1, 2, 3]").unwrap();
    match result {
        JSValue::Object(_) => {
            // 配列はオブジェクトとして扱われる
            assert!(true);
        }
        _ => panic!("Expected object for array literal"),
    }
}

#[test]
fn test_array_literal_empty() {
    let mut engine = JSEngine::new();
    let result = engine.eval("[]").unwrap();
    match result {
        JSValue::Object(_) => assert!(true),
        _ => panic!("Expected object for empty array"),
    }
}

#[test]
fn test_object_literal() {
    let mut engine = JSEngine::new();

    // 基本的なオブジェクトリテラル
    let result = engine.eval(r#"{ name: "Alice", age: 30 }"#).unwrap();
    match result {
        JSValue::Object(_) => assert!(true),
        _ => panic!("Expected object for object literal"),
    }
}

#[test]
fn test_object_literal_empty() {
    let mut engine = JSEngine::new();
    let result = engine.eval("{}").unwrap();
    match result {
        JSValue::Object(_) => assert!(true),
        _ => panic!("Expected object for empty object"),
    }
}

#[test]
fn test_member_access_dot() {
    let mut engine = JSEngine::new();

    // ドット記法でのプロパティアクセス
    let result = engine
        .eval(
            r#"
        let obj = { x: 10 };
        obj.x
    "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Number(10.0));
}

#[test]
fn test_member_access_bracket() {
    let mut engine = JSEngine::new();

    // ブラケット記法でのプロパティアクセス
    let result = engine
        .eval(
            r#"
        let obj = { name: "Bob" };
        obj["name"]
    "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::String("Bob".to_string()));
}

#[test]
fn test_member_assignment() {
    let mut engine = JSEngine::new();

    // プロパティへの代入
    let result = engine
        .eval(
            r#"
        let obj = {};
        obj.value = 42;
        obj.value
    "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Number(42.0));
}

#[test]
fn test_array_index_access() {
    let mut engine = JSEngine::new();

    // 配列のインデックスアクセス
    let result = engine
        .eval(
            r#"
        let arr = [10, 20, 30];
        arr[1]
    "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Number(20.0));
}

#[test]
fn test_nested_object() {
    let mut engine = JSEngine::new();

    // ネストされたオブジェクト
    let result = engine
        .eval(
            r#"
        let obj = { inner: { value: 100 } };
        obj.inner.value
    "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Number(100.0));
}
