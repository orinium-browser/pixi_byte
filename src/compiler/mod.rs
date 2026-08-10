use crate::error::{JSError, JSResult};
use crate::parser::{
    BinaryOp, BindingPattern, ClassMethod, ClassMethodKind, Expression, Literal, ObjectProperty,
    Program, Statement, UnaryOp, VarKind,
};
use crate::value::JSValue;
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_BYTECODE_ID: AtomicU64 = AtomicU64::new(1);

/// バイトコード命令
#[derive(Debug, Clone, PartialEq)]
pub enum Opcode {
    // スタック操作
    LoadConst(usize),  // 定数をスタックにロード
    LoadVar(String),   // 変数をスタックにロード
    StoreVar(String),  // スタックトップを変数に格納
    DefineVar(String), // スタックトップを現在のスコープへ新しく束縛
    Pop,               // スタックトップを削除
    Dup,               // スタックトップを複製
    Dup2,              // スタックトップの2値を複製

    LoadThis,

    // 算術演算
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Power,

    // 単項演算
    Neg,    // 符号反転
    Not,    // 論理否定
    BitNot, // ビット否定

    // 比較演算
    Eq,
    NotEq,
    StrictEq,
    StrictNotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    In,
    Instanceof,

    // 論理演算
    And,
    Or,

    // ビット演算
    BitAnd,
    BitOr,
    BitXor,
    LeftShift,
    RightShift,
    UnsignedRightShift,

    // 配列・オブジェクト操作
    NewArray(usize), // 空の配列を作成（サイズ指定）
    NewObject,       // 空のオブジェクトを作成
    NewRegExp(String, String),
    GetProperty,        // obj[key] - スタックから key, obj をポップ、結果をプッシュ
    SetProperty,        // obj[key] = value - スタックから value, key, obj をポップ
    SetPropertyKeepOld, // postfix update用: obj, key, old, newからoldを残す
    DeleteProperty,     // delete obj[key] - スタックから key, obj をポップ
    ArrayPush,          // arr.push(value) - スタックから index, value をポップ、arr は残る
    ArrayAppend,
    ArrayExtend,
    ObjectSetProperty, // obj[key] = value - スタックから key, value をポップ、obj は残る
    ObjectSpread,      // source の列挙可能なown propertyをobjectへコピー
    ObjectRest(Vec<String>),
    ObjectDefineGetter, // object literal getter; object remains on the stack
    ObjectDefineSetter, // object literal setter; object remains on the stack
    Enumerate,          // for-in用の列挙可能なプロパティ名配列を生成

    // 関数操作
    CreateFunction(usize), // 定数プール内の関数オブジェクトを生成してプッシュ（func chunk idx）
    CallFunction(usize),   // 呼び出し（引数個数） - スタックから argN..arg1, func を使う
    CallFunctionNamed(usize, String),
    CallFunctionArray, // 呼び出し（展開済み引数配列） - スタック: ..., func, args
    CallMethod(usize), // メソッド呼び出し（arg count） - スタック: ..., object, property, arg1..argN
    CallMethodArray,
    CallFunctionOptional(usize),
    CallMethodOptional(usize),
    Construct(usize, Option<String>), // コンストラクタ呼び出し（引数個数、診断用の名前）

    // 制御フロー
    Jump(usize),        // 無条件ジャンプ
    JumpIfFalse(usize), // false の場合ジャンプ
    JumpIfTrue(usize),  // true の場合ジャンプ
    JumpIfNotNullish(usize),
    PushTry {
        catch_target: Option<usize>,
        finally_target: Option<usize>,
    },
    PopTry,
    BeginFinally,
    EndFinally,
    Throw,
    Return, // 関数から戻る

    // その他
    Typeof,
    Void,
}

impl Opcode {
    fn remap_constants(mut self, offset: usize) -> Self {
        match self {
            Self::LoadConst(ref mut u)
            | Self::CreateFunction(ref mut u)
            | Self::Jump(ref mut u)
            | Self::JumpIfFalse(ref mut u) => {
                *u = u.checked_add(offset).expect("constant index overflow")
            }
            _ => {}
        }
        self
    }
}

/// バイトコードチャンク
#[derive(Debug, Clone)]
pub struct BytecodeChunk {
    /// Stable identity retained when a function value is cloned.
    pub identity: u64,
    /// バイトコード命令列
    pub code: Vec<Opcode>,
    /// 定数プール
    pub constants: Vec<JSValue>,
}

impl BytecodeChunk {
    /// 新しいバイトコードチャンクを作成
    pub fn new() -> Self {
        Self {
            identity: NEXT_BYTECODE_ID.fetch_add(1, Ordering::Relaxed),
            code: Vec::new(),
            constants: Vec::new(),
        }
    }

    /// Merges BytecodeChunk.
    ///
    /// # Panics
    /// Panics if merging the chunks would cause a constant index to overflow.
    pub fn merge(&mut self, other: BytecodeChunk) {
        let constant_offset = self.constants.len();

        self.constants.extend(other.constants);

        for opcode in other.code {
            self.code.push(opcode.remap_constants(constant_offset));
        }
    }

    /// 定数プールに値を追加し、そのインデックスを返す
    pub fn add_constant(&mut self, value: JSValue) -> usize {
        // 既存の定数を探す
        for (i, constant) in self.constants.iter().enumerate() {
            if constant == &value {
                return i;
            }
        }

        // 新しい定数を追加
        let index = self.constants.len();
        self.constants.push(value);
        index
    }

    /// バイトコード命令を追加
    pub fn emit(&mut self, opcode: Opcode) {
        self.code.push(opcode);
    }
}

impl Default for BytecodeChunk {
    /// デフォルト実装
    fn default() -> Self {
        Self::new()
    }
}

/// コンパイラ
pub struct Compiler {
    /// 生成されたバイトコードチャンク
    chunk: BytecodeChunk,
    loops: Vec<LoopContext>,
    break_scopes: Vec<Vec<usize>>,
    labels: Vec<LabelContext>,
    pending_loop_label: Option<String>,
    next_temporary: usize,
    super_binding: Option<String>,
    generator_output: Option<String>,
}

struct LoopContext {
    continue_jumps: Vec<usize>,
}

struct LabelContext {
    name: String,
    is_iteration: bool,
    break_jumps: Vec<usize>,
    continue_jumps: Vec<usize>,
}

impl Compiler {
    /// 新しいコンパイラインスタンスを作成
    pub fn new() -> Self {
        Self {
            chunk: BytecodeChunk::new(),
            loops: Vec::new(),
            break_scopes: Vec::new(),
            labels: Vec::new(),
            pending_loop_label: None,
            next_temporary: 0,
            super_binding: None,
            generator_output: None,
        }
    }

    fn with_super_binding(super_binding: Option<String>) -> Self {
        Self {
            super_binding,
            ..Self::new()
        }
    }

    /// ASTをバイトコードにコンパイル
    pub fn compile(mut self, program: Program) -> JSResult<BytecodeChunk> {
        self.emit_var_declarations(&program.body);
        let mut executable = Vec::new();
        for statement in program.body {
            if matches!(&statement, Statement::FunctionDeclaration { .. }) {
                self.compile_statement(statement, false)?;
            } else {
                executable.push(statement);
            }
        }

        let len = executable.len();
        for (i, statement) in executable.into_iter().enumerate() {
            let is_last = i == len - 1;
            self.compile_statement(statement, is_last)?;
        }

        Ok(self.chunk)
    }

    fn compile_function(mut self, program: Program) -> JSResult<BytecodeChunk> {
        if let Some(output) = self.generator_output.clone() {
            self.chunk.emit(Opcode::NewArray(0));
            self.chunk.emit(Opcode::DefineVar(output));
        }
        self.emit_var_declarations(&program.body);
        let mut executable = Vec::new();
        for statement in program.body {
            if matches!(&statement, Statement::FunctionDeclaration { .. }) {
                self.compile_statement(statement, false)?;
            } else {
                executable.push(statement);
            }
        }
        for statement in executable {
            self.compile_statement(statement, false)?;
        }
        if let Some(output) = self.generator_output {
            self.chunk.emit(Opcode::LoadVar(output));
        } else {
            let undefined = self.chunk.add_constant(JSValue::Undefined);
            self.chunk.emit(Opcode::LoadConst(undefined));
        }
        self.chunk.emit(Opcode::Return);
        Ok(self.chunk)
    }

    fn emit_var_declarations(&mut self, statements: &[Statement]) {
        let mut names = Vec::new();
        collect_var_declarations(statements, &mut names);
        let mut emitted = HashSet::new();
        for name in names {
            if emitted.insert(name.clone()) {
                let undefined = self.chunk.add_constant(JSValue::Undefined);
                self.chunk.emit(Opcode::LoadConst(undefined));
                self.chunk.emit(Opcode::DefineVar(name));
            }
        }
    }

    /// ステートメントをコンパイル
    fn compile_statement(&mut self, statement: Statement, is_last: bool) -> JSResult<()> {
        match statement {
            Statement::Empty => {
                if is_last {
                    let undefined = self.chunk.add_constant(JSValue::Undefined);
                    self.chunk.emit(Opcode::LoadConst(undefined));
                }
            }
            Statement::Block(body) => {
                self.compile_statements(body, is_last)?;
            }
            Statement::Labeled { label, body } => {
                let is_iteration = matches!(
                    &*body,
                    Statement::While { .. }
                        | Statement::DoWhile { .. }
                        | Statement::For { .. }
                        | Statement::ForIn { .. }
                        | Statement::ForOf { .. }
                );
                self.labels.push(LabelContext {
                    name: label.clone(),
                    is_iteration,
                    break_jumps: Vec::new(),
                    continue_jumps: Vec::new(),
                });
                if is_iteration {
                    self.pending_loop_label = Some(label);
                }
                self.compile_statement(*body, is_last)?;
                let context = self.labels.pop().expect("label context must exist");
                if !context.continue_jumps.is_empty() {
                    return Err(JSError::InternalError(
                        "labeled continue target is not an iteration statement".to_string(),
                    ));
                }
                let end = self.chunk.code.len();
                for jump in context.break_jumps {
                    self.patch_jump(jump, end);
                }
            }
            Statement::Expression(expr) => {
                self.compile_expression(expr)?;
                // 最後の式文の結果はスタックに残す（REPLスタイル）
                if !is_last {
                    self.chunk.emit(Opcode::Pop);
                }
            }
            Statement::VariableDeclaration { kind, declarations } => {
                for (name, init) in declarations {
                    if let Some(expr) = init {
                        self.compile_expression(expr)?;
                    } else {
                        let idx = self.chunk.add_constant(JSValue::Undefined);
                        self.chunk.emit(Opcode::LoadConst(idx));
                    }
                    self.chunk.emit(if kind == VarKind::Var {
                        Opcode::StoreVar(name)
                    } else {
                        Opcode::DefineVar(name)
                    });
                }

                // 変数宣言の文は常にundefinedを返す
                if is_last {
                    let idx = self.chunk.add_constant(JSValue::Undefined);
                    self.chunk.emit(Opcode::LoadConst(idx));
                }
            }
            Statement::PatternDeclaration {
                kind,
                binding,
                init,
            } => {
                self.compile_expression(init)?;
                self.store_binding_pattern(&binding, kind != VarKind::Var)?;
                if is_last {
                    let idx = self.chunk.add_constant(JSValue::Undefined);
                    self.chunk.emit(Opcode::LoadConst(idx));
                }
            }
            Statement::Return(expr) => {
                if let Some(output) = self.generator_output.clone() {
                    if let Some(expr) = expr {
                        self.compile_expression(expr)?;
                        self.chunk.emit(Opcode::Pop);
                    }
                    self.chunk.emit(Opcode::LoadVar(output));
                    self.chunk.emit(Opcode::Return);
                    return Ok(());
                }
                if let Some(expr) = expr {
                    self.compile_expression(expr)?;
                } else {
                    let idx = self.chunk.add_constant(JSValue::Undefined);
                    self.chunk.emit(Opcode::LoadConst(idx));
                }
                self.chunk.emit(Opcode::Return);
            }
            Statement::FunctionDeclaration {
                name,
                params,
                body,
                is_generator,
            } => {
                self.emit_function_value(Some(name.clone()), params, body, None, is_generator)?;
                self.chunk.emit(Opcode::DefineVar(name));
            }
            Statement::If {
                test,
                consequent,
                alternate,
            } => {
                self.compile_expression(test)?;
                let branch = self.chunk.code.len();
                self.chunk.emit(Opcode::JumpIfFalse(usize::MAX));
                self.compile_statements(consequent, is_last && alternate.is_none())?;

                if let Some(alternate) = alternate {
                    let end_jump = self.chunk.code.len();
                    self.chunk.emit(Opcode::Jump(usize::MAX));
                    let alternate_start = self.chunk.code.len();
                    self.patch_jump(branch, alternate_start);
                    self.compile_statements(alternate, is_last)?;
                    let end = self.chunk.code.len();
                    self.patch_jump(end_jump, end);
                } else {
                    let end = self.chunk.code.len();
                    self.patch_jump(branch, end);
                }
            }
            Statement::While { test, body } => {
                let loop_label = self.pending_loop_label.take();
                let loop_start = self.chunk.code.len();
                self.compile_expression(test)?;
                let exit_jump = self.chunk.code.len();
                self.chunk.emit(Opcode::JumpIfFalse(usize::MAX));
                self.loops.push(LoopContext {
                    continue_jumps: Vec::new(),
                });
                self.break_scopes.push(Vec::new());
                self.compile_statements(body, false)?;
                let continue_jumps = {
                    let loop_context = self.loops.last_mut().expect("loop context must exist");
                    std::mem::take(&mut loop_context.continue_jumps)
                };
                for continue_jump in continue_jumps {
                    self.patch_jump(continue_jump, loop_start);
                }
                self.patch_labeled_continues(loop_label.as_deref(), loop_start)?;
                self.chunk.emit(Opcode::Jump(loop_start));

                let exit_target = self.chunk.code.len();
                self.patch_jump(exit_jump, exit_target);
                self.loops.pop().expect("loop context must exist");
                for break_jump in self.break_scopes.pop().expect("break scope must exist") {
                    self.patch_jump(break_jump, exit_target);
                }
                if is_last {
                    let undefined = self.chunk.add_constant(JSValue::Undefined);
                    self.chunk.emit(Opcode::LoadConst(undefined));
                }
            }
            Statement::DoWhile { body, test } => {
                let loop_label = self.pending_loop_label.take();
                let loop_start = self.chunk.code.len();
                self.loops.push(LoopContext {
                    continue_jumps: Vec::new(),
                });
                self.break_scopes.push(Vec::new());
                self.compile_statements(body, false)?;

                let condition_start = self.chunk.code.len();
                let continue_jumps = {
                    let loop_context = self.loops.last_mut().expect("loop context must exist");
                    std::mem::take(&mut loop_context.continue_jumps)
                };
                for continue_jump in continue_jumps {
                    self.patch_jump(continue_jump, condition_start);
                }
                self.patch_labeled_continues(loop_label.as_deref(), condition_start)?;
                self.compile_expression(test)?;
                let exit_jump = self.chunk.code.len();
                self.chunk.emit(Opcode::JumpIfFalse(usize::MAX));
                self.chunk.emit(Opcode::Jump(loop_start));

                let exit_target = self.chunk.code.len();
                self.patch_jump(exit_jump, exit_target);
                self.loops.pop().expect("loop context must exist");
                for break_jump in self.break_scopes.pop().expect("break scope must exist") {
                    self.patch_jump(break_jump, exit_target);
                }
                if is_last {
                    let undefined = self.chunk.add_constant(JSValue::Undefined);
                    self.chunk.emit(Opcode::LoadConst(undefined));
                }
            }
            Statement::For {
                init,
                test,
                update,
                body,
            } => {
                let loop_label = self.pending_loop_label.take();
                if let Some(init) = init {
                    self.compile_statement(*init, false)?;
                }
                let loop_start = self.chunk.code.len();
                if let Some(test) = test {
                    self.compile_expression(test)?;
                } else {
                    let truthy = self.chunk.add_constant(JSValue::Boolean(true));
                    self.chunk.emit(Opcode::LoadConst(truthy));
                }
                let exit_jump = self.chunk.code.len();
                self.chunk.emit(Opcode::JumpIfFalse(usize::MAX));
                self.loops.push(LoopContext {
                    continue_jumps: Vec::new(),
                });
                self.break_scopes.push(Vec::new());
                self.compile_statements(body, false)?;

                let update_start = self.chunk.code.len();
                let continue_jumps = {
                    let loop_context = self.loops.last_mut().expect("loop context must exist");
                    std::mem::take(&mut loop_context.continue_jumps)
                };
                for continue_jump in continue_jumps {
                    self.patch_jump(continue_jump, update_start);
                }
                self.patch_labeled_continues(loop_label.as_deref(), update_start)?;
                for update in update {
                    self.compile_expression(update)?;
                    self.chunk.emit(Opcode::Pop);
                }
                self.chunk.emit(Opcode::Jump(loop_start));

                let exit_target = self.chunk.code.len();
                self.patch_jump(exit_jump, exit_target);
                self.loops.pop().expect("loop context must exist");
                for break_jump in self.break_scopes.pop().expect("break scope must exist") {
                    self.patch_jump(break_jump, exit_target);
                }
                if is_last {
                    let undefined = self.chunk.add_constant(JSValue::Undefined);
                    self.chunk.emit(Opcode::LoadConst(undefined));
                }
            }
            Statement::ForIn {
                binding,
                kind,
                right,
                body,
            } => {
                let loop_label = self.pending_loop_label.take();
                let keys = format!("__pixi_for_in_keys_{}", self.next_temporary);
                let index = format!("__pixi_for_in_index_{}", self.next_temporary);
                self.next_temporary += 1;

                self.compile_expression(right)?;
                self.chunk.emit(Opcode::Enumerate);
                self.chunk.emit(Opcode::DefineVar(keys.clone()));
                let zero = self.chunk.add_constant(JSValue::Number(0.0));
                self.chunk.emit(Opcode::LoadConst(zero));
                self.chunk.emit(Opcode::DefineVar(index.clone()));
                if matches!(kind, Some(VarKind::Let | VarKind::Const)) {
                    self.define_binding_pattern(&binding)?;
                }

                let loop_start = self.chunk.code.len();
                self.chunk.emit(Opcode::LoadVar(index.clone()));
                self.chunk.emit(Opcode::LoadVar(keys.clone()));
                let length = self
                    .chunk
                    .add_constant(JSValue::String("length".to_string()));
                self.chunk.emit(Opcode::LoadConst(length));
                self.chunk.emit(Opcode::GetProperty);
                self.chunk.emit(Opcode::Lt);
                let exit_jump = self.chunk.code.len();
                self.chunk.emit(Opcode::JumpIfFalse(usize::MAX));

                self.chunk.emit(Opcode::LoadVar(keys));
                self.chunk.emit(Opcode::LoadVar(index.clone()));
                self.chunk.emit(Opcode::GetProperty);
                self.store_binding_pattern(&binding, false)?;

                self.loops.push(LoopContext {
                    continue_jumps: Vec::new(),
                });
                self.break_scopes.push(Vec::new());
                self.compile_statements(body, false)?;

                let update_start = self.chunk.code.len();
                let continue_jumps = {
                    let loop_context = self.loops.last_mut().expect("loop context must exist");
                    std::mem::take(&mut loop_context.continue_jumps)
                };
                for continue_jump in continue_jumps {
                    self.patch_jump(continue_jump, update_start);
                }
                self.patch_labeled_continues(loop_label.as_deref(), update_start)?;
                self.chunk.emit(Opcode::LoadVar(index.clone()));
                let one = self.chunk.add_constant(JSValue::Number(1.0));
                self.chunk.emit(Opcode::LoadConst(one));
                self.chunk.emit(Opcode::Add);
                self.chunk.emit(Opcode::StoreVar(index));
                self.chunk.emit(Opcode::Jump(loop_start));

                let exit_target = self.chunk.code.len();
                self.patch_jump(exit_jump, exit_target);
                self.loops.pop().expect("loop context must exist");
                for break_jump in self.break_scopes.pop().expect("break scope must exist") {
                    self.patch_jump(break_jump, exit_target);
                }
                if is_last {
                    let undefined = self.chunk.add_constant(JSValue::Undefined);
                    self.chunk.emit(Opcode::LoadConst(undefined));
                }
            }
            Statement::ForOf {
                binding,
                kind,
                right,
                body,
            } => {
                let loop_label = self.pending_loop_label.take();
                let values = format!("__pixi_for_of_values_{}", self.next_temporary);
                let index = format!("__pixi_for_of_index_{}", self.next_temporary);
                self.next_temporary += 1;

                self.compile_expression(right)?;
                self.chunk.emit(Opcode::DefineVar(values.clone()));
                let zero = self.chunk.add_constant(JSValue::Number(0.0));
                self.chunk.emit(Opcode::LoadConst(zero));
                self.chunk.emit(Opcode::DefineVar(index.clone()));
                if matches!(kind, Some(VarKind::Let | VarKind::Const)) {
                    self.define_binding_pattern(&binding)?;
                }

                let loop_start = self.chunk.code.len();
                self.chunk.emit(Opcode::LoadVar(index.clone()));
                self.chunk.emit(Opcode::LoadVar(values.clone()));
                let length = self
                    .chunk
                    .add_constant(JSValue::String("length".to_string()));
                self.chunk.emit(Opcode::LoadConst(length));
                self.chunk.emit(Opcode::GetProperty);
                self.chunk.emit(Opcode::Lt);
                let exit_jump = self.chunk.code.len();
                self.chunk.emit(Opcode::JumpIfFalse(usize::MAX));

                self.chunk.emit(Opcode::LoadVar(values));
                self.chunk.emit(Opcode::LoadVar(index.clone()));
                self.chunk.emit(Opcode::GetProperty);
                self.store_binding_pattern(&binding, false)?;

                self.loops.push(LoopContext {
                    continue_jumps: Vec::new(),
                });
                self.break_scopes.push(Vec::new());
                self.compile_statements(body, false)?;

                let update_start = self.chunk.code.len();
                let continue_jumps = {
                    let loop_context = self.loops.last_mut().expect("loop context must exist");
                    std::mem::take(&mut loop_context.continue_jumps)
                };
                for continue_jump in continue_jumps {
                    self.patch_jump(continue_jump, update_start);
                }
                self.patch_labeled_continues(loop_label.as_deref(), update_start)?;
                self.chunk.emit(Opcode::LoadVar(index.clone()));
                let one = self.chunk.add_constant(JSValue::Number(1.0));
                self.chunk.emit(Opcode::LoadConst(one));
                self.chunk.emit(Opcode::Add);
                self.chunk.emit(Opcode::StoreVar(index));
                self.chunk.emit(Opcode::Jump(loop_start));

                let exit_target = self.chunk.code.len();
                self.patch_jump(exit_jump, exit_target);
                self.loops.pop().expect("loop context must exist");
                for break_jump in self.break_scopes.pop().expect("break scope must exist") {
                    self.patch_jump(break_jump, exit_target);
                }
                if is_last {
                    let undefined = self.chunk.add_constant(JSValue::Undefined);
                    self.chunk.emit(Opcode::LoadConst(undefined));
                }
            }
            Statement::Throw(expression) => {
                self.compile_expression(expression)?;
                self.chunk.emit(Opcode::Throw);
            }
            Statement::Try {
                block,
                handler,
                finalizer,
            } => {
                let try_start = self.chunk.code.len();
                self.chunk.emit(Opcode::PushTry {
                    catch_target: None,
                    finally_target: None,
                });
                self.compile_statements(block, false)?;
                self.chunk.emit(Opcode::PopTry);
                let try_exit = self.chunk.code.len();
                self.chunk.emit(Opcode::Jump(usize::MAX));

                let catch_start = handler.as_ref().map(|_| self.chunk.code.len());
                let mut catch_try = None;
                let mut catch_exit = None;
                if let Some((binding, body)) = handler {
                    if finalizer.is_some() {
                        catch_try = Some(self.chunk.code.len());
                        self.chunk.emit(Opcode::PushTry {
                            catch_target: None,
                            finally_target: None,
                        });
                    }
                    if let Some(binding) = binding {
                        self.chunk.emit(Opcode::DefineVar(binding));
                    } else {
                        self.chunk.emit(Opcode::Pop);
                    }
                    self.compile_statements(body, false)?;
                    if finalizer.is_some() {
                        self.chunk.emit(Opcode::PopTry);
                    }
                    catch_exit = Some(self.chunk.code.len());
                    self.chunk.emit(Opcode::Jump(usize::MAX));
                }

                if let Some(finalizer) = finalizer {
                    let normal_finally = self.chunk.code.len();
                    self.chunk.emit(Opcode::BeginFinally);
                    let finally_start = self.chunk.code.len();
                    self.compile_statements(finalizer, false)?;
                    self.chunk.emit(Opcode::EndFinally);

                    self.patch_jump(try_exit, normal_finally);
                    if let Some(catch_exit) = catch_exit {
                        self.patch_jump(catch_exit, normal_finally);
                    }
                    self.patch_try(try_start, catch_start, Some(finally_start));
                    if let Some(catch_try) = catch_try {
                        self.patch_try(catch_try, None, Some(finally_start));
                    }
                    if is_last {
                        let undefined = self.chunk.add_constant(JSValue::Undefined);
                        self.chunk.emit(Opcode::LoadConst(undefined));
                    }
                } else {
                    let end = self.chunk.code.len();
                    self.patch_jump(try_exit, end);
                    if let Some(catch_exit) = catch_exit {
                        self.patch_jump(catch_exit, end);
                    }
                    self.patch_try(try_start, catch_start, None);
                    if is_last {
                        let undefined = self.chunk.add_constant(JSValue::Undefined);
                        self.chunk.emit(Opcode::LoadConst(undefined));
                    }
                }
            }
            Statement::Switch {
                discriminant,
                cases,
            } => {
                let temporary = format!("__pixi_switch_{}", self.next_temporary);
                self.next_temporary += 1;
                self.compile_expression(discriminant)?;
                self.chunk.emit(Opcode::DefineVar(temporary.clone()));

                let mut case_jumps = Vec::new();
                for (case_index, (test, _)) in cases.iter().enumerate() {
                    let Some(test) = test else {
                        continue;
                    };
                    self.chunk.emit(Opcode::LoadVar(temporary.clone()));
                    self.compile_expression(test.clone())?;
                    self.chunk.emit(Opcode::StrictEq);
                    let jump = self.chunk.code.len();
                    self.chunk.emit(Opcode::JumpIfTrue(usize::MAX));
                    case_jumps.push((jump, case_index));
                }
                let default_jump = self.chunk.code.len();
                self.chunk.emit(Opcode::Jump(usize::MAX));

                self.break_scopes.push(Vec::new());
                let mut case_starts = Vec::with_capacity(cases.len());
                let mut default_index = None;
                for (case_index, (test, body)) in cases.into_iter().enumerate() {
                    case_starts.push(self.chunk.code.len());
                    if test.is_none() {
                        default_index = Some(case_index);
                    }
                    self.compile_statements(body, false)?;
                }
                let end = self.chunk.code.len();
                for (jump, case_index) in case_jumps {
                    self.patch_jump(jump, case_starts[case_index]);
                }
                self.patch_jump(
                    default_jump,
                    default_index.map(|index| case_starts[index]).unwrap_or(end),
                );
                for break_jump in self.break_scopes.pop().expect("break scope must exist") {
                    self.patch_jump(break_jump, end);
                }
                if is_last {
                    let undefined = self.chunk.add_constant(JSValue::Undefined);
                    self.chunk.emit(Opcode::LoadConst(undefined));
                }
            }
            Statement::Break(label) => {
                let jump = self.chunk.code.len();
                self.chunk.emit(Opcode::Jump(usize::MAX));
                if let Some(label) = label {
                    let Some(context) = self
                        .labels
                        .iter_mut()
                        .rev()
                        .find(|context| context.name == label)
                    else {
                        return Err(JSError::InternalError(format!(
                            "unknown break label '{label}'"
                        )));
                    };
                    context.break_jumps.push(jump);
                } else {
                    let Some(break_scope) = self.break_scopes.last_mut() else {
                        return Err(JSError::InternalError(
                            "break used outside of a loop or switch".to_string(),
                        ));
                    };
                    break_scope.push(jump);
                }
            }
            Statement::Continue(label) => {
                let jump = self.chunk.code.len();
                self.chunk.emit(Opcode::Jump(usize::MAX));
                if let Some(label) = label {
                    let Some(context) = self
                        .labels
                        .iter_mut()
                        .rev()
                        .find(|context| context.name == label)
                    else {
                        return Err(JSError::InternalError(format!(
                            "unknown continue label '{label}'"
                        )));
                    };
                    if !context.is_iteration {
                        return Err(JSError::InternalError(format!(
                            "continue label '{label}' is not an iteration statement"
                        )));
                    }
                    context.continue_jumps.push(jump);
                } else {
                    let Some(loop_context) = self.loops.last_mut() else {
                        return Err(JSError::InternalError(
                            "continue used outside of a loop".to_string(),
                        ));
                    };
                    loop_context.continue_jumps.push(jump);
                }
            }
        }
        Ok(())
    }

    fn compile_statements(&mut self, statements: Vec<Statement>, keep_last: bool) -> JSResult<()> {
        let len = statements.len();
        for (index, statement) in statements.into_iter().enumerate() {
            self.compile_statement(statement, keep_last && index + 1 == len)?;
        }
        Ok(())
    }

    fn patch_jump(&mut self, index: usize, target: usize) {
        match &mut self.chunk.code[index] {
            Opcode::Jump(destination)
            | Opcode::JumpIfFalse(destination)
            | Opcode::JumpIfTrue(destination)
            | Opcode::JumpIfNotNullish(destination) => *destination = target,
            _ => unreachable!("attempted to patch a non-jump opcode"),
        }
    }

    fn patch_labeled_continues(&mut self, label: Option<&str>, target: usize) -> JSResult<()> {
        let Some(label) = label else {
            return Ok(());
        };
        let jumps = {
            let Some(context) = self
                .labels
                .iter_mut()
                .rev()
                .find(|context| context.name == label)
            else {
                return Err(JSError::InternalError(format!(
                    "unknown continue label '{label}'"
                )));
            };
            std::mem::take(&mut context.continue_jumps)
        };
        for jump in jumps {
            self.patch_jump(jump, target);
        }
        Ok(())
    }

    fn patch_try(
        &mut self,
        index: usize,
        catch_target: Option<usize>,
        finally_target: Option<usize>,
    ) {
        let Opcode::PushTry {
            catch_target: catch,
            finally_target: finally,
        } = &mut self.chunk.code[index]
        else {
            unreachable!("attempted to patch a non-try opcode");
        };
        *catch = catch_target;
        *finally = finally_target;
    }

    /// 式をコンパイル
    fn compile_expression(&mut self, expression: Expression) -> JSResult<()> {
        match expression {
            Expression::Literal(lit) => {
                let value = match lit {
                    Literal::Undefined => JSValue::Undefined,
                    Literal::Null => JSValue::Null,
                    Literal::Boolean(b) => JSValue::Boolean(b),
                    Literal::Number(n) => JSValue::Number(n),
                    Literal::String(s) => JSValue::String(s),
                };
                let idx = self.chunk.add_constant(value);
                self.chunk.emit(Opcode::LoadConst(idx));
            }
            Expression::Identifier(name) => {
                self.chunk.emit(Opcode::LoadVar(name));
            }
            Expression::Binary { op, left, right } => {
                if matches!(op, BinaryOp::And | BinaryOp::Or | BinaryOp::Nullish) {
                    self.compile_expression(*left)?;
                    self.chunk.emit(Opcode::Dup);
                    let branch = self.chunk.code.len();
                    self.chunk.emit(match op {
                        BinaryOp::And => Opcode::JumpIfFalse(usize::MAX),
                        BinaryOp::Or => Opcode::JumpIfTrue(usize::MAX),
                        BinaryOp::Nullish => Opcode::JumpIfNotNullish(usize::MAX),
                        _ => unreachable!(),
                    });
                    self.chunk.emit(Opcode::Pop);
                    self.compile_expression(*right)?;
                    let end = self.chunk.code.len();
                    self.patch_jump(branch, end);
                    return Ok(());
                }

                self.compile_expression(*left)?;
                self.compile_expression(*right)?;

                let opcode = match op {
                    BinaryOp::Add => Opcode::Add,
                    BinaryOp::Sub => Opcode::Sub,
                    BinaryOp::Mul => Opcode::Mul,
                    BinaryOp::Div => Opcode::Div,
                    BinaryOp::Mod => Opcode::Mod,
                    BinaryOp::Power => Opcode::Power,
                    BinaryOp::Eq => Opcode::Eq,
                    BinaryOp::NotEq => Opcode::NotEq,
                    BinaryOp::StrictEq => Opcode::StrictEq,
                    BinaryOp::StrictNotEq => Opcode::StrictNotEq,
                    BinaryOp::Lt => Opcode::Lt,
                    BinaryOp::Gt => Opcode::Gt,
                    BinaryOp::LtEq => Opcode::LtEq,
                    BinaryOp::GtEq => Opcode::GtEq,
                    BinaryOp::In => Opcode::In,
                    BinaryOp::Instanceof => Opcode::Instanceof,
                    BinaryOp::And | BinaryOp::Or | BinaryOp::Nullish => unreachable!(),
                    BinaryOp::BitAnd => Opcode::BitAnd,
                    BinaryOp::BitOr => Opcode::BitOr,
                    BinaryOp::BitXor => Opcode::BitXor,
                    BinaryOp::LeftShift => Opcode::LeftShift,
                    BinaryOp::RightShift => Opcode::RightShift,
                    BinaryOp::UnsignedRightShift => Opcode::UnsignedRightShift,
                };
                self.chunk.emit(opcode);
            }
            Expression::Unary { op, arg } => {
                if op == UnaryOp::Delete {
                    match *arg {
                        Expression::MemberAccess {
                            object, property, ..
                        } => {
                            self.compile_expression(*object)?;
                            self.compile_expression(*property)?;
                            self.chunk.emit(Opcode::DeleteProperty);
                        }
                        Expression::Identifier(_) => {
                            let value = self.chunk.add_constant(JSValue::Boolean(false));
                            self.chunk.emit(Opcode::LoadConst(value));
                        }
                        expression => {
                            self.compile_expression(expression)?;
                            self.chunk.emit(Opcode::Pop);
                            let value = self.chunk.add_constant(JSValue::Boolean(true));
                            self.chunk.emit(Opcode::LoadConst(value));
                        }
                    }
                    return Ok(());
                }
                self.compile_expression(*arg)?;

                let opcode = match op {
                    UnaryOp::Plus => return Ok(()), // +x は x と同じ
                    UnaryOp::Minus => Opcode::Neg,
                    UnaryOp::Not => Opcode::Not,
                    UnaryOp::BitNot => Opcode::BitNot,
                    UnaryOp::Typeof => Opcode::Typeof,
                    UnaryOp::Void => Opcode::Void,
                    UnaryOp::Delete => unreachable!(),
                };
                self.chunk.emit(opcode);
            }
            Expression::Assignment { left, right } => {
                match *left {
                    Expression::Identifier(name) => {
                        self.compile_expression(*right)?;
                        self.chunk.emit(Opcode::StoreVar(name.clone()));
                        self.chunk.emit(Opcode::LoadVar(name));
                    }
                    Expression::MemberAccess {
                        object,
                        property,
                        computed,
                    } => {
                        // obj[prop] = value の形式
                        // スタック順序: [obj, key, value]
                        self.compile_expression(*object)?;
                        if computed {
                            self.compile_expression(*property)?;
                        } else {
                            // obj.prop の場合、property は文字列リテラル
                            self.compile_expression(*property)?;
                        }
                        self.compile_expression(*right)?;
                        self.chunk.emit(Opcode::SetProperty);
                    }
                    target @ (Expression::ArrayLiteral(_) | Expression::ObjectLiteral(_)) => {
                        let pattern = assignment_binding_pattern(target)?;
                        self.compile_expression(*right)?;
                        self.chunk.emit(Opcode::Dup);
                        self.store_binding_pattern(&pattern, false)?;
                    }
                    target => {
                        return Err(JSError::TypeError(format!(
                            "Invalid assignment target: {target:?}"
                        )));
                    }
                }
            }
            Expression::Update {
                arg,
                increment,
                prefix,
            } => match *arg {
                Expression::Identifier(name) => {
                    self.chunk.emit(Opcode::LoadVar(name.clone()));
                    if !prefix {
                        self.chunk.emit(Opcode::Dup);
                    }
                    let one = self.chunk.add_constant(JSValue::Number(1.0));
                    self.chunk.emit(Opcode::LoadConst(one));
                    self.chunk
                        .emit(if increment { Opcode::Add } else { Opcode::Sub });
                    if prefix {
                        self.chunk.emit(Opcode::Dup);
                    }
                    self.chunk.emit(Opcode::StoreVar(name));
                }
                Expression::MemberAccess {
                    object, property, ..
                } => {
                    self.compile_expression(*object)?;
                    self.compile_expression(*property)?;
                    self.chunk.emit(Opcode::Dup2);
                    self.chunk.emit(Opcode::GetProperty);
                    if !prefix {
                        self.chunk.emit(Opcode::Dup);
                    }
                    let one = self.chunk.add_constant(JSValue::Number(1.0));
                    self.chunk.emit(Opcode::LoadConst(one));
                    self.chunk
                        .emit(if increment { Opcode::Add } else { Opcode::Sub });
                    self.chunk.emit(if prefix {
                        Opcode::SetProperty
                    } else {
                        Opcode::SetPropertyKeepOld
                    });
                }
                _ => {
                    return Err(JSError::TypeError("Invalid update target".to_string()));
                }
            },
            Expression::Conditional {
                test,
                consequent,
                alternate,
            } => {
                self.compile_expression(*test)?;
                let alternate_jump = self.chunk.code.len();
                self.chunk.emit(Opcode::JumpIfFalse(usize::MAX));
                self.compile_expression(*consequent)?;
                let end_jump = self.chunk.code.len();
                self.chunk.emit(Opcode::Jump(usize::MAX));
                let alternate_start = self.chunk.code.len();
                self.patch_jump(alternate_jump, alternate_start);
                self.compile_expression(*alternate)?;
                let end = self.chunk.code.len();
                self.patch_jump(end_jump, end);
            }
            Expression::Sequence(expressions) => {
                let len = expressions.len();
                for (index, expression) in expressions.into_iter().enumerate() {
                    self.compile_expression(expression)?;
                    if index + 1 < len {
                        self.chunk.emit(Opcode::Pop);
                    }
                }
            }
            Expression::This => {
                self.chunk.emit(Opcode::LoadThis);
            }
            Expression::Super => {
                let binding = self.super_binding.clone().ok_or_else(|| {
                    JSError::InternalError("'super' is only valid inside a derived class".into())
                })?;
                self.chunk.emit(Opcode::LoadVar(binding));
            }
            Expression::ArrayLiteral(elements) => {
                // 空の配列を作成してスタックにプッシュ
                self.chunk.emit(Opcode::NewArray(0));

                // 各要素をコンパイルして配列に追加
                for element in elements {
                    match element {
                        Expression::Spread(value) => {
                            self.compile_expression(*value)?;
                            self.chunk.emit(Opcode::ArrayExtend);
                        }
                        value => {
                            self.compile_expression(value)?;
                            self.chunk.emit(Opcode::ArrayAppend);
                        }
                    }
                }
            }
            Expression::TemplateObject { cooked, raw } => {
                self.chunk.emit(Opcode::NewArray(0));
                for value in cooked {
                    let value = self.chunk.add_constant(JSValue::String(value));
                    self.chunk.emit(Opcode::LoadConst(value));
                    self.chunk.emit(Opcode::ArrayAppend);
                }
                self.chunk.emit(Opcode::NewArray(0));
                for value in raw {
                    let value = self.chunk.add_constant(JSValue::String(value));
                    self.chunk.emit(Opcode::LoadConst(value));
                    self.chunk.emit(Opcode::ArrayAppend);
                }
                let key = self.chunk.add_constant(JSValue::String("raw".to_string()));
                self.chunk.emit(Opcode::LoadConst(key));
                self.chunk.emit(Opcode::ObjectSetProperty);
            }
            Expression::ObjectLiteral(properties) => {
                // 空のオブジェクトを作成してスタックにプッシュ
                self.chunk.emit(Opcode::NewObject);

                // 各プロパティを設定
                for property in properties {
                    if let ObjectProperty::Spread(source) = property {
                        self.compile_expression(source)?;
                        self.chunk.emit(Opcode::ObjectSpread);
                        continue;
                    }
                    if let ObjectProperty::ComputedData { key, value } = property {
                        self.compile_expression(value)?;
                        self.compile_expression(key)?;
                        self.chunk.emit(Opcode::ObjectSetProperty);
                        continue;
                    }
                    let (key, value, opcode) = match property {
                        ObjectProperty::Data { key, value } => {
                            (key, value, Opcode::ObjectSetProperty)
                        }
                        ObjectProperty::Getter { key, body } => (
                            key,
                            Expression::Function {
                                name: None,
                                params: Vec::new(),
                                body,
                                is_generator: false,
                            },
                            Opcode::ObjectDefineGetter,
                        ),
                        ObjectProperty::Setter {
                            key,
                            parameter,
                            body,
                        } => (
                            key,
                            Expression::Function {
                                name: None,
                                params: vec![parameter],
                                body,
                                is_generator: false,
                            },
                            Opcode::ObjectDefineSetter,
                        ),
                        ObjectProperty::ComputedData { .. } | ObjectProperty::Spread(_) => {
                            unreachable!()
                        }
                    };
                    self.compile_expression(value)?;
                    let key_idx = self.chunk.add_constant(JSValue::String(key));
                    self.chunk.emit(Opcode::LoadConst(key_idx));
                    self.chunk.emit(opcode);
                }
            }
            Expression::RegExpLiteral { pattern, flags } => {
                self.chunk.emit(Opcode::NewRegExp(pattern, flags));
            }
            Expression::MemberAccess {
                object,
                property,
                computed,
            } => {
                // obj[prop] または obj.prop
                self.compile_expression(*object)?;
                if computed {
                    // obj[prop] - property を評価
                    self.compile_expression(*property)?;
                } else {
                    // obj.prop - property は文字列リテラル
                    self.compile_expression(*property)?;
                }
                self.chunk.emit(Opcode::GetProperty);
            }
            Expression::OptionalMemberAccess {
                object,
                property,
                computed: _,
            } => {
                self.compile_expression(*object)?;
                self.compile_expression(*property)?;
                self.chunk.emit(Opcode::GetProperty);
            }
            Expression::Function {
                name,
                params,
                body,
                is_generator,
            } => {
                self.emit_function_value(name, params, body, None, is_generator)?;
            }
            Expression::ArrowFunction { params, body } => {
                let program = Program { body };
                let function_chunk = Compiler::new().compile_function(program)?;
                let func_value =
                    JSValue::ArrowFunction(function_chunk, params.clone(), None, None, 0);
                let idx = self.chunk.add_constant(func_value);
                self.chunk.emit(Opcode::CreateFunction(idx));
            }
            Expression::Call { callee, args } => {
                if matches!(&*callee, Expression::Super) {
                    let binding = self.super_binding.clone().ok_or_else(|| {
                        JSError::InternalError(
                            "'super' is only valid inside a derived class".into(),
                        )
                    })?;
                    self.chunk.emit(Opcode::LoadVar(binding));
                    let call = self.chunk.add_constant(JSValue::String("call".to_string()));
                    self.chunk.emit(Opcode::LoadConst(call));
                    self.chunk.emit(Opcode::LoadThis);
                    for arg in &args {
                        self.compile_expression(arg.clone())?;
                    }
                    self.chunk.emit(Opcode::CallMethod(args.len() + 1));
                    return Ok(());
                }
                // MemberAccess (obj.prop(args)) は receiver を使うので専用の CallMethod を出す
                if let Expression::MemberAccess {
                    object,
                    property,
                    computed: _,
                } = *callee
                {
                    self.compile_expression(*object)?;
                    self.compile_expression(*property)?;

                    if args.iter().any(|arg| matches!(arg, Expression::Spread(_))) {
                        self.chunk.emit(Opcode::NewArray(0));
                        for arg in args {
                            match arg {
                                Expression::Spread(value) => {
                                    self.compile_expression(*value)?;
                                    self.chunk.emit(Opcode::ArrayExtend);
                                }
                                value => {
                                    self.compile_expression(value)?;
                                    self.chunk.emit(Opcode::ArrayAppend);
                                }
                            }
                        }
                        self.chunk.emit(Opcode::CallMethodArray);
                    } else {
                        for arg in &args {
                            self.compile_expression(arg.clone())?;
                        }
                        self.chunk.emit(Opcode::CallMethod(args.len()));
                    }
                } else {
                    // 通常の関数呼び出し
                    let callee_name = match &*callee {
                        Expression::Identifier(name) => Some(name.clone()),
                        _ => None,
                    };
                    self.compile_expression(*callee)?;
                    if args.iter().any(|arg| matches!(arg, Expression::Spread(_))) {
                        self.chunk.emit(Opcode::NewArray(0));
                        for arg in args {
                            match arg {
                                Expression::Spread(value) => {
                                    self.compile_expression(*value)?;
                                    self.chunk.emit(Opcode::ArrayExtend);
                                }
                                value => {
                                    self.compile_expression(value)?;
                                    self.chunk.emit(Opcode::ArrayAppend);
                                }
                            }
                        }
                        self.chunk.emit(Opcode::CallFunctionArray);
                    } else {
                        for arg in &args {
                            self.compile_expression(arg.clone())?;
                        }
                        let arg_count = args.len();
                        self.chunk.emit(if let Some(name) = callee_name {
                            Opcode::CallFunctionNamed(arg_count, name)
                        } else {
                            Opcode::CallFunction(arg_count)
                        });
                    }
                }
            }
            Expression::OptionalCall { callee, args } => match *callee {
                Expression::OptionalMemberAccess {
                    object, property, ..
                }
                | Expression::MemberAccess {
                    object, property, ..
                } => {
                    self.compile_expression(*object)?;
                    self.compile_expression(*property)?;
                    for arg in &args {
                        self.compile_expression(arg.clone())?;
                    }
                    self.chunk.emit(Opcode::CallMethodOptional(args.len()));
                }
                callee => {
                    self.compile_expression(callee)?;
                    for arg in &args {
                        self.compile_expression(arg.clone())?;
                    }
                    self.chunk.emit(Opcode::CallFunctionOptional(args.len()));
                }
            },
            Expression::New { callee, args } => {
                let constructor_name = match callee.as_ref() {
                    Expression::Identifier(name) => Some(name.clone()),
                    _ => None,
                };
                self.compile_expression(*callee)?;
                for arg in &args {
                    self.compile_expression(arg.clone())?;
                }
                self.chunk
                    .emit(Opcode::Construct(args.len(), constructor_name));
            }
            Expression::Class {
                name,
                super_class,
                constructor,
                methods,
            } => {
                self.compile_class_expression(
                    name,
                    super_class.map(|value| *value),
                    constructor,
                    methods,
                )?;
            }
            Expression::Yield { value, delegate } => {
                let output = self
                    .generator_output
                    .clone()
                    .ok_or_else(|| JSError::InternalError("yield outside generator".to_string()))?;
                self.chunk.emit(Opcode::LoadVar(output));
                self.compile_expression(*value)?;
                self.chunk.emit(if delegate {
                    Opcode::ArrayExtend
                } else {
                    Opcode::ArrayAppend
                });
                self.chunk.emit(Opcode::Pop);
                let undefined = self.chunk.add_constant(JSValue::Undefined);
                self.chunk.emit(Opcode::LoadConst(undefined));
            }
            Expression::Spread(_) => {
                return Err(JSError::InternalError(
                    "spread expression is not valid in this position".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn emit_function_value(
        &mut self,
        name: Option<String>,
        params: Vec<String>,
        body: Vec<Statement>,
        super_binding: Option<String>,
        is_generator: bool,
    ) -> JSResult<()> {
        let mut compiler = Compiler::with_super_binding(super_binding);
        if is_generator {
            compiler.generator_output = Some("__pixi_generator_values".to_string());
        }
        let function_chunk = compiler.compile_function(Program { body })?;
        let value = JSValue::Function(function_chunk, params, None, name, 0);
        let index = self.chunk.add_constant(value);
        self.chunk.emit(Opcode::CreateFunction(index));
        Ok(())
    }

    fn define_binding_pattern(&mut self, pattern: &BindingPattern) -> JSResult<()> {
        let mut names = Vec::new();
        binding_pattern_names(pattern, &mut names);
        for name in names {
            let undefined = self.chunk.add_constant(JSValue::Undefined);
            self.chunk.emit(Opcode::LoadConst(undefined));
            self.chunk.emit(Opcode::DefineVar(name));
        }
        Ok(())
    }

    fn store_binding_pattern(&mut self, pattern: &BindingPattern, define: bool) -> JSResult<()> {
        match pattern {
            BindingPattern::Identifier(name) => {
                self.chunk.emit(if define {
                    Opcode::DefineVar(name.clone())
                } else {
                    Opcode::StoreVar(name.clone())
                });
            }
            BindingPattern::Target(Expression::MemberAccess {
                object, property, ..
            }) => {
                let value = format!("__pixi_pattern_value_{}", self.next_temporary);
                self.next_temporary += 1;
                self.chunk.emit(Opcode::DefineVar(value.clone()));
                self.compile_expression(*object.clone())?;
                self.compile_expression(*property.clone())?;
                self.chunk.emit(Opcode::LoadVar(value));
                self.chunk.emit(Opcode::SetProperty);
                self.chunk.emit(Opcode::Pop);
            }
            BindingPattern::Target(target) => {
                return Err(JSError::TypeError(format!(
                    "Invalid destructuring assignment target: {target:?}"
                )));
            }
            BindingPattern::Array(items) => {
                let source = format!("__pixi_pattern_{}", self.next_temporary);
                self.next_temporary += 1;
                self.chunk.emit(Opcode::DefineVar(source.clone()));
                for (index, item) in items.iter().enumerate() {
                    let Some(item) = item else { continue };
                    match item {
                        BindingPattern::Rest(rest) => {
                            self.emit_array_slice(&source, index);
                            self.store_binding_pattern(rest, define)?;
                        }
                        item => {
                            self.chunk.emit(Opcode::LoadVar(source.clone()));
                            let index = self.chunk.add_constant(JSValue::Number(index as f64));
                            self.chunk.emit(Opcode::LoadConst(index));
                            self.chunk.emit(Opcode::GetProperty);
                            self.store_binding_pattern(item, define)?;
                        }
                    }
                }
            }
            BindingPattern::Object(properties) => {
                let source = format!("__pixi_pattern_{}", self.next_temporary);
                self.next_temporary += 1;
                self.chunk.emit(Opcode::DefineVar(source.clone()));
                let mut excluded = Vec::new();
                for (key, item) in properties {
                    if let BindingPattern::Rest(rest) = item {
                        self.chunk.emit(Opcode::LoadVar(source.clone()));
                        self.chunk.emit(Opcode::ObjectRest(excluded.clone()));
                        self.store_binding_pattern(rest, define)?;
                        continue;
                    }
                    excluded.push(key.clone());
                    self.chunk.emit(Opcode::LoadVar(source.clone()));
                    let key = self.chunk.add_constant(JSValue::String(key.clone()));
                    self.chunk.emit(Opcode::LoadConst(key));
                    self.chunk.emit(Opcode::GetProperty);
                    self.store_binding_pattern(item, define)?;
                }
            }
            BindingPattern::Rest(rest) => self.store_binding_pattern(rest, define)?,
            BindingPattern::Default(pattern, default) => {
                let source = format!("__pixi_pattern_default_{}", self.next_temporary);
                self.next_temporary += 1;
                self.chunk.emit(Opcode::DefineVar(source.clone()));
                self.chunk.emit(Opcode::LoadVar(source.clone()));
                let undefined = self.chunk.add_constant(JSValue::Undefined);
                self.chunk.emit(Opcode::LoadConst(undefined));
                self.chunk.emit(Opcode::StrictEq);
                let keep_value = self.chunk.code.len();
                self.chunk.emit(Opcode::JumpIfFalse(usize::MAX));
                self.compile_expression(default.clone())?;
                self.chunk.emit(Opcode::StoreVar(source.clone()));
                let target = self.chunk.code.len();
                self.patch_jump(keep_value, target);
                self.chunk.emit(Opcode::LoadVar(source));
                self.store_binding_pattern(pattern, define)?;
            }
        }
        Ok(())
    }

    fn emit_array_slice(&mut self, source: &str, start: usize) {
        self.chunk.emit(Opcode::LoadVar("Array".to_string()));
        let prototype = self
            .chunk
            .add_constant(JSValue::String("prototype".to_string()));
        self.chunk.emit(Opcode::LoadConst(prototype));
        self.chunk.emit(Opcode::GetProperty);
        let slice = self
            .chunk
            .add_constant(JSValue::String("slice".to_string()));
        self.chunk.emit(Opcode::LoadConst(slice));
        self.chunk.emit(Opcode::GetProperty);
        let call = self.chunk.add_constant(JSValue::String("call".to_string()));
        self.chunk.emit(Opcode::LoadConst(call));
        self.chunk.emit(Opcode::LoadVar(source.to_string()));
        let start = self.chunk.add_constant(JSValue::Number(start as f64));
        self.chunk.emit(Opcode::LoadConst(start));
        self.chunk.emit(Opcode::CallMethod(2));
    }

    fn compile_class_expression(
        &mut self,
        name: Option<String>,
        super_class: Option<Expression>,
        constructor: Option<ClassMethod>,
        methods: Vec<ClassMethod>,
    ) -> JSResult<()> {
        let id = self.next_temporary;
        self.next_temporary += 1;
        let class_binding = format!("__pixi_class_{id}");
        let super_binding = super_class.as_ref().map(|_| format!("__pixi_super_{id}"));

        if let (Some(expression), Some(binding)) = (super_class, super_binding.as_ref()) {
            self.compile_expression(expression)?;
            self.chunk.emit(Opcode::DefineVar(binding.clone()));
        }

        let (params, body) = constructor
            .map(|method| (method.params, method.body))
            .unwrap_or_default();
        self.emit_function_value(name.clone(), params, body, super_binding.clone(), false)?;
        self.chunk.emit(Opcode::DefineVar(class_binding.clone()));

        if let Some(super_binding) = super_binding.as_ref() {
            self.emit_set_prototype(&class_binding, super_binding, false);
            self.emit_set_prototype(&class_binding, super_binding, true);
        }

        for method in methods {
            if method.is_static {
                self.chunk.emit(Opcode::LoadVar(class_binding.clone()));
            } else {
                self.chunk.emit(Opcode::LoadVar(class_binding.clone()));
                let prototype = self
                    .chunk
                    .add_constant(JSValue::String("prototype".to_string()));
                self.chunk.emit(Opcode::LoadConst(prototype));
                self.chunk.emit(Opcode::GetProperty);
            }
            let accessor_opcode = match method.kind {
                ClassMethodKind::Method => None,
                ClassMethodKind::Getter => Some(Opcode::ObjectDefineGetter),
                ClassMethodKind::Setter => Some(Opcode::ObjectDefineSetter),
            };
            if accessor_opcode.is_some() {
                self.emit_function_value(
                    Some(method.name.clone()),
                    method.params.clone(),
                    method.body.clone(),
                    super_binding.clone(),
                    method.is_generator,
                )?;
            }
            if let Some(computed_name) = method.computed_name {
                self.compile_expression(*computed_name)?;
            } else {
                let key = self
                    .chunk
                    .add_constant(JSValue::String(method.name.clone()));
                self.chunk.emit(Opcode::LoadConst(key));
            }
            if let Some(opcode) = accessor_opcode {
                self.chunk.emit(opcode);
            } else {
                self.emit_function_value(
                    Some(method.name.clone()),
                    method.params,
                    method.body,
                    super_binding.clone(),
                    method.is_generator,
                )?;
                self.chunk.emit(Opcode::SetProperty);
            }
            self.chunk.emit(Opcode::Pop);
        }

        self.chunk.emit(Opcode::LoadVar(class_binding));
        Ok(())
    }

    fn emit_set_prototype(&mut self, class_binding: &str, super_binding: &str, prototype: bool) {
        self.chunk.emit(Opcode::LoadVar("Object".to_string()));
        let method = self
            .chunk
            .add_constant(JSValue::String("setPrototypeOf".to_string()));
        self.chunk.emit(Opcode::LoadConst(method));
        if prototype {
            self.chunk.emit(Opcode::LoadVar(class_binding.to_string()));
            let property = self
                .chunk
                .add_constant(JSValue::String("prototype".to_string()));
            self.chunk.emit(Opcode::LoadConst(property));
            self.chunk.emit(Opcode::GetProperty);
            self.chunk.emit(Opcode::LoadVar(super_binding.to_string()));
            let property = self
                .chunk
                .add_constant(JSValue::String("prototype".to_string()));
            self.chunk.emit(Opcode::LoadConst(property));
            self.chunk.emit(Opcode::GetProperty);
        } else {
            self.chunk.emit(Opcode::LoadVar(class_binding.to_string()));
            self.chunk.emit(Opcode::LoadVar(super_binding.to_string()));
        }
        self.chunk.emit(Opcode::CallMethod(2));
        self.chunk.emit(Opcode::Pop);
    }
}

fn collect_var_declarations(statements: &[Statement], names: &mut Vec<String>) {
    for statement in statements {
        match statement {
            Statement::VariableDeclaration { kind, declarations } if *kind == VarKind::Var => {
                names.extend(declarations.iter().map(|(name, _)| name.clone()));
            }
            Statement::PatternDeclaration {
                kind: VarKind::Var,
                binding,
                ..
            } => binding_pattern_names(binding, names),
            Statement::Block(body)
            | Statement::While { body, .. }
            | Statement::DoWhile { body, .. } => collect_var_declarations(body, names),
            Statement::Labeled { body, .. } => {
                collect_var_declarations(std::slice::from_ref(body.as_ref()), names);
            }
            Statement::If {
                consequent,
                alternate,
                ..
            } => {
                collect_var_declarations(consequent, names);
                if let Some(alternate) = alternate {
                    collect_var_declarations(alternate, names);
                }
            }
            Statement::For { init, body, .. } => {
                if let Some(init) = init {
                    collect_var_declarations(std::slice::from_ref(init.as_ref()), names);
                }
                collect_var_declarations(body, names);
            }
            Statement::ForIn {
                binding,
                kind,
                body,
                ..
            } => {
                if *kind == Some(VarKind::Var) {
                    binding_pattern_names(binding, names);
                }
                collect_var_declarations(body, names);
            }
            Statement::ForOf {
                binding,
                kind,
                body,
                ..
            } => {
                if *kind == Some(VarKind::Var) {
                    binding_pattern_names(binding, names);
                }
                collect_var_declarations(body, names);
            }
            Statement::Try {
                block,
                handler,
                finalizer,
            } => {
                collect_var_declarations(block, names);
                if let Some((_, body)) = handler {
                    collect_var_declarations(body, names);
                }
                if let Some(finalizer) = finalizer {
                    collect_var_declarations(finalizer, names);
                }
            }
            Statement::Switch { cases, .. } => {
                for (_, body) in cases {
                    collect_var_declarations(body, names);
                }
            }
            Statement::Empty
            | Statement::Expression(_)
            | Statement::VariableDeclaration { .. }
            | Statement::PatternDeclaration { .. }
            | Statement::Return(_)
            | Statement::FunctionDeclaration { .. }
            | Statement::Throw(_)
            | Statement::Break(_)
            | Statement::Continue(_) => {}
        }
    }
}

fn binding_pattern_names(pattern: &BindingPattern, names: &mut Vec<String>) {
    match pattern {
        BindingPattern::Identifier(name) => names.push(name.clone()),
        BindingPattern::Target(_) => {}
        BindingPattern::Array(items) => {
            for item in items.iter().flatten() {
                binding_pattern_names(item, names);
            }
        }
        BindingPattern::Object(properties) => {
            for (_, value) in properties {
                binding_pattern_names(value, names);
            }
        }
        BindingPattern::Rest(value) => binding_pattern_names(value, names),
        BindingPattern::Default(value, _) => binding_pattern_names(value, names),
    }
}

fn assignment_binding_pattern(expression: Expression) -> JSResult<BindingPattern> {
    match expression {
        Expression::Identifier(name) => Ok(BindingPattern::Identifier(name)),
        target @ Expression::MemberAccess { .. } => Ok(BindingPattern::Target(target)),
        Expression::ArrayLiteral(items) => items
            .into_iter()
            .map(|item| {
                let pattern = match item {
                    Expression::Spread(value) => {
                        BindingPattern::Rest(Box::new(assignment_binding_pattern(*value)?))
                    }
                    Expression::Assignment { left, right } => BindingPattern::Default(
                        Box::new(assignment_binding_pattern(*left)?),
                        *right,
                    ),
                    value => assignment_binding_pattern(value)?,
                };
                Ok(Some(pattern))
            })
            .collect::<JSResult<Vec<_>>>()
            .map(BindingPattern::Array),
        Expression::ObjectLiteral(properties) => {
            let mut bindings = Vec::new();
            for property in properties {
                match property {
                    ObjectProperty::Data { key, value } => {
                        bindings.push((key, assignment_binding_pattern(value)?));
                    }
                    ObjectProperty::Spread(value) => bindings.push((
                        String::new(),
                        BindingPattern::Rest(Box::new(assignment_binding_pattern(value)?)),
                    )),
                    _ => {
                        return Err(JSError::TypeError(
                            "Invalid property in object assignment pattern".to_string(),
                        ));
                    }
                }
            }
            Ok(BindingPattern::Object(bindings))
        }
        target => Err(JSError::TypeError(format!(
            "Invalid destructuring assignment target: {target:?}"
        ))),
    }
}

impl Default for Compiler {
    /// デフォルト実装
    fn default() -> Self {
        Self::new()
    }
}
