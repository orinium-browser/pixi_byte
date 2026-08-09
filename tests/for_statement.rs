use pixi_byte::JSEngine;

#[test]
fn executes_classic_for_loop() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval("let total = 0; for (let index = 0; index < 5; index++) { total += index; } total;")
        .unwrap();
    assert_eq!(result.to_number(), 10.0);
}

#[test]
fn accepts_multiple_initializer_bindings() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval("let total = 0; for (var left = 0, right = 4; left < right; left++, right--) { total += left + right; } total;")
        .unwrap();
    assert_eq!(result.to_number(), 8.0);
}

#[test]
fn continue_runs_update_and_break_exits() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval(
            "let total = 0; for (let index = 0; index < 10; index++) { if (index === 1) { continue; } if (index === 4) { break; } total += index; } total;",
        )
        .unwrap();
    assert_eq!(result.to_number(), 5.0);
}

#[test]
fn supports_expression_initializer_and_single_statement_body() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval("let index = 0; let total = 0; for (index = 1; index < 4; index++) total += index; total;")
        .unwrap();
    assert_eq!(result.to_number(), 6.0);
}

#[test]
fn supports_an_empty_statement_as_the_loop_body() {
    let mut engine = JSEngine::new();
    let result = engine.eval("let i = 0; for (; i < 3; i++); i;").unwrap();
    assert_eq!(result.to_number(), 3.0);
}
