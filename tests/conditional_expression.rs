use pixi_byte::JSEngine;

#[test]
fn evaluates_only_the_selected_conditional_branch() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval("let side = 0; let value = true ? (side = 1) : (side = 2); value * 10 + side;")
        .unwrap();
    assert_eq!(result.to_number(), 11.0);
}

#[test]
fn conditional_expression_is_right_associative() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval("let first = false, second = true; first ? 1 : second ? 2 : 3;")
        .unwrap();
    assert_eq!(result.to_number(), 2.0);
}

#[test]
fn conditional_expression_can_feed_an_assignment() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval("let enabled = false; let value = enabled ? 'on' : 'off'; value;")
        .unwrap();
    assert_eq!(result.to_string(), "off");
}
