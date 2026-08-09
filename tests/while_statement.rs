use pixi_byte::JSEngine;

#[test]
fn executes_while_loop_with_updates() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval("let index = 0; let total = 0; while (index < 5) { total += index; index++; } total;")
        .unwrap();
    assert_eq!(result.to_number(), 10.0);
}

#[test]
fn break_exits_and_continue_restarts_the_loop() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval(
            "let index = 0; let total = 0; while (index < 10) { index++; if (index === 2) { continue; } if (index === 5) { break; } total += index; } total;",
        )
        .unwrap();
    assert_eq!(result.to_number(), 8.0);
}

#[test]
fn supports_single_statement_while_body() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval("let index = 0; while (index < 3) index++; index;")
        .unwrap();
    assert_eq!(result.to_number(), 3.0);
}
