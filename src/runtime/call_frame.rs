use super::Environment;
use crate::JSValue;
use crate::intern::{InternTable, NameId};

use std::{cell::RefCell, rc::Rc};

/// フレーム内で遅延解決される関数名。
///
/// 呼び出しごとに `String` を確保する代わりに、インターニングテーブルと
/// 名の ID だけを保持し、スタックトレースの整形時にのみ文字列へ解決する。
/// 生成コストは `Rc` クローンのみで、ヒープアロケーションは発生しない。
pub struct FunctionName {
    intern: Rc<RefCell<InternTable>>,
    id: NameId,
}

impl FunctionName {
    pub fn new(intern: Rc<RefCell<InternTable>>, id: NameId) -> Self {
        Self { intern, id }
    }

    /// 名前を解決して所有文字列で返す。呼び出しごとではなく、スタックトレースの
    /// 整形時（エラーパスのみ）に呼ばれるため、ここでのアロケーションは問題にならない。
    pub fn as_str(&self) -> String {
        self.intern.borrow().name(self.id).to_string()
    }
}

pub struct CallFrame {
    pub env: Rc<RefCell<Environment>>,
    pub this: JSValue,
    pub function_name: Option<FunctionName>,
}

impl CallFrame {
    pub fn new(env: Environment, this: JSValue, function_name: Option<FunctionName>) -> Self {
        CallFrame {
            env: Rc::new(RefCell::new(env)),
            this,
            function_name,
        }
    }
}
