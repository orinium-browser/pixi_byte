use crate::error::{JSError, JSResult};
use crate::parser::{BinaryOp, Expression, Literal, Program, Statement, UnaryOp};
use crate::value::JSValue;

/// バイトコード命令
#[derive(Debug, Clone, PartialEq)]
pub enum Opcode {
    // スタック操作
    LoadConst(usize), // 定数をスタックにロード
    LoadVar(String),  // 変数をスタックにロード
    StoreVar(String), // スタックトップを変数に格納
    Pop,              // スタックトップを削除

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
    NewArray(usize),   // 空の配列を作成（サイズ指定）
    NewObject,         // 空のオブジェクトを作成
    GetProperty,       // obj[key] - スタックから key, obj をポップ、結果をプッシュ
    SetProperty,       // obj[key] = value - スタックから value, key, obj をポップ
    ArrayPush,         // arr.push(value) - スタックから index, value をポップ、arr は残る
    ObjectSetProperty, // obj[key] = value - スタックから key, value をポップ、obj は残る

    // 関数操作
    CreateFunction(usize), // 定数プール内の関数オブジェクトを生成してプッシュ（func chunk idx）
    CallFunction(usize),   // 呼び出し（引数個数） - スタックから argN..arg1, func を使う

    // 制御フロー
    Jump(usize),        // 無条件ジャンプ
    JumpIfFalse(usize), // false の場合ジャンプ
    Return,             // 関数から戻る

    // その他
    Typeof,
    Void,
}

/// バイトコードチャンク
#[derive(Debug, Clone)]
pub struct BytecodeChunk {
    /// バイトコード命令列
    pub code: Vec<Opcode>,
    /// 定数プール
    pub constants: Vec<JSValue>,
}

impl BytecodeChunk {
    /// 新しいバイトコードチャンクを作成
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
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
}

impl Compiler {
    /// 新しいコンパイラインスタンスを作成
    pub fn new() -> Self {
        Self {
            chunk: BytecodeChunk::new(),
        }
    }

    /// ASTをバイトコードにコンパイル
    pub fn compile(&mut self, program: Program) -> JSResult<BytecodeChunk> {
        let len = program.body.len();
        for (i, statement) in program.body.into_iter().enumerate() {
            let is_last = i == len - 1;
            self.compile_statement(statement, is_last)?;
        }

        Ok(self.chunk.clone())
    }

    /// ステートメントをコンパイル
    fn compile_statement(&mut self, statement: Statement, is_last: bool) -> JSResult<()> {
        match statement {
            Statement::Expression(expr) => {
                self.compile_expression(expr)?;
                // 最後の式文の結果はスタックに残す（REPLスタイル）
                if !is_last {
                    self.chunk.emit(Opcode::Pop);
                }
            }
            Statement::VariableDeclaration {
                kind: _,
                name,
                init,
            } => {
                if let Some(expr) = init {
                    self.compile_expression(expr)?;
                } else {
                    // 初期化なしの場合はundefined
                    let idx = self.chunk.add_constant(JSValue::Undefined);
                    self.chunk.emit(Opcode::LoadConst(idx));
                }
                self.chunk.emit(Opcode::StoreVar(name));

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
                let function_chunk = Compiler::new().compile(program)?;

                // 現在のチャンクに関数を追加 (chunk, params)
                let idx = self
                    .chunk
                    .add_constant(JSValue::Function(function_chunk, params.clone()));
                self.chunk.emit(Opcode::CreateFunction(idx));

                // 関数名を変数としてストア
                self.chunk.emit(Opcode::StoreVar(name));
            }
        }
        Ok(())
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
                    BinaryOp::And => Opcode::And,
                    BinaryOp::Or => Opcode::Or,
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
                self.compile_expression(*arg)?;

                let opcode = match op {
                    UnaryOp::Plus => return Ok(()), // +x は x と同じ
                    UnaryOp::Minus => Opcode::Neg,
                    UnaryOp::Not => Opcode::Not,
                    UnaryOp::BitNot => Opcode::BitNot,
                    UnaryOp::Typeof => Opcode::Typeof,
                    UnaryOp::Void => Opcode::Void,
                    UnaryOp::Delete => {
                        // Delete は現時点では未実装
                        return Err(JSError::InternalError(
                            "delete operator not yet implemented".to_string(),
                        ));
                    }
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
                        return Err(JSError::SyntaxError(
                            "Invalid assignment target".to_string(),
                        ));
                    }
                }
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
                for (key, value) in properties {
                    // 値をコンパイル
                    self.compile_expression(value)?;
                    // キーをプッシュ
                    let key_idx = self.chunk.add_constant(JSValue::String(key));
                    self.chunk.emit(Opcode::LoadConst(key_idx));
                    // スタック: [object, value, key]
                    self.chunk.emit(Opcode::ObjectSetProperty);
                }
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
            Expression::Function { params, body } => {
                // 関数本体をコンパイル
                let program = Program { body };
                let function_chunk = Compiler::new().compile(program)?;

                // 現在のチャンクに関数オブジェクト（チャンク + params）を追加
                let func_value = JSValue::Function(function_chunk, params.clone());
                let idx = self.chunk.add_constant(func_value);
                self.chunk.emit(Opcode::CreateFunction(idx));
            }
            Expression::Call { callee, args } => {
                // 呼び出し対象をコンパイル
                self.compile_expression(*callee)?;

                // 引数をコンパイル
                for arg in &args {
                    self.compile_expression(arg.clone())?;
                }

                // 引数の数だけスタックからポップ
                let arg_count = args.len();
                self.chunk.emit(Opcode::CallFunction(arg_count));
            }
        }
        Ok(())
    }
}

impl Default for Compiler {
    /// デフォルト実装
    fn default() -> Self {
        Self::new()
    }
}
