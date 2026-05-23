//! Bytecode Virtual Machine (VM)
//!
//! シンプルなスタックベースのバイトコードインタープリタです。
//! - スタック (Vec<JSValue>) を使用
//! - 関数呼び出し時はスタック/環境を切り替える

use crate::compiler::{BytecodeChunk, Opcode};
use crate::error::{JSError, JSResult};
use crate::runtime::{CallFrame, Environment};
use crate::value::JSValue;
use std::cell::RefCell;
use std::rc::Rc;

/// 仮想マシン
pub struct VM {
    /// オペランドスタック
    pub(crate) stack: Vec<JSValue>,
    /// コールフレーム
    pub frames: Vec<CallFrame>,
    /// グローバルオブジェクト（非モジュールスクリプトの `this` などに利用）
    pub global_object: Rc<RefCell<crate::value::jsobject::JSObject>>,
}

enum ControlFlow {
    Continue,
    Jump(usize),
    Return(JSValue),
}

impl VM {
    /// 新しい VM インスタンスを作成します。
    pub fn new() -> Self {
        // グローバルオブジェクトを作成し、グローバル環境を初期化
        let global_obj = crate::value::jsobject::JSObject::new();
        let global_rc = Rc::new(RefCell::new(global_obj));
        // builtins を初期化してグローバルに組み込みを登録
        crate::builtins::Builtins::new().init(&global_rc);

        let global_env = Rc::new(RefCell::new(crate::runtime::Environment::new()));

        let global_frame = CallFrame {
            env: global_env,
            this: JSValue::Object(global_rc.clone()),
        };

        Self {
            stack: Vec::new(),
            frames: vec![global_frame],
            global_object: global_rc,
        }
    }

    pub fn execute(&mut self, chunk: BytecodeChunk) -> JSResult<JSValue> {
        let mut pc = 0; // プログラムカウンタ

        while pc < chunk.code.len() {
            let opcode = &chunk.code[pc];
            pc += 1;

            match self.execute_opcode(opcode, &chunk)? {
                ControlFlow::Continue => {}

                ControlFlow::Jump(target) => {
                    pc = target;
                }

                ControlFlow::Return(value) => {
                    return Ok(value);
                }
            }
        }

        Ok(self.stack.pop().unwrap_or(JSValue::Undefined))
    }

    /// バイトコードを実行（トップレベルはグローバル環境を使用）
    fn execute_opcode(&mut self, opcode: &Opcode, chunk: &BytecodeChunk) -> JSResult<ControlFlow> {
        match opcode {
            Opcode::LoadConst(idx) => {
                let value = chunk.constants[*idx].clone();
                self.stack.push(value);
            }
            Opcode::LoadVar(name) => {
                let value = self
                    .current_env()
                    .borrow()
                    .get(name)
                    .unwrap_or(JSValue::Undefined);
                self.stack.push(value);
            }
            Opcode::StoreVar(name) => {
                if let Some(value) = self.stack.pop() {
                    // 既存のスコープチェーンに存在すれば set、なければ現在の env に define
                    if !self.current_env().borrow().set(name, value.clone()) {
                        self.current_env().borrow().define(name.clone(), value);
                    }
                } else {
                    return Err(JSError::InternalError("Stack underflow".to_string()));
                }
            }
            Opcode::Pop => {
                self.stack.pop();
            }

            Opcode::LoadThis => {
                let value = self.current_frame().this.clone();
                self.stack.push(value);
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

                        // Try to get property descriptor on the object itself first
                        if let Some(prop) = obj_ref.borrow().get_property_descriptor(&key_str) {
                            // If accessor getter exists, call it
                            if let Some(getter_val) = prop.getter.clone() {
                                match getter_val {
                                    JSValue::NativeFunction(native_fn) => {
                                        // call native getter with receiver as first arg
                                        let result = native_fn(
                                            self,
                                            vec![JSValue::Object(obj_ref.clone())],
                                        )?;
                                        self.stack.push(result);
                                    }
                                    JSValue::Function(func_chunk, _params, captured_env_opt, _) => {
                                        // call JS getter as method with receiver as this and no args
                                        let outer = match captured_env_opt {
                                            Some(env_rc) => env_rc,
                                            None => self.current_env(),
                                        };
                                        let new_env =
                                            Rc::new(RefCell::new(Environment::with_outer(outer)));

                                        let res = self.with_call_frame(
                                            new_env,
                                            JSValue::Object(obj_ref.clone()),
                                            func_chunk,
                                        )?;

                                        self.stack.push(res);
                                    }
                                    _ => {
                                        // getter not callable
                                        self.stack.push(JSValue::Undefined);
                                    }
                                }
                                return Ok(ControlFlow::Continue); // processed getter
                            }

                            // No getter: return data value
                            self.stack.push(prop.value.clone());
                            return Ok(ControlFlow::Continue);
                        }

                        // Not an own property: use prototype chain lookup via get (existing behavior)
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

                        // Obtain property descriptor in a short scope so RefCell borrow ends
                        let maybe_prop = {
                            let borrowed = obj_ref.borrow();
                            borrowed.get_property_descriptor(&key_str)
                        };

                        // If there is an own property with a setter, call it
                        if let Some(prop) = maybe_prop
                            && let Some(setter_val) = prop.setter.clone()
                        {
                            match setter_val {
                                JSValue::NativeFunction(native_fn) => {
                                    // call native setter with receiver and value
                                    let _res = native_fn(
                                        self,
                                        vec![JSValue::Object(obj_ref.clone()), value.clone()],
                                    )?;
                                    // setters usually return undefined; we push the object as per prior behavior
                                    self.stack.push(JSValue::Object(obj_ref.clone()));
                                }
                                JSValue::Function(func_chunk, params, captured_env_opt, _) => {
                                    // call JS setter with receiver as this and value as first param
                                    let outer = match captured_env_opt {
                                        Some(env_rc) => env_rc,
                                        None => self.current_env(),
                                    };
                                    let new_env =
                                        Rc::new(RefCell::new(Environment::with_outer(outer)));

                                    // bind parameter (if exists)
                                    if !params.is_empty() {
                                        new_env.borrow().define(params[0].clone(), value.clone());
                                    }

                                    self.with_call_frame(
                                        new_env,
                                        JSValue::Object(obj_ref.clone()),
                                        func_chunk,
                                    )?;
                                }
                                _ => {
                                    return Err(JSError::TypeError(
                                        "SetProperty: setter is not callable".to_string(),
                                    ));
                                }
                            }
                            return Ok(ControlFlow::Continue);
                        }

                        // No setter: perform normal set
                        let key_str = key.to_string();
                        obj_ref.borrow_mut().set(key_str, value.clone());
                        self.stack.push(JSValue::Object(obj_ref.clone()));
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
                    JSValue::Function(func_chunk, params, _maybe_env, name_opt) => {
                        let captured = Some(self.current_env());
                        let func =
                            JSValue::Function(func_chunk, params, captured, name_opt.clone());
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
                let this = JSValue::Object(self.global_object.clone());

                let result = self.call(func, this, args)?;

                self.stack.push(result);
            }
            Opcode::CallMethod(arg_count) => {
                // スタック: ..., object, property, arg1, arg2, ..., argN
                // まず引数を取り出す
                let mut args = Vec::new();
                for _ in 0..*arg_count {
                    args.push(self.pop()?);
                }
                args.reverse();

                // 次に property と object を取り出す
                let property = self.pop()?;
                let object = self.pop()?;

                // property を文字列化してプロパティアクセス
                let key_str = property.to_string();

                match object.clone() {
                    JSValue::Object(obj_ref) => {
                        let method = obj_ref.borrow().get(&key_str);
                        match method {
                            JSValue::Function(func_chunk, params, captured_env_opt, name_opt) => {
                                // outer は関数生成時のキャプチャまたは現在の env
                                let outer = match captured_env_opt {
                                    Some(env_rc) => env_rc,
                                    None => self.current_env(),
                                };
                                let new_env = Rc::new(RefCell::new(Environment::with_outer(outer)));

                                // パラメータをバインド
                                for (i, arg) in args.into_iter().enumerate() {
                                    if i < params.len() {
                                        new_env.borrow().define(params[i].clone(), arg);
                                    } else {
                                        new_env.borrow().define(format!("arg{}", i), arg);
                                    }
                                }

                                // named function expression の場合は名前を環境に定義
                                if let Some(name) = name_opt.clone() {
                                    new_env.borrow().define(name, JSValue::Undefined);
                                }

                                // 実行
                                let res = self.with_call_frame(new_env, object, func_chunk)?;

                                self.stack.push(res);
                            }
                            JSValue::NativeFunction(native_fn) => {
                                // For methods, inject receiver as first arg
                                let mut call_args = Vec::new();
                                call_args.push(object.clone());
                                call_args.extend(args);
                                let res = native_fn(self, call_args)?;
                                self.stack.push(res);
                            }
                            _ => {
                                return Err(JSError::TypeError(
                                    "CallMethod: property is not a function".to_string(),
                                ));
                            }
                        }
                    }
                    _ => {
                        return Err(JSError::TypeError(
                            "CallMethod: receiver is not an object".to_string(),
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
                return Ok(ControlFlow::Jump(*offset));
            }
            Opcode::JumpIfFalse(offset) => {
                let condition = self.pop()?;
                if !condition.to_boolean() {
                    return Ok(ControlFlow::Jump(*offset));
                } else {
                    return Ok(ControlFlow::Continue);
                }
            }
            Opcode::Return => {
                let value = self.pop()?;
                return Ok(ControlFlow::Return(value));
            }
        }

        Ok(ControlFlow::Continue)
    }

    fn with_call_frame(
        &mut self,
        env: Rc<RefCell<Environment>>,
        this: JSValue,
        func: BytecodeChunk,
    ) -> JSResult<JSValue> {
        let old_stack = std::mem::take(&mut self.stack);
        self.frames.push(CallFrame { env, this });

        let result = self.execute(func);

        self.frames.pop();
        self.stack = old_stack;

        result
    }

    /// 現在の CallFrame を返す
    fn current_frame(&self) -> &CallFrame {
        self.frames.last().expect("no call frame")
    }

    /// 現在の Environment を返す
    pub(crate) fn current_env(&self) -> Rc<RefCell<Environment>> {
        self.current_frame().env.clone()
    }

    /// スタックから値をポップ
    fn pop(&mut self) -> JSResult<JSValue> {
        self.stack
            .pop()
            .ok_or_else(|| JSError::InternalError("Stack underflow".to_string()))
    }

    fn call(&mut self, callee: JSValue, this: JSValue, args: Vec<JSValue>) -> JSResult<JSValue> {
        let callee_clone = callee.clone();

        match callee {
            JSValue::BoundFunction(bound) => {
                let mut all = bound.bound_args.clone();

                all.extend(args);

                self.call(*bound.target, bound.bound_this.clone(), all)
            }

            JSValue::NativeFunction(f) => {
                let mut all = vec![this];

                all.extend(args);

                f(self, all)
            }

            JSValue::Function(chunk, params, env, name) => {
                let env = self.create_function_env(callee_clone, env, params, args, name);

                self.with_call_frame(env, this, chunk)
            }

            _ => Err(JSError::TypeError("not a function".into())),
        }
    }

    fn create_function_env(
        &self,
        func: JSValue,
        captured_env: Option<Rc<RefCell<Environment>>>,
        params: Vec<String>,
        args: Vec<JSValue>,
        name: Option<String>,
    ) -> Rc<RefCell<Environment>> {
        let outer = captured_env.unwrap_or_else(|| self.current_env());

        let env = Rc::new(RefCell::new(Environment::with_outer(outer)));

        for (i, arg) in args.into_iter().enumerate() {
            let key = params
                .get(i)
                .cloned()
                .unwrap_or_else(|| format!("arg{}", i));

            env.borrow().define(key, arg);
        }

        if let Some(name) = name {
            env.borrow().define(name, func);
        }

        env
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
