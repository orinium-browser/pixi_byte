use super::JSValue;
use rustc_hash::FxHashMap;
use std::cell::RefCell;
use std::rc::Rc;

/// JavaScript オブジェクトの内部表現
#[derive(Debug, Clone)]
pub struct JSObject {
    /// プロパティマップ
    properties: Rc<RefCell<FxHashMap<String, Property>>>,
    /// プロトタイプチェーン（__proto__）
    prototype: Option<Rc<RefCell<JSObject>>>,
}

/// プロパティディスクリプタ
#[derive(Debug, Clone)]
pub struct Property {
    /// プロパティの値
    pub value: JSValue,
    /// 列挙可能かどうか
    pub enumerable: bool,
    /// 書き込み可能かどうか
    pub writable: bool,
    /// 設定変更可能かどうか
    pub configurable: bool,
}

impl Property {
    /// データプロパティを作成（デフォルト設定）
    pub fn data(value: JSValue) -> Self {
        Self {
            value,
            enumerable: true,
            writable: true,
            configurable: true,
        }
    }

    /// 読み取り専用プロパティを作成
    pub fn read_only(value: JSValue) -> Self {
        Self {
            value,
            enumerable: true,
            writable: false,
            configurable: false,
        }
    }
}

impl JSObject {
    /// 新しい空のJSオブジェクトを作成
    pub fn new() -> Self {
        Self {
            properties: Rc::new(RefCell::new(FxHashMap::default())),
            prototype: None,
        }
    }

    /// プロトタイプを指定してオブジェクトを作成
    pub fn with_prototype(prototype: Option<Rc<RefCell<JSObject>>>) -> Self {
        Self {
            properties: Rc::new(RefCell::new(FxHashMap::default())),
            prototype,
        }
    }

    /// プロパティを取得
    pub fn get(&self, key: &str) -> JSValue {
        // 自身のプロパティを検索
        if let Some(prop) = self.properties.borrow().get(key) {
            return prop.value.clone();
        }

        // プロトタイプチェーンを辿る
        if let Some(ref proto) = self.prototype {
            return proto.borrow().get(key);
        }

        JSValue::Undefined
    }

    /// プロパティを設定
    pub fn set(&mut self, key: String, value: JSValue) -> bool {
        // 既存のプロパティを確認
        if let Some(prop) = self.properties.borrow_mut().get_mut(&key) {
            if !prop.writable {
                return false; // 書き込み不可
            }
            prop.value = value;
            return true;
        }

        // 新しいプロパティを追加
        self.properties
            .borrow_mut()
            .insert(key, Property::data(value));
        true
    }

    /// プロパティが存在するか確認（自身のプロパティのみ）
    pub fn has_own_property(&self, key: &str) -> bool {
        self.properties.borrow().contains_key(key)
    }

    /// プロパティが存在するか確認（プロトタイプチェーン含む）
    pub fn has_property(&self, key: &str) -> bool {
        if self.has_own_property(key) {
            return true;
        }

        if let Some(ref proto) = self.prototype {
            return proto.borrow().has_property(key);
        }

        false
    }

    /// プロパティを削除
    pub fn delete(&mut self, key: &str) -> bool {
        if let Some(prop) = self.properties.borrow().get(key) {
            if !prop.configurable {
                return false; // 設定変更不可
            }
        }

        self.properties.borrow_mut().remove(key).is_some()
    }

    /// プロトタイプを取得
    pub fn get_prototype(&self) -> Option<Rc<RefCell<JSObject>>> {
        self.prototype.clone()
    }

    /// プロトタイプを設定
    pub fn set_prototype(&mut self, prototype: Option<Rc<RefCell<JSObject>>>) {
        self.prototype = prototype;
    }

    /// 全てのプロパティキーを取得（列挙可能なもののみ）
    pub fn keys(&self) -> Vec<String> {
        self.properties
            .borrow()
            .iter()
            .filter(|(_, prop)| prop.enumerable)
            .map(|(key, _)| key.clone())
            .collect()
    }

    /// プロパティディスクリプタを定義
    pub fn define_property(&mut self, key: String, property: Property) {
        self.properties.borrow_mut().insert(key, property);
    }

    /// プロパティディスクリプタを取得
    pub fn get_property_descriptor(&self, key: &str) -> Option<Property> {
        self.properties.borrow().get(key).cloned()
    }
}

impl Default for JSObject {
    fn default() -> Self {
        Self::new()
    }
}
