//! JavaScript 値 (JSValue) の内部表現
//!
//! NaN-boxing による 64 ビット表現。
//!
//! `JSValue` は `#[repr(transparent)]` の 64 ビット値（内部は非公開 `u64`）で、
//! 以下の 2 領域に分けてエンコードする。
//!
//! - **Number 領域**: 指数部が 0x7FF でなく、かつ「符号=1 & 指数=0x7FF &
//!   mantissa 最上位ビット=1」でないビット列は、そのまま `f64` として読める。
//!   （±Infinity と正の quiet NaN は数値として扱う。）
//! - **ボックス領域（負の quiet NaN）**: 符号=1, 指数=0x7FF, mantissa bit51=1 の
//!   パターン。タグは bits 48-51（bit 51 は常に 1）、参照型ポインタは bits 0-47
//!   に載せる。プリミティブ（undefined / null / boolean）は即値としてインライン化し、
//!   参照型はヒープに置いた `BoxedValue` への `Rc` ポインタを積む。
//!   ユーザー空間のポインタは 2^47 未満（bit 47 = 0）なので 48 ビットで欠損なく
//!   復元できる。
//!
//! 生の `u64` は **非公開** であり、外部から直接読み書きすることはできない。
//! 生成・型判定・参照はすべて本モジュールの公開 API（`kind` / `as_*` / `from_*` 等）
//! を経由する。`unsafe`（`Rc::into_raw` / `Rc::from_raw`）はこのモジュール内部に
//! 完結している。

use super::jsobject::JSObject;
use crate::compiler::BytecodeChunk;
use crate::intern::{FunctionParam, NameId};
use crate::runtime::Environment;
use num_bigint::BigInt;
use num_traits::{ToPrimitive, Zero};
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_BOUND_FUNCTION_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_FUNCTION_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) fn next_function_identity() -> u64 {
    NEXT_FUNCTION_ID.fetch_add(1, Ordering::Relaxed)
}

/// ネイティブ関数の型エイリアス。
/// Rust 側で実装された組み込み関数はこのシグネチャを持ちます。
pub type NativeFunctionType =
    fn(&mut crate::vm::VM, Vec<JSValue>) -> crate::error::JSResult<JSValue>;

// ---------------------------------------------------------------------------
// エンコーディング定数
// ---------------------------------------------------------------------------

/// 負の quiet NaN ファミリを示すマスク（符号 + 指数 + mantissa bit51）。
const NON_NUMBER_MARK: u64 = 0xFFF8_0000_0000_0000;

/// タグのシフト位置とマスク（ペイロード上位ビット）。
///
/// mantissa ビット 51 は boxed 領域を示すマーカとして予約済みのため、
/// タグは bits 48-51 の 4 ビットを使うが、常に bit 51 = 1 に保つ
/// （タグ値 0x8..=0xF）。
const TAG_SHIFT: u32 = 48;
const TAG_MASK: u64 = 0xF;

/// 参照型（ボックス）を示すタグ。
const TAG_BOXED: u64 = 0xF;

/// ポインタ部分として利用する下位ビットのマスク（48 ビット = 256 TiB）。
///
/// ユーザー空間のポインタは 2^47 未満に収まる（bit 47 = 0）ため、
/// bits 0-47 の 48 ビットでそのまま復元できる。
const PTR_MASK: u64 = (1_u64 << 48) - 1;

/// 即値タグ（undefined / null / boolean など）。
const TAG_UNDEFINED: u64 = 0x8;
const TAG_NULL: u64 = 0x9;
const TAG_FALSE: u64 = 0xA;
const TAG_TRUE: u64 = 0xB;

/// 即値を判定する際に比較対象となる可変ペイロード領域
/// （ポインタ bits 0-47 + タグ bits 48-51）。
/// bits 52-63 と bit 51 は `NON_NUMBER_MARK` で保証されるためどちら側もマスクする。
const IMMEDIATE_MASK: u64 = PTR_MASK | (TAG_MASK << TAG_SHIFT);

/// `bits` が「数値として扱える」かどうか（非 non-number 領域）。
#[inline(always)]
fn is_number_bits(bits: u64) -> bool {
    (bits & NON_NUMBER_MARK) != NON_NUMBER_MARK
}

/// 即値（undefined / null / boolean）を組み立てる。
#[inline(always)]
const fn immediate(tag: u64) -> u64 {
    // プレフィックス（NON_NUMBER_MARK のうち、タグと重ならない上位部分）
    let prefix = NON_NUMBER_MARK | (tag << TAG_SHIFT);
    // 即値の場合は下位 48 ビットは必ず 0（ポインタと衝突しないよう PTR_MASK 域は 0）
    prefix & !PTR_MASK
}

/// ボックス値（`Rc<BoxedValue>`）を組み立てる。
fn boxed_value(kind: JsValueKind, payload: BoxedPayload) -> JSValue {
    let boxed = BoxedValue { kind, payload };
    let rc: Rc<BoxedValue> = Rc::new(boxed);
    let ptr = Rc::into_raw(rc) as u64;
    debug_assert!(ptr & !PTR_MASK == 0, "pointer does not fit in NaN payload");
    JSValue(immediate(TAG_BOXED) | (ptr & PTR_MASK))
}

/// この値がボックス値かどうか。
#[inline(always)]
fn is_boxed_bits(bits: u64) -> bool {
    // 負の quiet NaN 領域（NON_NUMBER_MARK）内であることを併せて要求する。
    // タグビットだけを見ると、指数部/mantissa 上位に 0xF を持つ実数
    // （例: 2147483647.0 = 0x41DF_FFFF_FFC0_0000）を誤ってボックス値と判定してしまう。
    (bits & NON_NUMBER_MARK) == NON_NUMBER_MARK && ((bits >> TAG_SHIFT) & TAG_MASK) == TAG_BOXED
}

/// 値の種類（公開 enum）。
///
/// `JSValue::kind()` で得られる。`match` で網羅的に分岐するために公開している。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsValueKind {
    Undefined,
    Null,
    Boolean,
    Number,
    BigInt,
    String,
    Object,
    Function,
    ArrowFunction,
    NativeFunction,
    BoundFunction,
}

/// 参照型の実体。`Rc<...>` で共有し、ナンスペイロードにはその `Rc` の生ポインタを積む。
/// クローン時は `Rc` の参照カウントを増やすだけ（アロケートしない）。
struct BoxedValue {
    kind: JsValueKind,
    payload: BoxedPayload,
}

/// ボックス値の実データ。
enum BoxedPayload {
    Str(Box<str>),
    BigInt(Box<BigInt>),
    Object(Rc<RefCell<JSObject>>),
    Function(FunctionData),
    ArrowFunction(ArrowFunctionData),
    NativeFunction(NativeFunctionType),
    BoundFunction(BoundFunctionData),
}

/// 関数オブジェクトの内部データ。
#[derive(Debug, Clone)]
pub struct FunctionData {
    /// 関数本体のバイトコード。
    pub chunk: Rc<BytecodeChunk>,
    /// 引数パラメータの NameId。
    pub params: Vec<FunctionParam>,
    /// キャプチャされた環境（非クロージャの場合は None）。
    pub env: Option<Rc<RefCell<Environment>>>,
    /// 関数名（あれば）。
    pub name: Option<NameId>,
    /// 関数の一意な ID。
    pub identity: u64,
}

/// アロー関数の内部データ。
#[derive(Debug, Clone)]
pub struct ArrowFunctionData {
    pub chunk: Rc<BytecodeChunk>,
    pub params: Vec<FunctionParam>,
    pub env: Option<Rc<RefCell<Environment>>>,
    /// キャプチャされたレキシカル `this`。
    pub lexical_this: Option<Box<JSValue>>,
    pub identity: u64,
}

/// Internal representation for bound functions
#[derive(Debug, Clone)]
pub struct BoundFunctionData {
    pub identity: u64,
    pub target: Box<JSValue>,
    pub bound_this: JSValue,
    pub bound_args: Vec<JSValue>,
}

impl BoundFunctionData {
    pub fn new(target: JSValue, bound_this: JSValue, bound_args: Vec<JSValue>) -> Self {
        Self {
            identity: NEXT_BOUND_FUNCTION_ID.fetch_add(1, Ordering::Relaxed),
            target: Box::new(target),
            bound_this,
            bound_args,
        }
    }
}

/// JavaScript の値型（NaN-boxed）。
///
/// 内部の `u64` は非公開であり、直接読み書きできない。生成は `from_*` /
/// `From` 実装、型判定・値の取り出しは `kind` / `is_*` / `as_*` を経由する。
#[repr(transparent)]
pub struct JSValue(u64);

impl JSValue {
    // -----------------------------------------------------------------------
    // コンストラクタ
    // -----------------------------------------------------------------------

    #[inline(always)]
    pub const fn undefined() -> Self {
        JSValue(immediate(TAG_UNDEFINED))
    }

    #[inline(always)]
    pub const fn null() -> Self {
        JSValue(immediate(TAG_NULL))
    }

    #[inline(always)]
    pub fn from_bool(b: bool) -> Self {
        JSValue(immediate(if b { TAG_TRUE } else { TAG_FALSE }))
    }

    /// 数値を作る。非 number 領域に突入する特殊な NaN パターンは正の canonical NaN へ正規化する。
    #[inline(always)]
    pub fn from_number(v: f64) -> Self {
        let mut bits = v.to_bits();
        if !is_number_bits(bits) {
            // NaN-boxing の予約領域に衝突しないよう canonical NaN にする
            bits = f64::NAN.to_bits();
        }
        JSValue(bits)
    }

    pub fn from_bigint(v: BigInt) -> Self {
        boxed_value(JsValueKind::BigInt, BoxedPayload::BigInt(Box::new(v)))
    }

    pub fn from_string(s: String) -> Self {
        boxed_value(JsValueKind::String, BoxedPayload::Str(s.into_boxed_str()))
    }

    pub fn from_str(s: &str) -> Self {
        Self::from_string(s.to_string())
    }

    pub fn from_object(o: Rc<RefCell<JSObject>>) -> Self {
        boxed_value(JsValueKind::Object, BoxedPayload::Object(o))
    }

    pub fn from_function(data: FunctionData) -> Self {
        boxed_value(JsValueKind::Function, BoxedPayload::Function(data))
    }

    pub fn from_arrow_function(data: ArrowFunctionData) -> Self {
        boxed_value(
            JsValueKind::ArrowFunction,
            BoxedPayload::ArrowFunction(data),
        )
    }

    pub fn from_native_function(f: NativeFunctionType) -> Self {
        boxed_value(JsValueKind::NativeFunction, BoxedPayload::NativeFunction(f))
    }

    pub fn from_bound_function(data: BoundFunctionData) -> Self {
        boxed_value(
            JsValueKind::BoundFunction,
            BoxedPayload::BoundFunction(data),
        )
    }

    // -----------------------------------------------------------------------
    // 型判定・アクセサ
    // -----------------------------------------------------------------------

    /// 値の種類を返す。
    #[inline(always)]
    pub fn kind(&self) -> JsValueKind {
        let bits = self.0;
        if is_number_bits(bits) {
            return JsValueKind::Number;
        }
        if is_boxed_bits(bits) {
            // unsafe をこのモジュール内に閉じ込める
            let ptr = (bits & PTR_MASK) as *const BoxedValue;
            // Rc が生存を保証している
            let boxed = unsafe { &*ptr };
            return boxed.kind;
        }
        match (bits >> TAG_SHIFT) & TAG_MASK {
            TAG_UNDEFINED => JsValueKind::Undefined,
            TAG_NULL => JsValueKind::Null,
            TAG_FALSE | TAG_TRUE => JsValueKind::Boolean,
            _ => JsValueKind::Undefined,
        }
    }

    #[inline(always)]
    pub fn is_undefined(&self) -> bool {
        !is_number_bits(self.0)
            && (self.0 & IMMEDIATE_MASK) == (immediate(TAG_UNDEFINED) & IMMEDIATE_MASK)
    }

    #[inline(always)]
    pub fn is_null(&self) -> bool {
        !is_number_bits(self.0)
            && (self.0 & IMMEDIATE_MASK) == (immediate(TAG_NULL) & IMMEDIATE_MASK)
    }

    #[inline(always)]
    pub fn is_boolean(&self) -> bool {
        if is_number_bits(self.0) {
            return false;
        }
        matches!(self.0 >> TAG_SHIFT & TAG_MASK, TAG_FALSE | TAG_TRUE)
    }

    #[inline(always)]
    pub fn is_number(&self) -> bool {
        is_number_bits(self.0)
    }

    /// 数値として取り出す。数値でなければ `None`。
    #[inline(always)]
    pub fn as_number(&self) -> Option<f64> {
        if is_number_bits(self.0) {
            Some(f64::from_bits(self.0))
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn as_boolean(&self) -> Option<bool> {
        match (self.0 >> TAG_SHIFT) & TAG_MASK {
            TAG_TRUE => Some(true),
            TAG_FALSE => Some(false),
            _ => None,
        }
    }

    /// ボックス値を参照として取得する（生存保証は Rc が持つ）。
    #[inline(always)]
    fn boxed(&self) -> &BoxedValue {
        debug_assert!(is_boxed_bits(self.0));
        let ptr = (self.0 & PTR_MASK) as *const BoxedValue;
        unsafe { &*ptr }
    }

    pub fn is_bigint(&self) -> bool {
        self.kind() == JsValueKind::BigInt
    }
    pub fn is_string(&self) -> bool {
        self.kind() == JsValueKind::String
    }
    pub fn is_object(&self) -> bool {
        self.kind() == JsValueKind::Object
    }
    pub fn is_function(&self) -> bool {
        self.kind() == JsValueKind::Function
    }
    pub fn is_arrow_function(&self) -> bool {
        self.kind() == JsValueKind::ArrowFunction
    }
    pub fn is_native_function(&self) -> bool {
        self.kind() == JsValueKind::NativeFunction
    }
    pub fn is_bound_function(&self) -> bool {
        self.kind() == JsValueKind::BoundFunction
    }

    pub fn is_string_value(&self) -> bool {
        matches!(
            self.kind(),
            JsValueKind::String | JsValueKind::Number | JsValueKind::BigInt | JsValueKind::Boolean
        )
    }

    /// 文字列を取り出す（存在しなければ `None`）。
    pub fn as_string(&self) -> Option<&str> {
        if !is_boxed_bits(self.0) {
            return None;
        }
        match &self.boxed().payload {
            BoxedPayload::Str(s) => Some(s.as_ref()),
            _ => None,
        }
    }

    /// 文字列を所有値として取り出す。
    pub fn as_string_owned(&self) -> Option<String> {
        self.as_string().map(str::to_string)
    }

    pub fn as_bigint(&self) -> Option<&BigInt> {
        if !is_boxed_bits(self.0) {
            return None;
        }
        match &self.boxed().payload {
            BoxedPayload::BigInt(b) => Some(b.as_ref()),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<Rc<RefCell<JSObject>>> {
        if !is_boxed_bits(self.0) {
            return None;
        }
        match &self.boxed().payload {
            BoxedPayload::Object(o) => Some(Rc::clone(o)),
            _ => None,
        }
    }

    pub fn as_function(&self) -> Option<&FunctionData> {
        if !is_boxed_bits(self.0) {
            return None;
        }
        match &self.boxed().payload {
            BoxedPayload::Function(d) => Some(d),
            _ => None,
        }
    }

    pub fn as_arrow_function(&self) -> Option<&ArrowFunctionData> {
        if !is_boxed_bits(self.0) {
            return None;
        }
        match &self.boxed().payload {
            BoxedPayload::ArrowFunction(d) => Some(d),
            _ => None,
        }
    }

    pub fn as_native_function(&self) -> Option<NativeFunctionType> {
        if !is_boxed_bits(self.0) {
            return None;
        }
        match &self.boxed().payload {
            BoxedPayload::NativeFunction(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_bound_function(&self) -> Option<&BoundFunctionData> {
        if !is_boxed_bits(self.0) {
            return None;
        }
        match &self.boxed().payload {
            BoxedPayload::BoundFunction(d) => Some(d),
            _ => None,
        }
    }

    /// 呼び出し可能かどうか。
    pub fn is_callable(&self) -> bool {
        matches!(
            self.kind(),
            JsValueKind::Function
                | JsValueKind::ArrowFunction
                | JsValueKind::NativeFunction
                | JsValueKind::BoundFunction
                | JsValueKind::Object // host callable（__call__ 有無は呼び出し側で判定）
        )
    }

    pub(crate) fn user_function_identity(&self) -> Option<u64> {
        match self.kind() {
            JsValueKind::Function => {
                let d = self.as_function().unwrap();
                Some(d.identity)
            }
            JsValueKind::ArrowFunction => {
                let d = self.as_arrow_function().unwrap();
                Some(d.identity)
            }
            _ => None,
        }
    }

    pub(crate) fn callable_storage_identity(&self) -> Option<u64> {
        match self.kind() {
            JsValueKind::BoundFunction => {
                let d = self.as_bound_function().unwrap();
                Some(d.identity | (1_u64 << 63))
            }
            _ => self.user_function_identity(),
        }
    }

    // -----------------------------------------------------------------------
    // 変換・比較
    // -----------------------------------------------------------------------

    /// 値をコンソール向け文字列に変換します（ToString 相当）。
    pub fn to_console_string(&self) -> String {
        match self.kind() {
            JsValueKind::Undefined => "undefined".to_string(),
            JsValueKind::Null => "null".to_string(),
            JsValueKind::Boolean => self.as_boolean().unwrap().to_string(),
            JsValueKind::Number => {
                let n = self.as_number().unwrap();
                if n.is_nan() {
                    "NaN".to_string()
                } else if n.is_infinite() {
                    if n.is_sign_positive() {
                        "Infinity".to_string()
                    } else {
                        "-Infinity".to_string()
                    }
                } else {
                    n.to_string()
                }
            }
            JsValueKind::BigInt => self.as_bigint().unwrap().to_string(),
            JsValueKind::String => self.as_string().unwrap_or("").to_string(),
            JsValueKind::Object => "[object Object]".to_string(),
            JsValueKind::Function | JsValueKind::ArrowFunction => "[function]".to_string(),
            JsValueKind::NativeFunction => "[native function]".to_string(),
            JsValueKind::BoundFunction => "[bound function]".to_string(),
        }
    }

    /// 値を数値に変換します（ToNumber）。
    pub fn to_number(&self) -> f64 {
        match self.kind() {
            JsValueKind::Undefined => f64::NAN,
            JsValueKind::Null => 0.0,
            JsValueKind::Boolean => {
                if self.as_boolean().unwrap() {
                    1.0
                } else {
                    0.0
                }
            }
            JsValueKind::Number => self.as_number().unwrap(),
            JsValueKind::BigInt => self.as_bigint().unwrap().to_f64().unwrap_or_else(|| {
                if self.as_bigint().unwrap().sign() == num_bigint::Sign::Minus {
                    f64::NEG_INFINITY
                } else {
                    f64::INFINITY
                }
            }),
            JsValueKind::String => {
                let trimmed = self.as_string().unwrap_or("").trim();
                if trimmed.is_empty() {
                    return 0.0;
                }
                trimmed.parse().unwrap_or(f64::NAN)
            }
            JsValueKind::Object
            | JsValueKind::Function
            | JsValueKind::ArrowFunction
            | JsValueKind::NativeFunction => f64::NAN,
            JsValueKind::BoundFunction => self.as_bound_function().unwrap().target.to_number(),
        }
    }

    /// 値を真偽値に変換します（ToBoolean）。
    pub fn to_boolean(&self) -> bool {
        match self.kind() {
            JsValueKind::Undefined | JsValueKind::Null => false,
            JsValueKind::Boolean => self.as_boolean().unwrap(),
            JsValueKind::Number => {
                let n = self.as_number().unwrap();
                !n.is_nan() && n != 0.0
            }
            JsValueKind::BigInt => !self.as_bigint().unwrap().is_zero(),
            JsValueKind::String => !self.as_string().unwrap_or("").is_empty(),
            JsValueKind::Object
            | JsValueKind::Function
            | JsValueKind::ArrowFunction
            | JsValueKind::NativeFunction
            | JsValueKind::BoundFunction => true,
        }
    }

    /// `typeof` の戻り値を返します。
    pub fn type_of(&self) -> &'static str {
        match self.kind() {
            JsValueKind::Undefined => "undefined",
            JsValueKind::Null => "object",
            JsValueKind::Boolean => "boolean",
            JsValueKind::Number => "number",
            JsValueKind::BigInt => "bigint",
            JsValueKind::String => "string",
            JsValueKind::Object => {
                let callable = self
                    .as_object()
                    .map(|object| {
                        object
                            .try_borrow()
                            .map(|object| !object.get("__call__").is_undefined())
                            .unwrap_or(false)
                    })
                    .unwrap_or(false);
                if callable { "function" } else { "object" }
            }
            JsValueKind::Function
            | JsValueKind::ArrowFunction
            | JsValueKind::NativeFunction
            | JsValueKind::BoundFunction => "function",
        }
    }

    /// 厳密等価比較（===）。
    pub fn strict_equals(&self, other: &JSValue) -> bool {
        // 高速パス: 両方 Number の場合はビット比較（NaN/±0 を考慮）
        if self.is_number() && other.is_number() {
            let a = self.as_number().unwrap();
            let b = other.as_number().unwrap();
            return if a.is_nan() || b.is_nan() {
                false
            } else {
                a == b
            };
        }

        match (self.kind(), other.kind()) {
            (JsValueKind::Undefined, JsValueKind::Undefined) => true,
            (JsValueKind::Null, JsValueKind::Null) => true,
            (JsValueKind::Boolean, JsValueKind::Boolean) => self.as_boolean() == other.as_boolean(),
            (JsValueKind::Number, JsValueKind::Number) => self.as_number() == other.as_number(),
            (JsValueKind::BigInt, JsValueKind::BigInt) => self.as_bigint() == other.as_bigint(),
            (JsValueKind::String, JsValueKind::String) => self.as_string() == other.as_string(),
            (JsValueKind::Object, JsValueKind::Object) => {
                // オブジェクトは参照が同じ場合のみ true
                let a = self.as_object().unwrap();
                let b = other.as_object().unwrap();
                Rc::ptr_eq(&a, &b)
            }
            (JsValueKind::Function, JsValueKind::Function)
            | (JsValueKind::ArrowFunction, JsValueKind::ArrowFunction) => {
                let (self_id, self_chunk) = function_identity_and_chunk(self);
                let (other_id, other_chunk) = function_identity_and_chunk(other);
                (self_id != 0 && self_id == other_id)
                    || (self_id == 0
                        && other_id == 0
                        && self_chunk.identity == other_chunk.identity)
            }
            (JsValueKind::NativeFunction, JsValueKind::NativeFunction) => std::ptr::fn_addr_eq(
                self.as_native_function().unwrap(),
                other.as_native_function().unwrap(),
            ),
            (JsValueKind::BoundFunction, JsValueKind::BoundFunction) => {
                self.as_bound_function().unwrap().identity
                    == other.as_bound_function().unwrap().identity
            }
            _ => false,
        }
    }

    /// 抽象等価比較（==）。
    pub fn abstract_equals(&self, other: &JSValue) -> bool {
        // 同じ種類の場合は厳密等価
        if self.kind() == other.kind() {
            return self.strict_equals(other);
        }

        match (self.kind(), other.kind()) {
            (JsValueKind::Null, JsValueKind::Undefined)
            | (JsValueKind::Undefined, JsValueKind::Null) => true,
            (JsValueKind::Number, JsValueKind::String) => {
                self.as_number().unwrap() == other.to_number()
            }
            (JsValueKind::String, JsValueKind::Number) => {
                self.to_number() == other.as_number().unwrap()
            }
            (JsValueKind::BigInt, JsValueKind::Number) => {
                let integer = self.as_bigint().unwrap();
                let number = other.as_number().unwrap();
                number.is_finite() && number.fract() == 0.0 && integer.to_f64() == Some(number)
            }
            (JsValueKind::Number, JsValueKind::BigInt) => {
                let number = self.as_number().unwrap();
                let integer = other.as_bigint().unwrap();
                number.is_finite() && number.fract() == 0.0 && integer.to_f64() == Some(number)
            }
            (JsValueKind::Boolean, _) => {
                JSValue::from_number(self.to_number()).abstract_equals(other)
            }
            (_, JsValueKind::Boolean) => {
                self.abstract_equals(&JSValue::from_number(other.to_number()))
            }
            _ => false,
        }
    }

    /// 生のビット列（テスト・デバッグ用・詳細実装の検証に限る）。
    ///
    /// 通常のユースケースでは使わない。エンコーディングの検査（イディオムの単体テスト）でのみ利用。
    #[doc(hidden)]
    pub fn to_raw_bits(&self) -> u64 {
        self.0
    }
}

fn function_identity_and_chunk(v: &JSValue) -> (u64, Rc<BytecodeChunk>) {
    match v.kind() {
        JsValueKind::Function => {
            let d = v.as_function().unwrap();
            (d.identity, Rc::clone(&d.chunk))
        }
        JsValueKind::ArrowFunction => {
            let d = v.as_arrow_function().unwrap();
            (d.identity, Rc::clone(&d.chunk))
        }
        _ => unreachable!(),
    }
}

impl From<bool> for JSValue {
    fn from(b: bool) -> Self {
        JSValue::from_bool(b)
    }
}

impl From<f64> for JSValue {
    fn from(v: f64) -> Self {
        JSValue::from_number(v)
    }
}

impl From<i32> for JSValue {
    fn from(v: i32) -> Self {
        JSValue::from_number(v as f64)
    }
}

impl From<u32> for JSValue {
    fn from(v: u32) -> Self {
        JSValue::from_number(v as f64)
    }
}

impl From<usize> for JSValue {
    fn from(v: usize) -> Self {
        JSValue::from_number(v as f64)
    }
}

impl From<String> for JSValue {
    fn from(s: String) -> Self {
        JSValue::from_string(s)
    }
}

impl From<&str> for JSValue {
    fn from(s: &str) -> Self {
        JSValue::from_str(s)
    }
}

impl From<BigInt> for JSValue {
    fn from(v: BigInt) -> Self {
        JSValue::from_bigint(v)
    }
}

impl PartialEq for JSValue {
    fn eq(&self, other: &Self) -> bool {
        self.strict_equals(other)
    }
}

impl fmt::Debug for JSValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind() {
            JsValueKind::Undefined => write!(f, "Undefined"),
            JsValueKind::Null => write!(f, "Null"),
            JsValueKind::Boolean => write!(f, "Boolean({})", self.as_boolean().unwrap()),
            JsValueKind::Number => write!(f, "Number({})", self.as_number().unwrap()),
            JsValueKind::BigInt => write!(f, "BigInt({})", self.as_bigint().unwrap()),
            JsValueKind::String => write!(f, "String(\"{}\")", self.as_string().unwrap_or("")),
            JsValueKind::Object => write!(f, "Object(...)"),
            JsValueKind::Function => write!(f, "Function(...)"),
            JsValueKind::ArrowFunction => write!(f, "ArrowFunction(...)"),
            JsValueKind::NativeFunction => write!(f, "NativeFunction(...)"),
            JsValueKind::BoundFunction => write!(f, "BoundFunction(...)"),
        }
    }
}

impl Clone for JSValue {
    fn clone(&self) -> Self {
        if is_number_bits(self.0) || !is_boxed_bits(self.0) {
            // 数値または即値: ビット列をそのままコピー
            JSValue(self.0)
        } else {
            // ボックス値: Rc の参照カウントを増やす（アロケートしない）
            let ptr = (self.0 & PTR_MASK) as *const BoxedValue;
            // from_raw で借用し、clone で参照を増やし、forget で借用の drop を抑止
            let rc = unsafe { Rc::from_raw(ptr) };
            let new_rc = Rc::clone(&rc);
            std::mem::forget(rc);
            let new_ptr = Rc::into_raw(new_rc) as u64;
            JSValue(immediate(TAG_BOXED) | (new_ptr & PTR_MASK))
        }
    }
}

impl Drop for JSValue {
    fn drop(&mut self) {
        if is_boxed_bits(self.0) {
            let ptr = (self.0 & PTR_MASK) as *const BoxedValue;
            // from_raw が所有権を引き継ぎ、drop で参照カウントを減らす
            unsafe {
                drop(Rc::from_raw(ptr));
            }
        }
    }
}

impl fmt::Display for JSValue {
    /// 値をフォーマット表示
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_console_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_integers_are_inline_numbers() {
        for v in [0.0, -0.0, 1.0, -1.0, 42.0, 3.14, 1e100] {
            let j = JSValue::from_number(v);
            assert!(j.is_number());
            assert_eq!(j.as_number(), Some(v));
        }
    }

    #[test]
    fn nan_and_infinity_are_numbers() {
        assert!(JSValue::from_number(f64::NAN).is_number());
        assert!(JSValue::from_number(f64::INFINITY).is_number());
        assert!(JSValue::from_number(f64::NEG_INFINITY).is_number());
    }

    #[test]
    fn negative_nan_number_is_canonicalized_to_number() {
        // -NaN のビット列は NaN-boxing の予約領域に衝突しうる → canonical NaN に正規化される
        let j = JSValue::from_number(-f64::NAN);
        assert!(j.is_number());
        assert!(j.as_number().unwrap().is_nan());
    }

    #[test]
    fn primitives_are_inline() {
        assert!(JSValue::undefined().is_undefined());
        assert!(JSValue::null().is_null());
        assert_eq!(JSValue::from_bool(true).as_boolean(), Some(true));
        assert_eq!(JSValue::from_bool(false).as_boolean(), Some(false));
        // プリミティブはボックスされない（オブジェクト/関数等と同じではない）
        assert!(JSValue::undefined().to_raw_bits() & PTR_MASK == 0);
    }

    #[test]
    fn strings_roundtrip() {
        let j = JSValue::from_string("hello".to_string());
        assert!(j.is_string());
        assert_eq!(j.as_string(), Some("hello"));
        assert_eq!(j.to_string(), "hello");
        // clone は共有される
        let c = j.clone();
        assert_eq!(c.as_string(), Some("hello"));
    }

    #[test]
    fn object_identity_preserved_across_clone() {
        use std::cell::RefCell;
        let o = Rc::new(RefCell::new(JSObject::new()));
        let j = JSValue::from_object(Rc::clone(&o));
        let c = j.clone();
        let a = j.as_object().unwrap();
        let b = c.as_object().unwrap();
        assert!(Rc::ptr_eq(&a, &b));
    }

    #[test]
    fn bigints_roundtrip() {
        let j = JSValue::from_bigint(BigInt::from(12345678901234567890_u64));
        assert!(j.is_bigint());
        assert_eq!(j.as_bigint(), Some(&BigInt::from(12345678901234567890_u64)));
    }

    #[test]
    fn numbers_are_not_confused_with_primitives() {
        for v in [0.0, 1.0, -2.5, f64::NAN] {
            let j = JSValue::from_number(v);
            assert!(!j.is_undefined());
            assert!(!j.is_null());
            assert!(!j.is_boolean());
        }
    }

    #[test]
    fn size_is_one_word() {
        assert_eq!(std::mem::size_of::<JSValue>(), 8);
    }
}
