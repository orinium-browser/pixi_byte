use super::Environment;
use crate::JSValue;

use std::{cell::RefCell, rc::Rc};

pub struct CallFrame {
    pub env: Rc<RefCell<Environment>>,
    pub this: JSValue,
    pub function_name: Option<String>,
}

impl CallFrame {
    pub fn new(env: Environment, this: JSValue, function_name: Option<String>) -> Self {
        CallFrame {
            env: Rc::new(RefCell::new(env)),
            this,
            function_name,
        }
    }
}
