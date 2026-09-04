use super::{JSObject, JSValue, Property};
use std::cell::RefCell;
use std::rc::Rc;

/// JavaScript 配列の内部表現
#[derive(Debug, Clone)]
pub struct JSArray {
    /// 配列要素（密な配列として扱う）
    elements: Vec<JSValue>,
}

impl JSArray {
    /// 新しい空の配列を作成
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
        }
    }

    /// 配列から作成
    pub fn from_vec(elements: Vec<JSValue>) -> Self {
        Self { elements }
    }

    /// length プロパティを取得
    pub fn length(&self) -> usize {
        self.elements.len()
    }

    /// インデックスで要素を取得
    pub fn get(&self, index: usize) -> JSValue {
        self.elements
            .get(index)
            .cloned()
            .unwrap_or_else(JSValue::undefined)
    }

    /// インデックスで要素を設定
    pub fn set(&mut self, index: usize, value: JSValue) {
        // インデックスが配列の長さを超える場合、undefinedで埋める
        if index >= self.elements.len() {
            self.elements.resize(index + 1, JSValue::undefined());
        }
        self.elements[index] = value;
    }

    /// 配列の末尾に要素を追加（push）
    pub fn push(&mut self, value: JSValue) {
        self.elements.push(value);
    }

    /// 配列の末尾から要素を削除（pop）
    pub fn pop(&mut self) -> JSValue {
        self.elements.pop().unwrap_or_else(JSValue::undefined)
    }

    /// 配列の先頭に要素を追加（unshift）
    pub fn unshift(&mut self, value: JSValue) {
        self.elements.insert(0, value);
    }

    /// 配列の先頭から要素を削除（shift）
    pub fn shift(&mut self) -> JSValue {
        if self.elements.is_empty() {
            JSValue::undefined()
        } else {
            self.elements.remove(0)
        }
    }

    /// 配列をJSObjectに変換
    pub fn to_object(self) -> JSValue {
        let len = self.elements.len();

        // 要素ベクトルをそのまま JSObject の要素ストレージへ移す。
        // 従来は要素ごとに i.to_string() で String キーを作って HashMap に
        // 展開していたが、その変換コスト（String 生成 / ハッシュ / Property 生成）を
        // 排除するため、dense な Vec のまま共有する。
        let mut obj = JSObject::with_elements(self.elements);

        obj.define_property(
            "__pixi_array__".to_string(),
            Property {
                value: JSValue::from_bool(true),
                enumerable: false,
                writable: false,
                configurable: false,
                getter: None,
                setter: None,
            },
        );
        obj.define_property(
            "length".to_string(),
            Property {
                value: JSValue::from_number(len as f64),
                enumerable: false,
                writable: true,
                configurable: false,
                getter: None,
                setter: None,
            },
        );

        JSValue::from_object(Rc::new(RefCell::new(obj)))
    }
}

impl Default for JSArray {
    fn default() -> Self {
        Self::new()
    }
}
