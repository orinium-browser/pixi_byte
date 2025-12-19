use pixi_byte::{Lexer, TokenKind};

#[test]
fn test_tokenize_numbers() {
    let mut lexer = Lexer::new("123 45.67 .89");
    let tokens = lexer.tokenize().unwrap();

    assert!(matches!(tokens[0].kind, TokenKind::Number(123.0)));
    assert!(matches!(tokens[1].kind, TokenKind::Number(45.67)));
    assert!(matches!(tokens[2].kind, TokenKind::Number(0.89)));
}

#[test]
fn test_weird_number_digits() {
    let mut lexer = Lexer::new(".0.1 0.2.1");
    let tokens = lexer.tokenize().unwrap();

    assert!(matches!(tokens[0].kind, TokenKind::Number(0.0)));
    assert!(matches!(tokens[1].kind, TokenKind::Dot));
    assert!(matches!(tokens[2].kind, TokenKind::Number(1.0)));
    assert!(matches!(tokens[3].kind, TokenKind::Number(0.2)));
    assert!(matches!(tokens[4].kind, TokenKind::Dot));
    assert!(matches!(tokens[5].kind, TokenKind::Number(1.0)));
}

#[test]
fn test_tokenize_operators() {
    let mut lexer = Lexer::new("+ - * / % ** ++ --");
    let tokens = lexer.tokenize().unwrap();

    assert_eq!(tokens[0].kind, TokenKind::Plus);
    assert_eq!(tokens[1].kind, TokenKind::Minus);
    assert_eq!(tokens[2].kind, TokenKind::Star);
    assert_eq!(tokens[3].kind, TokenKind::Slash);
    assert_eq!(tokens[4].kind, TokenKind::Percent);
    assert_eq!(tokens[5].kind, TokenKind::Power);
    assert_eq!(tokens[6].kind, TokenKind::PlusPlus);
    assert_eq!(tokens[7].kind, TokenKind::MinusMinus);
}

#[test]
fn test_tokenize_keywords() {
    let mut lexer = Lexer::new("let const var function return");
    let tokens = lexer.tokenize().unwrap();

    assert_eq!(tokens[0].kind, TokenKind::Let);
    assert_eq!(tokens[1].kind, TokenKind::Const);
    assert_eq!(tokens[2].kind, TokenKind::Var);
    assert_eq!(tokens[3].kind, TokenKind::Function);
    assert_eq!(tokens[4].kind, TokenKind::Return);
}

#[test]
fn test_tokenize_strings() {
    let mut lexer = Lexer::new(r#""hello" 'world'"#);
    let tokens = lexer.tokenize().unwrap();

    assert!(matches!(tokens[0].kind, TokenKind::String(ref s) if s == "hello"));
    assert!(matches!(tokens[1].kind, TokenKind::String(ref s) if s == "world"));
}

#[test]
fn test_tokenize_identifiers() {
    let mut lexer = Lexer::new("foo bar123 _test $value");
    let tokens = lexer.tokenize().unwrap();

    assert!(matches!(tokens[0].kind, TokenKind::Identifier(ref s) if s == "foo"));
    assert!(matches!(tokens[1].kind, TokenKind::Identifier(ref s) if s == "bar123"));
    assert!(matches!(tokens[2].kind, TokenKind::Identifier(ref s) if s == "_test"));
    assert!(matches!(tokens[3].kind, TokenKind::Identifier(ref s) if s == "$value"));
}
