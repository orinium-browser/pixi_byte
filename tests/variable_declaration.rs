use pixi_byte::JSEngine;

#[test]
fn declares_multiple_bindings_in_one_statement() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval("var first = 1, second = first + 2, third; third = 4; first + second + third;")
        .unwrap();
    assert_eq!(result.to_number(), 8.0);
}

#[test]
fn supports_multiple_let_and_const_bindings() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval("let first = 2, second = 3; const third = 4, fourth = 5; first + second + third + fourth;")
        .unwrap();
    assert_eq!(result.to_number(), 14.0);
}
