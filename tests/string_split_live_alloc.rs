//! 繰り返し `split` を行っても live allocation が増え続けない（プレートーする）
//! ことの検証。
//!
//! `String.prototype.split` の結果要素は背後の共有文字列 `Rc<str>` を保持する
//! `StrSlice`（`src/builtins/string.rs` の `string_split` 参照）。結果を捨てれば
//! 参照カウントが減って解放されるはずで、Rc の参照サイクルやリークがあれば
//! ライブな確保量が反復ごとに線形で増え続ける。
//!
//! dhat の `HeapStats::curr_blocks` / `curr_bytes`（その時点で live な確保量）を
//! 反復ごとに一定間隔でサンプリングし、後半のチェックポイント間で増加しない
//! ことを確認する。
//!
//! ```sh
//! cargo test --release --test string_split_live_alloc -- --nocapture
//! ```
//!
//! 出力例（正常時。iteration 0 は初回実行前なので最初の 1 回分だけ段差があり、
//! その後は頭打ちになる）:
//!
//! ```text
//! [dhat] separator split
//! [dhat]   iteration    0: live blocks=   ... (engine baseline, pre-execute)
//! [dhat]   iteration  100: ...
//! [dhat]   iteration  200: ... (同じ値で頭打ち)
//! ...
//! ```

use dhat::{HeapStats, Profiler};
use pixi_byte::JSEngine;

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

/// チェックポイント間隔（この反復数ごとに 1 回サンプリングする）。
const SAMPLE_EVERY: usize = 100;

/// 実行する反復数（`0..=ITERATIONS` の末尾で最終サンプルを取る）。
const ITERATIONS: usize = 1000;

/// リークが無い場合に後半チェックポイント間で許容するライブバイト数の増加。
///
/// リークがあると反復ごとに split 結果（配列オブジェクト + `StrSlice` 数個 +
/// 共有 `Rc<str>`）が残り続けるため、この workload では 1000 回で数十万バイト
/// に達する。この緩衝はそうした線形増加を確実に検知できる大きさにしてあり、
/// 一方でプラットフォーム/rustc の僅かな差では超過しない。
const LIVE_BYTES_SLACK: usize = 100_000;

/// リークが無い場合に後半チェックポイント間で許容するライブブロック数の増加。
///
/// リークがあると反復ごとに 5 個前後のブロック（共有 Rc<str> + StrSlice +
/// 配列）が残るため、1000 回で 5000 個近くに達する。
const LIVE_BLOCKS_SLACK: usize = 500;

#[test]
fn repeated_split_plateaus_live_allocations() {
    // デバッグビルドではエンジンが遅く、確保量も最適化の有無で変動するため
    // release ビルドでのみ実行する（CI では --release で実行される）。
    if cfg!(debug_assertions) {
        eprintln!("skipped: repeated_split_plateaus_live_allocations runs only with `--release`");
        return;
    }

    let _profiler = Profiler::new_heap();

    // 要素が 5 バイト超になる separator split。各要素は boxed `StrSlice` になり、
    // 背後で共有 `Rc<str>`（ベース本体）を保持する。これが「live Rc
    // allocations」を構成するホットパス。
    plateau_chart(
        "separator split (boxed StrSlice, shared Rc<str>)",
        r#""firstsegment.secondsegment.thirdsegment".split(".")"#,
    );

    // 空文字 separator split。要素は全て 1 文字（最大 4 バイト）でインライン化
    // されるため要素自体は確保ゼロ。配列（結果）だけが毎回確保・解放される。
    plateau_chart(
        "empty-char split (inline per-char slices)",
        r#""あa😀éz".split("")"#,
    );
}

/// workload を `ITERATIONS` 回実行し、`SAMPLE_EVERY` 間隔で live 確保量を
/// 記録する。その後半チェックポイント間で増加が無いことを検証する。
fn plateau_chart(label: &str, source: &str) {
    let mut engine = JSEngine::new();
    let chunk = engine.compile(source).expect("workload must compile");

    eprintln!("[dhat] {label}");
    let mut samples: Vec<(usize, usize)> = Vec::new();
    for i in 0..=ITERATIONS {
        let result = engine.execute(&chunk).expect("workload must run");
        // result はここで drop される（配列と StrSlice の Rc が解放される）。
        drop(result);
        if i % SAMPLE_EVERY == 0 {
            let stats = HeapStats::get();
            eprintln!(
                "[dhat]   iteration {i:>4}: live blocks={:>6} bytes={:>9}",
                stats.curr_blocks, stats.curr_bytes
            );
            samples.push((stats.curr_blocks, stats.curr_bytes));
        }
    }

    // iteration 0 のサンプルは初回実行前（遅延初期化の前）なので、プレートーの
    // 判定は iteration 100 以降のサンプルで行う。
    let mid = samples
        .get(1)
        .copied()
        .unwrap_or_else(|| samples.first().copied().unwrap());
    let last = *samples.last().expect("must have at least one sample");
    let growth_blocks = last.0.saturating_sub(mid.0);
    let growth_bytes = last.1.saturating_sub(mid.1);
    eprintln!(
        "[dhat]   late growth: +{growth_blocks} blocks / +{growth_bytes} bytes \
         (iteration {SAMPLE_EVERY} {mid:?} -> last {last:?})"
    );
    assert!(
        growth_bytes <= LIVE_BYTES_SLACK,
        "live bytes grew by {growth_bytes} between iteration {SAMPLE_EVERY} and {ITERATIONS}: \
         possible Rc reference cycle / leak.\n\
         live allocations must plateau; {mid:?} -> {last:?}"
    );
    assert!(
        growth_blocks <= LIVE_BLOCKS_SLACK,
        "live blocks grew by {growth_blocks} between iteration {SAMPLE_EVERY} and {ITERATIONS}: \
         possible Rc reference cycle / leak.\n\
         live allocations must plateau; {mid:?} -> {last:?}"
    );
}