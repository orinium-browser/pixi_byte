use std::fmt;

/// JSトークンの種類
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // リテラル
    NumberLiteral(String),
    String(String),
    True,
    False,
    Null,
    Undefined,

    // 識別子とキーワード
    Identifier(String),

    // キーワード
    Let,
    Const,
    Var,
    Function,
    Return,
    If,
    Else,
    For,
    While,
    Break,
    Continue,
    Class,
    New,
    This,
    Super,
    Import,
    Export,
    From,
    As,
    Async,
    Await,
    Try,
    Catch,
    Finally,
    Throw,
    Typeof,
    Delete,
    Void,
    In,
    Of,
    Instanceof,

    // 演算子
    Plus,    // +
    Minus,   // -
    Star,    // *
    Slash,   // /
    Percent, // %
    Power,   // **

    Eq,      // =
    EqEq,    // ==
    EqEqEq,  // ===
    NotEq,   // !=
    NotEqEq, // !==

    Lt,   // <
    Gt,   // >
    LtEq, // <=
    GtEq, // >=

    And, // &&
    Or,  // ||
    Not, // !

    BitAnd,             // &
    BitOr,              // |
    BitXor,             // ^
    BitNot,             // ~
    LeftShift,          // <<
    RightShift,         // >>
    UnsignedRightShift, // >>>

    PlusPlus,   // ++
    MinusMinus, // --

    Question, // ?
    Colon,    // :

    // 代入演算子
    PlusEq,    // +=
    MinusEq,   // -=
    StarEq,    // *=
    SlashEq,   // /=
    PercentEq, // %=

    // 区切り文字
    LeftParen,    // (
    RightParen,   // )
    LeftBrace,    // {
    RightBrace,   // }
    LeftBracket,  // [
    RightBracket, // ]

    Semicolon, // ;
    Comma,     // ,
    Dot,       // .
    DotDotDot, // ...
    Arrow,     // =>

    // 特殊
    Eof,
    Unknown(char),
}

/// トークン位置情報
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

impl Span {
    /// 新しいSpanを作成
    pub fn new(start: usize, end: usize, line: usize, column: usize) -> Self {
        Self {
            start,
            end,
            line,
            column,
        }
    }
}

/// トークン
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    /// 新しいトークンを作成
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

impl fmt::Display for TokenKind {
    /// トークンの種類をフォーマット表示
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::NumberLiteral(n) => write!(f, "JsNumberLiteral({})", n),
            TokenKind::String(s) => write!(f, "JsString(\"{}\")", s),
            TokenKind::Identifier(s) => write!(f, "JsIdentifier({})", s),
            _ => write!(f, "{:?}", self),
        }
    }
}
