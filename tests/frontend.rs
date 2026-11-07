use pixi_byte::{Compiler, Lexer, Opcode, Parser, TokenKind};

fn test_tokenize_numbers() {
    let lexer = Lexer::new("123 45.67 .89");
    let kinds: Vec<TokenKind> = lexer
        .iter()
        .map(|v| v.unwrap())
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

fn test_weird_number_digits() {
    let lexer = Lexer::new(".0.1 0.2.1");
    let kinds: Vec<TokenKind> = lexer
        .iter()
        .map(|v| v.unwrap())
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

fn test_tokenize_operators() {
    let lexer = Lexer::new("a + b - c * d / e % f ** g; h++; --i");
    let operators = lexer
        .iter()
        .map(|token| token.unwrap().kind)
        .filter(|kind| {
            !matches!(
                kind,
                TokenKind::Identifier(_) | TokenKind::Semicolon | TokenKind::Eof
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        operators,
        vec![
            TokenKind::Plus,
            TokenKind::Minus,
            TokenKind::Star,
            TokenKind::Slash,
            TokenKind::Percent,
            TokenKind::Power,
            TokenKind::PlusPlus,
            TokenKind::MinusMinus,
        ]
    );
}

fn non_decimal_integer_literals_are_tokenized_as_single_numbers() {
    let kinds: Vec<TokenKind> = Lexer::new("0x10 0o10 0b10 0x10n")
        .iter()
        .map(|token| token.unwrap().kind)
        .filter(|kind| *kind != TokenKind::Eof)
        .collect();

    assert_eq!(
        kinds,
        vec![
            TokenKind::NumberLiteral("16".to_string()),
            TokenKind::NumberLiteral("8".to_string()),
            TokenKind::NumberLiteral("2".to_string()),
            TokenKind::BigIntLiteral("16".to_string()),
        ]
    );
}

fn test_tokenize_keywords() {
    let lexer = Lexer::new("let const var function return");
    let tokens = lexer.iter().map(|v| v.unwrap()).collect::<Vec<_>>();

    assert_eq!(tokens[0].kind, TokenKind::Let);
    assert_eq!(tokens[1].kind, TokenKind::Const);
    assert_eq!(tokens[2].kind, TokenKind::Var);
    assert_eq!(tokens[3].kind, TokenKind::Function);
    assert_eq!(tokens[4].kind, TokenKind::Return);
}

fn test_tokenize_strings() {
    let lexer = Lexer::new(r#""hello" 'world'"#);
    let tokens = lexer.iter().map(|v| v.unwrap()).collect::<Vec<_>>();

    assert!(matches!(tokens[0].kind, TokenKind::String(ref s) if s == "hello"));
    assert!(matches!(tokens[1].kind, TokenKind::String(ref s) if s == "world"));
}

fn test_tokenize_string_escape_sequences() {
    let lexer = Lexer::new(r#""\x3cscript\x3e \u65E5\u672C\b\f\v\0""#);
    let tokens = lexer.iter().map(|value| value.unwrap()).collect::<Vec<_>>();

    assert!(matches!(
        tokens[0].kind,
        TokenKind::String(ref value)
            if value == "<script> 日本\u{0008}\u{000C}\u{000B}\0"
    ));
}

fn invalid_hexadecimal_string_escape_is_a_syntax_error() {
    let mut lexer = Lexer::new(r#""\xZZ""#);
    assert!(lexer.next().unwrap().is_err());
}

fn test_tokenize_identifiers() {
    let lexer = Lexer::new("foo bar123 _test $value");
    let tokens = lexer.iter().map(|v| v.unwrap()).collect::<Vec<_>>();

    assert!(matches!(tokens[0].kind, TokenKind::Identifier(ref s) if s == "foo"));
    assert!(matches!(tokens[1].kind, TokenKind::Identifier(ref s) if s == "bar123"));
    assert!(matches!(tokens[2].kind, TokenKind::Identifier(ref s) if s == "_test"));
    assert!(matches!(tokens[3].kind, TokenKind::Identifier(ref s) if s == "$value"));
}

fn test_single_line_comment() {
    let lexer = Lexer::new(
        r#"
        // this is a comment
        let a = 1;
    "#,
    );

    let tokens = lexer.iter().map(|v| v.unwrap()).collect::<Vec<_>>();

    assert!(matches!(tokens[0].kind, TokenKind::Let));
    assert!(matches!(tokens[1].kind, TokenKind::Identifier(ref s) if s == "a"));
    assert!(matches!(tokens[2].kind, TokenKind::Eq));
    assert!(
        matches!(tokens[3].kind, TokenKind::NumberLiteral(ref n) if n.parse::<i32>().unwrap() == 1)
    );
}

fn test_parse_literal() {
    let lexer = Lexer::new("42");
    let mut parser = Parser::new(lexer).unwrap();
    let program = parser.parse().unwrap();

    assert_eq!(program.body.len(), 1);
}

fn test_parse_binary_expr() {
    let lexer = Lexer::new("1 + 2");
    let mut parser = Parser::new(lexer).unwrap();
    let program = parser.parse().unwrap();

    assert_eq!(program.body.len(), 1);
}

fn test_parse_var_declaration() {
    let lexer = Lexer::new("let x = 10");
    let mut parser = Parser::new(lexer).unwrap();
    let program = parser.parse().unwrap();

    assert_eq!(program.body.len(), 1);
}

fn test_keyword_after_dot_is_an_identifier_name() {
    let lexer = Lexer::new("promise.catch(handler)");
    let mut parser = Parser::new(lexer).unwrap();
    let program = parser.parse().unwrap();

    assert_eq!(program.body.len(), 1);
}

fn contextual_keywords_are_allowed_as_function_names() {
    let mut engine = pixi_byte::JSEngine::new();
    assert_eq!(
        engine
            .eval("function of(value) { return value; } of(42);")
            .unwrap(),
        pixi_byte::JSValue::from_number(42.0)
    );
}

fn test_compile_literal() {
    let lexer = Lexer::new("42");
    let mut parser = Parser::new(lexer).unwrap();
    let program = parser.parse().unwrap();

    let compiler = Compiler::new();
    let chunk = compiler.compile(program).unwrap();

    assert!(!chunk.code.is_empty());
}

fn test_compile_binary_expr() {
    let lexer = Lexer::new("1 + 2");
    let mut parser = Parser::new(lexer).unwrap();
    let program = parser.parse().unwrap();

    let compiler = Compiler::new();
    let chunk = compiler.compile(program).unwrap();

    assert!(chunk.code.contains(&Opcode::Add));
}

#[test]
fn suite_lexer_tests_1() {
    test_tokenize_numbers();
    test_weird_number_digits();
    test_tokenize_operators();
    non_decimal_integer_literals_are_tokenized_as_single_numbers();
}

#[test]
fn suite_lexer_tests_2() {
    test_tokenize_keywords();
    test_tokenize_strings();
    test_tokenize_string_escape_sequences();
    invalid_hexadecimal_string_escape_is_a_syntax_error();
}

#[test]
fn suite_lexer_tests_3() {
    test_tokenize_identifiers();
    test_single_line_comment();
}

#[test]
fn suite_parser_tests_1() {
    test_parse_literal();
    test_parse_binary_expr();
    test_parse_var_declaration();
    test_keyword_after_dot_is_an_identifier_name();
}

#[test]
fn suite_parser_tests_2() {
    contextual_keywords_are_allowed_as_function_names();
}

#[test]
fn suite_compiler_tests() {
    test_compile_literal();
    test_compile_binary_expr();
}
