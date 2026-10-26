use std::fmt;

/// JSトークンの種類
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // リテラル
    NumberLiteral(String),
    BigIntLiteral(String),
    String(String),
    TemplateLiteral {
        strings: Vec<String>,
        raw_strings: Vec<String>,
        expressions: Vec<String>,
    },
    RegExpLiteral(String, String),
    True,
    False,
    Null,
    Undefined,

    // 識別子とキーワード
    Identifier(String),
    PrivateName(String),

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
    Do,
    Break,
    Continue,
    Switch,
    Case,
    Default,
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

    Question,      // ?
    OptionalChain, // ?.
    Nullish,       // ??
    NullishAssign, // ??=
    AndAssign,     // &&=
    OrAssign,      // ||=
    Colon,         // :

    // 代入演算子
    PlusEq,               // +=
    MinusEq,              // -=
    StarEq,               // *=
    SlashEq,              // /=
    PercentEq,            // %=
    BitAndEq,             // &=
    BitOrEq,              // |=
    BitXorEq,             // ^=
    LeftShiftEq,          // <<=
    RightShiftEq,         // >>=
    UnsignedRightShiftEq, // >>>=

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

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{} ({}..{})",
            self.line, self.column, self.start, self.end,
        )
    }
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
            TokenKind::String(s) => write!(f, "JsString({})", truncate_debug_string(s)),
            TokenKind::RegExpLiteral(pattern, flags) => {
                let pattern_str = truncate_debug_string(pattern);
                let flags_str = truncate_debug_string(flags);
                write!(f, "JsRegExp(/{pattern_str}/{flags_str})")
            }
            TokenKind::Identifier(s) => write!(f, "JsIdentifier({})", truncate_debug_string(s)),
            _ => fmt::Display::fmt(self, f),
        }
    }
}

/// Debug helper for large token payloads to avoid gigantic logs.
fn truncate_debug_string(s: &str) -> String {
    if s.len() > 120 {
        format!("...{}...", &s[..120])
    } else {
        s.to_string()
    }
}
