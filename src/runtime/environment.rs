use crate::intern::NameId;
use crate::value::JSValue;
use rustc_hash::FxHashMap;
use std::cell::RefCell;
use std::rc::Rc;

/// 環境レコード（レキシカルスコープチェーン）。
///
/// 束縛の**キーは [`NameId`]（インターニングされた変数名）**であり、`String` を
/// 直接は持たない。`NameId` は table-local なので、この環境の束縛キーと、
/// バイトコード中の `LoadVar`/`StoreVar` 等が参照する `NameId` は、**必ず
/// 同一の [`InternTable`](crate::intern::InternTable) で採番されたものである**
/// ことを呼び出し側（VM とコンパイラ）が保証する。
///
/// 環境自身は `InternTable` への参照を保持しない。`NameId` から名前文字列へ
/// 戻す必要がある場合（グローバルオブジェクトへのフォールバックなど）は、
/// 実行中のチャンクが共有するテーブル経由で行う（`BytecodeChunk::intern`）。
#[derive(Debug, Clone)]
pub struct Environment {
    bindings: Rc<RefCell<FxHashMap<NameId, JSValue>>>,
    outer: Option<Rc<RefCell<Environment>>>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            bindings: Rc::new(RefCell::new(FxHashMap::default())),
            outer: None,
        }
    }

    pub fn with_outer(outer: Rc<RefCell<Environment>>) -> Self {
        Self {
            bindings: Rc::new(RefCell::new(FxHashMap::default())),
            outer: Some(outer),
        }
    }

    pub fn outer(&self) -> Option<Rc<RefCell<Environment>>> {
        self.outer.clone()
    }

    pub fn define(&self, name: NameId, value: JSValue) {
        self.bindings.borrow_mut().insert(name, value);
    }

    pub fn define_if_absent(&self, name: NameId, value: JSValue) {
        self.bindings.borrow_mut().entry(name).or_insert(value);
    }

    /// 束縛を更新する。レキシカルなスコープチェーン（自身 + `outer`）を先生き、
    /// 見つかれば更新して `true`、見つからなければ `false` を返す。
    ///
    /// グローバルオブジェクト（`object_env`）には書き込まない。
    pub fn set(&self, name: NameId, value: JSValue) -> bool {
        {
            let mut bindings = self.bindings.borrow_mut();
            if bindings.contains_key(&name) {
                bindings.insert(name, value);
                return true;
            }
        }
        if let Some(ref outer) = self.outer {
            return outer.borrow().set(name, value);
        }
        false
    }

    /// レキシカルなスコープチェーン上で `NameId` を探索して束縛値を返す。
    ///
    /// **グローバルオブジェクトへのフォールバックはここでは行わない**。未束縛なら
    /// `None` を返し、グローバル解決は VM 側（`LoadVar` のハンドラ）が
    /// `BytecodeChunk::intern` で `NameId` を名前に戻して実施する。
    pub fn get_lexical(&self, name: NameId) -> Option<JSValue> {
        if let Some(v) = self.bindings.borrow().get(&name) {
            return Some(v.clone());
        }
        if let Some(ref outer) = self.outer {
            return outer.borrow().get_lexical(name);
        }
        None
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}
