use crate::error::{JSError, JSResult};
use super::token::{Token, TokenKind, Span};

/// 字句解析器
pub struct Lexer {
    /// ソースコードの文字列
    source: Vec<char>,
    /// 現在の位置
    position: usize,
    /// 現在の行番号
    line: usize,
    /// 現在の列番号
    column: usize,
}

impl Lexer {
    /// 新しい字句解析器を作成
    pub fn new(source: &str) -> Self {
        Self {
            source: source.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
        }
    }

    /// ソースコードを字句解析してトークン列を生成
    pub fn tokenize(&mut self) -> JSResult<Vec<Token>> {
        let mut tokens = Vec::new();

        loop {
            self.skip_whitespace();
            
            if self.is_at_end() {
                let span = self.current_span();
                tokens.push(Token::new(TokenKind::Eof, span));
                break;
            }

            let token = self.next_token()?;
            tokens.push(token);
        }

        Ok(tokens)
    }

    /// 次のトークンを取得
    fn next_token(&mut self) -> JSResult<Token> {
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
            '?' => TokenKind::Question,
            ':' => TokenKind::Colon,

            // ドット
            '.' => {
                if self.peek() == Some('.') && self.peek_ahead(1) == Some('.') {
                    self.advance();
                    self.advance();
                    TokenKind::DotDotDot
                } else if self.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    return self.scan_number_starting_with_dot();
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
                    TokenKind::LeftShift
                } else if self.match_char('=') {
                    TokenKind::LtEq
                } else {
                    TokenKind::Lt
                }
            }
            '>' => {
                if self.match_char('>') {
                    if self.match_char('>') {
                        TokenKind::UnsignedRightShift
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
                } else {
                    TokenKind::BitAnd
                }
            }
            '|' => {
                if self.match_char('|') {
                    TokenKind::Or
                } else {
                    TokenKind::BitOr
                }
            }
            '^' => TokenKind::BitXor,

            // 文字列リテラル
            '"' | '\'' => return self.scan_string(ch),

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

        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }

        // 小数点
        if self.peek() == Some('.') && self.peek_ahead(1).map(|c| c.is_ascii_digit()).unwrap_or(false) {
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
        let value = text.parse::<f64>().map_err(|_| {
            JSError::SyntaxError(format!("Invalid number literal: {}", text))
        })?;

        let span = Span::new(start, self.position, start_line, start_column);
        Ok(Token::new(TokenKind::Number(value), span))
    }

    /// ドットで始まる数値リテラルのスキャン
    fn scan_number_starting_with_dot(&mut self) -> JSResult<Token> {
        let start = self.position - 1;
        let start_line = self.line;
        let start_column = self.column - 1;

        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }

        let text: String = self.source[start..self.position].iter().collect();
        let value = text.parse::<f64>().map_err(|_| {
            JSError::SyntaxError(format!("Invalid number literal: {}", text))
        })?;

        let span = Span::new(start, self.position, start_line, start_column);
        Ok(Token::new(TokenKind::Number(value), span))
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
                    let escaped_char = match escaped {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        '\\' => '\\',
                        '\'' => '\'',
                        '"' => '"',
                        _ => escaped,
                    };
                    value.push(escaped_char);
                    self.advance();
                }
            } else if ch == '\n' {
                return Err(JSError::SyntaxError("Unterminated string literal".to_string()));
            } else {
                value.push(ch);
                self.advance();
            }
        }

        let span = Span::new(start, self.position, start_line, start_column);
        Ok(Token::new(TokenKind::String(value), span))
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
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
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
            if ch == '\n' {
                break;
            }
            self.advance();
        }
    }

    /// ブロックコメントのスキップ
    fn skip_block_comment(&mut self) -> JSResult<()> {
        while let Some(ch) = self.peek() {
            if ch == '*' && self.peek_ahead(1) == Some('/') {
                self.advance();
                self.advance();
                return Ok(());
            }
            self.advance();
        }
        Err(JSError::SyntaxError("Unterminated block comment".to_string()))
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

