use std::fmt;

pub type JSResult<T> = Result<T, JSError>;

/// JavaScript エラー型
#[derive(Debug, Clone)]
pub enum JSError {
    /// 構文エラー
    SyntaxError(String),
    /// 参照エラー
    ReferenceError(String),
    /// 型エラー
    TypeError(String),
    /// 範囲エラー
    RangeError(String),
    /// 内部エラー
    InternalError(String),
}

impl fmt::Display for JSError {
    /// エラーをフォーマット表示
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JSError::SyntaxError(msg) => write!(f, "SyntaxError: {}", msg),
            JSError::ReferenceError(msg) => write!(f, "ReferenceError: {}", msg),
            JSError::TypeError(msg) => write!(f, "TypeError: {}", msg),
            JSError::RangeError(msg) => write!(f, "RangeError: {}", msg),
            JSError::InternalError(msg) => write!(f, "InternalError: {}", msg),
        }
    }
}

impl std::error::Error for JSError {}
