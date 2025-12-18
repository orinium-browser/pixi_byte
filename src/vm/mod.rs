use crate::error::{JSError, JSResult};
use crate::compiler::{BytecodeChunk, Opcode};
use crate::value::JSValue;
use rustc_hash::FxHashMap;

/// 仮想マシン
pub struct VM {
    /// オペランドスタック
    stack: Vec<JSValue>,
    /// グローバル変数テーブル
    globals: FxHashMap<String, JSValue>,
}

impl VM {
    /// 新しいVMインスタンスを作成
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            globals: FxHashMap::default(),
        }
    }

    /// バイトコードを実行
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
                    let value = self.globals.get(name).cloned().unwrap_or(JSValue::Undefined);
                    self.stack.push(value);
                }
                Opcode::StoreVar(name) => {
                    if let Some(value) = self.stack.pop() {
                        self.globals.insert(name.clone(), value);
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
                        (JSValue::String(s), _) => {
                            JSValue::String(format!("{}{}", s, b.to_string()))
                        }
                        (_, JSValue::String(s)) => {
                            JSValue::String(format!("{}{}", a.to_string(), s))
                        }
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
                    self.stack.push(JSValue::Number((a_u32 >> (b_u32 & 0x1f)) as f64));
                }
                
                // その他
                Opcode::Typeof => {
                    let value = self.pop()?;
                    self.stack.push(JSValue::String(value.type_of().to_string()));
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
        self.stack.pop().ok_or_else(|| {
            JSError::InternalError("Stack underflow".to_string())
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::compiler::Compiler;

    fn eval(source: &str) -> JSResult<JSValue> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        let program = parser.parse()?;
        let mut compiler = Compiler::new();
        let chunk = compiler.compile(program)?;
        let mut vm = VM::new();
        vm.execute(chunk)
    }

    #[test]
    fn test_execute_literal() {
        let result = eval("42").unwrap();
        assert_eq!(result, JSValue::Number(42.0));
    }

    #[test]
    fn test_execute_addition() {
        let result = eval("1 + 2").unwrap();
        assert_eq!(result, JSValue::Number(3.0));
    }

    #[test]
    fn test_execute_string_concat() {
        let result = eval(r#""hello" + " " + "world""#).unwrap();
        assert_eq!(result, JSValue::String("hello world".to_string()));
    }

    #[test]
    fn test_execute_comparison() {
        let result = eval("5 > 3").unwrap();
        assert_eq!(result, JSValue::Boolean(true));
    }

    #[test]
    fn test_execute_variable() {
        let result = eval("let x = 10; x + 5").unwrap();
        assert_eq!(result, JSValue::Number(15.0));
    }
}

