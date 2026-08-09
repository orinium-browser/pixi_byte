use std::fmt;

use crate::lexer::Span;
use crate::value::JSValue;

pub type JSResult<T> = Result<T, JSError>;

/// JavaScript エラー型
#[derive(Debug, Clone)]
pub enum JSError {
    /// 構文エラー
    SyntaxError(String, Span),
    /// 参照エラー
    ReferenceError(String),
    /// 型エラー
    TypeError(String),
    /// 範囲エラー
    RangeError(String),
    /// 内部エラー
    InternalError(String),
    /// JavaScriptのthrow文によって送出された値
    Thrown(JSValue),
}

impl fmt::Display for JSError {
    /// エラーをフォーマット表示
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JSError::SyntaxError(msg, span) => write!(f, "SyntaxError: {} at {}", msg, span),
            JSError::ReferenceError(msg) => write!(f, "ReferenceError: {}", msg),
            JSError::TypeError(msg) => write!(f, "TypeError: {}", msg),
            JSError::RangeError(msg) => write!(f, "RangeError: {}", msg),
            JSError::InternalError(msg) => write!(f, "InternalError: {}", msg),
            JSError::Thrown(value) => write!(f, "Uncaught {}", value.to_console_string()),
        }
    }
}

impl std::error::Error for JSError {}
