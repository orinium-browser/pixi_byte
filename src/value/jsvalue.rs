use std::fmt;

/// JavaScript の値型
#[derive(Debug, Clone, PartialEq)]
pub enum JSValue {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    // TODO: Object, Symbol, BigInt 等は後のフェーズで実装
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
        }
    }

    /// 値を真偽値に変換（ToBoolean 抽象操作）
    pub fn to_boolean(&self) -> bool {
        match self {
            JSValue::Undefined | JSValue::Null => false,
            JSValue::Boolean(b) => *b,
            JSValue::Number(n) => !n.is_nan() && *n != 0.0,
            JSValue::String(s) => !s.is_empty(),
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

impl fmt::Display for JSValue {
    /// 値をフォーマット表示
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_string() {
        assert_eq!(JSValue::Undefined.to_string(), "undefined");
        assert_eq!(JSValue::Null.to_string(), "null");
        assert_eq!(JSValue::Boolean(true).to_string(), "true");
        assert_eq!(JSValue::Number(42.0).to_string(), "42");
        assert_eq!(JSValue::String("hello".to_string()).to_string(), "hello");
    }

    #[test]
    fn test_to_number() {
        assert!(JSValue::Undefined.to_number().is_nan());
        assert_eq!(JSValue::Null.to_number(), 0.0);
        assert_eq!(JSValue::Boolean(true).to_number(), 1.0);
        assert_eq!(JSValue::Boolean(false).to_number(), 0.0);
        assert_eq!(JSValue::Number(42.5).to_number(), 42.5);
        assert_eq!(JSValue::String("123".to_string()).to_number(), 123.0);
    }

    #[test]
    fn test_to_boolean() {
        assert!(!JSValue::Undefined.to_boolean());
        assert!(!JSValue::Null.to_boolean());
        assert!(JSValue::Boolean(true).to_boolean());
        assert!(!JSValue::Boolean(false).to_boolean());
        assert!(JSValue::Number(1.0).to_boolean());
        assert!(!JSValue::Number(0.0).to_boolean());
        assert!(JSValue::String("hello".to_string()).to_boolean());
        assert!(!JSValue::String("".to_string()).to_boolean());
    }

    #[test]
    fn test_strict_equals() {
        assert!(JSValue::Undefined.strict_equals(&JSValue::Undefined));
        assert!(JSValue::Null.strict_equals(&JSValue::Null));
        assert!(!JSValue::Null.strict_equals(&JSValue::Undefined));
        assert!(JSValue::Number(42.0).strict_equals(&JSValue::Number(42.0)));
        assert!(!JSValue::Number(42.0).strict_equals(&JSValue::String("42".to_string())));
    }

    #[test]
    fn test_abstract_equals() {
        assert!(JSValue::Null.abstract_equals(&JSValue::Undefined));
        assert!(JSValue::Number(42.0).abstract_equals(&JSValue::String("42".to_string())));
        assert!(JSValue::Boolean(true).abstract_equals(&JSValue::Number(1.0)));
    }
}