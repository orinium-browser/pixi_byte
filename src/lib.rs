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
// Re-export native function type for convenience in tests and builtins
pub use value::jsvalue::NativeFunctionType;

// テストで使用するための再エクスポート
pub use compiler::{Compiler, Opcode};
pub use lexer::{Lexer, TokenKind};
pub use parser::Parser;

pub struct EvalOptions {
    pub dump_ast: bool,
    pub dump_bytecode: bool,
}

impl Default for EvalOptions {
    fn default() -> Self {
        Self {
            dump_ast: false,
            dump_bytecode: false,
        }
    }
}

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
    pub fn eval_with_options(&mut self, source: &str, options: &EvalOptions) -> JSResult<JSValue> {
        let ast = parser::Parser::new(Lexer::new(source))?.parse()?;

        if options.dump_ast {
            println!("=== AST ===");
            ast.dump();
        }

        let bytecode = compiler::Compiler::new().compile(ast)?;

        if options.dump_bytecode {
            println!("=== BYTECODE ===");

            for (i, op) in bytecode.code.iter().enumerate() {
                println!("{:04}: {:?}", i, op);
            }
        }

        self.vm.execute(bytecode)
    }

    /// JavaScriptコードを評価
    pub fn eval(&mut self, source: &str) -> JSResult<JSValue> {
        self.eval_with_options(source, &EvalOptions::default())
    }

    pub fn global_mut(
        &mut self,
    ) -> &mut std::rc::Rc<std::cell::RefCell<crate::value::jsobject::JSObject>> {
        &mut self.vm.global_object
    }
}

impl Default for JSEngine {
    fn default() -> Self {
        Self::new()
    }
}
