//! JavaScript オブジェクト表現
//!
//! `JSObject` はプロパティマップとプロトタイプチェーンを管理します。
//! 配列は通常のプロパティマップとは別に、密な要素ストレージ
//! （`elements`）を持つことができます。これにより配列リテラルなどの
//! 生成コスト（要素ごとの String キー / ハッシュ / Property 生成）を
//! 回避できます。

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

/// 正準な配列インデックス文字列（`"0"` から `"4294967294"` まで・先頭ゼロなし）なら
/// その index を返します。それ以外は `None` を返します。
fn canonical_array_index(key: &str) -> Option<usize> {
    if !key.as_bytes().first().is_some_and(|b| b.is_ascii_digit()) {
        return None;
    }
    let index: u32 = key.parse().ok()?;
    if index == u32::MAX {
        return None;
    }
    // "01" のような先頭ゼロ付き文字列は配列インデックスとして扱わない
    if key != index.to_string() {
        return None;
    }
    Some(index as usize)
}

/// JavaScript オブジェクトの内部表現
#[derive(Debug, Clone)]
pub struct JSObject {
    /// プロパティマップ
    properties: Rc<RefCell<FxHashMap<String, Property>>>,
    /// 密な配列要素ストレージ（配列のみ `Some`）。
    ///
    /// `elements[i]` は配列インデックス `i` の要素を表し、
    /// - `Some(value)` → 要素が存在（`undefined` も明示的に保持）
    /// - `None` → 穴（`delete arr[i]` 後など）
    /// という invariant を持つ（通常のプロパティマップと同じく存在判定に使われる）。
    /// プロパティマップを優先するため、`defineProperty` 等で上書きされた要素も正しく扱える。
    elements: Option<Rc<RefCell<Vec<Option<JSValue>>>>>,
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
            elements: None,
            prototype: None,
            prototype_is_explicit: false,
            extensible: true,
        }
    }

    /// プロトタイプを指定してオブジェクトを作成します。
    pub fn with_prototype(prototype: Option<Rc<RefCell<JSObject>>>) -> Self {
        Self {
            properties: Rc::new(RefCell::new(FxHashMap::default())),
            elements: None,
            prototype,
            prototype_is_explicit: true,
            extensible: true,
        }
    }

    /// 密な要素ベクトルを持つオブジェクト（配列）を作成します。
    ///
    /// 要素はプロパティマップへ展開せず、そのまま要素ストレージへ移します。
    /// 配列リテラル等の生成で、要素ごとの String キー生成・ハッシュ・Property
    /// 生成が不要になり、O(n) の move だけで済みます。
    pub fn with_elements(elements: Vec<JSValue>) -> Self {
        Self {
            properties: Rc::new(RefCell::new(FxHashMap::default())),
            elements: Some(Rc::new(RefCell::new(
                elements.into_iter().map(Some).collect(),
            ))),
            prototype: None,
            prototype_is_explicit: false,
            extensible: true,
        }
    }

    /// 配列要素ストレージから、正準配列インデックスのプロパティディスクリプタを
    /// 合成します（要素は writable / enumerable / configurable すべて true）。
    fn element_property_descriptor(&self, key: &str) -> Option<Property> {
        if let Some(index) = canonical_array_index(key)
            && let Some(elements) = &self.elements
        {
            let elements = elements.borrow();
            if let Some(Some(value)) = elements.get(index) {
                return Some(Property::data(value.clone()));
            }
        }
        None
    }

    /// 指定したキーのプロパティを取得します。プロトタイプチェーンを辿ります。
    pub fn get(&self, key: &str) -> JSValue {
        // 自身のプロパティを検索
        if let Some(prop) = self.properties.borrow().get(key) {
            return prop.value.clone();
        }

        // 配列要素を検索（穴は undefined として扱う）
        if let Some(index) = canonical_array_index(key)
            && let Some(elements) = &self.elements
        {
            if let Some(element) = elements.borrow().get(index).and_then(Option::as_ref) {
                return element.clone();
            }
        }

        // プロトタイプチェーンを辿る
        if let Some(ref proto) = self.prototype {
            return proto.borrow().get(key);
        }

        JSValue::undefined()
    }

    /// 指定したキーに値を設定します。既存のデータプロパティが writable でない場合は false を返します。
    pub fn set(&mut self, key: String, value: JSValue) -> bool {
        // 既存のプロパティを確認（defineProperty 等で上書きされた要素もここで優先される）
        if let Some(prop) = self.properties.borrow_mut().get_mut(&key) {
            if !prop.writable {
                return false; // 書き込み不可
            }
            prop.value = value;
            return true;
        }

        // 配列要素ストレージへの書き込み
        if let Some(index) = canonical_array_index(&key)
            && let Some(elements) = &self.elements
        {
            let mut elements = elements.borrow_mut();
            if index < elements.len() {
                // 既存要素の上書き
                elements[index] = Some(value);
                return true;
            }
            // 新しい要素: 拡張可能性を尊重
            if !self.extensible {
                return false;
            }
            elements.resize(index + 1, None);
            elements[index] = Some(value);
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
        if self.properties.borrow().contains_key(key) {
            return true;
        }
        if let Some(index) = canonical_array_index(key)
            && let Some(elements) = &self.elements
            && elements
                .borrow()
                .get(index)
                .is_some_and(|slot| slot.is_some())
        {
            return true;
        }
        false
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

        if self.properties.borrow_mut().remove(key).is_some() {
            // defineProperty 等でプロパティマップに移された配列要素も、
            // delete 後はマップと同じく「穴」として扱う（古い要素の復活を防ぐ）
            if let Some(index) = canonical_array_index(key)
                && let Some(elements) = &self.elements
            {
                if let Some(slot) = elements.borrow_mut().get_mut(index) {
                    *slot = None;
                }
            }
            return true;
        }

        // 配列要素は穴（None）にする
        if let Some(index) = canonical_array_index(key)
            && let Some(elements) = &self.elements
        {
            if let Some(slot) = elements.borrow_mut().get_mut(index) {
                *slot = None;
            }
        }

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
        let mut keys: Vec<String> = self
            .properties
            .borrow()
            .iter()
            .filter(|(_, prop)| prop.enumerable)
            .map(|(key, _)| key.clone())
            .collect();
        // 配列要素はプロパティマップに無いものだけ追加する（上書き済み要素の二重列挙を防ぐ）
        if let Some(elements) = &self.elements {
            let elements = elements.borrow();
            let props = self.properties.borrow();
            for (index, slot) in elements.iter().enumerate() {
                if slot.is_some() && !props.contains_key(&index.to_string()) {
                    keys.push(index.to_string());
                }
            }
        }
        keys
    }

    /// 文字列のown property名を列挙可能性に関係なく取得します。
    pub fn own_property_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.properties.borrow().keys().cloned().collect();
        if let Some(elements) = &self.elements {
            let elements = elements.borrow();
            let props = self.properties.borrow();
            for (index, slot) in elements.iter().enumerate() {
                if slot.is_some() && !props.contains_key(&index.to_string()) {
                    names.push(index.to_string());
                }
            }
        }
        names
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
        if let Some(old) = self.properties.borrow().get(&key)
            && !old.configurable
        {
            return false;
        }

        self.properties.borrow_mut().insert(key, property);

        true
    }

    /// 指定された descriptor object からプロパティ定義を行います。
    /// 簡易 ECMAScript 互換の振る舞いを模倣します。
    pub fn define_property_descriptor(&mut self, key: String, desc_obj: Rc<RefCell<JSObject>>) {
        // check existing property（配列要素は合成ディスクリプタとして既存扱いにする）
        let existing = self
            .properties
            .borrow()
            .get(&key)
            .cloned()
            .or_else(|| self.element_property_descriptor(&key));

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
        if let Some(prop) = self.properties.borrow().get(key) {
            return Some(prop.clone());
        }
        self.element_property_descriptor(key)
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
