pub mod builtins;
pub mod compiler;
pub mod error;
pub mod gc;
pub mod intern;
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
        let bytecode = self.compile_with_options(source, options)?;
        self.execute(&bytecode)
    }

    /// JavaScriptコードを評価
    pub fn eval(&mut self, source: &str) -> JSResult<JSValue> {
        self.eval_with_options(source, &EvalOptions::default())
    }

    /// JavaScriptソースを lex → parse → compile してバイトコードを生成します。
    ///
    /// 生成した [`BytecodeChunk`] は [`execute`](Self::execute) で繰り返し実行できます。
    pub fn compile(&mut self, source: &str) -> JSResult<compiler::BytecodeChunk> {
        self.compile_with_options(source, &EvalOptions::default())
    }

    /// `dump_ast` / `dump_bytecode` オプション付きでバイトコードを生成します。
    pub fn compile_with_options(
        &self,
        source: &str,
        options: &EvalOptions,
    ) -> JSResult<compiler::BytecodeChunk> {
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

        Ok(bytecode)
    }

    /// コンパイル済みバイトコードを現在の VM 状態で実行します。
    pub fn execute(&mut self, bytecode: &compiler::BytecodeChunk) -> JSResult<JSValue> {
        self.vm.execute(bytecode)
    }

    pub fn global_mut(
        &mut self,
    ) -> &mut std::rc::Rc<std::cell::RefCell<crate::value::jsobject::JSObject>> {
        &mut self.vm.global_object
    }

    /// Returns the underlying VM.
    ///
    /// The host reads `vm.host` (the shared host-data slot) to access its own
    /// state from outside a native-function call.
    pub fn vm(&self) -> &vm::VM {
        &self.vm
    }

    /// Sets the host data.
    ///
    /// Stores arbitrary host state (e.g. the browser DOM tree) as
    /// `Rc<RefCell<dyn Any>>` in the VM. Native functions read it via
    /// `downcast_ref` on `vm.host`.
    pub fn set_host(&mut self, host: std::rc::Rc<std::cell::RefCell<dyn std::any::Any>>) {
        self.vm.host = Some(host);
    }

    /// Calls a JS function value from Rust.
    ///
    /// `callee` must be `JSValue::Function`, `JSValue::NativeFunction`, or
    /// `JSValue::BoundFunction`.
    pub fn call(
        &mut self,
        callee: JSValue,
        this: JSValue,
        args: Vec<JSValue>,
    ) -> JSResult<JSValue> {
        self.vm.call(callee, this, args)
    }

    /// Enqueues a callable job for execution at the next host checkpoint.
    pub fn enqueue_job(&mut self, callback: JSValue, this: JSValue, args: Vec<JSValue>) {
        self.vm.enqueue_job(callback, this, args);
    }

    /// Runs all queued jobs, including jobs enqueued while draining the queue.
    pub fn run_jobs(&mut self) -> JSResult<()> {
        self.vm.run_jobs()
    }
}

impl Default for JSEngine {
    fn default() -> Self {
        Self::new()
    }
}
