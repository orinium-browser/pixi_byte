// Built-in Objects & Functions - ECMAScript標準の組み込み機能
// Phase 2 でいくつかの最小実装を追加

pub mod object;
pub mod array;
pub mod function;

use std::cell::RefCell;
use std::rc::Rc;

pub struct Builtins {}

impl Builtins {
    pub fn new() -> Self {
        Self {}
    }

    /// グローバルオブジェクトに組み込みを初期配置する
    pub fn init(&self, global: &Rc<RefCell<crate::value::jsobject::JSObject>>) {
        // Object 関連の組み込みを登録
        self::object::install(global);
        // Array 関連の組み込みを登録
        self::array::install(global);
        // Function 関連の組み込みを登録
        self::function::install(global);
    }
}

impl Default for Builtins {
    fn default() -> Self {
        Self::new()
    }
}
