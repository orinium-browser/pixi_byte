use pixi_byte::{JSEngine, JSValue};
#[test]
fn test_vm_execute_literal() {
    let mut engine = JSEngine::new();
    let result = engine.eval("42").unwrap();
    assert_eq!(result, JSValue::from_number(42.0));
}
#[test]
fn test_vm_execute_addition() {
    let mut engine = JSEngine::new();
    let result = engine.eval("1 + 2").unwrap();
    assert_eq!(result, JSValue::from_number(3.0));
}
#[test]
fn test_vm_execute_string_concat() {
    let mut engine = JSEngine::new();
    let source = r#""hello" + " " + "world""#;
    let result = engine.eval(source).unwrap();
    assert_eq!(result, JSValue::from_string("hello world".to_string()));
}
#[test]
fn test_vm_execute_comparison() {
    let mut engine = JSEngine::new();
    let result = engine.eval("5 > 3").unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}
#[test]
fn test_vm_execute_variable() {
    let mut engine = JSEngine::new();
    let result = engine.eval("let x = 10; x + 5").unwrap();
    assert_eq!(result, JSValue::from_number(15.0));
}

#[test]
fn test_vm_execute_add_function() {
    let source = r#"
        function add(a,b) { return a + b; }

        add(098,2)
        "#;
    let mut engine = JSEngine::new();
    let result = engine.eval(source).unwrap();
    assert_eq!(result, JSValue::from_number(100.0));
}
