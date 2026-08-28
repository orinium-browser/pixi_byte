use crate::error::{JSError, JSResult};
use rustc_hash::FxHashMap;

/// インターニングされた変数名の一意な識別子。
///
/// # 不変条件 (table-local)
///
/// `NameId` は**生成元の [`InternTable`] に対してのみ**意味を持つ。
/// 値は単なる `u32` インデックスであり、**別の `InternTable` で採番された
/// `NameId` どうしを比較・交換・混在させる操作は無意味**であり、誤った
/// 変数を指すバグの原因となる。
///
/// 具体的には、ある `compile()` 呼び出しで生成された全てのチャンク
/// (トップレベル + ネスト関数) と、その実行時に作成される環境の束縛キーは、
/// **必ず同一の `InternTable` を共有**しなければならない。ネスト関数の
/// 自由変数 (free variable) の `NameId` が外側環境のキーと一致するのは、
/// この単一テーブル共有が正しく保たれた場合のみである。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NameId(pub u32);

/// 関数・アロー関数の引数パラメータ定義。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionParam {
    /// 通常の引数 (`a`, `b` など)
    Positional(NameId),
    /// 可変長引数 (`...rest` など)
    Rest(NameId),
}

/// 変数名をコンパイル単位ごとにインターニングするテーブル。
///
/// # 所有権のライフサイクル
///
/// - **コンパイル中**: [`Compiler`](crate::compiler::Compiler) が
///   `Rc<RefCell<InternTable>>` として共有保持し、`intern()` で名前を登録する。
///   ネストの `Compiler` は **必ず** 親の `Rc<RefCell<_>>` をクローンして共有し、
///   独自テーブルを生成しない。
/// - **実行中**: 各 [`BytecodeChunk`](crate::compiler::BytecodeChunk) が同じ
///   `Rc<RefCell<InternTable>>` を保持し、`LoadVar` のグローバルフォールバック
///   時に `name(id)` で `&str` を取り出す（同一チャンク由来の `NameId` のみ使用）。
///
/// `intern()` は**既存名の lookup では一切アロケートしない**。新規名の場合のみ
/// `Box<str>` を生成する。
#[derive(Debug, Default)]
pub struct InternTable {
    /// `NameId` -> 名前。インデックスがそのまま `NameId.0` に対応する。
    names: Vec<Box<str>>,
    /// 名前 -> `NameId`。オーナーは `Box<str>`。
    ids: FxHashMap<Box<str>, NameId>,
}

impl InternTable {
    /// 新しく空のテーブルを作成する。
    pub fn new() -> Self {
        Self::default()
    }

    /// 名前をインターニングし、対応する `NameId` を返す。
    ///
    /// 既存の名前であればハッシュ lookup のみで即座に返る（アロケートなし）。
    /// 新規の名前であれば `Box<str>` を1回生成して登録する。
    pub fn intern(&mut self, name: &str) -> JSResult<NameId> {
        if let Some(id) = self.ids.get(name) {
            return Ok(*id);
        }
        if self.names.len() as u64 >= u32::MAX as u64 {
            return Err(JSError::InternalError(
                "name interning table overflowed (more than u32::MAX names)".to_string(),
            ));
        }
        let id = NameId(self.names.len() as u32);
        let owned: Box<str> = name.into();
        self.names.push(owned.clone());
        self.ids.insert(owned, id);
        Ok(id)
    }

    /// `NameId` を名前文字列へ解決する。
    ///
    /// # Panics
    /// `id` がこのテーブルに存在しない場合に panic する。これは内部不変条件の
    /// 違反であり（`NameId` は table-local）、呼び出し側は必ず同一テーブル由来の
    /// `NameId` のみを渡すこと。
    pub fn name(&self, id: NameId) -> &str {
        self.names
            .get(id.0 as usize)
            .expect("NameId was not produced by this InternTable")
            .as_ref()
    }

    /// 登録済みの名前の総数。
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// 名前が登録されていないことを確認する。
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}
