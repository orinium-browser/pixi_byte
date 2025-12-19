use pixi_byte::JSEngine;

#[test]
fn test_basic_arithmetic() {
    let mut engine = JSEngine::new();

    let result = engine.eval("2 + 3").unwrap();
    assert_eq!(result.to_number(), 5.0);

    let result = engine.eval("10 - 4").unwrap();
    assert_eq!(result.to_number(), 6.0);

    let result = engine.eval("3 * 4").unwrap();
    assert_eq!(result.to_number(), 12.0);

    let result = engine.eval("15 / 3").unwrap();
    assert_eq!(result.to_number(), 5.0);
}

#[test]
fn test_complex_expressions() {
    let mut engine = JSEngine::new();

    let result = engine.eval("(1 + 2) * 3").unwrap();
    assert_eq!(result.to_number(), 9.0);

    let result = engine.eval("10 / 2 + 3 * 4").unwrap();
    assert_eq!(result.to_number(), 17.0);
}

#[test]
fn test_variables() {
    let mut engine = JSEngine::new();

    engine.eval("let x = 10").unwrap();
    let result = engine.eval("x + 5").unwrap();
    assert_eq!(result.to_number(), 15.0);

    engine.eval("let y = 20").unwrap();
    let result = engine.eval("x + y").unwrap();
    assert_eq!(result.to_number(), 30.0);
}

#[test]
fn test_string_operations() {
    let mut engine = JSEngine::new();

    let result = engine.eval(r#""hello" + " " + "world""#).unwrap();
    assert_eq!(result.to_string(), "hello world");

    let result = engine.eval(r#""value: " + 42"#).unwrap();
    assert_eq!(result.to_string(), "value: 42");
}

#[test]
fn test_boolean_operations() {
    let mut engine = JSEngine::new();

    let result = engine.eval("true && true").unwrap();
    assert_eq!(result.to_boolean(), true);

    let result = engine.eval("true && false").unwrap();
    assert_eq!(result.to_boolean(), false);

    let result = engine.eval("false || true").unwrap();
    assert_eq!(result.to_boolean(), true);

    let result = engine.eval("!true").unwrap();
    assert_eq!(result.to_boolean(), false);
}

#[test]
fn test_comparisons() {
    let mut engine = JSEngine::new();

    let result = engine.eval("5 > 3").unwrap();
    assert_eq!(result.to_boolean(), true);

    let result = engine.eval("2 < 1").unwrap();
    assert_eq!(result.to_boolean(), false);

    let result = engine.eval("5 === 5").unwrap();
    assert_eq!(result.to_boolean(), true);

    let result = engine.eval("5 === '5'").unwrap();
    assert_eq!(result.to_boolean(), false);

    let result = engine.eval("5 == '5'").unwrap();
    assert_eq!(result.to_boolean(), true);
}

#[test]
fn test_typeof() {
    let mut engine = JSEngine::new();

    let result = engine.eval("typeof 42").unwrap();
    assert_eq!(result.to_string(), "number");

    let result = engine.eval(r#"typeof "hello""#).unwrap();
    assert_eq!(result.to_string(), "string");

    let result = engine.eval("typeof true").unwrap();
    assert_eq!(result.to_string(), "boolean");

    let result = engine.eval("typeof undefined").unwrap();
    assert_eq!(result.to_string(), "undefined");
}
