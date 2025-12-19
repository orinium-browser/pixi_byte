pub mod builtins;
pub mod compiler;
pub mod error;
pub mod gc;
pub mod lexer;
pub mod parser;
pub mod runtime;
pub mod value;
pub mod vm;

pub use error::{JSError, JSResult};
pub use value::JSValue;

// テストで使用するための再エクスポート
pub use compiler::{Compiler, Opcode};
pub use lexer::{Lexer, TokenKind};
pub use parser::Parser;

/// メインインターフェース
pub struct JSEngine {
    /// 仮想マシンインスタンス
    vm: vm::VM,
}

impl JSEngine {
    /// 新しいJSエンジンインスタンスを作成
    pub fn new() -> Self {
        Self { vm: vm::VM::new() }
    }

    /// JavaScriptコードを評価
    pub fn eval(&mut self, source: &str) -> JSResult<JSValue> {
        let tokens = lexer::Lexer::new(source).tokenize()?;
        let ast = parser::Parser::new(tokens).parse()?;
        let bytecode = compiler::Compiler::new().compile(ast)?;
        self.vm.execute(bytecode)
    }
}

impl Default for JSEngine {
    fn default() -> Self {
        Self::new()
    }
}
