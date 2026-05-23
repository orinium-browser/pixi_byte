mod impl_dump;

use crate::Lexer;
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
    FunctionDeclaration {
        name: String,
        params: Vec<String>,
        body: Vec<Statement>,
    },
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
    This,
    ArrayLiteral(Vec<Expression>),
    ObjectLiteral(Vec<(String, Expression)>),
    MemberAccess {
        object: Box<Expression>,
        property: Box<Expression>,
        computed: bool,
    },
    Call {
        callee: Box<Expression>,
        args: Vec<Expression>,
    },
    Function {
        name: Option<String>,
        params: Vec<String>,
        body: Vec<Statement>,
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
    lexer: Lexer,

    current: Token,
    next: Token,
}

impl Parser {
    /// 新しいパーサーを生成
    pub fn new(mut lexer: Lexer) -> JSResult<Self> {
        let current = lexer
            .next()
            .transpose()?
            .unwrap_or_else(|| lexer.eof_token());

        let next = lexer
            .next()
            .transpose()?
            .unwrap_or_else(|| lexer.eof_token());

        Ok(Self {
            lexer,
            current,
            next,
        })
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
        match &self.current().kind {
            TokenKind::Var => self.parse_var_declaration(VarKind::Var),
            TokenKind::Let => self.parse_var_declaration(VarKind::Let),
            TokenKind::Const => self.parse_var_declaration(VarKind::Const),
            TokenKind::Return => self.parse_return_statement(),
            TokenKind::Function => self.parse_function_declaration(),
            _ => {
                let expr = self.parse_expression()?;
                self.consume_semicolon()?;
                Ok(Statement::Expression(expr))
            }
        }
    }

    /// ブロックをパースして文のベクタを返す
    fn parse_block(&mut self) -> JSResult<Vec<Statement>> {
        self.expect(&TokenKind::LeftBrace, "Expected '{'")?;

        let mut body = Vec::new();

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            body.push(self.parse_statement()?);
        }

        self.expect(&TokenKind::RightBrace, "Expected '}'")?;

        Ok(body)
    }

    /// 関数宣言をパース: function name(params) { body }
    fn parse_function_declaration(&mut self) -> JSResult<Statement> {
        let (name, params, body) = self.parse_function(true)?;

        Ok(Statement::FunctionDeclaration {
            name: name.unwrap(),
            params,
            body,
        })
    }

    fn parse_function(
        &mut self,
        require_name: bool,
    ) -> JSResult<(Option<String>, Vec<String>, Vec<Statement>)> {
        self.expect(&TokenKind::Function, "Expected function")?;

        let name = if require_name || matches!(&self.current().kind, TokenKind::Identifier(_)) {
            Some(self.expect_identifier("Expected function name")?)
        } else {
            None
        };

        self.expect(&TokenKind::LeftParen, "Expected '('")?;

        let mut params = Vec::new();

        while !self.check(&TokenKind::RightParen) {
            params.push(self.expect_identifier("Expected parameter name")?);

            if !self.check(&TokenKind::RightParen) {
                self.expect(&TokenKind::Comma, "Expected ','")?;
            }
        }

        self.expect(&TokenKind::RightParen, "Expected ')'")?;

        let body = self.parse_block()?;

        Ok((name, params, body))
    }

    /// 式をパース
    fn parse_expression(&mut self) -> JSResult<Expression> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> JSResult<Expression> {
        let left = self.parse_expression_bp(0)?;

        if self.eat(&TokenKind::Eq)? {
            let right = self.parse_assignment()?; // right-associative

            return Ok(Expression::Assignment {
                left: Box::new(left),
                right: Box::new(right),
            });
        }

        Ok(left)
    }

    fn parse_expression_bp(&mut self, min_bp: u8) -> JSResult<Expression> {
        // bp: Binding Power
        let mut left = self.parse_unary()?;

        while let Some((bp, op)) = precedence(&self.current().kind) {
            if bp < min_bp {
                break;
            }

            self.advance()?;

            let right = self.parse_expression_bp(bp + 1)?;

            left = Expression::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> JSResult<Expression> {
        let op = match &self.current().kind {
            TokenKind::Plus => UnaryOp::Plus,
            TokenKind::Minus => UnaryOp::Minus,
            TokenKind::Not => UnaryOp::Not,
            TokenKind::BitNot => UnaryOp::BitNot,
            TokenKind::Typeof => UnaryOp::Typeof,
            TokenKind::Void => UnaryOp::Void,
            TokenKind::Delete => UnaryOp::Delete,
            _ => return self.parse_postfix(),
        };

        self.advance()?;

        Ok(Expression::Unary {
            op,
            arg: Box::new(self.parse_unary()?),
        })
    }

    /// 変数宣言をパース
    fn parse_var_declaration(&mut self, kind: VarKind) -> JSResult<Statement> {
        self.advance()?; // var/let/const

        let name = self.expect_identifier("Expected variable name")?;

        let init = if self.eat(&TokenKind::Eq)? {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.consume_semicolon()?;

        Ok(Statement::VariableDeclaration { kind, name, init })
    }

    fn parse_function_expression(&mut self) -> JSResult<Expression> {
        let (name, params, body) = self.parse_function(false)?;

        Ok(Expression::Function { name, params, body })
    }

    /// return 文をパース
    fn parse_return_statement(&mut self) -> JSResult<Statement> {
        self.advance()?; // consume 'return'
        if self.check(&TokenKind::Semicolon)
            || self.check(&TokenKind::Eof)
            || self.check(&TokenKind::RightBrace)
        {
            self.consume_semicolon()?;
            return Ok(Statement::Return(None));
        }
        let expr = self.parse_expression()?;
        self.consume_semicolon()?;
        Ok(Statement::Return(Some(expr)))
    }

    /// 後置式をパース（メンバーアクセス等）
    fn parse_postfix(&mut self) -> JSResult<Expression> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.eat(&TokenKind::Dot)? {
                let property = self.expect_identifier("Expected property name")?;

                expr = Expression::MemberAccess {
                    object: Box::new(expr),
                    property: Box::new(Expression::Literal(Literal::String(property))),
                    computed: false,
                };
            } else if self.eat(&TokenKind::LeftBracket)? {
                let property = self.parse_expression()?;

                self.expect(&TokenKind::RightBracket, "Expected '}'")?;

                expr = Expression::MemberAccess {
                    object: Box::new(expr),
                    property: Box::new(property),
                    computed: true,
                };
            } else if self.eat(&TokenKind::LeftParen)? {
                let args = self.parse_arguments()?;

                self.expect(&TokenKind::RightParen, "Expected ')'")?;

                expr = Expression::Call {
                    callee: Box::new(expr),
                    args,
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_arguments(&mut self) -> JSResult<Vec<Expression>> {
        let mut args = Vec::new();

        while !self.check(&TokenKind::RightParen) {
            args.push(self.parse_expression()?);

            if !self.eat(&TokenKind::Comma)? {
                break;
            }
        }

        Ok(args)
    }

    /// 基本式をパース
    fn parse_primary(&mut self) -> JSResult<Expression> {
        match &self.current().kind {
            TokenKind::NumberLiteral(n) => {
                let n = n.parse().unwrap();

                self.advance()?;

                Ok(Expression::Literal(Literal::Number(n)))
            }
            TokenKind::String(s) => {
                let s = s.clone();

                self.advance()?;

                Ok(Expression::Literal(Literal::String(s)))
            }
            TokenKind::True => {
                self.advance()?;
                Ok(Expression::Literal(Literal::Boolean(true)))
            }
            TokenKind::False => {
                self.advance()?;
                Ok(Expression::Literal(Literal::Boolean(false)))
            }
            TokenKind::Null => {
                self.advance()?;
                Ok(Expression::Literal(Literal::Null))
            }
            TokenKind::Undefined => {
                self.advance()?;
                Ok(Expression::Literal(Literal::Undefined))
            }
            TokenKind::This => {
                self.advance()?;
                Ok(Expression::This)
            }
            TokenKind::Identifier(s) => {
                let s = s.clone();

                self.advance()?;

                Ok(Expression::Identifier(s))
            }
            TokenKind::LeftParen => {
                self.advance()?;

                let expr = self.parse_expression()?;

                self.expect(&TokenKind::RightParen, "Expected ')'")?;

                Ok(expr)
            }
            TokenKind::LeftBracket => self.parse_array_literal(),
            TokenKind::LeftBrace => self.parse_object_literal(),
            TokenKind::Function => self.parse_function_expression(),

            _ => Err(JSError::SyntaxError(
                format!("Unexpected token {:?}", self.current().kind),
                self.current().span,
            )),
        }
    }

    /// 配列リテラルをパース: [1, 2, 3]
    fn parse_array_literal(&mut self) -> JSResult<Expression> {
        self.advance()?; // consume '['

        let mut elements = Vec::new();

        while !self.check(&TokenKind::RightBracket) && !self.is_at_end() {
            // Support empty slots: [1,,3]
            if self.eat(&TokenKind::Comma)? {
                elements.push(Expression::Literal(Literal::Undefined));

                continue;
            }

            elements.push(self.parse_expression()?);

            if !self.check(&TokenKind::RightBracket) && !self.eat(&TokenKind::Comma)? {
                return Err(JSError::SyntaxError(
                    "Expected ',' or ']' in array literal".into(),
                    self.current().span,
                ));
            }
        }

        self.expect(&TokenKind::RightBracket, "Expected ']'")?;

        Ok(Expression::ArrayLiteral(elements))
    }

    /// オブジェクトリテラルをパース: { key: value }
    fn parse_object_literal(&mut self) -> JSResult<Expression> {
        self.advance()?; // consume '{'

        let mut properties = Vec::new();

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            // Parse property key
            let key = self.expect_identifier("Expected property key")?;

            self.expect(&TokenKind::Colon, "Expected ':' after property key")?;

            let value = self.parse_expression()?;

            properties.push((key, value));

            if !self.check(&TokenKind::RightBrace) && !self.eat(&TokenKind::Comma)? {
                return Err(JSError::SyntaxError(
                    "Expected ',' or '}' in object literal".into(),
                    self.current().span,
                ));
            }
        }

        self.expect(&TokenKind::RightBrace, "Expected '}'")?;

        Ok(Expression::ObjectLiteral(properties))
    }

    /// セミコロンを消費
    fn consume_semicolon(&mut self) -> JSResult<()> {
        // JavaScriptでは自動セミコロン挿入があるため、セミコロンは任意
        self.eat(&TokenKind::Semicolon)?;
        Ok(())
    }

    /// トークン列の終端かチェック
    fn is_at_end(&self) -> bool {
        matches!(self.current().kind, TokenKind::Eof)
    }
}

impl Parser {
    /// Current token
    fn current(&self) -> &Token {
        &self.current
    }

    /*
    /// Lookahead token
    fn next(&self) -> &Token {
        &self.next
    }
    */

    /// Advance one token
    fn advance(&mut self) -> JSResult<()> {
        self.current = std::mem::replace(
            &mut self.next,
            self.lexer
                .next()
                .transpose()?
                .unwrap_or_else(|| self.lexer.eof_token()),
        );

        Ok(())
    }

    /// Check current token kind
    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.current().kind) == std::mem::discriminant(kind)
    }

    /// Consume token if matched
    fn eat(&mut self, kind: &TokenKind) -> JSResult<bool> {
        if self.check(kind) {
            self.advance()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Require token
    fn expect(&mut self, kind: &TokenKind, message: &str) -> JSResult<()> {
        if self.check(kind) {
            self.advance()?;
            Ok(())
        } else {
            Err(JSError::SyntaxError(
                format!("{}: found {:?}", message, self.current().kind),
                self.current().span,
            ))
        }
    }

    /// Read identifier
    fn expect_identifier(&mut self, message: &str) -> JSResult<String> {
        match &self.current().kind {
            TokenKind::Identifier(s) => {
                let s = s.clone();

                self.advance()?;

                Ok(s)
            }

            _ => Err(JSError::SyntaxError(
                format!("{}: found {:?}", message, self.current().kind),
                self.current().span,
            )),
        }
    }
}

/// TokenKind を binding power 付き BinaryOp にして返す
fn precedence(kind: &TokenKind) -> Option<(u8, BinaryOp)> {
    match kind {
        // logical (lowest)
        TokenKind::Or => Some((1, BinaryOp::Or)),
        TokenKind::And => Some((2, BinaryOp::And)),

        // equality
        TokenKind::EqEqEq => Some((3, BinaryOp::StrictEq)),
        TokenKind::EqEq => Some((3, BinaryOp::Eq)),
        TokenKind::NotEqEq => Some((3, BinaryOp::StrictNotEq)),
        TokenKind::NotEq => Some((3, BinaryOp::NotEq)),

        // relational
        TokenKind::Lt => Some((4, BinaryOp::Lt)),
        TokenKind::Gt => Some((4, BinaryOp::Gt)),
        TokenKind::LtEq => Some((4, BinaryOp::LtEq)),
        TokenKind::GtEq => Some((4, BinaryOp::GtEq)),

        // additive
        TokenKind::Plus => Some((5, BinaryOp::Add)),
        TokenKind::Minus => Some((5, BinaryOp::Sub)),

        // multiplicative
        TokenKind::Star => Some((6, BinaryOp::Mul)),
        TokenKind::Slash => Some((6, BinaryOp::Div)),
        TokenKind::Percent => Some((6, BinaryOp::Mod)),

        _ => None,
    }
}
