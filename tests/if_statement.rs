use pixi_byte::JSEngine;

#[test]
fn executes_if_and_else_branches() {
    let mut engine = JSEngine::new();

    let truthy = engine
        .eval("let truthy = 0; if (true) { truthy = 1; } else { truthy = 2; } truthy;")
        .unwrap();
    assert_eq!(truthy.to_number(), 1.0);

    let falsy = engine
        .eval("let falsy = 0; if (false) { falsy = 1; } else { falsy = 2; } falsy;")
        .unwrap();
    assert_eq!(falsy.to_number(), 2.0);
}

#[test]
fn supports_else_if_and_single_statement_bodies() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval("let value = 0; if (false) value = 1; else if (true) value = 2; value;")
        .unwrap();
    assert_eq!(result.to_number(), 2.0);
}

#[test]
fn returns_from_inside_an_if_statement() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval("function choose(value) { if (value) { return 10; } return 20; } choose(true);")
        .unwrap();
    assert_eq!(result.to_number(), 10.0);
}
