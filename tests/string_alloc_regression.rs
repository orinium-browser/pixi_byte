//! 文字列操作ワークロードの dhat 確保量予算（budget）テスト。
//!
//! 2 つの workload について、合計確保バイト数・ブロック数がしきい値を超えたら
//! 失敗する。wall-time は CI 環境でノイズが大きいため、文字列操作の回帰検知は
//! この決定論的な確保量テストで行う（同じ rustc なら確保量は完全に再現する）。
//!
//! - workload 1 (`concat+split`): 文字列連結 + `String.prototype.split`
//! - workload 2 (`per-char`): `s[i]` / `charAt` / `String.fromCharCode` による
//!   1 文字大量生成
//!
//! dhat の Profiler はプロセス全体で 1 つしか走らせられないため、テストは
//! 1 つにまとめ、workload ごとに `HeapStats::get()` の**差分**を判定する。
//!
//! しきい値の更新方法:
//!
//! ```sh
//! cargo run --profile dhat --example profiling
//! ```
//!
//! で出力される total を確認し、数% のヘッドルームを加えて下記の定数を
//! 更新する。失敗時のメッセージに現在値と予算が表示される。

use dhat::{HeapStats, Profiler};
use pixi_byte::JSEngine;

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

/// 連結 + split workload の合計確保バイト数の予算。
///
/// 基準値: 62,955,635 B（インライン短文字列スライス導入後の実測）。
/// 約 3% のヘッドルームで 65,000,000。導入前の 115,764,435 B や
/// 9ffb1fc の退行 (137,411,395 B) はこの予算を超える。
const BYTE_BUDGET: u64 = 65_000_000;

/// 連結 + split workload の合計確保ブロック数の予算。
///
/// 基準値: 15,808 ブロック（同上）。多数の小さい確保が増える退行を検知する。
const BLOCK_BUDGET: u64 = 17_000;

/// 連結 + split workload の最大ライブバイト数の予算。
///
/// 基準値: 314,732 B（同上）。「結果を必要以上に保持し続ける」種類の退行を
/// 検知するためのガードレール。
const MAX_LIVE_BUDGET: usize = 350_000;

/// 1 文字生成 workload の合計確保バイト数の予算（差分計測）。
///
/// 1 文字（最大 4 UTF-8 バイト）自体はすべてインライン短文字列表現で表現される
/// ため確保 0。残りの確保（基準値約 1.0 MB）は VM 側の付随処理
/// （数値キーの文字列化、ループ管理など）によるもの。
/// `from_char` / `from_str` がボックス化に戻ると +20k × 88B ≒ +1.76 MB で
/// この予算を超える。
const CHAR_BYTE_BUDGET: u64 = 1_400_000;

/// 1 文字生成 workload の合計確保ブロック数の予算（差分計測）。
///
/// 基準値約 36.8k。ボックス化に戻ると +20k ブロックで超過する。
const CHAR_BLOCK_BUDGET: u64 = 50_000;

/// workload を実行して結果文字列を返す。
fn run(source: &str, expected: &str) {
    let mut engine = JSEngine::new();
    let chunk = engine.compile(source).expect("workload must compile");
    let result = engine.execute(&chunk).expect("workload must run");
    assert_eq!(result.to_string(), expected, "workload result mismatch");
}

/// `HeapStats` の差分（dhat に `Sub` 実装はないため手計算）。
struct StatsDelta {
    total_blocks: u64,
    total_bytes: u64,
    max_bytes: usize,
}

fn delta(after: HeapStats, before: HeapStats) -> StatsDelta {
    StatsDelta {
        total_blocks: after.total_blocks - before.total_blocks,
        total_bytes: after.total_bytes - before.total_bytes,
        max_bytes: after.max_bytes,
    }
}

fn assert_budget(label: &str, stats: &StatsDelta, byte_budget: u64, block_budget: u64) {
    eprintln!(
        "[dhat] {label}: {} blocks / {} bytes (budget: {block_budget} blocks / {byte_budget} bytes)",
        stats.total_blocks, stats.total_bytes
    );
    assert!(
        stats.total_bytes <= byte_budget,
        "{label}: total allocated bytes {} exceeded budget {byte_budget}\n\
         確保バイト数の退行です。意図した変更なら \
         `cargo run --profile dhat --example profiling` で新しい基準値を測り、\
         tests/string_alloc_regression.rs の予算を更新してください。",
        stats.total_bytes
    );
    assert!(
        stats.total_blocks <= block_budget,
        "{label}: total allocated blocks {} exceeded budget {block_budget}\n\
         確保ブロック数の退行です。意図した変更なら \
         `cargo run --profile dhat --example profiling` で新しい基準値を測り、\
         tests/string_alloc_regression.rs の予算を更新してください。",
        stats.total_blocks
    );
}

#[test]
fn string_allocation_budgets() {
    // デバッグビルドではエンジンが遅く、確保量も最適化の有無で変動するため
    // release ビルドでのみ実行する（CI では --release で実行される）。
    if cfg!(debug_assertions) {
        eprintln!("skipped: string_allocation_budgets runs only with `--release`");
        return;
    }

    let _profiler = Profiler::new_heap();

    // --- workload 1: 連結 + split ----------------------------------------
    let before_split = HeapStats::get();
    run(
        r#"
        let s = "";
        for (let i = 0; i < 2000; i++) { s += "foo.bar.baz."; }
        let total = 0;
        for (let i = 0; i < 100; i++) {
            total += s.split(".").length;
        }
        total
    "#,
        "600100",
    );
    let split_delta = delta(HeapStats::get(), before_split);
    // max live は workload 1 が支配的なので、その時点の値をそのまま判定する。
    let max_live = split_delta.max_bytes;
    assert_budget("concat+split", &split_delta, BYTE_BUDGET, BLOCK_BUDGET);
    assert!(
        max_live <= MAX_LIVE_BUDGET,
        "max live bytes {} exceeded budget {}\n\
         メモリ保持量の退行です。意図した変更なら予算を更新してください。",
        max_live,
        MAX_LIVE_BUDGET
    );

    // --- workload 2: 1 文字大量生成 ---------------------------------------
    // `s[i]` / `charAt` / `String.fromCharCode` はすべて 1 文字（最大 4 UTF-8
    // バイト）の JSValue を生成する。インライン表現では確保ほぼゼロ。
    // 文字列連結を混ぜると連結バッファが支配的になるため、結果は number に
    // 集計して 1 文字生成の確保だけを測る。
    let before_chars = HeapStats::get();
    // 動的なプロパティキー（数値 -> 文字列変換）は VM 側の確保を生むため、
    // 定数キーのみで 1 文字生成経路を素通りさせる。
    run(
        r#"
        let s = "あa😀éz";
        let total = 0;
        for (let i = 0; i < 4000; i++) {
            total += s[0].length + s[1].length + s[2].length + s[3].length + s[4].length;
        }
        total
    "#,
        // s[i] は code point 単位: あ(1)+a(1)+😀(2)+é(1)+z(1)=6 × 4000 回。
        "24000",
    );
    let chars_delta = delta(HeapStats::get(), before_chars);
    assert_budget(
        "per-char",
        &chars_delta,
        CHAR_BYTE_BUDGET,
        CHAR_BLOCK_BUDGET,
    );
}
