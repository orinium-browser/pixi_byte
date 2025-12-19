use pixi_byte::{Lexer, Parser};

#[test]
fn test_parse_literal() {
    let mut lexer = Lexer::new("42");
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    assert_eq!(program.body.len(), 1);
}

#[test]
fn test_parse_binary_expr() {
    let mut lexer = Lexer::new("1 + 2");
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    assert_eq!(program.body.len(), 1);
}

#[test]
fn test_parse_var_declaration() {
    let mut lexer = Lexer::new("let x = 10");
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    assert_eq!(program.body.len(), 1);
}
