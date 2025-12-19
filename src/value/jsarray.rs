use std::rc::Rc;
use std::cell::RefCell;
use super::{JSValue, JSObject};

/// JavaScript 配列の内部表現
#[derive(Debug, Clone)]
pub struct JSArray {
    /// 配列要素（密な配列として扱う）
    elements: Vec<JSValue>,
    /// オブジェクトとしてのプロパティ（継承）
    object: JSObject,
}

impl JSArray {
    /// 新しい空の配列を作成
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            object: JSObject::new(),
        }
    }

    /// 配列から作成
    pub fn from_vec(elements: Vec<JSValue>) -> Self {
        Self {
            elements,
            object: JSObject::new(),
        }
    }

    /// length プロパティを取得
    pub fn length(&self) -> usize {
        self.elements.len()
    }

    /// インデックスで要素を取得
    pub fn get(&self, index: usize) -> JSValue {
        self.elements.get(index).cloned().unwrap_or(JSValue::Undefined)
    }

    /// インデックスで要素を設定
    pub fn set(&mut self, index: usize, value: JSValue) {
        // インデックスが配列の長さを超える場合、undefinedで埋める
        if index >= self.elements.len() {
            self.elements.resize(index + 1, JSValue::Undefined);
        }
        self.elements[index] = value;
    }

    /// 配列の末尾に要素を追加（push）
    pub fn push(&mut self, value: JSValue) {
        self.elements.push(value);
    }

    /// 配列の末尾から要素を削除（pop）
    pub fn pop(&mut self) -> JSValue {
        self.elements.pop().unwrap_or(JSValue::Undefined)
    }

    /// 配列の先頭に要素を追加（unshift）
    pub fn unshift(&mut self, value: JSValue) {
        self.elements.insert(0, value);
    }

    /// 配列の先頭から要素を削除（shift）
    pub fn shift(&mut self) -> JSValue {
        if self.elements.is_empty() {
            JSValue::Undefined
        } else {
            self.elements.remove(0)
        }
    }

    /// 配列をJSObjectに変換
    pub fn to_object(self) -> JSValue {
        // 現在の実装では、配列は単純にオブジェクトとして扱う
        // 将来的には、Array.prototypeを持つオブジェクトとして実装
        let mut obj = self.object;
        let len = self.elements.len();

        // 配列要素をプロパティとして設定
        for (i, value) in self.elements.into_iter().enumerate() {
            obj.set(i.to_string(), value);
        }

        // lengthプロパティを設定
        obj.set("length".to_string(), JSValue::Number(len as f64));

        JSValue::Object(Rc::new(RefCell::new(obj)))
    }

    /// 配列の参照を取得
    pub fn as_ref(&self) -> &JSObject {
        &self.object
    }

    /// 配列の可変参照を取得
    pub fn as_mut(&mut self) -> &mut JSObject {
        &mut self.object
    }
}

impl Default for JSArray {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_array_create() {
        let arr = JSArray::new();
        assert_eq!(arr.length(), 0);
    }

    #[test]
    fn test_array_push_pop() {
        let mut arr = JSArray::new();
        arr.push(JSValue::Number(1.0));
        arr.push(JSValue::Number(2.0));
        arr.push(JSValue::Number(3.0));

        assert_eq!(arr.length(), 3);
        assert_eq!(arr.pop(), JSValue::Number(3.0));
        assert_eq!(arr.length(), 2);
    }

    #[test]
    fn test_array_get_set() {
        let mut arr = JSArray::new();
        arr.set(0, JSValue::String("first".to_string()));
        arr.set(2, JSValue::String("third".to_string()));

        assert_eq!(arr.get(0), JSValue::String("first".to_string()));
        assert_eq!(arr.get(1), JSValue::Undefined);
        assert_eq!(arr.get(2), JSValue::String("third".to_string()));
    }

    #[test]
    fn test_array_shift_unshift() {
        let mut arr = JSArray::from_vec(vec![
            JSValue::Number(2.0),
            JSValue::Number(3.0),
        ]);

        arr.unshift(JSValue::Number(1.0));
        assert_eq!(arr.length(), 3);
        assert_eq!(arr.get(0), JSValue::Number(1.0));

        let first = arr.shift();
        assert_eq!(first, JSValue::Number(1.0));
        assert_eq!(arr.length(), 2);
    }

    #[test]
    fn test_array_from_vec() {
        let arr = JSArray::from_vec(vec![
            JSValue::String("a".to_string()),
            JSValue::String("b".to_string()),
            JSValue::String("c".to_string()),
        ]);

        assert_eq!(arr.length(), 3);
        assert_eq!(arr.get(1), JSValue::String("b".to_string()));
    }
}

