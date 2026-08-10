use super::token::{Span, Token, TokenKind};
use crate::error::{JSError, JSResult};

/// 字句解析器
#[derive(Clone)]
pub struct Lexer {
    /// ソースコードの文字列
    source: Vec<char>,
    /// 現在の位置
    position: usize,
    /// 現在の行番号
    line: usize,
    /// 現在の列番号
    column: usize,
    /// 直前の有意なトークン。正規表現リテラルと除算の判別に使う。
    previous: Option<TokenKind>,
}

impl Lexer {
    /// 新しい字句解析器を作成
    pub fn new(source: &str) -> Self {
        Self {
            source: source.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
            previous: None,
        }
    }

    pub fn iter(self) -> Self {
        self
    }

    pub fn eof_token(&self) -> Token {
        Token::new(TokenKind::Eof, self.current_span())
    }
}

impl Iterator for Lexer {
    type Item = JSResult<Token>;
    fn next(&mut self) -> Option<Self::Item> {
        self.skip_whitespace();

        if self.is_at_end() {
            None
        } else {
            let token_result = self.next_token();
            if let Ok(token) = &token_result {
                self.previous = Some(token.kind.clone());
            }
            if token_result
                .as_ref()
                .is_ok_and(|token| token.kind == TokenKind::Eof)
            {
                None
            } else {
                Some(token_result)
            }
        }
    }
}

impl Lexer {
    /// 次のトークンを取得
    fn next_token(&mut self) -> JSResult<Token> {
        if self.is_at_end() {
            return Ok(self.eof_token());
        }

        let start = self.position;
        let start_line = self.line;
        let start_column = self.column;

        let ch = self.advance();

        let kind = match ch {
            // 1文字トークン
            '(' => TokenKind::LeftParen,
            ')' => TokenKind::RightParen,
            '{' => TokenKind::LeftBrace,
            '}' => TokenKind::RightBrace,
            '[' => TokenKind::LeftBracket,
            ']' => TokenKind::RightBracket,
            ';' => TokenKind::Semicolon,
            ',' => TokenKind::Comma,
            '~' => TokenKind::BitNot,
            '?' => {
                if self.match_char('.') {
                    TokenKind::OptionalChain
                } else if self.match_char('?') {
                    TokenKind::Nullish
                } else {
                    TokenKind::Question
                }
            }
            ':' => TokenKind::Colon,

            // ドット
            '.' => {
                if self.peek() == Some('.') && self.peek_ahead(1) == Some('.') {
                    self.advance();
                    self.advance();
                    TokenKind::DotDotDot
                } else if start > 0 && self.source[start - 1].is_ascii_digit() {
                    TokenKind::Dot
                } else if self.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    return self.scan_number();
                } else {
                    TokenKind::Dot
                }
            }

            // 演算子
            '+' => {
                if self.match_char('=') {
                    TokenKind::PlusEq
                } else if self.match_char('+') {
                    TokenKind::PlusPlus
                } else {
                    TokenKind::Plus
                }
            }
            '-' => {
                if self.match_char('=') {
                    TokenKind::MinusEq
                } else if self.match_char('-') {
                    TokenKind::MinusMinus
                } else {
                    TokenKind::Minus
                }
            }
            '*' => {
                if self.match_char('*') {
                    TokenKind::Power
                } else if self.match_char('=') {
                    TokenKind::StarEq
                } else {
                    TokenKind::Star
                }
            }
            '/' => {
                if self.match_char('/') {
                    // 行コメント
                    self.skip_line_comment();
                    return self.next_token();
                } else if self.match_char('*') {
                    // ブロックコメント
                    self.skip_block_comment()?;
                    return self.next_token();
                } else if self.match_char('=') {
                    TokenKind::SlashEq
                } else if self.can_start_regular_expression() {
                    return self.scan_regular_expression();
                } else {
                    TokenKind::Slash
                }
            }
            '%' => {
                if self.match_char('=') {
                    TokenKind::PercentEq
                } else {
                    TokenKind::Percent
                }
            }

            // 比較・等価演算子
            '=' => {
                if self.match_char('=') {
                    if self.match_char('=') {
                        TokenKind::EqEqEq
                    } else {
                        TokenKind::EqEq
                    }
                } else if self.match_char('>') {
                    TokenKind::Arrow
                } else {
                    TokenKind::Eq
                }
            }
            '!' => {
                if self.match_char('=') {
                    if self.match_char('=') {
                        TokenKind::NotEqEq
                    } else {
                        TokenKind::NotEq
                    }
                } else {
                    TokenKind::Not
                }
            }
            '<' => {
                if self.match_char('<') {
                    if self.match_char('=') {
                        TokenKind::LeftShiftEq
                    } else {
                        TokenKind::LeftShift
                    }
                } else if self.match_char('=') {
                    TokenKind::LtEq
                } else {
                    TokenKind::Lt
                }
            }
            '>' => {
                if self.match_char('>') {
                    if self.match_char('>') {
                        if self.match_char('=') {
                            TokenKind::UnsignedRightShiftEq
                        } else {
                            TokenKind::UnsignedRightShift
                        }
                    } else if self.match_char('=') {
                        TokenKind::RightShiftEq
                    } else {
                        TokenKind::RightShift
                    }
                } else if self.match_char('=') {
                    TokenKind::GtEq
                } else {
                    TokenKind::Gt
                }
            }

            // 論理・ビット演算子
            '&' => {
                if self.match_char('&') {
                    TokenKind::And
                } else if self.match_char('=') {
                    TokenKind::BitAndEq
                } else {
                    TokenKind::BitAnd
                }
            }
            '|' => {
                if self.match_char('|') {
                    TokenKind::Or
                } else if self.match_char('=') {
                    TokenKind::BitOrEq
                } else {
                    TokenKind::BitOr
                }
            }
            '^' => {
                if self.match_char('=') {
                    TokenKind::BitXorEq
                } else {
                    TokenKind::BitXor
                }
            }

            // 文字列リテラル
            '"' | '\'' => return self.scan_string(ch),
            '`' => return self.scan_template_literal(),

            // 数値リテラル
            '0'..='9' => return self.scan_number(),

            // 識別子・キーワード
            _ if ch.is_alphabetic() || ch == '_' || ch == '$' => {
                return self.scan_identifier();
            }

            _ => TokenKind::Unknown(ch),
        };

        let span = Span::new(start, self.position, start_line, start_column);
        Ok(Token::new(kind, span))
    }

    /// 数値リテラルのスキャン
    fn scan_number(&mut self) -> JSResult<Token> {
        let start = self.position - 1;
        let start_line = self.line;
        let start_column = self.column - 1;

        let start_from_dot = self.source[start] == '.';

        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }

        // 小数点
        if self.peek() == Some('.')
            && !start_from_dot
            && self
                .peek_ahead(1)
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
        {
            self.advance(); // '.'
            while let Some(ch) = self.peek() {
                if ch.is_ascii_digit() {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        // 指数表記
        if let Some('e') | Some('E') = self.peek() {
            self.advance();
            if let Some('+') | Some('-') = self.peek() {
                self.advance();
            }
            while let Some(ch) = self.peek() {
                if ch.is_ascii_digit() {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        let text: String = self.source[start..self.position].iter().collect();
        if text.parse::<f64>().is_err() {
            return Err(JSError::SyntaxError(
                format!("Invalid number literal: {}", text),
                Span::new(start, self.position, start_line, start_column),
            ));
        }

        let kind = if self.peek() == Some('n') {
            self.advance();
            TokenKind::BigIntLiteral(text)
        } else {
            TokenKind::NumberLiteral(text)
        };

        let span = Span::new(start, self.position, start_line, start_column);
        Ok(Token::new(kind, span))
    }

    /// 文字列リテラルのスキャン
    fn scan_string(&mut self, quote: char) -> JSResult<Token> {
        let start = self.position - 1;
        let start_line = self.line;
        let start_column = self.column - 1;
        let mut value = String::new();

        while let Some(ch) = self.peek() {
            if ch == quote {
                self.advance();
                break;
            } else if ch == '\\' {
                self.advance();
                if let Some(escaped) = self.peek() {
                    self.advance();
                    match escaped {
                        'b' => value.push('\u{0008}'),
                        'f' => value.push('\u{000C}'),
                        'n' => value.push('\n'),
                        'r' => value.push('\r'),
                        't' => value.push('\t'),
                        'v' => value.push('\u{000B}'),
                        '0' => value.push('\0'),
                        'x' => {
                            let code = self.scan_hex_escape(2, start, start_line, start_column)?;
                            value.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
                        }
                        'u' => {
                            let code = self.scan_hex_escape(4, start, start_line, start_column)?;
                            value.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
                        }
                        '\n' => {}
                        '\\' => value.push('\\'),
                        '\'' => value.push('\''),
                        '"' => value.push('"'),
                        _ => value.push(escaped),
                    }
                }
            } else if ch == '\n' {
                return Err(JSError::SyntaxError(
                    "Unterminated string literal".to_string(),
                    Span::new(start, self.position, start_line, start_column),
                ));
            } else {
                value.push(ch);
                self.advance();
            }
        }

        let span = Span::new(start, self.position, start_line, start_column);
        Ok(Token::new(TokenKind::String(value), span))
    }

    fn scan_template_literal(&mut self) -> JSResult<Token> {
        let start = self.position - 1;
        let start_line = self.line;
        let start_column = self.column - 1;
        let mut strings = Vec::new();
        let mut raw_strings = Vec::new();
        let mut expressions = Vec::new();
        let mut value = String::new();
        let mut raw_value = String::new();

        loop {
            let Some(ch) = self.peek() else {
                return Err(JSError::SyntaxError(
                    "Unterminated template literal".to_string(),
                    Span::new(start, self.position, start_line, start_column),
                ));
            };
            match ch {
                '`' => {
                    self.advance();
                    strings.push(value);
                    raw_strings.push(raw_value);
                    break;
                }
                '$' if self.peek_ahead(1) == Some('{') => {
                    self.advance();
                    self.advance();
                    strings.push(std::mem::take(&mut value));
                    raw_strings.push(std::mem::take(&mut raw_value));
                    expressions.push(self.scan_template_expression(
                        start,
                        start_line,
                        start_column,
                    )?);
                }
                '\\' => {
                    self.advance();
                    let Some(escaped) = self.peek() else {
                        continue;
                    };
                    raw_value.push('\\');
                    raw_value.push(escaped);
                    self.advance();
                    match escaped {
                        'n' => value.push('\n'),
                        'r' => value.push('\r'),
                        't' => value.push('\t'),
                        '\n' => {}
                        other => value.push(other),
                    }
                }
                _ => {
                    value.push(ch);
                    raw_value.push(ch);
                    self.advance();
                }
            }
        }

        let span = Span::new(start, self.position, start_line, start_column);
        Ok(Token::new(
            TokenKind::TemplateLiteral {
                strings,
                raw_strings,
                expressions,
            },
            span,
        ))
    }

    fn scan_template_expression(
        &mut self,
        literal_start: usize,
        start_line: usize,
        start_column: usize,
    ) -> JSResult<String> {
        let mut expression = String::new();
        let mut brace_depth = 1_usize;
        let mut quote = None;
        let mut escaped = false;

        while let Some(ch) = self.peek() {
            self.advance();
            if let Some(active_quote) = quote {
                expression.push(ch);
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == active_quote {
                    quote = None;
                }
                continue;
            }
            match ch {
                '\'' | '"' | '`' => {
                    quote = Some(ch);
                    expression.push(ch);
                }
                '{' => {
                    brace_depth += 1;
                    expression.push(ch);
                }
                '}' => {
                    brace_depth -= 1;
                    if brace_depth == 0 {
                        return Ok(expression);
                    }
                    expression.push(ch);
                }
                _ => expression.push(ch),
            }
        }

        Err(JSError::SyntaxError(
            "Unterminated template expression".to_string(),
            Span::new(literal_start, self.position, start_line, start_column),
        ))
    }

    fn scan_hex_escape(
        &mut self,
        digits: usize,
        literal_start: usize,
        start_line: usize,
        start_column: usize,
    ) -> JSResult<u32> {
        let mut value = 0_u32;
        for _ in 0..digits {
            let Some(character) = self.peek() else {
                return Err(JSError::SyntaxError(
                    "Invalid hexadecimal escape sequence".to_string(),
                    Span::new(literal_start, self.position, start_line, start_column),
                ));
            };
            let Some(digit) = character.to_digit(16) else {
                return Err(JSError::SyntaxError(
                    "Invalid hexadecimal escape sequence".to_string(),
                    Span::new(literal_start, self.position, start_line, start_column),
                ));
            };
            self.advance();
            value = value * 16 + digit;
        }
        Ok(value)
    }

    fn scan_regular_expression(&mut self) -> JSResult<Token> {
        let start = self.position - 1;
        let start_line = self.line;
        let start_column = self.column - 1;
        let mut pattern = String::new();
        let mut in_character_class = false;
        let mut terminated = false;

        while let Some(character) = self.peek() {
            if character == '\n' || character == '\r' {
                break;
            }
            self.advance();
            match character {
                '\\' => {
                    pattern.push(character);
                    let Some(escaped) = self.peek() else {
                        break;
                    };
                    self.advance();
                    pattern.push(escaped);
                }
                '[' => {
                    in_character_class = true;
                    pattern.push(character);
                }
                ']' => {
                    in_character_class = false;
                    pattern.push(character);
                }
                '/' if !in_character_class => {
                    terminated = true;
                    break;
                }
                _ => pattern.push(character),
            }
        }
        if !terminated {
            return Err(JSError::SyntaxError(
                "Unterminated regular expression literal".to_string(),
                Span::new(start, self.position, start_line, start_column),
            ));
        }

        let mut flags = String::new();
        while let Some(flag) = self.peek().filter(|flag| flag.is_ascii_alphabetic()) {
            flags.push(flag);
            self.advance();
        }
        let span = Span::new(start, self.position, start_line, start_column);
        Ok(Token::new(TokenKind::RegExpLiteral(pattern, flags), span))
    }

    fn can_start_regular_expression(&self) -> bool {
        if matches!(
            self.previous.as_ref(),
            Some(TokenKind::Identifier(name)) if name == "yield"
        ) {
            return true;
        }
        !matches!(
            self.previous.as_ref(),
            Some(
                TokenKind::NumberLiteral(_)
                    | TokenKind::String(_)
                    | TokenKind::RegExpLiteral(_, _)
                    | TokenKind::True
                    | TokenKind::False
                    | TokenKind::Null
                    | TokenKind::Undefined
                    | TokenKind::Identifier(_)
                    | TokenKind::This
                    | TokenKind::RightParen
                    | TokenKind::RightBracket
                    | TokenKind::RightBrace
                    | TokenKind::PlusPlus
                    | TokenKind::MinusMinus
            )
        )
    }

    /// 識別子・キーワードのスキャン
    fn scan_identifier(&mut self) -> JSResult<Token> {
        let start = self.position - 1;
        let start_line = self.line;
        let start_column = self.column - 1;

        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' || ch == '$' {
                self.advance();
            } else {
                break;
            }
        }

        let text: String = self.source[start..self.position].iter().collect();
        let kind = match text.as_str() {
            "let" => TokenKind::Let,
            "const" => TokenKind::Const,
            "var" => TokenKind::Var,
            "function" => TokenKind::Function,
            "return" => TokenKind::Return,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "for" => TokenKind::For,
            "while" => TokenKind::While,
            "do" => TokenKind::Do,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "switch" => TokenKind::Switch,
            "case" => TokenKind::Case,
            "default" => TokenKind::Default,
            "class" => TokenKind::Class,
            "new" => TokenKind::New,
            "this" => TokenKind::This,
            "super" => TokenKind::Super,
            "import" => TokenKind::Import,
            "export" => TokenKind::Export,
            "from" => TokenKind::From,
            "as" => TokenKind::As,
            "async" => TokenKind::Async,
            "await" => TokenKind::Await,
            "try" => TokenKind::Try,
            "catch" => TokenKind::Catch,
            "finally" => TokenKind::Finally,
            "throw" => TokenKind::Throw,
            "typeof" => TokenKind::Typeof,
            "delete" => TokenKind::Delete,
            "void" => TokenKind::Void,
            "in" => TokenKind::In,
            "of" => TokenKind::Of,
            "instanceof" => TokenKind::Instanceof,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "null" => TokenKind::Null,
            "undefined" => TokenKind::Undefined,
            _ => TokenKind::Identifier(text),
        };

        let span = Span::new(start, self.position, start_line, start_column);
        Ok(Token::new(kind, span))
    }

    /// 空白文字のスキップ
    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// 行コメントのスキップ
    fn skip_line_comment(&mut self) {
        while let Some(ch) = self.peek() {
            self.advance();
            if ch == '\n' {
                self.skip_whitespace();
                break;
            }
        }
    }

    /// ブロックコメントのスキップ
    fn skip_block_comment(&mut self) -> JSResult<()> {
        let start = self.position - 1;
        let start_line = self.line;
        let start_column = self.column - 1;

        while let Some(ch) = self.peek() {
            if ch == '*' && self.peek_ahead(1) == Some('/') {
                self.advance();
                self.advance();
                self.skip_whitespace();
                return Ok(());
            }
            self.advance();
        }

        Err(JSError::SyntaxError(
            "Unterminated block comment".to_string(),
            Span::new(start, self.position, start_line, start_column),
        ))
    }

    /// 次の文字を取得して位置を進める
    fn advance(&mut self) -> char {
        let ch = self.source[self.position];
        self.position += 1;

        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }

        ch
    }

    /// 次の文字を覗き見る
    fn peek(&self) -> Option<char> {
        if self.is_at_end() {
            None
        } else {
            Some(self.source[self.position])
        }
    }

    /// n文字先を覗き見る（ｷｬｰ！ﾍﾝﾀｲ！）
    fn peek_ahead(&self, n: usize) -> Option<char> {
        let pos = self.position + n;
        if pos >= self.source.len() {
            None
        } else {
            Some(self.source[pos])
        }
    }

    /// 期待する文字と一致する場合に位置を進める
    fn match_char(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// ソースコードの終端に達しているか
    fn is_at_end(&self) -> bool {
        self.position >= self.source.len()
    }

    /// 現在の位置のSpanを取得
    fn current_span(&self) -> Span {
        Span::new(self.position, self.position, self.line, self.column)
    }
}
