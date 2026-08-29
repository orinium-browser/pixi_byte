use pixi_byte::{Lexer, Parser};

#[test]
fn test_parse_literal() {
    let lexer = Lexer::new("42");
    let mut parser = Parser::new(lexer).unwrap();
    let program = parser.parse().unwrap();

    assert_eq!(program.body.len(), 1);
}

#[test]
fn test_parse_binary_expr() {
    let lexer = Lexer::new("1 + 2");
    let mut parser = Parser::new(lexer).unwrap();
    let program = parser.parse().unwrap();

    assert_eq!(program.body.len(), 1);
}

#[test]
fn test_parse_var_declaration() {
    let lexer = Lexer::new("let x = 10");
    let mut parser = Parser::new(lexer).unwrap();
    let program = parser.parse().unwrap();

    assert_eq!(program.body.len(), 1);
}

#[test]
fn test_keyword_after_dot_is_an_identifier_name() {
    let lexer = Lexer::new("promise.catch(handler)");
    let mut parser = Parser::new(lexer).unwrap();
    let program = parser.parse().unwrap();

    assert_eq!(program.body.len(), 1);
}

#[test]
fn contextual_keywords_are_allowed_as_function_names() {
    let mut engine = pixi_byte::JSEngine::new();
    assert_eq!(
        engine
            .eval("function of(value) { return value; } of(42);")
            .unwrap(),
        pixi_byte::JSValue::from_number(42.0)
    );
}
