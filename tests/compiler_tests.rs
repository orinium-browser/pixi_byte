use pixi_byte::{Lexer, Parser, Compiler, Opcode};

#[test]
fn test_compile_literal() {
    let mut lexer = Lexer::new("42");
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    let mut compiler = Compiler::new();
    let chunk = compiler.compile(program).unwrap();

    assert!(!chunk.code.is_empty());
}

#[test]
fn test_compile_binary_expr() {
    let mut lexer = Lexer::new("1 + 2");
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    let mut compiler = Compiler::new();
    let chunk = compiler.compile(program).unwrap();

    assert!(chunk.code.contains(&Opcode::Add));
}

