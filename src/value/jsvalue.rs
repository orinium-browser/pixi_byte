//! JavaScript 値 (JSValue) の内部表現
//!
//! このモジュールはエンジン内部で使用する JavaScript の値表現を定義します。
//! 初期実装では enum ベースの単純な表現を使い、後で NaN-boxing 等で最適化する計画です。

use super::jsobject::JSObject;
use crate::compiler::BytecodeChunk;
use crate::runtime::Environment;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

/// ネイティブ関数の型エイリアス。
/// Rust 側で実装された組み込み関数はこのシグネチャを持ちます。
pub type NativeFunctionType = fn(&mut crate::vm::VM, Vec<JSValue>) -> crate::error::JSResult<JSValue>;

/// JavaScript の値型（Value）を表す列挙型。
///
/// - プリミティブ（Undefined, Null, Boolean, Number, String）
/// - オブジェクト/関数（Object, Function, NativeFunction）
///
/// 将来的には `Symbol`, `BigInt` などを追加します。
pub enum JSValue {
    /// undefined
    Undefined,
    /// null
    Null,
    /// boolean
    Boolean(bool),
    /// IEEE754 double
    Number(f64),
    /// Heap に格納された文字列（簡素化版）
    String(String),
    /// オブジェクト参照（内部は Rc<RefCell<JSObject>>）
    Object(Rc<RefCell<JSObject>>),
    /// Bytecode と引数名、キャプチャ環境を持つ JS 関数オブジェクト
    Function(
        BytecodeChunk,
        Vec<String>,
        Option<Rc<RefCell<Environment>>>,
        Option<String>,
    ),
    /// ネイティブ（Rust側）で実装された関数を表す。テストなどでクロージャを渡すために使用する。
    NativeFunction(NativeFunctionType),
    /// Bound function created by Function.prototype.bind
    BoundFunction(Box<BoundFunctionData>),
    // TODO: Symbol, BigInt 等は後のフェーズで実装
}

/// Internal representation for bound functions
#[derive(Debug, Clone)]
pub struct BoundFunctionData {
    pub target: Box<JSValue>,
    pub bound_this: JSValue,
    pub bound_args: Vec<JSValue>,
}

impl JSValue {
    /// 値をコンソール向け文字列に変換します（ToString 相当）。
    ///
    /// デバッグやログ出力に使います。
    pub fn to_console_string(&self) -> String {
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
            JSValue::Function(_, _, _, _) => "[function]".to_string(),
            JSValue::NativeFunction(_) => "[native function]".to_string(),
            JSValue::BoundFunction(_) => "[bound function]".to_string(),
        }
    }

    /// 値を数値に変換します（ToNumber の簡易実装）。
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
            JSValue::Object(_) => f64::NAN,      // オブジェクトはNaN（簡易）
            JSValue::Function(_, _, _, _) => f64::NAN,
            JSValue::NativeFunction(_) => f64::NAN,
            JSValue::BoundFunction(data) => data.target.to_number(),
        }
    }

    /// 値を真偽値に変換します（ToBoolean の簡易実装）。
    pub fn to_boolean(&self) -> bool {
        match self {
            JSValue::Undefined | JSValue::Null => false,
            JSValue::Boolean(b) => *b,
            JSValue::Number(n) => !n.is_nan() && *n != 0.0,
            JSValue::String(s) => !s.is_empty(),
            JSValue::Object(_) => true,      // オブジェクトは常にtrue
            JSValue::Function(_, _, _, _) => true,
            JSValue::NativeFunction(_) => true,
            JSValue::BoundFunction(_) => true,
        }
    }

    /// `typeof` の戻り値を返します（仕様に沿った簡易実装）。
    pub fn type_of(&self) -> &'static str {
        match self {
            JSValue::Undefined => "undefined",
            JSValue::Null => "object", // JavaScriptの歴史的バグ
            JSValue::Boolean(_) => "boolean",
            JSValue::Number(_) => "number",
            JSValue::String(_) => "string",
            JSValue::Object(_) => "object",
            JSValue::Function(_, _, _, _) => "function",
            JSValue::NativeFunction(_) => "function",
            JSValue::BoundFunction(_) => "function",
        }
    }

    /// 厳密等価比較（===）の簡易実装。
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
            // 簡易実装ではその他は false
            _ => false,
        }
    }

    /// 抽象等価比較（==）の簡易実装。
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

// 手動で Debug と Clone を実装
impl fmt::Debug for JSValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JSValue::Undefined => write!(f, "Undefined"),
            JSValue::Null => write!(f, "Null"),
            JSValue::Boolean(b) => write!(f, "Boolean({})", b),
            JSValue::Number(n) => write!(f, "Number({})", n),
            JSValue::String(s) => write!(f, "String(\"{}\")", s),
            JSValue::Object(_) => write!(f, "Object(...)"),
            JSValue::Function(_, _, _, _) => write!(f, "Function(...)"),
            JSValue::NativeFunction(_) => write!(f, "NativeFunction(...)"),
            JSValue::BoundFunction(_) => write!(f, "BoundFunction(...)"),
        }
    }
}

impl Clone for JSValue {
    fn clone(&self) -> Self {
        match self {
            JSValue::Undefined => JSValue::Undefined,
            JSValue::Null => JSValue::Null,
            JSValue::Boolean(b) => JSValue::Boolean(*b),
            JSValue::Number(n) => JSValue::Number(*n),
            JSValue::String(s) => JSValue::String(s.clone()),
            JSValue::Object(o) => JSValue::Object(o.clone()),
            JSValue::Function(chunk, params, env_opt, name_opt) => JSValue::Function(
                chunk.clone(),
                params.clone(),
                env_opt.clone(),
                name_opt.clone(),
            ),
            JSValue::NativeFunction(f) => JSValue::NativeFunction(*f),
            JSValue::BoundFunction(b) => JSValue::BoundFunction(Box::new((**b).clone())),
        }
    }
}

impl fmt::Display for JSValue {
    /// 値をフォーマット表示
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_console_string())
    }
}
