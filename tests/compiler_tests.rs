use pixi_byte::{Compiler, Lexer, Opcode, Parser};

#[test]
fn test_compile_literal() {
    let lexer = Lexer::new("42");
    let mut parser = Parser::new(lexer).unwrap();
    let program = parser.parse().unwrap();

    let compiler = Compiler::new();
    let chunk = compiler.compile(program).unwrap();

    assert!(!chunk.code.is_empty());
}

#[test]
fn test_compile_binary_expr() {
    let lexer = Lexer::new("1 + 2");
    let mut parser = Parser::new(lexer).unwrap();
    let program = parser.parse().unwrap();

    let compiler = Compiler::new();
    let chunk = compiler.compile(program).unwrap();

    assert!(chunk.code.contains(&Opcode::Add));
}
