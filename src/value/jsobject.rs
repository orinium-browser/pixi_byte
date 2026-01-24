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
    /// プロパティの値（データプロパティ）
    pub value: JSValue,
    /// 列挙可能かどうか
    pub enumerable: bool,
    /// 書き込み可能かどうか
    pub writable: bool,
    /// 設定変更可能かどうか
    pub configurable: bool,
    /// アクセサの getter（あれば関数オブジェクト）
    pub getter: Option<JSValue>,
    /// アクセサの setter（あれば関数オブジェクト）
    pub setter: Option<JSValue>,
}

impl Property {
    /// データプロパティを作成（デフォルト設定）
    pub fn data(value: JSValue) -> Self {
        Self {
            value,
            enumerable: true,
            writable: true,
            configurable: true,
            getter: None,
            setter: None,
        }
    }

    /// 読み取り専用プロパティを作成
    pub fn read_only(value: JSValue) -> Self {
        Self {
            value,
            enumerable: true,
            writable: false,
            configurable: false,
            getter: None,
            setter: None,
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
        if let Some(prop) = self.properties.borrow().get(key)
            && !prop.configurable
        {
            return false; // 設定変更不可
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

    /// 新しい: プロパティディスクリプタのオブジェクトから定義を行う
    /// この実装は ECMAScript の簡易的な振る舞いを模倣します:
    /// - 指定されている属性のみを適用し、未指定の属性は既存プロパティから継承、存在しない場合はデフォルト値を使用します
    pub fn define_property_descriptor(&mut self, key: String, desc_obj: Rc<RefCell<JSObject>>) {
        // check existing property
        let existing = self.properties.borrow().get(&key).cloned();

        // helper to check whether descriptor has own property
        let has = |name: &str| desc_obj.borrow().has_own_property(name);

        // Determine if accessor descriptor
        let is_accessor = has("get") || has("set");

        if is_accessor {
            // Accessor descriptor
            let getter = if has("get") {
                let g = desc_obj.borrow().get("get");
                match g {
                    JSValue::Undefined => None,
                    v => Some(v),
                }
            } else {
                existing.as_ref().and_then(|p| p.getter.clone())
            };

            let setter = if has("set") {
                let s = desc_obj.borrow().get("set");
                match s {
                    JSValue::Undefined => None,
                    v => Some(v),
                }
            } else {
                existing.as_ref().and_then(|p| p.setter.clone())
            };

            let enumerable = if has("enumerable") {
                desc_obj.borrow().get("enumerable").to_boolean()
            } else {
                existing.as_ref().map(|p| p.enumerable).unwrap_or(false)
            };

            let configurable = if has("configurable") {
                desc_obj.borrow().get("configurable").to_boolean()
            } else {
                existing.as_ref().map(|p| p.configurable).unwrap_or(false)
            };

            let property = Property {
                value: JSValue::Undefined,
                enumerable,
                writable: false,
                configurable,
                getter,
                setter,
            };

            self.properties.borrow_mut().insert(key, property);
        } else {
            // Data descriptor
            let value = if has("value") {
                desc_obj.borrow().get("value")
            } else {
                existing.as_ref().map(|p| p.value.clone()).unwrap_or(JSValue::Undefined)
            };

            let writable = if has("writable") {
                desc_obj.borrow().get("writable").to_boolean()
            } else {
                existing.as_ref().map(|p| p.writable).unwrap_or(false)
            };

            let enumerable = if has("enumerable") {
                desc_obj.borrow().get("enumerable").to_boolean()
            } else {
                existing.as_ref().map(|p| p.enumerable).unwrap_or(false)
            };

            let configurable = if has("configurable") {
                desc_obj.borrow().get("configurable").to_boolean()
            } else {
                existing.as_ref().map(|p| p.configurable).unwrap_or(false)
            };

            let property = Property {
                value,
                enumerable,
                writable,
                configurable,
                getter: None,
                setter: None,
            };

            self.properties.borrow_mut().insert(key, property);
        }
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
