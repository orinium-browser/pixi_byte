use pixi_byte::JSEngine;

#[test]
fn prefix_and_postfix_updates_preserve_their_result_values() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval("let value = 1; let old = value++; let current = ++value; old * 100 + current;")
        .unwrap();
    assert_eq!(result.to_number(), 103.0);
}

#[test]
fn decrement_updates_identifier_bindings() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval("let value = 3; value--; --value; value;")
        .unwrap();
    assert_eq!(result.to_number(), 1.0);
}

#[test]
fn compound_assignments_use_the_corresponding_operator() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval("let value = 5; value += 3; value *= 2; value -= 4; value /= 2; value %= 5; value;")
        .unwrap();
    assert_eq!(result.to_number(), 1.0);
}
