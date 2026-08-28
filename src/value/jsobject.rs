//! JavaScript オブジェクト表現
//!
//! `JSObject` はプロパティマップとプロトタイプチェーンを管理します。

use super::JSValue;
use rustc_hash::FxHashMap;
use std::cell::RefCell;
use std::rc::Rc;

/// Property name used by host objects to handle otherwise missing reads.
pub const HOST_GET_PROPERTY: &str = "__host_get_property__";

/// Property name used by host objects to handle otherwise missing writes.
pub const HOST_SET_PROPERTY: &str = "__host_set_property__";

/// Property name used by host constructors to implement `instanceof`.
pub const HOST_HAS_INSTANCE: &str = "__host_has_instance__";

/// JavaScript オブジェクトの内部表現
#[derive(Debug, Clone)]
pub struct JSObject {
    /// プロパティマップ
    properties: Rc<RefCell<FxHashMap<String, Property>>>,
    /// プロトタイプチェーン（__proto__）
    prototype: Option<Rc<RefCell<JSObject>>>,
    /// `false` means an ordinary object whose implicit prototype is supplied
    /// by the VM. `true` preserves an explicitly supplied prototype, including
    /// the null prototype created by `Object.create(null)`.
    prototype_is_explicit: bool,
    /// オブジェクトが拡張可能か（Object.preventExtensions/Seal/Freeze に影響）
    extensible: bool,
}

/// プロパティディスクリプタ
#[derive(Debug, Clone)]
pub struct Property {
    /// データプロパティの値
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
    /// 新しい空の JSObject を作成します。
    pub fn new() -> Self {
        Self {
            properties: Rc::new(RefCell::new(FxHashMap::default())),
            prototype: None,
            prototype_is_explicit: false,
            extensible: true,
        }
    }

    /// プロトタイプを指定してオブジェクトを作成します。
    pub fn with_prototype(prototype: Option<Rc<RefCell<JSObject>>>) -> Self {
        Self {
            properties: Rc::new(RefCell::new(FxHashMap::default())),
            prototype,
            prototype_is_explicit: true,
            extensible: true,
        }
    }

    /// 指定したキーのプロパティを取得します。プロトタイプチェーンを辿ります。
    pub fn get(&self, key: &str) -> JSValue {
        // 自身のプロパティを検索
        if let Some(prop) = self.properties.borrow().get(key) {
            return prop.value.clone();
        }

        // プロトタイプチェーンを辿る
        if let Some(ref proto) = self.prototype {
            return proto.borrow().get(key);
        }

        JSValue::undefined()
    }

    /// 指定したキーに値を設定します。既存のデータプロパティが writable でない場合は false を返します。
    pub fn set(&mut self, key: String, value: JSValue) -> bool {
        // 既存のプロパティを確認
        if let Some(prop) = self.properties.borrow_mut().get_mut(&key) {
            if !prop.writable {
                return false; // 書き込み不可
            }
            prop.value = value;
            return true;
        }

        // 新しいプロパティの追加は extensible を尊重
        if !self.extensible {
            return false;
        }

        self.properties
            .borrow_mut()
            .insert(key, Property::data(value));
        true
    }

    /// 自身のプロパティのみを確認します。
    pub fn has_own_property(&self, key: &str) -> bool {
        self.properties.borrow().contains_key(key)
    }

    /// プロパティが存在するか（プロトタイプチェーン含む）
    pub fn has_property(&self, key: &str) -> bool {
        if self.has_own_property(key) {
            return true;
        }

        if let Some(ref proto) = self.prototype {
            return proto.borrow().has_property(key);
        }

        false
    }

    /// プロパティを削除します。non-configurable の場合は削除に失敗します。
    pub fn delete(&mut self, key: &str) -> bool {
        if let Some(prop) = self.properties.borrow().get(key)
            && !prop.configurable
        {
            return false; // 設定変更不可
        }

        self.properties.borrow_mut().remove(key);
        true
    }

    /// オブジェクトの拡張を禁止する
    pub fn prevent_extensions(&mut self) {
        self.extensible = false;
    }

    /// 現在拡張可能かどうかを返す
    pub fn is_extensible(&self) -> bool {
        self.extensible
    }

    /// プロトタイプを取得します（クローンを返します）。
    pub fn get_prototype(&self) -> Option<Rc<RefCell<JSObject>>> {
        self.prototype.clone()
    }

    /// プロトタイプを設定します。注意: 循環チェックは行いません。
    pub fn set_prototype(&mut self, prototype: Option<Rc<RefCell<JSObject>>>) {
        self.prototype = prototype;
        self.prototype_is_explicit = true;
    }

    pub fn has_explicit_prototype(&self) -> bool {
        self.prototype_is_explicit
    }

    /// 列挙可能なプロパティキーを取得します。
    pub fn keys(&self) -> Vec<String> {
        self.properties
            .borrow()
            .iter()
            .filter(|(_, prop)| prop.enumerable)
            .map(|(key, _)| key.clone())
            .collect()
    }

    /// 文字列のown property名を列挙可能性に関係なく取得します。
    pub fn own_property_names(&self) -> Vec<String> {
        self.properties.borrow().keys().cloned().collect()
    }

    /// Returns enumerable string keys from this object and its prototype chain.
    pub fn enumerable_keys(&self) -> Vec<String> {
        let mut keys = self.keys();
        if let Some(prototype) = &self.prototype {
            for key in prototype.borrow().enumerable_keys() {
                if !keys.contains(&key) {
                    keys.push(key);
                }
            }
        }
        keys
    }

    /// 指定されたプロパティディスクリプタでプロパティを定義します。
    ///
    /// ディスクリプタは部分的に指定されることがあり、未指定属性は既存のプロパティから継承されます。
    pub fn define_property(&mut self, key: String, property: Property) -> bool {
        // New properties cannot be added
        if !self.extensible && !self.has_own_property(&key) {
            return false;
        }

        // Non-configurable property cannot be redefined
        if let Some(old) = self.properties.borrow().get(&key) {
            if !old.configurable {
                return false;
            }
        }

        self.properties.borrow_mut().insert(key, property);

        true
    }

    /// 指定された descriptor object からプロパティ定義を行います。
    /// 簡易 ECMAScript 互換の振る舞いを模倣します。
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
                    g if g.clone().is_undefined() => None,
                    v => Some(v),
                }
            } else {
                existing.as_ref().and_then(|p| p.getter.clone())
            };

            let setter = if has("set") {
                let s = desc_obj.borrow().get("set");
                match s {
                    s if s.clone().is_undefined() => None,
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
                value: JSValue::undefined(),
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
                existing
                    .as_ref()
                    .map(|p| p.value.clone())
                    .unwrap_or_else(JSValue::undefined)
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

    /// プロパティディスクリプタを取得します（own property のみ）。
    pub fn get_property_descriptor(&self, key: &str) -> Option<Property> {
        self.properties.borrow().get(key).cloned()
    }

    pub fn dump_properties(&self) {
        let props = self.properties.borrow();

        for (key, value) in props.iter() {
            println!("{key}: {value:?}");
        }
    }
}

impl Default for JSObject {
    fn default() -> Self {
        Self::new()
    }
}
