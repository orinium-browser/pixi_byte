use crate::error::{JSError, JSResult};
use crate::parser::{
    BinaryOp, Expression, Literal, ObjectProperty, Program, Statement, UnaryOp, VarKind,
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
    ObjectSetProperty,  // obj[key] = value - スタックから key, value をポップ、obj は残る
    ObjectDefineGetter, // object literal getter; object remains on the stack
    ObjectDefineSetter, // object literal setter; object remains on the stack
    Enumerate,          // for-in用の列挙可能なプロパティ名配列を生成

    // 関数操作
    CreateFunction(usize), // 定数プール内の関数オブジェクトを生成してプッシュ（func chunk idx）
    CallFunction(usize),   // 呼び出し（引数個数） - スタックから argN..arg1, func を使う
    CallMethod(usize), // メソッド呼び出し（arg count） - スタック: ..., object, property, arg1..argN
    Construct(usize),  // コンストラクタ呼び出し（引数個数）

    // 制御フロー
    Jump(usize),        // 無条件ジャンプ
    JumpIfFalse(usize), // false の場合ジャンプ
    JumpIfTrue(usize),  // true の場合ジャンプ
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
        let undefined = self.chunk.add_constant(JSValue::Undefined);
        self.chunk.emit(Opcode::LoadConst(undefined));
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
            Statement::Return(expr) => {
                if let Some(expr) = expr {
                    self.compile_expression(expr)?;
                } else {
                    let idx = self.chunk.add_constant(JSValue::Undefined);
                    self.chunk.emit(Opcode::LoadConst(idx));
                }
                self.chunk.emit(Opcode::Return);
            }
            Statement::FunctionDeclaration { name, params, body } => {
                // 関数本体をコンパイル
                let program = Program { body };
                let function_chunk = Compiler::new().compile_function(program)?;

                // 現在のチャンクに関数を追加 (chunk, params)
                let idx = self.chunk.add_constant(JSValue::Function(
                    function_chunk,
                    params.clone(),
                    None,
                    Some(name.clone()),
                    0,
                ));
                self.chunk.emit(Opcode::CreateFunction(idx));

                // 関数名を変数としてストア
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
                    let undefined = self.chunk.add_constant(JSValue::Undefined);
                    self.chunk.emit(Opcode::LoadConst(undefined));
                    self.chunk.emit(Opcode::DefineVar(binding.clone()));
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
                self.chunk.emit(Opcode::StoreVar(binding));

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
            | Opcode::JumpIfTrue(destination) => *destination = target,
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
                if matches!(op, BinaryOp::And | BinaryOp::Or) {
                    self.compile_expression(*left)?;
                    self.chunk.emit(Opcode::Dup);
                    let branch = self.chunk.code.len();
                    self.chunk.emit(match op {
                        BinaryOp::And => Opcode::JumpIfFalse(usize::MAX),
                        BinaryOp::Or => Opcode::JumpIfTrue(usize::MAX),
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
                    BinaryOp::And | BinaryOp::Or => unreachable!(),
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
                    _ => {
                        return Err(JSError::TypeError("Invalid assignment target".to_string()));
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
            Expression::ArrayLiteral(elements) => {
                // 空の配列を作成してスタックにプッシュ
                self.chunk.emit(Opcode::NewArray(0));

                // 各要素をコンパイルして配列に追加
                for (i, element) in elements.into_iter().enumerate() {
                    // 値をコンパイル
                    self.compile_expression(element)?;
                    // インデックスをプッシュ
                    let idx = self.chunk.add_constant(JSValue::Number(i as f64));
                    self.chunk.emit(Opcode::LoadConst(idx));
                    // スタック: [array, value, index]
                    // ArraySetElementを使用（新しいオペコード）
                    self.chunk.emit(Opcode::ArrayPush);
                }
            }
            Expression::ObjectLiteral(properties) => {
                // 空のオブジェクトを作成してスタックにプッシュ
                self.chunk.emit(Opcode::NewObject);

                // 各プロパティを設定
                for property in properties {
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
                            },
                            Opcode::ObjectDefineSetter,
                        ),
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
            Expression::Function { name, params, body } => {
                // 関数本体をコンパイル
                let program = Program { body };
                let function_chunk = Compiler::new().compile_function(program)?;

                // 現在のチャンクに関数オブジェクト（チャンク + params）を追加
                // 関数式の場合は name があれば保持する
                let func_value =
                    JSValue::Function(function_chunk, params.clone(), None, name.clone(), 0);
                let idx = self.chunk.add_constant(func_value);
                self.chunk.emit(Opcode::CreateFunction(idx));
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
                // MemberAccess (obj.prop(args)) は receiver を使うので専用の CallMethod を出す
                if let Expression::MemberAccess {
                    object,
                    property,
                    computed: _,
                } = *callee
                {
                    self.compile_expression(*object)?;
                    self.compile_expression(*property)?;

                    // 引数をコンパイル
                    for arg in &args {
                        self.compile_expression(arg.clone())?;
                    }

                    // CallMethod は property と object を使ってメソッドを取得し呼び出す
                    let arg_count = args.len();
                    // 新 opcode を直接 encode as CallFunction for non-member and CallMethod for member
                    self.chunk.emit(Opcode::CallMethod(arg_count));
                } else {
                    // 通常の関数呼び出し
                    self.compile_expression(*callee)?;
                    for arg in &args {
                        self.compile_expression(arg.clone())?;
                    }
                    let arg_count = args.len();
                    self.chunk.emit(Opcode::CallFunction(arg_count));
                }
            }
            Expression::New { callee, args } => {
                self.compile_expression(*callee)?;
                for arg in &args {
                    self.compile_expression(arg.clone())?;
                }
                self.chunk.emit(Opcode::Construct(args.len()));
            }
        }
        Ok(())
    }
}

fn collect_var_declarations(statements: &[Statement], names: &mut Vec<String>) {
    for statement in statements {
        match statement {
            Statement::VariableDeclaration { kind, declarations } if *kind == VarKind::Var => {
                names.extend(declarations.iter().map(|(name, _)| name.clone()));
            }
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
                    names.push(binding.clone());
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
            | Statement::Return(_)
            | Statement::FunctionDeclaration { .. }
            | Statement::Throw(_)
            | Statement::Break(_)
            | Statement::Continue(_) => {}
        }
    }
}

impl Default for Compiler {
    /// デフォルト実装
    fn default() -> Self {
        Self::new()
    }
}
