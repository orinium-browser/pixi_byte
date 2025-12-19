use super::jsobject::JSObject;
use crate::compiler::BytecodeChunk;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

/// JavaScript の値型
#[derive(Debug, Clone)]
pub enum JSValue {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Object(Rc<RefCell<JSObject>>),
    Function(BytecodeChunk, Vec<String>),
    // TODO: Symbol, BigInt 等は後のフェーズで実装
}

impl JSValue {
    /// 値を文字列に変換（ToString 抽象操作）
    pub fn to_string(&self) -> String {
        match self {
            JSValue::Undefined => "undefined".to_string(),
            JSValue::Null => "null".to_string(),
            JSValue::Boolean(b) => b.to_string(),
            JSValue::Number(n) => {
                if n.is_nan() {
                    "NaN".to_string()
                } else if n.is_infinite() {
                    if n.is_sign_positive() {
                        "Infinity".to_string()
                    } else {
                        "-Infinity".to_string()
                    }
                } else {
                    n.to_string()
                }
            }
            JSValue::String(s) => s.clone(),
            JSValue::Object(_) => "[object Object]".to_string(),
            JSValue::Function(_, _) => "[function]".to_string(),
        }
    }

    /// 値を数値に変換（ToNumber 抽象操作）
    pub fn to_number(&self) -> f64 {
        match self {
            JSValue::Undefined => f64::NAN,
            JSValue::Null => 0.0,
            JSValue::Boolean(true) => 1.0,
            JSValue::Boolean(false) => 0.0,
            JSValue::Number(n) => *n,
            JSValue::String(s) => {
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    return 0.0;
                }
                trimmed.parse().unwrap_or(f64::NAN)
            }
            JSValue::Object(_) => f64::NAN,      // オブジェクトはNaN
            JSValue::Function(_, _) => f64::NAN, // 関数もNaN
        }
    }

    /// 値を真偽値に変換（ToBoolean 抽象操作）
    pub fn to_boolean(&self) -> bool {
        match self {
            JSValue::Undefined | JSValue::Null => false,
            JSValue::Boolean(b) => *b,
            JSValue::Number(n) => !n.is_nan() && *n != 0.0,
            JSValue::String(s) => !s.is_empty(),
            JSValue::Object(_) => true,      // オブジェクトは常にtrue
            JSValue::Function(_, _) => true, // 関数も常にtrue
        }
    }

    /// 型名を取得
    pub fn type_of(&self) -> &'static str {
        match self {
            JSValue::Undefined => "undefined",
            JSValue::Null => "object", // JavaScriptの歴史的バグ
            JSValue::Boolean(_) => "boolean",
            JSValue::Number(_) => "number",
            JSValue::String(_) => "string",
            JSValue::Object(_) => "object",
            JSValue::Function(_, _) => "function",
        }
    }

    /// 厳密等価比較（===）
    pub fn strict_equals(&self, other: &JSValue) -> bool {
        match (self, other) {
            (JSValue::Undefined, JSValue::Undefined) => true,
            (JSValue::Null, JSValue::Null) => true,
            (JSValue::Boolean(a), JSValue::Boolean(b)) => a == b,
            (JSValue::Number(a), JSValue::Number(b)) => {
                if a.is_nan() || b.is_nan() {
                    false
                } else {
                    a == b
                }
            }
            (JSValue::String(a), JSValue::String(b)) => a == b,
            (JSValue::Object(a), JSValue::Object(b)) => {
                // オブジェクトは参照が同じ場合のみtrue
                Rc::ptr_eq(a, b)
            }
            _ => false,
        }
    }

    /// 抽象等価比較（==）
    pub fn abstract_equals(&self, other: &JSValue) -> bool {
        // 同じ型の場合は厳密等価
        if std::mem::discriminant(self) == std::mem::discriminant(other) {
            return self.strict_equals(other);
        }

        match (self, other) {
            // null == undefined
            (JSValue::Null, JSValue::Undefined) | (JSValue::Undefined, JSValue::Null) => true,

            // 数値と文字列の比較
            (JSValue::Number(n), JSValue::String(_)) => *n == other.to_number(),
            (JSValue::String(_), JSValue::Number(n)) => self.to_number() == *n,

            // 真偽値は数値に変換して比較
            (JSValue::Boolean(_), _) => JSValue::Number(self.to_number()).abstract_equals(other),
            (_, JSValue::Boolean(_)) => self.abstract_equals(&JSValue::Number(other.to_number())),

            _ => false,
        }
    }
}

impl PartialEq for JSValue {
    fn eq(&self, other: &Self) -> bool {
        self.strict_equals(other)
    }
}

impl fmt::Display for JSValue {
    /// 値をフォーマット表示
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}
