use crate::error::{JSError, JSResult};
use crate::lexer::{Token, TokenKind};

/// AST（抽象構文木）のプログラムノード
#[derive(Debug, Clone)]
pub struct Program {
    pub body: Vec<Statement>,
}

/// 文
#[derive(Debug, Clone)]
pub enum Statement {
    Expression(Expression),
    VariableDeclaration {
        kind: VarKind,
        name: String,
        init: Option<Expression>,
    },
    Return(Option<Expression>),
    // TODO: 他の文を追加
}

/// 変数宣言の種類
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarKind {
    Var,
    Let,
    Const,
}

/// 式
#[derive(Debug, Clone)]
pub enum Expression {
    Literal(Literal),
    Identifier(String),
    Binary {
        op: BinaryOp,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Unary {
        op: UnaryOp,
        arg: Box<Expression>,
    },
    Assignment {
        left: Box<Expression>,
        right: Box<Expression>,
    },
    // TODO: 他の式を追加
}

/// リテラル
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
}

/// 二項演算子
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Power,
    Eq,
    NotEq,
    StrictEq,
    StrictNotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    LeftShift,
    RightShift,
    UnsignedRightShift,
}

/// 単項演算子
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Plus,
    Minus,
    Not,
    BitNot,
    Typeof,
    Void,
    Delete,
}

/// パーサー
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    /// 新しいパーサーを生成
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    /// トークン列をパースしてASTを生成
    pub fn parse(&mut self) -> JSResult<Program> {
        let mut body = Vec::new();

        while !self.is_at_end() {
            body.push(self.parse_statement()?);
        }

        Ok(Program { body })
    }

    /// 文をパース
    fn parse_statement(&mut self) -> JSResult<Statement> {
        match &self.peek().kind {
            TokenKind::Var => self.parse_var_declaration(VarKind::Var),
            TokenKind::Let => self.parse_var_declaration(VarKind::Let),
            TokenKind::Const => self.parse_var_declaration(VarKind::Const),
            TokenKind::Return => self.parse_return_statement(),
            _ => {
                let expr = self.parse_expression()?;
                self.consume_semicolon();
                Ok(Statement::Expression(expr))
            }
        }
    }

    /// 変数宣言をパース
    fn parse_var_declaration(&mut self, kind: VarKind) -> JSResult<Statement> {
        self.advance(); // var/let/const

        let name = match &self.peek().kind {
            TokenKind::Identifier(s) => {
                let name = s.clone();
                self.advance();
                name
            }
            _ => return Err(JSError::SyntaxError("Expected identifier".to_string())),
        };

        let init = if self.match_token(&TokenKind::Eq) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.consume_semicolon();

        Ok(Statement::VariableDeclaration { kind, name, init })
    }

    /// return文をパース
    fn parse_return_statement(&mut self) -> JSResult<Statement> {
        self.advance(); // return

        let value = if self.check(&TokenKind::Semicolon) || self.is_at_end() {
            None
        } else {
            Some(self.parse_expression()?)
        };

        self.consume_semicolon();

        Ok(Statement::Return(value))
    }

    /// 式をパース
    fn parse_expression(&mut self) -> JSResult<Expression> {
        self.parse_assignment()
    }

    /// 代入式をパース
    fn parse_assignment(&mut self) -> JSResult<Expression> {
        let expr = self.parse_logical_or()?;

        if self.match_token(&TokenKind::Eq) {
            let right = self.parse_assignment()?;
            return Ok(Expression::Assignment {
                left: Box::new(expr),
                right: Box::new(right),
            });
        }

        Ok(expr)
    }

    /// 論理和式をパース
    fn parse_logical_or(&mut self) -> JSResult<Expression> {
        let mut left = self.parse_logical_and()?;

        while self.match_token(&TokenKind::Or) {
            let right = self.parse_logical_and()?;
            left = Expression::Binary {
                op: BinaryOp::Or,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// 論理積式をパース
    fn parse_logical_and(&mut self) -> JSResult<Expression> {
        let mut left = self.parse_equality()?;

        while self.match_token(&TokenKind::And) {
            let right = self.parse_equality()?;
            left = Expression::Binary {
                op: BinaryOp::And,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// 等価式をパース
    fn parse_equality(&mut self) -> JSResult<Expression> {
        let mut left = self.parse_comparison()?;

        loop {
            let op = match &self.peek().kind {
                TokenKind::EqEq => BinaryOp::Eq,
                TokenKind::NotEq => BinaryOp::NotEq,
                TokenKind::EqEqEq => BinaryOp::StrictEq,
                TokenKind::NotEqEq => BinaryOp::StrictNotEq,
                _ => break,
            };
            self.advance();
            let right = self.parse_comparison()?;
            left = Expression::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// 比較式をパース
    fn parse_comparison(&mut self) -> JSResult<Expression> {
        let mut left = self.parse_additive()?;

        loop {
            let op = match &self.peek().kind {
                TokenKind::Lt => BinaryOp::Lt,
                TokenKind::Gt => BinaryOp::Gt,
                TokenKind::LtEq => BinaryOp::LtEq,
                TokenKind::GtEq => BinaryOp::GtEq,
                _ => break,
            };
            self.advance();
            let right = self.parse_additive()?;
            left = Expression::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// 加算式をパース
    fn parse_additive(&mut self) -> JSResult<Expression> {
        let mut left = self.parse_multiplicative()?;

        loop {
            let op = match &self.peek().kind {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expression::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// 乗算式をパース
    fn parse_multiplicative(&mut self) -> JSResult<Expression> {
        let mut left = self.parse_unary()?;

        loop {
            let op = match &self.peek().kind {
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                TokenKind::Percent => BinaryOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expression::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// 単項式をパース
    fn parse_unary(&mut self) -> JSResult<Expression> {
        let op = match &self.peek().kind {
            TokenKind::Plus => UnaryOp::Plus,
            TokenKind::Minus => UnaryOp::Minus,
            TokenKind::Not => UnaryOp::Not,
            TokenKind::BitNot => UnaryOp::BitNot,
            TokenKind::Typeof => UnaryOp::Typeof,
            TokenKind::Void => UnaryOp::Void,
            TokenKind::Delete => UnaryOp::Delete,
            _ => return self.parse_primary(),
        };
        self.advance();
        let arg = self.parse_unary()?;
        Ok(Expression::Unary {
            op,
            arg: Box::new(arg),
        })
    }

    /// 基本式をパース
    fn parse_primary(&mut self) -> JSResult<Expression> {
        let token = self.peek().clone();

        match &token.kind {
            TokenKind::Number(n) => {
                self.advance();
                Ok(Expression::Literal(Literal::Number(*n)))
            }
            TokenKind::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expression::Literal(Literal::String(s)))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expression::Literal(Literal::Boolean(true)))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expression::Literal(Literal::Boolean(false)))
            }
            TokenKind::Null => {
                self.advance();
                Ok(Expression::Literal(Literal::Null))
            }
            TokenKind::Undefined => {
                self.advance();
                Ok(Expression::Literal(Literal::Undefined))
            }
            TokenKind::Identifier(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expression::Identifier(s))
            }
            TokenKind::LeftParen => {
                self.advance();
                let expr = self.parse_expression()?;
                if !self.match_token(&TokenKind::RightParen) {
                    return Err(JSError::SyntaxError("Expected ')'".to_string()));
                }
                Ok(expr)
            }
            _ => Err(JSError::SyntaxError(format!(
                "Unexpected token: {:?}",
                token.kind
            ))),
        }
    }

    /// 現在のトークンを取得
    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    /// トークンを1つ進めて取得
    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        &self.tokens[self.current - 1]
    }

    /// 現在のトークンが指定の種類かチェック
    fn check(&self, kind: &TokenKind) -> bool {
        if self.is_at_end() {
            return false;
        }
        std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind)
    }

    /// 現在のトークンが指定の種類なら進めてtrueを返す
    fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// セミコロンを消費
    fn consume_semicolon(&mut self) {
        // JavaScriptでは自動セミコロン挿入があるため、セミコロンは任意
        self.match_token(&TokenKind::Semicolon);
    }

    /// トークン列の終端かチェック
    fn is_at_end(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Eof)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

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
}

