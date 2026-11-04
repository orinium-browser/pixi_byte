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
#[derive(Debug)]
pub struct Environment {
    bindings: Rc<RefCell<FxHashMap<NameId, JSValue>>>,
    outer: Option<Rc<RefCell<Environment>>>,
}

impl Clone for Environment {
    fn clone(&self) -> Self {
        crate::census::environment_created();
        Self {
            bindings: Rc::clone(&self.bindings),
            outer: self.outer.as_ref().map(Rc::clone),
        }
    }
}

impl Drop for Environment {
    fn drop(&mut self) {
        crate::census::environment_dropped();
    }
}

impl Environment {
    pub fn new() -> Self {
        crate::census::environment_created();
        Self {
            bindings: Rc::new(RefCell::new(FxHashMap::default())),
            outer: None,
        }
    }

    pub fn with_outer(outer: Rc<RefCell<Environment>>) -> Self {
        crate::census::environment_created();
        Self {
            bindings: Rc::new(RefCell::new(FxHashMap::default())),
            outer: Some(outer),
        }
    }

    /// Snapshot of the values currently bound in this environment.
    ///
    /// Used by the VM's end-of-scope cycle reclamation, which needs to inspect
    /// the scope's bindings without holding the `RefCell` borrow.
    pub fn values(&self) -> Vec<JSValue> {
        self.bindings.borrow().values().cloned().collect()
    }

    /// Snapshot this environment's local bindings for debugger inspection.
    pub fn bindings_snapshot(&self) -> Vec<(NameId, JSValue)> {
        self.bindings
            .borrow()
            .iter()
            .map(|(name, value)| (*name, value.clone()))
            .collect()
    }

    /// Drops every binding, breaking cycles between this scope and closures
    /// created inside it (`env -> f -> env`).
    ///
    /// Only called once the VM has established that no reference to the scope
    /// exists outside that cycle, so the cleared bindings were already
    /// unobservable.
    pub fn clear_bindings(&self) {
        let mut bindings = self.bindings.borrow_mut();
        bindings.clear();
        bindings.shrink_to_fit();
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
            if let std::collections::hash_map::Entry::Occupied(mut e) = bindings.entry(name) {
                e.insert(value);
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
