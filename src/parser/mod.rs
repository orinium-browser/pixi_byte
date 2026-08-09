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
    Block(Vec<Statement>),
    Labeled {
        label: String,
        body: Box<Statement>,
    },
    Expression(Expression),
    VariableDeclaration {
        kind: VarKind,
        declarations: Vec<(String, Option<Expression>)>,
    },
    Return(Option<Expression>),
    FunctionDeclaration {
        name: String,
        params: Vec<String>,
        body: Vec<Statement>,
    },
    If {
        test: Expression,
        consequent: Vec<Statement>,
        alternate: Option<Vec<Statement>>,
    },
    While {
        test: Expression,
        body: Vec<Statement>,
    },
    DoWhile {
        body: Vec<Statement>,
        test: Expression,
    },
    For {
        init: Option<Box<Statement>>,
        test: Option<Expression>,
        update: Vec<Expression>,
        body: Vec<Statement>,
    },
    ForIn {
        binding: String,
        right: Expression,
        body: Vec<Statement>,
    },
    Throw(Expression),
    Try {
        block: Vec<Statement>,
        handler: Option<(Option<String>, Vec<Statement>)>,
        finalizer: Option<Vec<Statement>>,
    },
    Switch {
        discriminant: Expression,
        cases: Vec<(Option<Expression>, Vec<Statement>)>,
    },
    Break(Option<String>),
    Continue(Option<String>),
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
    Update {
        arg: Box<Expression>,
        increment: bool,
        prefix: bool,
    },
    Conditional {
        test: Box<Expression>,
        consequent: Box<Expression>,
        alternate: Box<Expression>,
    },
    Sequence(Vec<Expression>),
    This,
    ArrayLiteral(Vec<Expression>),
    ObjectLiteral(Vec<(String, Expression)>),
    RegExpLiteral {
        pattern: String,
        flags: String,
    },
    MemberAccess {
        object: Box<Expression>,
        property: Box<Expression>,
        computed: bool,
    },
    Call {
        callee: Box<Expression>,
        args: Vec<Expression>,
    },
    New {
        callee: Box<Expression>,
        args: Vec<Expression>,
    },
    Function {
        name: Option<String>,
        params: Vec<String>,
        body: Vec<Statement>,
    },
    ArrowFunction {
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
    In,
    Instanceof,
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
#[derive(Clone)]
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
        if matches!(self.current.kind, TokenKind::Identifier(_))
            && matches!(self.next.kind, TokenKind::Colon)
        {
            let label = self.expect_identifier("Expected statement label")?;
            self.expect(&TokenKind::Colon, "Expected ':' after statement label")?;
            let body = self.parse_statement()?;
            return Ok(Statement::Labeled {
                label,
                body: Box::new(body),
            });
        }

        match &self.current().kind {
            TokenKind::LeftBrace => Ok(Statement::Block(self.parse_block()?)),
            TokenKind::Var => self.parse_var_declaration(VarKind::Var),
            TokenKind::Let => self.parse_var_declaration(VarKind::Let),
            TokenKind::Const => self.parse_var_declaration(VarKind::Const),
            TokenKind::Return => self.parse_return_statement(),
            TokenKind::Function => self.parse_function_declaration(),
            TokenKind::If => self.parse_if_statement(),
            TokenKind::While => self.parse_while_statement(),
            TokenKind::Do => self.parse_do_while_statement(),
            TokenKind::For => self.parse_for_statement(),
            TokenKind::Throw => self.parse_throw_statement(),
            TokenKind::Try => self.parse_try_statement(),
            TokenKind::Switch => self.parse_switch_statement(),
            TokenKind::Break => {
                let line = self.current().span.line;
                self.advance()?;
                let label = if self.current().span.line == line {
                    if let TokenKind::Identifier(label) = &self.current().kind {
                        let label = label.clone();
                        self.advance()?;
                        Some(label)
                    } else {
                        None
                    }
                } else {
                    None
                };
                self.consume_semicolon()?;
                Ok(Statement::Break(label))
            }
            TokenKind::Continue => {
                let line = self.current().span.line;
                self.advance()?;
                let label = if self.current().span.line == line {
                    if let TokenKind::Identifier(label) = &self.current().kind {
                        let label = label.clone();
                        self.advance()?;
                        Some(label)
                    } else {
                        None
                    }
                } else {
                    None
                };
                self.consume_semicolon()?;
                Ok(Statement::Continue(label))
            }
            _ => {
                let expr = self.parse_expression()?;
                self.consume_semicolon()?;
                Ok(Statement::Expression(expr))
            }
        }
    }

    fn parse_throw_statement(&mut self) -> JSResult<Statement> {
        self.expect(&TokenKind::Throw, "Expected 'throw'")?;
        let value = self.parse_expression()?;
        self.consume_semicolon()?;
        Ok(Statement::Throw(value))
    }

    fn parse_try_statement(&mut self) -> JSResult<Statement> {
        self.expect(&TokenKind::Try, "Expected 'try'")?;
        let block = self.parse_block()?;
        let handler = if self.eat(&TokenKind::Catch)? {
            let binding = if self.eat(&TokenKind::LeftParen)? {
                let binding = self.expect_identifier("Expected catch binding")?;
                self.expect(&TokenKind::RightParen, "Expected ')' after catch binding")?;
                Some(binding)
            } else {
                None
            };
            Some((binding, self.parse_block()?))
        } else {
            None
        };
        let finalizer = if self.eat(&TokenKind::Finally)? {
            Some(self.parse_block()?)
        } else {
            None
        };
        if handler.is_none() && finalizer.is_none() {
            return Err(JSError::SyntaxError(
                "try statement requires catch or finally".to_string(),
                self.current().span,
            ));
        }
        Ok(Statement::Try {
            block,
            handler,
            finalizer,
        })
    }

    fn parse_switch_statement(&mut self) -> JSResult<Statement> {
        self.expect(&TokenKind::Switch, "Expected 'switch'")?;
        self.expect(&TokenKind::LeftParen, "Expected '(' after 'switch'")?;
        let discriminant = self.parse_expression()?;
        self.expect(&TokenKind::RightParen, "Expected ')' after switch value")?;
        self.expect(&TokenKind::LeftBrace, "Expected '{' after switch value")?;

        let mut cases = Vec::new();
        let mut has_default = false;
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            let test = if self.eat(&TokenKind::Case)? {
                let test = self.parse_expression()?;
                self.expect(&TokenKind::Colon, "Expected ':' after case value")?;
                Some(test)
            } else if self.eat(&TokenKind::Default)? {
                if has_default {
                    return Err(JSError::SyntaxError(
                        "switch statement has more than one default".to_string(),
                        self.current().span,
                    ));
                }
                has_default = true;
                self.expect(&TokenKind::Colon, "Expected ':' after default")?;
                None
            } else {
                return Err(JSError::SyntaxError(
                    "Expected 'case' or 'default' in switch".to_string(),
                    self.current().span,
                ));
            };

            let mut body = Vec::new();
            while !self.check(&TokenKind::Case)
                && !self.check(&TokenKind::Default)
                && !self.check(&TokenKind::RightBrace)
                && !self.is_at_end()
            {
                body.push(self.parse_statement()?);
            }
            cases.push((test, body));
        }
        self.expect(&TokenKind::RightBrace, "Expected '}' after switch")?;
        Ok(Statement::Switch {
            discriminant,
            cases,
        })
    }

    fn parse_while_statement(&mut self) -> JSResult<Statement> {
        self.expect(&TokenKind::While, "Expected 'while'")?;
        self.expect(&TokenKind::LeftParen, "Expected '(' after 'while'")?;
        let test = self.parse_expression()?;
        self.expect(&TokenKind::RightParen, "Expected ')' after condition")?;
        let body = if self.check(&TokenKind::LeftBrace) {
            self.parse_block()?
        } else {
            vec![self.parse_statement()?]
        };
        Ok(Statement::While { test, body })
    }

    fn parse_do_while_statement(&mut self) -> JSResult<Statement> {
        self.expect(&TokenKind::Do, "Expected 'do'")?;
        let body = if self.check(&TokenKind::LeftBrace) {
            self.parse_block()?
        } else {
            vec![self.parse_statement()?]
        };
        self.expect(&TokenKind::While, "Expected 'while' after do body")?;
        self.expect(&TokenKind::LeftParen, "Expected '(' after 'while'")?;
        let test = self.parse_expression()?;
        self.expect(&TokenKind::RightParen, "Expected ')' after condition")?;
        self.consume_semicolon()?;
        Ok(Statement::DoWhile { body, test })
    }

    fn parse_for_statement(&mut self) -> JSResult<Statement> {
        self.expect(&TokenKind::For, "Expected 'for'")?;
        self.expect(&TokenKind::LeftParen, "Expected '(' after 'for'")?;

        if matches!(
            self.current().kind,
            TokenKind::Var | TokenKind::Let | TokenKind::Const
        ) {
            let mut candidate = self.clone();
            candidate.advance()?;
            if matches!(candidate.current().kind, TokenKind::Identifier(_)) {
                candidate.advance()?;
                if candidate.check(&TokenKind::In) {
                    self.advance()?;
                    let binding = self.expect_identifier("Expected for-in binding")?;
                    self.expect(&TokenKind::In, "Expected 'in' after for-in binding")?;
                    return self.parse_for_in_tail(binding);
                }
            }
        } else if matches!(self.current().kind, TokenKind::Identifier(_))
            && matches!(self.next.kind, TokenKind::In)
        {
            let binding = self.expect_identifier("Expected for-in binding")?;
            self.expect(&TokenKind::In, "Expected 'in' after for-in binding")?;
            return self.parse_for_in_tail(binding);
        }

        let init = if self.eat(&TokenKind::Semicolon)? {
            None
        } else if matches!(
            self.current().kind,
            TokenKind::Var | TokenKind::Let | TokenKind::Const
        ) {
            let kind = match self.current().kind {
                TokenKind::Var => VarKind::Var,
                TokenKind::Let => VarKind::Let,
                TokenKind::Const => VarKind::Const,
                _ => unreachable!(),
            };
            Some(Box::new(self.parse_var_declaration(kind)?))
        } else {
            let expression = self.parse_expression()?;
            self.expect(&TokenKind::Semicolon, "Expected ';' after for initializer")?;
            Some(Box::new(Statement::Expression(expression)))
        };

        let test = if self.eat(&TokenKind::Semicolon)? {
            None
        } else {
            let expression = self.parse_expression()?;
            self.expect(&TokenKind::Semicolon, "Expected ';' after for condition")?;
            Some(expression)
        };
        let mut update = Vec::new();
        while !self.check(&TokenKind::RightParen) {
            update.push(self.parse_assignment()?);
            if !self.eat(&TokenKind::Comma)? {
                break;
            }
        }
        self.expect(&TokenKind::RightParen, "Expected ')' after for clauses")?;
        let body = if self.check(&TokenKind::LeftBrace) {
            self.parse_block()?
        } else {
            vec![self.parse_statement()?]
        };
        Ok(Statement::For {
            init,
            test,
            update,
            body,
        })
    }

    fn parse_for_in_tail(&mut self, binding: String) -> JSResult<Statement> {
        let right = self.parse_expression()?;
        self.expect(
            &TokenKind::RightParen,
            "Expected ')' after for-in expression",
        )?;
        let body = if self.check(&TokenKind::LeftBrace) {
            self.parse_block()?
        } else {
            vec![self.parse_statement()?]
        };
        Ok(Statement::ForIn {
            binding,
            right,
            body,
        })
    }

    fn parse_if_statement(&mut self) -> JSResult<Statement> {
        self.expect(&TokenKind::If, "Expected 'if'")?;
        self.expect(&TokenKind::LeftParen, "Expected '(' after 'if'")?;
        let test = self.parse_expression()?;
        self.expect(&TokenKind::RightParen, "Expected ')' after condition")?;
        let consequent = if self.check(&TokenKind::LeftBrace) {
            self.parse_block()?
        } else {
            vec![self.parse_statement()?]
        };
        let alternate = if self.eat(&TokenKind::Else)? {
            Some(if self.check(&TokenKind::LeftBrace) {
                self.parse_block()?
            } else {
                vec![self.parse_statement()?]
            })
        } else {
            None
        };
        Ok(Statement::If {
            test,
            consequent,
            alternate,
        })
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
        let first = self.parse_assignment()?;
        if !self.eat(&TokenKind::Comma)? {
            return Ok(first);
        }

        let mut expressions = vec![first];
        loop {
            expressions.push(self.parse_assignment()?);
            if !self.eat(&TokenKind::Comma)? {
                break;
            }
        }
        Ok(Expression::Sequence(expressions))
    }

    fn parse_assignment(&mut self) -> JSResult<Expression> {
        if let Some(arrow) = self.try_parse_arrow_function()? {
            return Ok(arrow);
        }

        let mut left = self.parse_expression_bp(0)?;

        if self.eat(&TokenKind::Question)? {
            let consequent = self.parse_assignment()?;
            self.expect(&TokenKind::Colon, "Expected ':' in conditional expression")?;
            let alternate = self.parse_assignment()?;
            left = Expression::Conditional {
                test: Box::new(left),
                consequent: Box::new(consequent),
                alternate: Box::new(alternate),
            };
        }

        let assignment = match self.current().kind {
            TokenKind::Eq => Some(None),
            TokenKind::PlusEq => Some(Some(BinaryOp::Add)),
            TokenKind::MinusEq => Some(Some(BinaryOp::Sub)),
            TokenKind::StarEq => Some(Some(BinaryOp::Mul)),
            TokenKind::SlashEq => Some(Some(BinaryOp::Div)),
            TokenKind::PercentEq => Some(Some(BinaryOp::Mod)),
            TokenKind::BitAndEq => Some(Some(BinaryOp::BitAnd)),
            TokenKind::BitOrEq => Some(Some(BinaryOp::BitOr)),
            TokenKind::BitXorEq => Some(Some(BinaryOp::BitXor)),
            TokenKind::LeftShiftEq => Some(Some(BinaryOp::LeftShift)),
            TokenKind::RightShiftEq => Some(Some(BinaryOp::RightShift)),
            TokenKind::UnsignedRightShiftEq => Some(Some(BinaryOp::UnsignedRightShift)),
            _ => None,
        };
        if let Some(operator) = assignment {
            self.advance()?;
            let right = self.parse_assignment()?; // right-associative
            let right = if let Some(operator) = operator {
                Expression::Binary {
                    op: operator,
                    left: Box::new(left.clone()),
                    right: Box::new(right),
                }
            } else {
                right
            };

            return Ok(Expression::Assignment {
                left: Box::new(left),
                right: Box::new(right),
            });
        }

        Ok(left)
    }

    fn try_parse_arrow_function(&mut self) -> JSResult<Option<Expression>> {
        if let TokenKind::Identifier(param) = &self.current().kind
            && matches!(self.next.kind, TokenKind::Arrow)
        {
            let param = param.clone();
            self.advance()?;
            self.advance()?;
            return self.parse_arrow_body(vec![param]).map(Some);
        }

        if !self.check(&TokenKind::LeftParen) {
            return Ok(None);
        }
        let mut candidate = self.clone();
        candidate.advance()?;
        let mut params = Vec::new();
        while !candidate.check(&TokenKind::RightParen) {
            let TokenKind::Identifier(param) = &candidate.current().kind else {
                return Ok(None);
            };
            params.push(param.clone());
            candidate.advance()?;
            if !candidate.check(&TokenKind::RightParen) && !candidate.eat(&TokenKind::Comma)? {
                return Ok(None);
            }
        }
        candidate.advance()?;
        if !candidate.eat(&TokenKind::Arrow)? {
            return Ok(None);
        }

        let arrow = candidate.parse_arrow_body(params)?;
        *self = candidate;
        Ok(Some(arrow))
    }

    fn parse_arrow_body(&mut self, params: Vec<String>) -> JSResult<Expression> {
        let body = if self.check(&TokenKind::LeftBrace) {
            self.parse_block()?
        } else {
            vec![Statement::Return(Some(self.parse_assignment()?))]
        };
        Ok(Expression::ArrowFunction { params, body })
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
        if self.check(&TokenKind::PlusPlus) || self.check(&TokenKind::MinusMinus) {
            let increment = self.check(&TokenKind::PlusPlus);
            self.advance()?;
            return Ok(Expression::Update {
                arg: Box::new(self.parse_unary()?),
                increment,
                prefix: true,
            });
        }
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

        let mut declarations = Vec::new();
        loop {
            let name = self.expect_identifier("Expected variable name")?;
            let init = if self.eat(&TokenKind::Eq)? {
                Some(self.parse_assignment()?)
            } else {
                None
            };
            declarations.push((name, init));
            if !self.eat(&TokenKind::Comma)? {
                break;
            }
        }

        self.consume_semicolon()?;

        Ok(Statement::VariableDeclaration { kind, declarations })
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
                let property = self.expect_identifier_name("Expected property name")?;

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

        if self.check(&TokenKind::PlusPlus) || self.check(&TokenKind::MinusMinus) {
            let increment = self.check(&TokenKind::PlusPlus);
            self.advance()?;
            return Ok(Expression::Update {
                arg: Box::new(expr),
                increment,
                prefix: false,
            });
        }

        Ok(expr)
    }

    fn parse_arguments(&mut self) -> JSResult<Vec<Expression>> {
        let mut args = Vec::new();

        while !self.check(&TokenKind::RightParen) {
            args.push(self.parse_assignment()?);

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
            TokenKind::RegExpLiteral(pattern, flags) => {
                let pattern = pattern.clone();
                let flags = flags.clone();
                self.advance()?;
                Ok(Expression::RegExpLiteral { pattern, flags })
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
            TokenKind::New => self.parse_new_expression(),

            _ => Err(JSError::SyntaxError(
                format!("Unexpected token {:?}", self.current().kind),
                self.current().span,
            )),
        }
    }

    fn parse_new_expression(&mut self) -> JSResult<Expression> {
        self.advance()?; // consume 'new'
        let callee = self.parse_primary()?;
        let args = if self.eat(&TokenKind::LeftParen)? {
            let args = self.parse_arguments()?;
            self.expect(
                &TokenKind::RightParen,
                "Expected ')' after constructor arguments",
            )?;
            args
        } else {
            Vec::new()
        };

        Ok(Expression::New {
            callee: Box::new(callee),
            args,
        })
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

            elements.push(self.parse_assignment()?);

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
            let (key, shorthand) = match &self.current().kind {
                TokenKind::String(key) => {
                    let key = key.clone();
                    self.advance()?;
                    (key, None)
                }
                TokenKind::NumberLiteral(key) => {
                    let key = key.clone();
                    self.advance()?;
                    (key, None)
                }
                TokenKind::Identifier(key) => {
                    let key = key.clone();
                    self.advance()?;
                    (key.clone(), Some(Expression::Identifier(key)))
                }
                _ => (self.expect_identifier_name("Expected property key")?, None),
            };

            let value = if self.eat(&TokenKind::Colon)? {
                self.parse_assignment()?
            } else if let Some(value) = shorthand {
                value
            } else {
                return Err(JSError::SyntaxError(
                    "Expected ':' after property key".to_string(),
                    self.current().span,
                ));
            };

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

    fn expect_identifier_name(&mut self, message: &str) -> JSResult<String> {
        let name = match &self.current().kind {
            TokenKind::Identifier(name) => name.as_str(),
            TokenKind::True => "true",
            TokenKind::False => "false",
            TokenKind::Null => "null",
            TokenKind::Undefined => "undefined",
            TokenKind::Let => "let",
            TokenKind::Const => "const",
            TokenKind::Var => "var",
            TokenKind::Function => "function",
            TokenKind::Return => "return",
            TokenKind::If => "if",
            TokenKind::Else => "else",
            TokenKind::For => "for",
            TokenKind::While => "while",
            TokenKind::Do => "do",
            TokenKind::Break => "break",
            TokenKind::Continue => "continue",
            TokenKind::Switch => "switch",
            TokenKind::Case => "case",
            TokenKind::Default => "default",
            TokenKind::Class => "class",
            TokenKind::New => "new",
            TokenKind::This => "this",
            TokenKind::Super => "super",
            TokenKind::Import => "import",
            TokenKind::Export => "export",
            TokenKind::From => "from",
            TokenKind::As => "as",
            TokenKind::Async => "async",
            TokenKind::Await => "await",
            TokenKind::Try => "try",
            TokenKind::Catch => "catch",
            TokenKind::Finally => "finally",
            TokenKind::Throw => "throw",
            TokenKind::Typeof => "typeof",
            TokenKind::Delete => "delete",
            TokenKind::Void => "void",
            TokenKind::In => "in",
            TokenKind::Of => "of",
            TokenKind::Instanceof => "instanceof",
            _ => {
                return Err(JSError::SyntaxError(
                    format!("{}: found {}", message, self.current().kind),
                    self.current().span,
                ));
            }
        }
        .to_string();
        self.advance()?;
        Ok(name)
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
        TokenKind::In => Some((4, BinaryOp::In)),
        TokenKind::Instanceof => Some((4, BinaryOp::Instanceof)),

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
