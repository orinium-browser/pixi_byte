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
    Empty,
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
        is_generator: bool,
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
        binding: BindingPattern,
        kind: Option<VarKind>,
        right: Expression,
        body: Vec<Statement>,
    },
    ForOf {
        binding: BindingPattern,
        kind: Option<VarKind>,
        right: Expression,
        body: Vec<Statement>,
    },
    PatternDeclaration {
        kind: VarKind,
        binding: BindingPattern,
        init: Expression,
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

#[derive(Debug, Clone)]
pub enum BindingPattern {
    Identifier(String),
    Target(Expression),
    Array(Vec<Option<BindingPattern>>),
    Object(Vec<(String, BindingPattern)>),
    Rest(Box<BindingPattern>),
    Default(Box<BindingPattern>, Expression),
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
    Super,
    ArrayLiteral(Vec<Expression>),
    TemplateObject {
        cooked: Vec<String>,
        raw: Vec<String>,
    },
    ObjectLiteral(Vec<ObjectProperty>),
    RegExpLiteral {
        pattern: String,
        flags: String,
    },
    MemberAccess {
        object: Box<Expression>,
        property: Box<Expression>,
        computed: bool,
    },
    OptionalMemberAccess {
        object: Box<Expression>,
        property: Box<Expression>,
        computed: bool,
    },
    Call {
        callee: Box<Expression>,
        args: Vec<Expression>,
    },
    OptionalCall {
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
        is_generator: bool,
    },
    ArrowFunction {
        params: Vec<String>,
        body: Vec<Statement>,
    },
    Yield {
        value: Box<Expression>,
        delegate: bool,
    },
    Spread(Box<Expression>),
    Class {
        name: Option<String>,
        super_class: Option<Box<Expression>>,
        constructor: Option<ClassMethod>,
        methods: Vec<ClassMethod>,
    },
    // TODO: 他の式を追加
}

#[derive(Debug, Clone)]
pub struct ClassMethod {
    pub name: String,
    pub computed_name: Option<Box<Expression>>,
    pub params: Vec<String>,
    pub body: Vec<Statement>,
    pub is_static: bool,
    pub is_generator: bool,
    pub kind: ClassMethodKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassMethodKind {
    Method,
    Getter,
    Setter,
}

#[derive(Debug, Clone)]
pub enum ObjectProperty {
    Data {
        key: String,
        value: Expression,
    },
    Getter {
        key: String,
        body: Vec<Statement>,
    },
    Setter {
        key: String,
        parameter: String,
        body: Vec<Statement>,
    },
    ComputedData {
        key: Expression,
        value: Expression,
    },
    Spread(Expression),
}

/// リテラル
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    BigInt(String),
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
    Nullish,
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
    next_temporary: usize,
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
            next_temporary: 0,
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
        if self.check(&TokenKind::Async) && matches!(self.next.kind, TokenKind::Function) {
            self.advance()?;
            return self.parse_function_declaration();
        }
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
            TokenKind::Semicolon => {
                self.advance()?;
                Ok(Statement::Empty)
            }
            TokenKind::LeftBrace => Ok(Statement::Block(self.parse_block()?)),
            TokenKind::Var => self.parse_var_declaration(VarKind::Var),
            TokenKind::Let => self.parse_var_declaration(VarKind::Let),
            TokenKind::Const => self.parse_var_declaration(VarKind::Const),
            TokenKind::Return => self.parse_return_statement(),
            TokenKind::Function => self.parse_function_declaration(),
            TokenKind::Class => self.parse_class_declaration(),
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
            let kind = match self.current().kind {
                TokenKind::Var => VarKind::Var,
                TokenKind::Let => VarKind::Let,
                TokenKind::Const => VarKind::Const,
                _ => unreachable!(),
            };
            let mut candidate = self.clone();
            candidate.advance()?;
            if let Ok(binding) = candidate.parse_binding_pattern()
                && (candidate.check(&TokenKind::In) || candidate.check(&TokenKind::Of))
            {
                *self = candidate;
                if self.eat(&TokenKind::In)? {
                    return self.parse_for_in_tail(binding, Some(kind));
                }
                self.expect(&TokenKind::Of, "Expected 'of' after for-of binding")?;
                return self.parse_for_of_tail(binding, Some(kind));
            }
        } else if matches!(self.current().kind, TokenKind::Identifier(_))
            && matches!(self.next.kind, TokenKind::In | TokenKind::Of)
        {
            let binding =
                BindingPattern::Identifier(self.expect_identifier("Expected for-in binding")?);
            if self.eat(&TokenKind::In)? {
                return self.parse_for_in_tail(binding, None);
            }
            self.expect(&TokenKind::Of, "Expected 'of' after for-of binding")?;
            return self.parse_for_of_tail(binding, None);
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

    fn parse_for_in_tail(
        &mut self,
        binding: BindingPattern,
        kind: Option<VarKind>,
    ) -> JSResult<Statement> {
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
            kind,
            right,
            body,
        })
    }

    fn parse_for_of_tail(
        &mut self,
        binding: BindingPattern,
        kind: Option<VarKind>,
    ) -> JSResult<Statement> {
        let right = self.parse_expression()?;
        self.expect(
            &TokenKind::RightParen,
            "Expected ')' after for-of expression",
        )?;
        let body = if self.check(&TokenKind::LeftBrace) {
            self.parse_block()?
        } else {
            vec![self.parse_statement()?]
        };
        Ok(Statement::ForOf {
            binding,
            kind,
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
        let (name, params, body, is_generator) = self.parse_function(true)?;

        Ok(Statement::FunctionDeclaration {
            name: name.unwrap(),
            params,
            body,
            is_generator,
        })
    }

    fn parse_function(
        &mut self,
        require_name: bool,
    ) -> JSResult<(Option<String>, Vec<String>, Vec<Statement>, bool)> {
        self.expect(&TokenKind::Function, "Expected function")?;
        let is_generator = self.eat(&TokenKind::Star)?;

        let name = if require_name || matches!(&self.current().kind, TokenKind::Identifier(_)) {
            Some(self.expect_identifier("Expected function name")?)
        } else {
            None
        };

        let (params, mut prologue) = self.parse_method_parameters()?;
        let mut body = self.parse_block()?;
        prologue.append(&mut body);

        Ok((name, params, prologue, is_generator))
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
        if self.check(&TokenKind::Async) {
            let mut candidate = self.clone();
            candidate.advance()?;
            if let TokenKind::Identifier(param) = &candidate.current().kind
                && matches!(candidate.next.kind, TokenKind::Arrow)
            {
                let param = param.clone();
                candidate.advance()?;
                candidate.advance()?;
                let arrow = candidate.parse_arrow_body(vec![param], Vec::new())?;
                *self = candidate;
                return Ok(Some(arrow));
            }
            if candidate.check(&TokenKind::LeftParen)
                && let Ok((params, prologue)) = candidate.parse_method_parameters()
                && candidate.eat(&TokenKind::Arrow)?
            {
                let arrow = candidate.parse_arrow_body(params, prologue)?;
                *self = candidate;
                return Ok(Some(arrow));
            }
        }

        if let TokenKind::Identifier(param) = &self.current().kind
            && matches!(self.next.kind, TokenKind::Arrow)
        {
            let param = param.clone();
            self.advance()?;
            self.advance()?;
            return self.parse_arrow_body(vec![param], Vec::new()).map(Some);
        }

        if !self.check(&TokenKind::LeftParen) {
            return Ok(None);
        }
        let mut candidate = self.clone();
        let Ok((params, prologue)) = candidate.parse_method_parameters() else {
            return Ok(None);
        };
        if !candidate.eat(&TokenKind::Arrow)? {
            return Ok(None);
        }

        let arrow = candidate.parse_arrow_body(params, prologue)?;
        *self = candidate;
        Ok(Some(arrow))
    }

    fn parse_arrow_body(
        &mut self,
        params: Vec<String>,
        mut prologue: Vec<Statement>,
    ) -> JSResult<Expression> {
        let mut body = if self.check(&TokenKind::LeftBrace) {
            self.parse_block()?
        } else {
            vec![Statement::Return(Some(self.parse_assignment()?))]
        };
        prologue.append(&mut body);
        Ok(Expression::ArrowFunction {
            params,
            body: prologue,
        })
    }

    fn parse_expression_bp(&mut self, min_bp: u8) -> JSResult<Expression> {
        // bp: Binding Power
        let mut left = self.parse_unary()?;

        while let Some((bp, op)) = precedence(&self.current().kind) {
            if bp < min_bp {
                break;
            }

            self.advance()?;

            let right =
                self.parse_expression_bp(if op == BinaryOp::Power { bp } else { bp + 1 })?;

            left = Expression::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> JSResult<Expression> {
        if self.eat(&TokenKind::Await)? {
            let value = self.parse_unary()?;
            return Ok(Expression::Call {
                callee: Box::new(Expression::Identifier("__pixi_await".to_string())),
                args: vec![value],
            });
        }
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

        let mut statements = Vec::new();
        loop {
            if self.check(&TokenKind::LeftBracket) || self.check(&TokenKind::LeftBrace) {
                let binding = self.parse_binding_pattern()?;
                self.expect(&TokenKind::Eq, "Expected initializer for destructuring")?;
                let init = self.parse_assignment()?;
                statements.push(Statement::PatternDeclaration {
                    kind,
                    binding,
                    init,
                });
            } else {
                let name = self.expect_identifier("Expected variable name")?;
                let init = if self.eat(&TokenKind::Eq)? {
                    Some(self.parse_assignment()?)
                } else {
                    None
                };
                statements.push(Statement::VariableDeclaration {
                    kind,
                    declarations: vec![(name, init)],
                });
            }

            if !self.eat(&TokenKind::Comma)? {
                break;
            }
        }

        self.consume_semicolon()?;

        if statements.len() == 1 {
            Ok(statements.pop().expect("declaration statement must exist"))
        } else {
            Ok(Statement::Block(statements))
        }
    }

    fn parse_binding_pattern(&mut self) -> JSResult<BindingPattern> {
        match &self.current().kind {
            TokenKind::Identifier(_) | TokenKind::Of | TokenKind::From | TokenKind::As => Ok(
                BindingPattern::Identifier(self.expect_identifier("Expected binding name")?),
            ),
            TokenKind::LeftBracket => {
                self.advance()?;
                let mut items = Vec::new();
                while !self.check(&TokenKind::RightBracket) {
                    if self.eat(&TokenKind::Comma)? {
                        items.push(None);
                        continue;
                    }
                    let pattern = if self.eat(&TokenKind::DotDotDot)? {
                        BindingPattern::Rest(Box::new(self.parse_binding_pattern()?))
                    } else {
                        self.parse_binding_pattern()?
                    };
                    let pattern = if self.eat(&TokenKind::Eq)? {
                        BindingPattern::Default(Box::new(pattern), self.parse_assignment()?)
                    } else {
                        pattern
                    };
                    items.push(Some(pattern));
                    if !self.check(&TokenKind::RightBracket) {
                        self.expect(&TokenKind::Comma, "Expected ',' in array pattern")?;
                    }
                }
                self.expect(&TokenKind::RightBracket, "Expected ']' after array pattern")?;
                Ok(BindingPattern::Array(items))
            }
            TokenKind::LeftBrace => {
                self.advance()?;
                let mut properties = Vec::new();
                while !self.check(&TokenKind::RightBrace) {
                    if self.eat(&TokenKind::DotDotDot)? {
                        let rest = self.parse_binding_pattern()?;
                        properties.push((String::new(), BindingPattern::Rest(Box::new(rest))));
                        if !self.check(&TokenKind::RightBrace) {
                            return Err(JSError::SyntaxError(
                                "Object rest binding must be last".to_string(),
                                self.current().span,
                            ));
                        }
                        break;
                    }
                    let key = self.expect_object_property_key()?;
                    let value = if self.eat(&TokenKind::Colon)? {
                        self.parse_binding_pattern()?
                    } else {
                        BindingPattern::Identifier(key.clone())
                    };
                    let value = if self.eat(&TokenKind::Eq)? {
                        BindingPattern::Default(Box::new(value), self.parse_assignment()?)
                    } else {
                        value
                    };
                    properties.push((key, value));
                    if !self.check(&TokenKind::RightBrace) {
                        self.expect(&TokenKind::Comma, "Expected ',' in object pattern")?;
                    }
                }
                self.expect(&TokenKind::RightBrace, "Expected '}' after object pattern")?;
                Ok(BindingPattern::Object(properties))
            }
            _ => Err(JSError::SyntaxError(
                format!("Expected binding pattern: found {:?}", self.current().kind),
                self.current().span,
            )),
        }
    }

    fn parse_function_expression(&mut self) -> JSResult<Expression> {
        let (name, params, body, is_generator) = self.parse_function(false)?;

        Ok(Expression::Function {
            name,
            params,
            body,
            is_generator,
        })
    }

    fn parse_class_declaration(&mut self) -> JSResult<Statement> {
        let class = self.parse_class_expression(true)?;
        let Expression::Class {
            name: Some(name), ..
        } = &class
        else {
            unreachable!("class declaration requires a name")
        };
        Ok(Statement::VariableDeclaration {
            kind: VarKind::Let,
            declarations: vec![(name.clone(), Some(class))],
        })
    }

    fn parse_class_expression(&mut self, require_name: bool) -> JSResult<Expression> {
        self.expect(&TokenKind::Class, "Expected class")?;
        let has_optional_name =
            matches!(&self.current().kind, TokenKind::Identifier(name) if name != "extends");
        let name = if require_name || has_optional_name {
            Some(self.expect_identifier("Expected class name")?)
        } else {
            None
        };
        let super_class = if self.eat_identifier_name("extends")? {
            Some(Box::new(self.parse_postfix()?))
        } else {
            None
        };
        self.expect(&TokenKind::LeftBrace, "Expected '{' before class body")?;
        let mut constructor = None;
        let mut methods = Vec::new();
        while !self.check(&TokenKind::RightBrace) {
            if self.eat(&TokenKind::Semicolon)? {
                continue;
            }
            let is_static = self.eat_identifier_name("static")?;
            if self.check(&TokenKind::Async) && !matches!(self.next.kind, TokenKind::LeftParen) {
                self.advance()?;
            }
            let is_generator = self.eat(&TokenKind::Star)?;
            let kind = if !is_generator
                && matches!(&self.current.kind, TokenKind::Identifier(name) if name == "get")
                && !matches!(self.next.kind, TokenKind::LeftParen)
            {
                self.advance()?;
                ClassMethodKind::Getter
            } else if !is_generator
                && matches!(&self.current.kind, TokenKind::Identifier(name) if name == "set")
                && !matches!(self.next.kind, TokenKind::LeftParen)
            {
                self.advance()?;
                ClassMethodKind::Setter
            } else {
                ClassMethodKind::Method
            };
            let (method_name, computed_name) = if self.eat(&TokenKind::LeftBracket)? {
                let expression = self.parse_expression()?;
                self.expect(&TokenKind::RightBracket, "Expected ']' after method name")?;
                ("<computed>".to_string(), Some(Box::new(expression)))
            } else {
                (
                    self.expect_identifier_name("Expected class method name")?,
                    None,
                )
            };
            let (params, mut prologue) = self.parse_method_parameters()?;
            let mut body = self.parse_block()?;
            prologue.append(&mut body);
            let method = ClassMethod {
                name: method_name.clone(),
                computed_name,
                params,
                body: prologue,
                is_static,
                is_generator,
                kind,
            };
            if method_name == "constructor" && !is_static && kind == ClassMethodKind::Method {
                constructor = Some(method);
            } else {
                methods.push(method);
            }
        }
        self.expect(&TokenKind::RightBrace, "Expected '}' after class body")?;
        Ok(Expression::Class {
            name,
            super_class,
            constructor,
            methods,
        })
    }

    fn parse_method_parameters(&mut self) -> JSResult<(Vec<String>, Vec<Statement>)> {
        self.expect(
            &TokenKind::LeftParen,
            "Expected '(' before method parameters",
        )?;
        let mut params = Vec::new();
        let mut prologue = Vec::new();
        while !self.check(&TokenKind::RightParen) {
            let rest = self.eat(&TokenKind::DotDotDot)?;
            let pattern = if matches!(
                self.current().kind,
                TokenKind::LeftBracket | TokenKind::LeftBrace
            ) {
                Some(self.parse_binding_pattern()?)
            } else {
                None
            };
            let parameter = if pattern.is_some() {
                let name = format!("__pixi_parameter_{}", self.next_temporary);
                self.next_temporary += 1;
                name
            } else {
                self.expect_identifier("Expected parameter name")?
            };
            params.push(if rest {
                format!("...{parameter}")
            } else {
                parameter.clone()
            });
            if self.eat(&TokenKind::Eq)? {
                let default = self.parse_assignment()?;
                prologue.push(Statement::If {
                    test: Expression::Binary {
                        op: BinaryOp::StrictEq,
                        left: Box::new(Expression::Identifier(parameter.clone())),
                        right: Box::new(Expression::Literal(Literal::Undefined)),
                    },
                    consequent: vec![Statement::Expression(Expression::Assignment {
                        left: Box::new(Expression::Identifier(parameter.clone())),
                        right: Box::new(default),
                    })],
                    alternate: None,
                });
            }
            if let Some(pattern) = pattern {
                prologue.push(Statement::PatternDeclaration {
                    kind: VarKind::Let,
                    binding: pattern,
                    init: Expression::Identifier(parameter),
                });
            }
            if rest {
                if !self.check(&TokenKind::RightParen) {
                    return Err(JSError::SyntaxError(
                        "Rest parameter must be last".to_string(),
                        self.current().span,
                    ));
                }
                break;
            }
            if !self.check(&TokenKind::RightParen) {
                self.expect(&TokenKind::Comma, "Expected ',' between parameters")?;
            }
        }
        self.expect(
            &TokenKind::RightParen,
            "Expected ')' after method parameters",
        )?;
        Ok((params, prologue))
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
        let mut optional_chain = false;

        loop {
            if self.eat(&TokenKind::OptionalChain)? {
                optional_chain = true;
                if self.eat(&TokenKind::LeftParen)? {
                    let args = self.parse_arguments()?;
                    self.expect(&TokenKind::RightParen, "Expected ')'")?;
                    expr = Expression::OptionalCall {
                        callee: Box::new(expr),
                        args,
                    };
                } else if self.eat(&TokenKind::LeftBracket)? {
                    let property = self.parse_expression()?;
                    self.expect(&TokenKind::RightBracket, "Expected ']'")?;
                    expr = Expression::OptionalMemberAccess {
                        object: Box::new(expr),
                        property: Box::new(property),
                        computed: true,
                    };
                } else {
                    let property = self.expect_identifier_name("Expected property name")?;
                    expr = Expression::OptionalMemberAccess {
                        object: Box::new(expr),
                        property: Box::new(Expression::Literal(Literal::String(property))),
                        computed: false,
                    };
                }
            } else if self.eat(&TokenKind::Dot)? {
                let property = self.expect_identifier_name("Expected property name")?;

                let member = if optional_chain {
                    Expression::OptionalMemberAccess {
                        object: Box::new(expr),
                        property: Box::new(Expression::Literal(Literal::String(property))),
                        computed: false,
                    }
                } else {
                    Expression::MemberAccess {
                        object: Box::new(expr),
                        property: Box::new(Expression::Literal(Literal::String(property))),
                        computed: false,
                    }
                };
                expr = member;
            } else if self.eat(&TokenKind::LeftBracket)? {
                let property = self.parse_expression()?;

                self.expect(&TokenKind::RightBracket, "Expected ']'")?;

                expr = if optional_chain {
                    Expression::OptionalMemberAccess {
                        object: Box::new(expr),
                        property: Box::new(property),
                        computed: true,
                    }
                } else {
                    Expression::MemberAccess {
                        object: Box::new(expr),
                        property: Box::new(property),
                        computed: true,
                    }
                };
            } else if self.eat(&TokenKind::LeftParen)? {
                let args = self.parse_arguments()?;

                self.expect(&TokenKind::RightParen, "Expected ')'")?;

                expr = if optional_chain {
                    Expression::OptionalCall {
                        callee: Box::new(expr),
                        args,
                    }
                } else {
                    Expression::Call {
                        callee: Box::new(expr),
                        args,
                    }
                };
            } else if let TokenKind::TemplateLiteral {
                strings,
                raw_strings,
                expressions,
            } = &self.current().kind
            {
                let strings = strings.clone();
                let raw_strings = raw_strings.clone();
                let expressions = expressions.clone();
                self.advance()?;
                let mut args = vec![Expression::TemplateObject {
                    cooked: strings,
                    raw: raw_strings,
                }];
                for source in expressions {
                    let mut parser = Parser::new(Lexer::new(&source))?;
                    let value = parser.parse_expression()?;
                    if !parser.is_at_end() {
                        return Err(JSError::SyntaxError(
                            "Unexpected token in tagged template expression".to_string(),
                            parser.current().span,
                        ));
                    }
                    args.push(value);
                }
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
            if self.eat(&TokenKind::DotDotDot)? {
                args.push(Expression::Spread(Box::new(self.parse_assignment()?)));
            } else {
                args.push(self.parse_assignment()?);
            }

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
            TokenKind::BigIntLiteral(n) => {
                let n = n.clone();
                self.advance()?;
                Ok(Expression::Literal(Literal::BigInt(n)))
            }
            TokenKind::String(s) => {
                let s = s.clone();

                self.advance()?;

                Ok(Expression::Literal(Literal::String(s)))
            }
            TokenKind::TemplateLiteral {
                strings,
                raw_strings: _,
                expressions,
            } => {
                let strings = strings.clone();
                let expressions = expressions.clone();
                self.advance()?;

                let mut result = Expression::Literal(Literal::String(
                    strings.first().cloned().unwrap_or_default(),
                ));
                for (index, source) in expressions.into_iter().enumerate() {
                    let mut parser = Parser::new(Lexer::new(&source))?;
                    let expression = parser.parse_expression()?;
                    if !parser.is_at_end() {
                        return Err(JSError::SyntaxError(
                            "Unexpected token in template expression".to_string(),
                            parser.current().span,
                        ));
                    }
                    result = Expression::Binary {
                        op: BinaryOp::Add,
                        left: Box::new(result),
                        right: Box::new(expression),
                    };
                    result = Expression::Binary {
                        op: BinaryOp::Add,
                        left: Box::new(result),
                        right: Box::new(Expression::Literal(Literal::String(
                            strings.get(index + 1).cloned().unwrap_or_default(),
                        ))),
                    };
                }
                Ok(result)
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
            TokenKind::Super => {
                self.advance()?;
                Ok(Expression::Super)
            }
            TokenKind::Identifier(s) => {
                let s = s.clone();

                self.advance()?;

                if s == "yield" {
                    let delegate = self.eat(&TokenKind::Star)?;
                    let value = self.parse_assignment()?;
                    return Ok(Expression::Yield {
                        value: Box::new(value),
                        delegate,
                    });
                }

                Ok(Expression::Identifier(s))
            }
            TokenKind::Of | TokenKind::From | TokenKind::As | TokenKind::Async => {
                if self.check(&TokenKind::Async) && matches!(self.next.kind, TokenKind::Function) {
                    self.advance()?;
                    return self.parse_function_expression();
                }
                let name = match self.current().kind {
                    TokenKind::Of => "of",
                    TokenKind::From => "from",
                    TokenKind::As => "as",
                    TokenKind::Async => "async",
                    _ => unreachable!(),
                }
                .to_string();
                self.advance()?;
                Ok(Expression::Identifier(name))
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
            TokenKind::Class => self.parse_class_expression(false),
            TokenKind::New => self.parse_new_expression(),

            _ => Err(JSError::SyntaxError(
                format!("Unexpected token {:?}", self.current().kind),
                self.current().span,
            )),
        }
    }

    fn parse_new_expression(&mut self) -> JSResult<Expression> {
        self.advance()?; // consume 'new'
        let mut callee = self.parse_primary()?;
        loop {
            if self.eat(&TokenKind::Dot)? {
                let property = self.expect_identifier_name("Expected property name")?;
                callee = Expression::MemberAccess {
                    object: Box::new(callee),
                    property: Box::new(Expression::Literal(Literal::String(property))),
                    computed: false,
                };
            } else if self.eat(&TokenKind::LeftBracket)? {
                let property = self.parse_expression()?;
                self.expect(
                    &TokenKind::RightBracket,
                    "Expected ']' after constructor property",
                )?;
                callee = Expression::MemberAccess {
                    object: Box::new(callee),
                    property: Box::new(property),
                    computed: true,
                };
            } else {
                break;
            }
        }
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

            if self.eat(&TokenKind::DotDotDot)? {
                elements.push(Expression::Spread(Box::new(self.parse_assignment()?)));
            } else {
                elements.push(self.parse_assignment()?);
            }

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
            if self.eat(&TokenKind::DotDotDot)? {
                properties.push(ObjectProperty::Spread(self.parse_assignment()?));
                if !self.check(&TokenKind::RightBrace) && !self.eat(&TokenKind::Comma)? {
                    return Err(JSError::SyntaxError(
                        "Expected ',' or '}' after object spread".into(),
                        self.current().span,
                    ));
                }
                continue;
            }

            if self.eat(&TokenKind::LeftBracket)? {
                let key = self.parse_expression()?;
                self.expect(&TokenKind::RightBracket, "Expected ']' after computed key")?;
                self.expect(&TokenKind::Colon, "Expected ':' after computed key")?;
                let value = self.parse_assignment()?;
                properties.push(ObjectProperty::ComputedData { key, value });
                if !self.check(&TokenKind::RightBrace) && !self.eat(&TokenKind::Comma)? {
                    return Err(JSError::SyntaxError(
                        "Expected ',' or '}' in object literal".into(),
                        self.current().span,
                    ));
                }
                continue;
            }
            if let TokenKind::Identifier(kind) = &self.current().kind
                && (kind == "get" || kind == "set")
                && !matches!(
                    self.next.kind,
                    TokenKind::Colon | TokenKind::Comma | TokenKind::LeftParen
                )
            {
                let is_getter = kind == "get";
                self.advance()?;
                let key = self.expect_object_property_key()?;
                self.expect(&TokenKind::LeftParen, "Expected '(' after accessor name")?;
                let property = if is_getter {
                    self.expect(&TokenKind::RightParen, "Expected ')' after getter name")?;
                    ObjectProperty::Getter {
                        key,
                        body: self.parse_block()?,
                    }
                } else {
                    let parameter = self.expect_identifier("Expected setter parameter")?;
                    self.expect(
                        &TokenKind::RightParen,
                        "Expected ')' after setter parameter",
                    )?;
                    ObjectProperty::Setter {
                        key,
                        parameter,
                        body: self.parse_block()?,
                    }
                };
                properties.push(property);

                if !self.check(&TokenKind::RightBrace) && !self.eat(&TokenKind::Comma)? {
                    return Err(JSError::SyntaxError(
                        "Expected ',' or '}' in object literal".into(),
                        self.current().span,
                    ));
                }
                continue;
            }

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

            let value = if self.check(&TokenKind::LeftParen) {
                let (params, mut prologue) = self.parse_method_parameters()?;
                let mut body = self.parse_block()?;
                prologue.append(&mut body);
                Expression::Function {
                    name: Some(key.clone()),
                    params,
                    body: prologue,
                    is_generator: false,
                }
            } else if self.eat(&TokenKind::Colon)? {
                self.parse_assignment()?
            } else if let Some(value) = shorthand {
                value
            } else {
                return Err(JSError::SyntaxError(
                    "Expected ':' after property key".to_string(),
                    self.current().span,
                ));
            };

            properties.push(ObjectProperty::Data { key, value });

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

    fn expect_object_property_key(&mut self) -> JSResult<String> {
        match &self.current().kind {
            TokenKind::String(key) | TokenKind::NumberLiteral(key) => {
                let key = key.clone();
                self.advance()?;
                Ok(key)
            }
            _ => self.expect_identifier_name("Expected property key"),
        }
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
        let identifier = match &self.current().kind {
            TokenKind::Identifier(identifier) => Some(identifier.clone()),
            TokenKind::Of => Some("of".to_string()),
            TokenKind::From => Some("from".to_string()),
            TokenKind::As => Some("as".to_string()),
            TokenKind::Async => Some("async".to_string()),
            _ => None,
        };
        if let Some(identifier) = identifier {
            self.advance()?;
            Ok(identifier)
        } else {
            Err(JSError::SyntaxError(
                format!("{}: found {:?}", message, self.current().kind),
                self.current().span,
            ))
        }
    }

    fn eat_identifier_name(&mut self, expected: &str) -> JSResult<bool> {
        if matches!(&self.current().kind, TokenKind::Identifier(name) if name == expected) {
            self.advance()?;
            Ok(true)
        } else {
            Ok(false)
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
        TokenKind::Nullish => Some((1, BinaryOp::Nullish)),
        TokenKind::And => Some((2, BinaryOp::And)),

        // bitwise
        TokenKind::BitOr => Some((3, BinaryOp::BitOr)),
        TokenKind::BitXor => Some((4, BinaryOp::BitXor)),
        TokenKind::BitAnd => Some((5, BinaryOp::BitAnd)),

        // equality
        TokenKind::EqEqEq => Some((6, BinaryOp::StrictEq)),
        TokenKind::EqEq => Some((6, BinaryOp::Eq)),
        TokenKind::NotEqEq => Some((6, BinaryOp::StrictNotEq)),
        TokenKind::NotEq => Some((6, BinaryOp::NotEq)),

        // relational
        TokenKind::Lt => Some((7, BinaryOp::Lt)),
        TokenKind::Gt => Some((7, BinaryOp::Gt)),
        TokenKind::LtEq => Some((7, BinaryOp::LtEq)),
        TokenKind::GtEq => Some((7, BinaryOp::GtEq)),
        TokenKind::In => Some((7, BinaryOp::In)),
        TokenKind::Instanceof => Some((7, BinaryOp::Instanceof)),

        // shift
        TokenKind::LeftShift => Some((8, BinaryOp::LeftShift)),
        TokenKind::RightShift => Some((8, BinaryOp::RightShift)),
        TokenKind::UnsignedRightShift => Some((8, BinaryOp::UnsignedRightShift)),

        // additive
        TokenKind::Plus => Some((9, BinaryOp::Add)),
        TokenKind::Minus => Some((9, BinaryOp::Sub)),

        // multiplicative
        TokenKind::Star => Some((10, BinaryOp::Mul)),
        TokenKind::Slash => Some((10, BinaryOp::Div)),
        TokenKind::Percent => Some((10, BinaryOp::Mod)),

        // exponentiation (right associative)
        TokenKind::Power => Some((11, BinaryOp::Power)),

        _ => None,
    }
}
