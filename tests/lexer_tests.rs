use pixi_byte::{Lexer, TokenKind};

#[test]
fn test_tokenize_numbers() {
    let mut lexer = Lexer::new("123 45.67 .89");
    let tokens = lexer.tokenize().unwrap();
    let kinds: Vec<TokenKind> = tokens
        .iter()
        .map(|t| t.kind.clone())
        .into_iter()
        .filter(|k| *k != TokenKind::Eof)
        .collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::NumberLiteral("123".to_string()),
            TokenKind::NumberLiteral("45.67".to_string()),
            TokenKind::NumberLiteral(".89".to_string()),
        ]
    );
}

#[test]
fn test_weird_number_digits() {
    let mut lexer = Lexer::new(".0.1 0.2.1");
    let tokens = lexer.tokenize().unwrap();
    let kinds: Vec<TokenKind> = tokens
        .iter()
        .map(|t| t.kind.clone())
        .into_iter()
        .filter(|k| *k != TokenKind::Eof)
        .collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::NumberLiteral(".0".to_string()),
            TokenKind::Dot,
            TokenKind::NumberLiteral("1".to_string()),
            TokenKind::NumberLiteral("0.2".to_string()),
            TokenKind::Dot,
            TokenKind::NumberLiteral("1".to_string()),
        ]
    );
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
