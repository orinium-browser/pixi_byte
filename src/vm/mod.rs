use crate::compiler::{BytecodeChunk, Opcode};
use crate::error::{JSError, JSResult};
use crate::runtime::Environment;
use crate::value::JSValue;
use std::cell::RefCell;
use std::rc::Rc;

/// 仮想マシン
pub struct VM {
    /// オペランドスタック
    stack: Vec<JSValue>,
    /// グローバル環境（レキシカルスコープチェーンのルート）
    pub env: Rc<RefCell<Environment>>,
}

impl VM {
    /// 新しいVMインスタンスを作成
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            env: Rc::new(RefCell::new(Environment::new())),
        }
    }

    /// バイトコードを実行（トップレベルはグローバル環境を使用）
    pub fn execute(&mut self, chunk: BytecodeChunk) -> JSResult<JSValue> {
        let mut pc = 0; // プログラムカウンタ

        while pc < chunk.code.len() {
            let opcode = &chunk.code[pc];
            pc += 1;

            match opcode {
                Opcode::LoadConst(idx) => {
                    let value = chunk.constants[*idx].clone();
                    self.stack.push(value);
                }
                Opcode::LoadVar(name) => {
                    let value = self
                        .env
                        .borrow()
                        .get(name)
                        .unwrap_or(JSValue::Undefined);
                    self.stack.push(value);
                }
                Opcode::StoreVar(name) => {
                    if let Some(value) = self.stack.pop() {
                        // 既存のスコープチェーンに存在すれば set、なければ現在の env に define
                        if !self.env.borrow().set(name, value.clone()) {
                            self.env.borrow().define(name.clone(), value);
                        }
                    } else {
                        return Err(JSError::InternalError("Stack underflow".to_string()));
                    }
                }
                Opcode::Pop => {
                    self.stack.pop();
                }

                // 算術演算
                Opcode::Add => self.binary_op(|a, b| {
                    // JavaScriptの加算は文字列連結も含む
                    match (&a, &b) {
                        (JSValue::String(s1), JSValue::String(s2)) => {
                            JSValue::String(format!("{}{}", s1, s2))
                        }
                        (JSValue::String(s), _) => JSValue::String(format!("{}{}", s, b)),
                        (_, JSValue::String(s)) => JSValue::String(format!("{}{}", a, s)),
                        _ => JSValue::Number(a.to_number() + b.to_number()),
                    }
                })?,
                Opcode::Sub => self.binary_numeric_op(|a, b| a - b)?,
                Opcode::Mul => self.binary_numeric_op(|a, b| a * b)?,
                Opcode::Div => self.binary_numeric_op(|a, b| a / b)?,
                Opcode::Mod => self.binary_numeric_op(|a, b| a % b)?,
                Opcode::Power => self.binary_numeric_op(|a, b| a.powf(b))?,

                // 単項演算
                Opcode::Neg => {
                    let value = self.pop()?;
                    self.stack.push(JSValue::Number(-value.to_number()));
                }
                Opcode::Not => {
                    let value = self.pop()?;
                    self.stack.push(JSValue::Boolean(!value.to_boolean()));
                }
                Opcode::BitNot => {
                    let value = self.pop()?;
                    let n = value.to_number() as i32;
                    self.stack.push(JSValue::Number((!n) as f64));
                }

                // 比較演算
                Opcode::Eq => self.comparison_op(|a, b| a.abstract_equals(b))?,
                Opcode::NotEq => self.comparison_op(|a, b| !a.abstract_equals(b))?,
                Opcode::StrictEq => self.comparison_op(|a, b| a.strict_equals(b))?,
                Opcode::StrictNotEq => self.comparison_op(|a, b| !a.strict_equals(b))?,
                Opcode::Lt => self.numeric_comparison_op(|a, b| a < b)?,
                Opcode::Gt => self.numeric_comparison_op(|a, b| a > b)?,
                Opcode::LtEq => self.numeric_comparison_op(|a, b| a <= b)?,
                Opcode::GtEq => self.numeric_comparison_op(|a, b| a >= b)?,

                // 論理演算
                Opcode::And => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    // JavaScriptの && は短絡評価で、最初の falsy な値か最後の値を返す
                    if !a.to_boolean() {
                        self.stack.push(a);
                    } else {
                        self.stack.push(b);
                    }
                }
                Opcode::Or => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    // JavaScriptの || は短絡評価で、最初の truthy な値か最後の値を返す
                    if a.to_boolean() {
                        self.stack.push(a);
                    } else {
                        self.stack.push(b);
                    }
                }

                // ビット演算
                Opcode::BitAnd => self.bitwise_op(|a, b| a & b)?,
                Opcode::BitOr => self.bitwise_op(|a, b| a | b)?,
                Opcode::BitXor => self.bitwise_op(|a, b| a ^ b)?,
                Opcode::LeftShift => self.bitwise_op(|a, b| a << (b & 0x1f))?,
                Opcode::RightShift => self.bitwise_op(|a, b| a >> (b & 0x1f))?,
                Opcode::UnsignedRightShift => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    let a_u32 = a.to_number() as u32;
                    let b_u32 = b.to_number() as u32;
                    self.stack
                        .push(JSValue::Number((a_u32 >> (b_u32 & 0x1f)) as f64));
                }

                // 配列・オブジェクト操作
                Opcode::NewArray(_size) => {
                    use crate::value::JSArray;
                    let arr = JSArray::new();
                    self.stack.push(arr.to_object());
                }
                Opcode::NewObject => {
                    use crate::value::JSObject;
                    use std::cell::RefCell;
                    use std::rc::Rc;
                    let obj = JSObject::new();
                    self.stack.push(JSValue::Object(Rc::new(RefCell::new(obj))));
                }
                Opcode::GetProperty => {
                    let key = self.pop()?;
                    let obj = self.pop()?;

                    match obj {
                        JSValue::Object(ref obj_ref) => {
                            let key_str = key.to_string();
                            let value = obj_ref.borrow().get(&key_str);
                            self.stack.push(value);
                        }
                        _ => {
                            // プリミティブ値のプロパティアクセスは後で実装
                            self.stack.push(JSValue::Undefined);
                        }
                    }
                }
                Opcode::SetProperty => {
                    let value = self.pop()?;
                    let key = self.pop()?;
                    let obj = self.pop()?;

                    match obj {
                        JSValue::Object(ref obj_ref) => {
                            let key_str = key.to_string();
                            obj_ref.borrow_mut().set(key_str, value.clone());
                            self.stack.push(obj.clone()); // オブジェクトを返す
                        }
                        _ => {
                            return Err(JSError::TypeError(
                                "Cannot set property on non-object".to_string(),
                            ));
                        }
                    }
                }
                Opcode::ArrayPush => {
                    // スタック: [array, value, index]
                    let index = self.pop()?;
                    let value = self.pop()?;

                    // 配列はスタックの一番下にあるが、ポップしない
                    if let Some(JSValue::Object(obj_ref)) = self.stack.last() {
                        let idx_num = index.to_number() as usize;
                        let key_str = idx_num.to_string();
                        obj_ref.borrow_mut().set(key_str, value);
                    } else {
                        return Err(JSError::TypeError("ArrayPush: not an object".to_string()));
                    }
                }
                Opcode::ObjectSetProperty => {
                    // スタック: [object, value, key]
                    let key = self.pop()?;
                    let value = self.pop()?;

                    // オブジェクトはスタックの一番下にあるが、ポップしない
                    if let Some(JSValue::Object(obj_ref)) = self.stack.last() {
                        let key_str = key.to_string();
                        obj_ref.borrow_mut().set(key_str, value);
                    } else {
                        return Err(JSError::TypeError(
                            "ObjectSetProperty: not an object".to_string(),
                        ));
                    }
                }
                Opcode::CreateFunction(idx) => {
                    // 定数プールの関数オブジェクト（BytecodeChunk）をそのままプッシュ
                    let func_const = chunk.constants[*idx].clone();
                    match func_const {
                        JSValue::Function(func_chunk, params, _maybe_env) => {
                            let captured = Some(self.env.clone());
                            let func = JSValue::Function(func_chunk, params, captured);
                            self.stack.push(func);
                        }
                        _other => {
                            // 不正な定数タイプ
                            return Err(JSError::TypeError(
                                "CreateFunction: constant is not a function".to_string(),
                            ));
                        }
                    }
                }
                Opcode::CallFunction(arg_count) => {
                    // スタック: [..., arg1, arg2, ..., func]
                    let mut args = Vec::new();
                    for _ in 0..*arg_count {
                        args.push(self.pop()?);
                    }
                    // argsは逆順なので反転
                    args.reverse();

                    let func = self.pop()?;

                    match func {
                        JSValue::Function(func_chunk, params, captured_env_opt) => {
                            // 新しい環境を作成し、キャプチャされた環境または現在の環境を外側に設定
                            let outer = match captured_env_opt {
                                Some(env_rc) => env_rc,
                                None => self.env.clone(),
                            };
                            let new_env = Rc::new(RefCell::new(Environment::with_outer(outer)));

                            // パラメータ名があれば、それに対応して引数をセット
                            for (i, arg) in args.into_iter().enumerate() {
                                if i < params.len() {
                                    new_env.borrow().define(params[i].clone(), arg);
                                } else {
                                    // 余分な引数は argN としても格納
                                    new_env.borrow().define(format!("arg{}", i), arg);
                                }
                            }

                            // スタックと環境を切り替えて同一VMで関数を実行
                            let old_env = self.env.clone();
                            let old_stack = std::mem::replace(&mut self.stack, Vec::new());
                            self.env = new_env;

                            let res = self.execute(func_chunk)?;

                            // 関数実行後、内部スタックを破棄して呼び出し元のスタックを復元
                            let _inner_stack = std::mem::replace(&mut self.stack, old_stack);
                            self.env = old_env;

                            self.stack.push(res);
                        }
                        _ => {
                            return Err(JSError::TypeError(
                                "CallFunction: not a function".to_string(),
                            ));
                        }
                    }
                }

                // その他
                Opcode::Typeof => {
                    let value = self.pop()?;
                    self.stack
                        .push(JSValue::String(value.type_of().to_string()));
                }
                Opcode::Void => {
                    self.pop()?;
                    self.stack.push(JSValue::Undefined);
                }

                // 制御フロー
                Opcode::Jump(offset) => {
                    pc = *offset;
                }
                Opcode::JumpIfFalse(offset) => {
                    let condition = self.pop()?;
                    if !condition.to_boolean() {
                        pc = *offset;
                    }
                }
                Opcode::Return => {
                    let value = self.pop()?;
                    return Ok(value);
                }
            }
        }

        // プログラム終了時、スタックに値があればそれを返す
        if let Some(value) = self.stack.pop() {
            Ok(value)
        } else {
            Ok(JSValue::Undefined)
        }
    }

    /// スタックから値をポップ
    fn pop(&mut self) -> JSResult<JSValue> {
        self.stack
            .pop()
            .ok_or_else(|| JSError::InternalError("Stack underflow".to_string()))
    }

    /// 二項演算ヘルパー
    fn binary_op<F>(&mut self, op: F) -> JSResult<()>
    where
        F: FnOnce(JSValue, JSValue) -> JSValue,
    {
        let b = self.pop()?;
        let a = self.pop()?;
        let result = op(a, b);
        self.stack.push(result);
        Ok(())
    }

    /// 数値二項演算ヘルパー
    fn binary_numeric_op<F>(&mut self, op: F) -> JSResult<()>
    where
        F: FnOnce(f64, f64) -> f64,
    {
        let b = self.pop()?;
        let a = self.pop()?;
        let result = op(a.to_number(), b.to_number());
        self.stack.push(JSValue::Number(result));
        Ok(())
    }

    /// 比較演算ヘルパー
    fn comparison_op<F>(&mut self, op: F) -> JSResult<()>
    where
        F: FnOnce(&JSValue, &JSValue) -> bool,
    {
        let b = self.pop()?;
        let a = self.pop()?;
        let result = op(&a, &b);
        self.stack.push(JSValue::Boolean(result));
        Ok(())
    }

    /// 数値比較演算ヘルパー
    fn numeric_comparison_op<F>(&mut self, op: F) -> JSResult<()>
    where
        F: FnOnce(f64, f64) -> bool,
    {
        let b = self.pop()?;
        let a = self.pop()?;
        let result = op(a.to_number(), b.to_number());
        self.stack.push(JSValue::Boolean(result));
        Ok(())
    }

    /// ビット演算ヘルパー
    fn bitwise_op<F>(&mut self, op: F) -> JSResult<()>
    where
        F: FnOnce(i32, i32) -> i32,
    {
        let b = self.pop()?;
        let a = self.pop()?;
        let result = op(a.to_number() as i32, b.to_number() as i32);
        self.stack.push(JSValue::Number(result as f64));
        Ok(())
    }
}

impl Default for VM {
    /// デフォルト実装
    fn default() -> Self {
        Self::new()
    }
}
