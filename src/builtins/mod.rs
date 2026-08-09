//! Builtins module
//!
//! This module registers ECMAScript built-in objects into the global object at VM startup.
//! Each built-in has an `install` function which receives the global object.

pub mod array;
pub mod function;
pub mod object;
pub mod promise;
pub mod regexp;
pub mod string;

use std::cell::RefCell;
use std::rc::Rc;

pub struct Builtins {}

impl Builtins {
    pub fn new() -> Self {
        Self {}
    }

    /// グローバルオブジェクトに組み込みを初期配置する
    /// 関数値のプロパティ検索で使用する Function.prototype を返します。
    pub fn init(
        &self,
        global: &Rc<RefCell<crate::value::jsobject::JSObject>>,
    ) -> Rc<RefCell<crate::value::jsobject::JSObject>> {
        // Object 関連の組み込みを登録
        self::object::install(global);
        // Array 関連の組み込みを登録
        self::array::install(global);
        self::promise::install(global);
        self::regexp::install(global);
        self::string::install(global);
        // Function 関連の組み込みを登録
        self::function::install(global)
    }
}

impl Default for Builtins {
    fn default() -> Self {
        Self::new()
    }
}
