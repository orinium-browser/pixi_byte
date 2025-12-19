use pixi_byte::{JSEngine, JSValue};

#[test]
fn test_simple_function() {
    let mut engine = JSEngine::new();
    let result = engine.eval(r#"
        function f() { return 5; }
        f();
    "#).unwrap();
    assert_eq!(result, JSValue::Number(5.0));
}

#[test]
fn test_function_with_args() {
    let mut engine = JSEngine::new();
    let result = engine.eval(r#"
        function add(a, b) { return a + b; }
        add(2, 3);
    "#).unwrap();
    assert_eq!(result, JSValue::Number(5.0));
}

