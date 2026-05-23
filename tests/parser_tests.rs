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
