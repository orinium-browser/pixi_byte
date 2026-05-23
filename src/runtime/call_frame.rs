use super::Environment;
use crate::JSValue;

use std::{cell::RefCell, rc::Rc};

pub struct CallFrame {
    pub env: Rc<RefCell<Environment>>,
    pub this: JSValue,
}

impl CallFrame {
    pub fn new(env: Environment, this: JSValue) -> Self {
        CallFrame {
            env: Rc::new(RefCell::new(env)),
            this,
        }
    }
}
