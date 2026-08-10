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
    /// Object.prototype used by ordinary and host-created objects.
    pub object_prototype: Rc<RefCell<JSObject>>,
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
        let object_constructor = global_rc.borrow().get("Object");
        let object_prototype = match object_constructor {
            JSValue::Object(constructor) => match constructor.borrow().get("prototype") {
                JSValue::Object(prototype) => prototype,
                _ => Rc::new(RefCell::new(JSObject::new())),
            },
            _ => Rc::new(RefCell::new(JSObject::new())),
        };
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
            None,
        );

        Self {
            stack: Vec::new(),
            frames: vec![global_frame],
            global_object: global_rc,
            function_prototype,
            object_prototype,
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
            let mut prototype = JSObject::with_prototype(Some(Rc::clone(&self.object_prototype)));
            prototype.set("constructor".to_string(), function.clone());
            properties.set(
                "prototype".to_string(),
                JSValue::Object(Rc::new(RefCell::new(prototype))),
            );
        }
        self.callable_objects
            .insert(identity, Rc::new(RefCell::new(properties)));
    }

    pub(crate) fn user_function_object(&self, value: &JSValue) -> Option<Rc<RefCell<JSObject>>> {
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
                    self.redirect_exception(error, &mut handlers, &mut pending_finally, &mut pc)?;
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
                    if let Some(value) =
                        self.redirect_return(value, &mut handlers, &mut pending_finally, &mut pc)
                    {
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
                ControlFlow::EndFinally => match pending_finally.pop() {
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
                },
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
            Opcode::DefineVar(name) => {
                let value = self.pop()?;
                self.current_env().borrow().define(name.clone(), value);
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
                let n = to_int32(value.to_number());
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
                    let result = self.call(host_has_instance, constructor.clone(), vec![value])?;
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
                let a_u32 = to_uint32(a.to_number());
                let b_u32 = to_uint32(b.to_number());
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
                let obj = JSObject::with_prototype(Some(Rc::clone(&self.object_prototype)));
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

                        if let Some(prop) = inherited_property_descriptor(obj_ref, &key_str) {
                            if let Some(getter) = prop.getter {
                                let result = self.call(getter, obj.clone(), vec![])?;
                                self.stack.push(result);
                            } else {
                                self.stack.push(prop.value);
                            }
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

                        let value = {
                            let object = obj_ref.borrow();
                            if object.has_property(&key_str) {
                                object.get(&key_str)
                            } else {
                                self.object_prototype.borrow().get(&key_str)
                            }
                        };

                        self.stack.push(value);
                    }
                    JSValue::Function(..) | JSValue::ArrowFunction(..) => {
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
                            self.stack
                                .push(JSValue::Number(string.encode_utf16().count() as f64));
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
            Opcode::ArrayAppend => {
                let value = self.pop()?;
                let array = self.pop()?;
                let JSValue::Object(array_ref) = &array else {
                    return Err(JSError::TypeError("ArrayAppend: not an array".to_string()));
                };
                let index = array_ref.borrow().get("length").to_number() as usize;
                array_ref.borrow_mut().set(index.to_string(), value);
                array_ref
                    .borrow_mut()
                    .set("length".to_string(), JSValue::Number((index + 1) as f64));
                self.stack.push(array);
            }
            Opcode::ArrayExtend => {
                let iterable = self.pop()?;
                let array = self.pop()?;
                let values = match iterable {
                    JSValue::Object(object) => {
                        let length = object.borrow().get("length").to_number() as usize;
                        (0..length)
                            .map(|index| object.borrow().get(&index.to_string()))
                            .collect::<Vec<_>>()
                    }
                    JSValue::String(value) => value
                        .chars()
                        .map(|character| JSValue::String(character.to_string()))
                        .collect(),
                    _ => Vec::new(),
                };
                let JSValue::Object(array_ref) = &array else {
                    return Err(JSError::TypeError("ArrayExtend: not an array".to_string()));
                };
                for value in values {
                    let index = array_ref.borrow().get("length").to_number() as usize;
                    array_ref.borrow_mut().set(index.to_string(), value);
                    array_ref
                        .borrow_mut()
                        .set("length".to_string(), JSValue::Number((index + 1) as f64));
                }
                self.stack.push(array);
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
            Opcode::ObjectSpread => {
                let source = self.pop()?;
                let target = self.stack.last().cloned();
                if let (Some(JSValue::Object(target)), JSValue::Object(source)) = (target, source) {
                    let keys = source.borrow().enumerable_keys();
                    for key in keys {
                        let value = source.borrow().get(&key);
                        target.borrow_mut().set(key, value);
                    }
                }
            }
            Opcode::ObjectRest(excluded) => {
                let source = self.pop()?;
                let mut result =
                    crate::value::JSObject::with_prototype(Some(Rc::clone(&self.object_prototype)));
                if let JSValue::Object(source) = source {
                    for key in source.borrow().enumerable_keys() {
                        if !excluded.contains(&key) {
                            let value = source.borrow().get(&key);
                            result.set(key, value);
                        }
                    }
                }
                self.stack
                    .push(JSValue::Object(Rc::new(RefCell::new(result))));
            }
            Opcode::ObjectDefineGetter | Opcode::ObjectDefineSetter => {
                let key = self.pop()?.to_string();
                let accessor = self.pop()?;
                let target = self.stack.last().cloned().ok_or_else(|| {
                    JSError::TypeError("Object accessor target is missing".to_string())
                })?;
                let object = match target {
                    JSValue::Object(object) => Some(object),
                    JSValue::Function(..) | JSValue::ArrowFunction(..) => {
                        self.user_function_object(&target)
                    }
                    _ => None,
                };
                let Some(object) = object else {
                    return Err(JSError::TypeError(
                        "Object accessor target is not an object".to_string(),
                    ));
                };
                let is_getter = matches!(opcode, Opcode::ObjectDefineGetter);
                let existing = object.borrow().get_property_descriptor(&key);
                object.borrow_mut().define_property(
                    key,
                    crate::value::jsobject::Property {
                        value: JSValue::Undefined,
                        enumerable: true,
                        writable: false,
                        configurable: true,
                        getter: if is_getter {
                            Some(accessor.clone())
                        } else {
                            existing
                                .as_ref()
                                .and_then(|property| property.getter.clone())
                        },
                        setter: if is_getter {
                            existing.and_then(|property| property.setter)
                        } else {
                            Some(accessor)
                        },
                    },
                );
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
            Opcode::CallFunctionNamed(arg_count, name) => {
                let mut args = Vec::new();
                for _ in 0..*arg_count {
                    args.push(self.pop()?);
                }
                args.reverse();
                let func = self.pop()?;
                if matches!(&func, JSValue::Undefined | JSValue::Null) {
                    return Err(JSError::TypeError(format!(
                        "function '{name}' is not callable (found {})",
                        func.type_of()
                    )));
                }
                let this = JSValue::Object(self.global_object.clone());
                let result = self.call(func, this, args)?;
                self.stack.push(result);
            }
            Opcode::CallFunctionArray => {
                let arguments = self.pop()?;
                let JSValue::Object(arguments) = arguments else {
                    return Err(JSError::TypeError(
                        "CallFunctionArray: arguments are not an array".to_string(),
                    ));
                };
                let length = arguments.borrow().get("length").to_number() as usize;
                let args = (0..length)
                    .map(|index| arguments.borrow().get(&index.to_string()))
                    .collect();
                let func = self.pop()?;
                if matches!(&func, JSValue::Undefined | JSValue::Null) {
                    return Err(JSError::TypeError(format!(
                        "spread call target is not callable (found {})",
                        func.type_of()
                    )));
                }
                let this = JSValue::Object(self.global_object.clone());
                let result = self.call(func, this, args)?;
                self.stack.push(result);
            }
            Opcode::CallFunctionOptional(arg_count) => {
                let mut args = Vec::new();
                for _ in 0..*arg_count {
                    args.push(self.pop()?);
                }
                args.reverse();
                let func = self.pop()?;
                if matches!(&func, JSValue::Null | JSValue::Undefined) {
                    self.stack.push(JSValue::Undefined);
                } else {
                    let this = JSValue::Object(self.global_object.clone());
                    let result = self.call(func, this, args)?;
                    self.stack.push(result);
                }
            }
            Opcode::CallMethodOptional(arg_count) => {
                let mut args = Vec::new();
                for _ in 0..*arg_count {
                    args.push(self.pop()?);
                }
                args.reverse();
                let property = self.pop()?;
                let object = self.pop()?;
                if matches!(&object, JSValue::Null | JSValue::Undefined) {
                    self.stack.push(JSValue::Undefined);
                } else {
                    let key = property.to_string();
                    let method = self.resolve_method_property(&object, &key)?;
                    if matches!(&method, JSValue::Null | JSValue::Undefined) {
                        self.stack.push(JSValue::Undefined);
                    } else {
                        let result = self.call(method, object, args)?;
                        self.stack.push(result);
                    }
                }
            }
            Opcode::CallMethodArray => {
                let arguments = self.pop()?;
                let JSValue::Object(arguments) = arguments else {
                    return Err(JSError::TypeError(
                        "CallMethodArray: arguments are not an array".to_string(),
                    ));
                };
                let length = arguments.borrow().get("length").to_number() as usize;
                let args = (0..length)
                    .map(|index| arguments.borrow().get(&index.to_string()))
                    .collect();
                let property = self.pop()?;
                let object = self.pop()?;
                let key = property.to_string();
                let method = self.resolve_method_property(&object, &key)?;
                if matches!(&method, JSValue::Undefined | JSValue::Null) {
                    return Err(JSError::TypeError(format!(
                        "spread call property '{key}' is not callable (found {})",
                        method.type_of()
                    )));
                }
                let result = self.call(method, object, args)?;
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
                    JSValue::Object(obj_ref) => {
                        let own_property = obj_ref.borrow().get_property_descriptor(&key);
                        if let Some(property) = own_property {
                            if let Some(getter) = property.getter {
                                self.call(getter, object.clone(), Vec::new())?
                            } else {
                                property.value
                            }
                        } else if let Some(property) = inherited_property_descriptor(obj_ref, &key)
                        {
                            if let Some(getter) = property.getter {
                                self.call(getter, object.clone(), Vec::new())?
                            } else {
                                property.value
                            }
                        } else {
                            let host_getter = obj_ref
                                .borrow()
                                .get(crate::value::jsobject::HOST_GET_PROPERTY);
                            if matches!(
                                &host_getter,
                                JSValue::Function(..)
                                    | JSValue::ArrowFunction(..)
                                    | JSValue::NativeFunction(..)
                                    | JSValue::BoundFunction(..)
                            ) {
                                self.call(
                                    host_getter,
                                    object.clone(),
                                    vec![JSValue::String(key.clone())],
                                )?
                            } else {
                                self.object_prototype.borrow().get(&key)
                            }
                        }
                    }
                    JSValue::Function(..) | JSValue::ArrowFunction(..) => self
                        .user_function_object(&object)
                        .map(|properties| properties.borrow().get(&key))
                        .unwrap_or_else(|| self.function_prototype.borrow().get(&key)),
                    JSValue::NativeFunction(..) | JSValue::BoundFunction(..) => {
                        self.function_prototype.borrow().get(&key)
                    }
                    JSValue::String(_) => self.string_prototype.borrow().get(&key),
                    JSValue::Number(_) => self.number_prototype.borrow().get(&key),
                    _ => {
                        let stack = self
                            .frames
                            .iter()
                            .filter_map(|frame| frame.function_name.as_deref())
                            .collect::<Vec<_>>()
                            .join(" -> ");
                        let stack = if stack.is_empty() {
                            String::new()
                        } else {
                            format!(" (JS stack: {stack})")
                        };
                        return Err(JSError::TypeError(
                            format!(
                                "cannot call property '{key}' on {} receiver{stack}",
                                object.type_of(),
                            )
                            .into(),
                        ));
                    }
                };

                if !matches!(
                    &method,
                    JSValue::Function(..)
                        | JSValue::ArrowFunction(..)
                        | JSValue::NativeFunction(..)
                        | JSValue::BoundFunction(..)
                        | JSValue::Object(..)
                ) {
                    return Err(JSError::TypeError(format!(
                        "property '{key}' is not callable (found {})",
                        method.type_of()
                    )));
                }

                let result = self.call(method, object, args)?;

                self.stack.push(result);
            }
            Opcode::Construct(arg_count, constructor_name) => {
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
                            let keys = object.borrow().keys();
                            let name = constructor_name
                                .as_deref()
                                .map(|name| format!(" '{name}'"))
                                .unwrap_or_default();
                            return Err(JSError::TypeError(format!(
                                "object{name} is not a constructor (own properties: {keys:?})"
                            )));
                        }
                        callable
                    }
                    JSValue::Undefined
                    | JSValue::Null
                    | JSValue::Boolean(_)
                    | JSValue::Number(_)
                    | JSValue::String(_) => {
                        let name = constructor_name
                            .as_deref()
                            .map(|name| format!(" '{name}'"))
                            .unwrap_or_default();
                        return Err(JSError::TypeError(format!(
                            "value{name} is not a constructor (found {})",
                            constructor.type_of()
                        )));
                    }
                    _ => constructor.clone(),
                };
                let prototype = match &constructor {
                    JSValue::Function(..) => {
                        self.user_function_object(&constructor)
                            .and_then(|properties| match properties.borrow().get("prototype") {
                                JSValue::Object(prototype) => Some(prototype),
                                _ => None,
                            })
                    }
                    JSValue::Object(object) => match object.borrow().get("prototype") {
                        JSValue::Object(prototype) => Some(prototype),
                        _ => None,
                    },
                    _ => Some(Rc::clone(&self.object_prototype)),
                };
                let this =
                    JSValue::Object(Rc::new(RefCell::new(JSObject::with_prototype(prototype))));
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
            Opcode::JumpIfNotNullish(offset) => {
                let condition = self.pop()?;
                if !matches!(condition, JSValue::Null | JSValue::Undefined) {
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
        function_name: Option<String>,
    ) -> JSResult<JSValue> {
        let old_stack = std::mem::take(&mut self.stack);
        self.frames.push(CallFrame::new(env, this, function_name));

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
            JSValue::Function(..) | JSValue::ArrowFunction(..) => self.user_function_object(object),
            _ => None,
        };
        let Some(object_ref) = object_ref else {
            return Err(JSError::TypeError(
                "Cannot set property on non-object".to_string(),
            ));
        };
        let key_string = key.to_string();
        let property = object_ref.borrow().get_property_descriptor(&key_string);
        if let Some(property) = property {
            if let Some(setter) = property.setter {
                self.call(setter, object.clone(), vec![value])?;
                return Ok(());
            }
            if property.getter.is_some() || !property.writable {
                return Ok(());
            }
            object_ref.borrow_mut().set(key_string, value);
            return Ok(());
        }

        if let Some(property) = inherited_property_descriptor(&object_ref, &key_string) {
            if let Some(setter) = property.setter {
                self.call(setter, object.clone(), vec![value])?;
                return Ok(());
            }
            if property.getter.is_some() || !property.writable {
                return Ok(());
            }
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

    fn resolve_method_property(&mut self, object: &JSValue, key: &str) -> JSResult<JSValue> {
        match object {
            JSValue::Object(obj_ref) => {
                let own_property = obj_ref.borrow().get_property_descriptor(key);
                if let Some(property) = own_property {
                    if let Some(getter) = property.getter {
                        self.call(getter, object.clone(), Vec::new())
                    } else {
                        Ok(property.value)
                    }
                } else if let Some(property) = inherited_property_descriptor(obj_ref, key) {
                    if let Some(getter) = property.getter {
                        self.call(getter, object.clone(), Vec::new())
                    } else {
                        Ok(property.value)
                    }
                } else {
                    let host_getter = obj_ref
                        .borrow()
                        .get(crate::value::jsobject::HOST_GET_PROPERTY);
                    if matches!(
                        &host_getter,
                        JSValue::Function(..)
                            | JSValue::ArrowFunction(..)
                            | JSValue::NativeFunction(..)
                            | JSValue::BoundFunction(..)
                    ) {
                        self.call(
                            host_getter,
                            object.clone(),
                            vec![JSValue::String(key.to_string())],
                        )
                    } else {
                        Ok(self.object_prototype.borrow().get(key))
                    }
                }
            }
            JSValue::Function(..) | JSValue::ArrowFunction(..) => Ok(self
                .user_function_object(object)
                .map(|properties| properties.borrow().get(key))
                .unwrap_or_else(|| self.function_prototype.borrow().get(key))),
            JSValue::NativeFunction(..) | JSValue::BoundFunction(..) => {
                Ok(self.function_prototype.borrow().get(key))
            }
            JSValue::String(_) => Ok(self.string_prototype.borrow().get(key)),
            JSValue::Number(_) => Ok(self.number_prototype.borrow().get(key)),
            _ => Ok(JSValue::Undefined),
        }
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
        const MAX_CALL_DEPTH: usize = 256;
        if self.frames.len() >= MAX_CALL_DEPTH {
            let stack = self
                .frames
                .iter()
                .rev()
                .take(32)
                .rev()
                .map(|frame| frame.function_name.as_deref().unwrap_or("<anonymous>"))
                .collect::<Vec<_>>()
                .join(" -> ");
            return Err(JSError::RangeError(format!(
                "Maximum call stack size exceeded (JS stack: {stack})"
            )));
        }
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
                let env =
                    self.create_function_env(callee_clone, env, params, args, name.clone(), true);

                self.with_call_frame(env, this, chunk, name)
            }

            JSValue::ArrowFunction(chunk, params, env, lexical_this, _) => {
                let env = self.create_function_env(callee_clone, env, params, args, None, false);
                let this = lexical_this.map(|this| *this).unwrap_or(this);
                self.with_call_frame(env, this, chunk, None)
            }

            JSValue::Object(object) => {
                let callable = object.borrow().get("__call__");
                if matches!(callable, JSValue::Undefined) {
                    let stack = self
                        .frames
                        .iter()
                        .filter_map(|frame| frame.function_name.as_deref())
                        .collect::<Vec<_>>()
                        .join(" -> ");
                    return Err(JSError::TypeError(format!(
                        "object is not callable; keys={:?}; JS stack: {stack}",
                        object.borrow().keys(),
                    )));
                }
                self.call(callable, this, args)
            }

            _ => {
                let stack = self
                    .frames
                    .iter()
                    .filter_map(|frame| frame.function_name.as_deref())
                    .collect::<Vec<_>>()
                    .join(" -> ");
                Err(JSError::TypeError(
                    format!(
                        "{} is not callable (JS stack: {stack})",
                        callee.to_console_string()
                    )
                    .into(),
                ))
            }
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

        if let Some(name) = name {
            env.define(name, func);
        }

        if bind_arguments {
            env.define(
                "arguments".to_string(),
                self.array_from_values(args.clone()),
            );
        }

        for (index, parameter) in params.into_iter().enumerate() {
            if let Some(name) = parameter.strip_prefix("...") {
                env.define(
                    name.to_string(),
                    self.array_from_values(args.get(index..).unwrap_or_default().to_vec()),
                );
                break;
            }
            env.define(
                parameter,
                args.get(index).cloned().unwrap_or(JSValue::Undefined),
            );
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
        let result = op(to_int32(a.to_number()), to_int32(b.to_number()));
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

fn inherited_property_descriptor(
    object: &Rc<RefCell<JSObject>>,
    key: &str,
) -> Option<crate::value::jsobject::Property> {
    let mut current = object.borrow().get_prototype();
    while let Some(prototype) = current {
        let (property, next) = {
            let prototype = prototype.borrow();
            (
                prototype.get_property_descriptor(key),
                prototype.get_prototype(),
            )
        };
        if property.is_some() {
            return property;
        }
        current = next;
    }
    None
}

fn to_uint32(value: f64) -> u32 {
    if !value.is_finite() || value == 0.0 {
        return 0;
    }
    value.trunc().rem_euclid(4_294_967_296.0) as u32
}

fn to_int32(value: f64) -> i32 {
    to_uint32(value) as i32
}
