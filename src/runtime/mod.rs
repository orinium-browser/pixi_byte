use std::rc::Rc;
use std::cell::RefCell;
use rustc_hash::FxHashMap;
use crate::value::JSValue;

/// 環境レコード（レキシカルスコープチェーン）
#[derive(Debug, Clone)]
pub struct Environment {
    pub bindings: Rc<RefCell<FxHashMap<String, JSValue>>>,
    pub outer: Option<Rc<RefCell<Environment>>>,
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

    pub fn define(&self, name: String, value: JSValue) {
        self.bindings.borrow_mut().insert(name, value);
    }

    pub fn set(&self, name: &str, value: JSValue) -> bool {
        if self.bindings.borrow().contains_key(name) {
            self.bindings.borrow_mut().insert(name.to_string(), value);
            return true;
        }
        if let Some(ref outer) = self.outer {
            return outer.borrow().set(name, value);
        }
        false
    }

    pub fn get(&self, name: &str) -> Option<JSValue> {
        if let Some(v) = self.bindings.borrow().get(name) {
            return Some(v.clone());
        }
        if let Some(ref outer) = self.outer {
            return outer.borrow().get(name);
        }
        None
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}
