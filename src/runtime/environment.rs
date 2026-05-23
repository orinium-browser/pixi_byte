use crate::value::JSValue;
use rustc_hash::FxHashMap;
use std::cell::RefCell;
use std::rc::Rc;

/// 環境レコード（レキシカルスコープチェーン）
#[derive(Debug, Clone)]
pub struct Environment {
    bindings: Rc<RefCell<FxHashMap<String, JSValue>>>,
    object_env: Option<Rc<RefCell<crate::value::JSObject>>>,
    outer: Option<Rc<RefCell<Environment>>>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            bindings: Rc::new(RefCell::new(FxHashMap::default())),
            object_env: None,
            outer: None,
        }
    }

    pub fn with_outer(outer: Rc<RefCell<Environment>>) -> Self {
        Self {
            bindings: Rc::new(RefCell::new(FxHashMap::default())),
            object_env: None,
            outer: Some(outer),
        }
    }

    pub fn with_object_env(object_env: Rc<RefCell<crate::value::JSObject>>) -> Self {
        Self {
            bindings: Rc::new(RefCell::new(FxHashMap::default())),
            object_env: Some(object_env),
            outer: None,
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
        if let Some(ref object) = self.object_env {
            return Some(object.borrow().get(name));
        }
        None
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}
