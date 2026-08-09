//! Bytecode Virtual Machine (VM)
//!
//! シンプルなスタックベースのバイトコードインタープリタです。
//! - スタック (Vec<JSValue>) を使用
//! - 関数呼び出し時はスタック/環境を切り替える

use crate::compiler::{BytecodeChunk, Opcode};
use crate::error::{JSError, JSResult};
use crate::runtime::{CallFrame, Environment};
use crate::value::JSValue;
use crate::value::jsobject::JSObject;
use std::any::Any;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

struct Job {
    callback: JSValue,
    this: JSValue,
    arguments: Vec<JSValue>,
}

/// 仮想マシン
pub struct VM {
    /// オペランドスタック
    pub(crate) stack: Vec<JSValue>,
    /// コールフレーム
    pub frames: Vec<CallFrame>,
    /// グローバルオブジェクト（非モジュールスクリプトの `this` などに利用）
    pub global_object: Rc<RefCell<crate::value::jsobject::JSObject>>,
    /// Function.prototype への参照（関数値のプロパティ検索に利用）
    pub function_prototype: Rc<RefCell<JSObject>>,
    /// String.prototype への参照（文字列プリミティブのメソッド検索に利用）
    pub string_prototype: Rc<RefCell<JSObject>>,
    /// Number.prototype への参照（数値プリミティブのメソッド検索に利用）
    pub number_prototype: Rc<RefCell<JSObject>>,
    /// Own properties for user-defined callable values, keyed by function identity.
    callable_objects: HashMap<u64, Rc<RefCell<JSObject>>>,
    /// Host data slot.
    ///
    /// A shared slot where the host (the embedding app) can store arbitrary state.
    /// Native functions are plain function pointers (`fn(&mut VM, Vec<JSValue>)`)
    /// and cannot capture closures, so host state (e.g. the DOM tree) is accessed
    /// from native functions through this slot via `downcast_ref`.
    pub host: Option<Rc<RefCell<dyn Any>>>,
    jobs: VecDeque<Job>,
}

enum ControlFlow {
    Continue,
    Jump(usize),
    Return(JSValue),
    PushTry {
        catch_target: Option<usize>,
        finally_target: Option<usize>,
    },
    PopTry,
    BeginFinally,
    EndFinally,
}

struct TryHandler {
    catch_target: Option<usize>,
    finally_target: Option<usize>,
}

enum PendingFinally {
    Normal,
    Throw(JSError),
    Return(JSValue),
}

impl VM {
    /// 新しい VM インスタンスを作成します。
    pub fn new() -> Self {
        // グローバルオブジェクトを作成し、グローバル環境を初期化
        let global_obj = crate::value::jsobject::JSObject::new();
        let global_rc = Rc::new(RefCell::new(global_obj));
        // builtins を初期化してグローバルに組み込みを登録
        let function_prototype = crate::builtins::Builtins::new().init(&global_rc);
        let string_constructor = global_rc.borrow().get("String");
        let string_prototype = match string_constructor {
            JSValue::Object(constructor) => {
                let prototype = constructor.borrow().get("prototype");
                match prototype {
                    JSValue::Object(prototype) => prototype,
                    _ => Rc::new(RefCell::new(JSObject::new())),
                }
            }
            _ => Rc::new(RefCell::new(JSObject::new())),
        };
        let number_constructor = global_rc.borrow().get("Number");
        let number_prototype = match number_constructor {
            JSValue::Object(constructor) => {
                let prototype = constructor.borrow().get("prototype");
                match prototype {
                    JSValue::Object(prototype) => prototype,
                    _ => Rc::new(RefCell::new(JSObject::new())),
                }
            }
            _ => Rc::new(RefCell::new(JSObject::new())),
        };

        let global_frame = CallFrame::new(
            Environment::with_object_env(global_rc.clone()),
            JSValue::Object(global_rc.clone()),
        );

        Self {
            stack: Vec::new(),
            frames: vec![global_frame],
            global_object: global_rc,
            function_prototype,
            string_prototype,
            number_prototype,
            callable_objects: HashMap::new(),
            host: None,
            jobs: VecDeque::new(),
        }
    }

    /// Enqueues a callable job for the next host microtask checkpoint.
    pub fn enqueue_job(&mut self, callback: JSValue, this: JSValue, arguments: Vec<JSValue>) {
        self.jobs.push_back(Job {
            callback,
            this,
            arguments,
        });
    }

    /// Creates an Array object connected to the realm's `Array.prototype`.
    pub fn array_from_values(&self, values: Vec<JSValue>) -> JSValue {
        let array = crate::value::JSArray::from_vec(values).to_object();
        let JSValue::Object(array_object) = &array else {
            unreachable!("JSArray must produce an object");
        };
        if let JSValue::Object(constructor) = self.global_object.borrow().get("Array")
            && let JSValue::Object(prototype) = constructor.borrow().get("prototype")
        {
            array_object.borrow_mut().set_prototype(Some(prototype));
        }
        array
    }

    fn register_user_function(
        &mut self,
        function: &JSValue,
        constructible: bool,
        length: usize,
        name: Option<&str>,
    ) {
        let Some(identity) = function.user_function_identity() else {
            return;
        };
        let mut properties = JSObject::with_prototype(Some(Rc::clone(&self.function_prototype)));
        properties.set("length".to_string(), JSValue::Number(length as f64));
        properties.set(
            "name".to_string(),
            JSValue::String(name.unwrap_or("").to_string()),
        );
        if constructible {
            let mut prototype = JSObject::new();
            prototype.set("constructor".to_string(), function.clone());
            properties.set(
                "prototype".to_string(),
                JSValue::Object(Rc::new(RefCell::new(prototype))),
            );
        }
        self.callable_objects
            .insert(identity, Rc::new(RefCell::new(properties)));
    }

    fn user_function_object(&self, value: &JSValue) -> Option<Rc<RefCell<JSObject>>> {
        let identity = value.user_function_identity()?;
        self.callable_objects.get(&identity).cloned()
    }

    /// Runs queued jobs in FIFO order until the queue is empty.
    pub fn run_jobs(&mut self) -> JSResult<()> {
        while let Some(job) = self.jobs.pop_front() {
            self.call(job.callback, job.this, job.arguments)?;
        }
        Ok(())
    }

    pub fn execute(&mut self, chunk: &BytecodeChunk) -> JSResult<JSValue> {
        let mut pc = 0; // プログラムカウンタ
        let mut handlers = Vec::new();
        let mut pending_finally = Vec::new();

        while pc < chunk.code.len() {
            let opcode = &chunk.code[pc];
            pc += 1;

            let control = match self.execute_opcode(opcode, &chunk) {
                Ok(control) => control,
                Err(error) => {
                    pending_finally.pop();
                    self.redirect_exception(
                        error,
                        &mut handlers,
                        &mut pending_finally,
                        &mut pc,
                    )?;
                    continue;
                }
            };
            match control {
                ControlFlow::Continue => {}

                ControlFlow::Jump(target) => {
                    pc = target;
                }

                ControlFlow::Return(value) => {
                    pending_finally.pop();
                    if let Some(value) = self.redirect_return(
                        value,
                        &mut handlers,
                        &mut pending_finally,
                        &mut pc,
                    ) {
                        return Ok(value);
                    }
                }
                ControlFlow::PushTry {
                    catch_target,
                    finally_target,
                } => handlers.push(TryHandler {
                    catch_target,
                    finally_target,
                }),
                ControlFlow::PopTry => {
                    handlers.pop();
                }
                ControlFlow::BeginFinally => pending_finally.push(PendingFinally::Normal),
                ControlFlow::EndFinally => {
                    match pending_finally.pop() {
                        Some(PendingFinally::Throw(error)) => {
                            self.redirect_exception(
                                error,
                                &mut handlers,
                                &mut pending_finally,
                                &mut pc,
                            )?;
                        }
                        Some(PendingFinally::Return(value)) => {
                            if let Some(value) = self.redirect_return(
                                value,
                                &mut handlers,
                                &mut pending_finally,
                                &mut pc,
                            ) {
                                return Ok(value);
                            }
                        }
                        Some(PendingFinally::Normal) | None => {}
                    }
                }
            }
        }

        Ok(self.stack.pop().unwrap_or(JSValue::Undefined))
    }

    fn redirect_exception(
        &mut self,
        error: JSError,
        handlers: &mut Vec<TryHandler>,
        pending_finally: &mut Vec<PendingFinally>,
        pc: &mut usize,
    ) -> JSResult<()> {
        let Some(handler) = handlers.pop() else {
            return Err(error);
        };
        if let Some(catch_target) = handler.catch_target {
            let value = match &error {
                JSError::Thrown(value) => value.clone(),
                _ => JSValue::String(error.to_string()),
            };
            self.stack.push(value);
            *pc = catch_target;
            return Ok(());
        }
        if let Some(finally_target) = handler.finally_target {
            pending_finally.push(PendingFinally::Throw(error));
            *pc = finally_target;
            return Ok(());
        }
        Err(error)
    }

    fn redirect_return(
        &mut self,
        value: JSValue,
        handlers: &mut Vec<TryHandler>,
        pending_finally: &mut Vec<PendingFinally>,
        pc: &mut usize,
    ) -> Option<JSValue> {
        while let Some(handler) = handlers.pop() {
            if let Some(finally_target) = handler.finally_target {
                pending_finally.push(PendingFinally::Return(value));
                *pc = finally_target;
                return None;
            }
        }
        Some(value)
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
            Opcode::Dup => {
                let value = self
                    .stack
                    .last()
                    .cloned()
                    .ok_or_else(|| JSError::InternalError("Stack underflow".to_string()))?;
                self.stack.push(value);
            }
            Opcode::Dup2 => {
                let length = self.stack.len();
                if length < 2 {
                    return Err(JSError::InternalError("Stack underflow".to_string()));
                }
                let first = self.stack[length - 2].clone();
                let second = self.stack[length - 1].clone();
                self.stack.push(first);
                self.stack.push(second);
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
            Opcode::In => {
                let object = self.pop()?;
                let key = self.pop()?.to_string();
                let object = match &object {
                    JSValue::Object(object) => Some(Rc::clone(object)),
                    JSValue::Function(..) | JSValue::ArrowFunction(..) => {
                        self.user_function_object(&object)
                    }
                    _ => None,
                };
                let Some(object) = object else {
                    return Err(JSError::TypeError(
                        "right-hand side of 'in' is not an object".to_string(),
                    ));
                };
                let contains = object.borrow().has_property(&key);
                self.stack.push(JSValue::Boolean(contains));
            }
            Opcode::Instanceof => {
                let constructor = self.pop()?;
                let value = self.pop()?;
                let constructor_object = match &constructor {
                    JSValue::Object(object) => Some(Rc::clone(object)),
                    JSValue::Function(..) => self.user_function_object(&constructor),
                    _ => None,
                };
                let Some(constructor_object) = constructor_object else {
                    return Err(JSError::TypeError(
                        "right-hand side of 'instanceof' is not an object".to_string(),
                    ));
                };

                let host_has_instance = constructor_object
                    .borrow()
                    .get(crate::value::jsobject::HOST_HAS_INSTANCE);
                if matches!(
                    &host_has_instance,
                    JSValue::Function(..)
                        | JSValue::ArrowFunction(..)
                        | JSValue::NativeFunction(..)
                        | JSValue::BoundFunction(..)
                ) {
                    let result = self.call(
                        host_has_instance,
                        constructor.clone(),
                        vec![value],
                    )?;
                    self.stack.push(JSValue::Boolean(result.to_boolean()));
                    return Ok(ControlFlow::Continue);
                }

                let JSValue::Object(target_prototype) =
                    constructor_object.borrow().get("prototype")
                else {
                    return Err(JSError::TypeError(
                        "constructor has a non-object prototype".to_string(),
                    ));
                };
                let JSValue::Object(object) = value else {
                    self.stack.push(JSValue::Boolean(false));
                    return Ok(ControlFlow::Continue);
                };
                let mut prototype = object.borrow().get_prototype();
                let mut matches = false;
                while let Some(current) = prototype {
                    if Rc::ptr_eq(&current, &target_prototype) {
                        matches = true;
                        break;
                    }
                    prototype = current.borrow().get_prototype();
                }
                self.stack.push(JSValue::Boolean(matches));
            }

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
                self.stack.push(self.array_from_values(Vec::new()));
            }
            Opcode::NewObject => {
                use crate::value::JSObject;
                use std::cell::RefCell;
                use std::rc::Rc;
                let obj = JSObject::new();
                self.stack.push(JSValue::Object(Rc::new(RefCell::new(obj))));
            }
            Opcode::NewRegExp(pattern, flags) => {
                self.stack
                    .push(crate::builtins::regexp::create(pattern, flags));
            }
            Opcode::GetProperty => {
                let key = self.pop()?;
                let obj = self.pop()?;

                match &obj {
                    JSValue::Object(obj_ref) => {
                        let key_str = key.to_string();

                        let maybe_prop = { obj_ref.borrow().get_property_descriptor(&key_str) };

                        if let Some(prop) = maybe_prop {
                            if let Some(getter) = prop.getter.clone() {
                                let result = self.call(getter, obj.clone(), vec![])?;
                                self.stack.push(result);

                                return Ok(ControlFlow::Continue);
                            }

                            self.stack.push(prop.value.clone());

                            return Ok(ControlFlow::Continue);
                        }

                        let host_getter = {
                            obj_ref
                                .borrow()
                                .get(crate::value::jsobject::HOST_GET_PROPERTY)
                        };

                        if matches!(
                            &host_getter,
                            JSValue::Function(..)
                                | JSValue::ArrowFunction(..)
                                | JSValue::NativeFunction(..)
                                | JSValue::BoundFunction(..)
                        ) {
                            let result = self.call(host_getter, obj.clone(), vec![key.clone()])?;
                            self.stack.push(result);

                            return Ok(ControlFlow::Continue);
                        }

                        let value = obj_ref.borrow().get(&key_str);

                        self.stack.push(value);
                    }
                    JSValue::Function(..)
                    | JSValue::ArrowFunction(..) => {
                        let key_str = key.to_string();
                        let object = self.user_function_object(&obj);
                        if let Some(object) = object {
                            let descriptor = object.borrow().get_property_descriptor(&key_str);
                            if let Some(getter) = descriptor.and_then(|property| property.getter) {
                                let value = self.call(getter, obj.clone(), Vec::new())?;
                                self.stack.push(value);
                            } else {
                                let value = object.borrow().get(&key_str);
                                self.stack.push(value);
                            }
                        } else {
                            let value = self.function_prototype.borrow().get(&key_str);
                            self.stack.push(value);
                        }
                    }
                    JSValue::NativeFunction(..) | JSValue::BoundFunction(..) => {
                        let key_str = key.to_string();
                        let value = self.function_prototype.borrow().get(&key_str);
                        self.stack.push(value);
                    }
                    JSValue::String(string) => {
                        let key = key.to_string();
                        if key == "length" {
                            self.stack.push(JSValue::Number(
                                string.encode_utf16().count() as f64,
                            ));
                        } else if let Ok(index) = key.parse::<usize>() {
                            let value = string
                                .chars()
                                .nth(index)
                                .map(|character| JSValue::String(character.to_string()))
                                .unwrap_or(JSValue::Undefined);
                            self.stack.push(value);
                        } else {
                            let value = self.string_prototype.borrow().get(&key);
                            self.stack.push(value);
                        }
                    }
                    JSValue::Number(_) => {
                        let key = key.to_string();
                        let value = self.number_prototype.borrow().get(&key);
                        self.stack.push(value);
                    }
                    _ => {
                        self.stack.push(JSValue::Undefined);
                    }
                }
            }
            Opcode::SetProperty => {
                let value = self.pop()?;
                let key = self.pop()?;
                let obj = self.pop()?;
                self.set_object_property(&obj, key, value.clone())?;
                self.stack.push(value);
            }
            Opcode::SetPropertyKeepOld => {
                let value = self.pop()?;
                let old_value = self.pop()?;
                let key = self.pop()?;
                let obj = self.pop()?;
                self.set_object_property(&obj, key, value)?;
                self.stack.push(old_value);
            }
            Opcode::DeleteProperty => {
                let key = self.pop()?.to_string();
                let object = self.pop()?;
                let object = match &object {
                    JSValue::Object(object) => Some(Rc::clone(object)),
                    JSValue::Function(..) | JSValue::ArrowFunction(..) => {
                        self.user_function_object(&object)
                    }
                    _ => None,
                };
                let Some(object) = object else {
                    return Err(JSError::TypeError(
                        "Cannot delete property on non-object".to_string(),
                    ));
                };
                let deleted = object.borrow_mut().delete(&key);
                self.stack.push(JSValue::Boolean(deleted));
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
                    // Update length if index >= current length
                    let current_len = obj_ref.borrow().get("length").to_number() as usize;
                    if idx_num >= current_len {
                        obj_ref
                            .borrow_mut()
                            .set("length".to_string(), JSValue::Number((idx_num + 1) as f64));
                    }
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
                    JSValue::Function(func_chunk, params, _maybe_env, name_opt, _) => {
                        let captured = Some(self.current_env());
                        let length = params.len();
                        let func = JSValue::Function(
                            func_chunk,
                            params,
                            captured,
                            name_opt.clone(),
                            crate::value::jsvalue::next_function_identity(),
                        );
                        self.register_user_function(&func, true, length, name_opt.as_deref());
                        self.stack.push(func);
                    }
                    JSValue::ArrowFunction(func_chunk, params, _maybe_env, _maybe_this, _) => {
                        let length = params.len();
                        let func = JSValue::ArrowFunction(
                            func_chunk,
                            params,
                            Some(self.current_env()),
                            Some(Box::new(self.current_frame().this.clone())),
                            crate::value::jsvalue::next_function_identity(),
                        );
                        self.register_user_function(&func, false, length, None);
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

                let key = property.to_string();

                let method = match &object {
                    JSValue::Object(obj_ref) => obj_ref.borrow().get(&key),
                    JSValue::Function(..)
                    | JSValue::ArrowFunction(..) => self
                        .user_function_object(&object)
                        .map(|properties| properties.borrow().get(&key))
                        .unwrap_or_else(|| self.function_prototype.borrow().get(&key)),
                    JSValue::NativeFunction(..) | JSValue::BoundFunction(..) => {
                        self.function_prototype.borrow().get(&key)
                    }
                    JSValue::String(_) => self.string_prototype.borrow().get(&key),
                    JSValue::Number(_) => self.number_prototype.borrow().get(&key),
                    _ => {
                        return Err(JSError::TypeError(
                            "CallMethod: receiver is not an object".into(),
                        ));
                    }
                };

                let result = self.call(method, object, args)?;

                self.stack.push(result);
            }
            Opcode::Construct(arg_count) => {
                let mut args = Vec::new();
                for _ in 0..*arg_count {
                    args.push(self.pop()?);
                }
                args.reverse();

                let constructor = self.pop()?;
                let callable = match &constructor {
                    JSValue::ArrowFunction(..) => {
                        return Err(JSError::TypeError(
                            "arrow function is not a constructor".to_string(),
                        ));
                    }
                    JSValue::Object(object) => {
                        let callable = object.borrow().get("__construct__");
                        if matches!(callable, JSValue::Undefined) {
                            return Err(JSError::TypeError(
                                "object is not a constructor".to_string(),
                            ));
                        }
                        callable
                    }
                    _ => constructor.clone(),
                };
                let prototype = match &constructor {
                    JSValue::Function(..) => self
                        .user_function_object(&constructor)
                        .and_then(|properties| match properties.borrow().get("prototype") {
                            JSValue::Object(prototype) => Some(prototype),
                            _ => None,
                        }),
                    JSValue::Object(object) => match object.borrow().get("prototype") {
                        JSValue::Object(prototype) => Some(prototype),
                        _ => None,
                    },
                    _ => None,
                };
                let this = JSValue::Object(Rc::new(RefCell::new(JSObject::with_prototype(
                    prototype,
                ))));
                let result = self.call(callable, this.clone(), args)?;
                self.stack.push(match result {
                    JSValue::Object(_)
                    | JSValue::Function(..)
                    | JSValue::ArrowFunction(..)
                    | JSValue::NativeFunction(..)
                    | JSValue::BoundFunction(..) => result,
                    _ => this,
                });
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
            Opcode::Enumerate => {
                let value = self.pop()?;
                let keys = match value {
                    JSValue::Object(object) => object
                        .borrow()
                        .enumerable_keys()
                        .into_iter()
                        .map(JSValue::String)
                        .collect(),
                    JSValue::Null | JSValue::Undefined => Vec::new(),
                    _ => Vec::new(),
                };
                self.stack.push(self.array_from_values(keys));
            }
            Opcode::JumpIfTrue(offset) => {
                let condition = self.pop()?;
                if condition.to_boolean() {
                    return Ok(ControlFlow::Jump(*offset));
                } else {
                    return Ok(ControlFlow::Continue);
                }
            }
            Opcode::PushTry {
                catch_target,
                finally_target,
            } => {
                return Ok(ControlFlow::PushTry {
                    catch_target: *catch_target,
                    finally_target: *finally_target,
                });
            }
            Opcode::PopTry => return Ok(ControlFlow::PopTry),
            Opcode::BeginFinally => return Ok(ControlFlow::BeginFinally),
            Opcode::EndFinally => return Ok(ControlFlow::EndFinally),
            Opcode::Throw => return Err(JSError::Thrown(self.pop()?)),
            Opcode::Return => {
                let value = self.pop()?;
                return Ok(ControlFlow::Return(value));
            }
        }

        Ok(ControlFlow::Continue)
    }

    fn with_call_frame(
        &mut self,
        env: Environment,
        this: JSValue,
        func: BytecodeChunk,
    ) -> JSResult<JSValue> {
        let old_stack = std::mem::take(&mut self.stack);
        self.frames.push(CallFrame::new(env, this));

        let result = self.execute(&func);

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

    fn set_object_property(
        &mut self,
        object: &JSValue,
        key: JSValue,
        value: JSValue,
    ) -> JSResult<()> {
        let object_ref = match object {
            JSValue::Object(object_ref) => Some(Rc::clone(object_ref)),
            JSValue::Function(..) | JSValue::ArrowFunction(..) => {
                self.user_function_object(object)
            }
            _ => None,
        };
        let Some(object_ref) = object_ref else {
            return Err(JSError::TypeError(
                "Cannot set property on non-object".to_string(),
            ));
        };
        let key_string = key.to_string();
        let property = object_ref.borrow().get_property_descriptor(&key_string);
        if let Some(setter) = property.and_then(|property| property.setter) {
            self.call(setter, object.clone(), vec![value])?;
            return Ok(());
        }

        let host_setter = object_ref
            .borrow()
            .get(crate::value::jsobject::HOST_SET_PROPERTY);
        if matches!(
            &host_setter,
            JSValue::Function(..)
                | JSValue::ArrowFunction(..)
                | JSValue::NativeFunction(..)
                | JSValue::BoundFunction(..)
        ) {
            self.call(host_setter, object.clone(), vec![key, value])?;
            return Ok(());
        }

        object_ref.borrow_mut().set(key_string, value);
        Ok(())
    }

    /// Calls a function (native / JS / bound function).
    ///
    /// Exposed so the host can invoke a JS function directly.
    pub fn call(
        &mut self,
        callee: JSValue,
        this: JSValue,
        args: Vec<JSValue>,
    ) -> JSResult<JSValue> {
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

            JSValue::Function(chunk, params, env, name, _) => {
                let env = self.create_function_env(callee_clone, env, params, args, name, true);

                self.with_call_frame(env, this, chunk)
            }

            JSValue::ArrowFunction(chunk, params, env, lexical_this, _) => {
                let env = self.create_function_env(callee_clone, env, params, args, None, false);
                let this = lexical_this.map(|this| *this).unwrap_or(this);
                self.with_call_frame(env, this, chunk)
            }

            JSValue::Object(object) => {
                let callable = object.borrow().get("__call__");
                if matches!(callable, JSValue::Undefined) {
                    return Err(JSError::TypeError("object is not callable".to_string()));
                }
                self.call(callable, this, args)
            }

            _ => Err(JSError::TypeError(
                format!("{} is not callable", callee.to_console_string()).into(),
            )),
        }
    }

    fn create_function_env(
        &self,
        func: JSValue,
        captured_env: Option<Rc<RefCell<Environment>>>,
        params: Vec<String>,
        args: Vec<JSValue>,
        name: Option<String>,
        bind_arguments: bool,
    ) -> Environment {
        let outer = captured_env.unwrap_or_else(|| self.current_env());

        let env = Environment::with_outer(outer);

        if bind_arguments {
            env.define("arguments".to_string(), self.array_from_values(args.clone()));
        }

        for (i, arg) in args.into_iter().enumerate() {
            let key = params
                .get(i)
                .cloned()
                .unwrap_or_else(|| format!("arg{}", i));

            env.define(key, arg);
        }

        if let Some(name) = name {
            env.define(name, func);
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
