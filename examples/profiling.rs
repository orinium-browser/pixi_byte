//! dhat によるヒープ計測用 example。
//!
//! 使い方:
//!
//! ```sh
//! # dhat で全 allocation を計測し、実行終了時に `dhat-heap.json` を生成
//! cargo run --profile dhat --features dhat-heap --example profiling
//!
//! # 計測なしの動作確認
//! cargo run --example profiling
//! ```
//!
//! workload は `String.prototype.split` がホットになるようにしてあり、
//! `tests/string_alloc_regression.rs` の確保量予算（budget）の基準値採取にも
//! 使う。生成された `dhat-heap.json` は dhat の付属ビューア
//! （dhat リポジトリの `dh_view.html`）で可視化できる。

use pixi_byte::JSEngine;

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() {
    #[cfg(feature = "dhat-heap")]
    let mut _profiler = dhat::Profiler::new_heap();

    // tests/string_alloc_regression.rs と同一の workload。
    // 1 回の split につき約 8k 要素を生成し、100 回繰り返すので
    // 合計 ~800k 個の StrSlice/JSValue を作る。
    let source = r#"
        let s = "";
        for (let i = 0; i < 2000; i++) { s += "foo.bar.baz."; }
        let total = 0;
        for (let i = 0; i < 100; i++) {
            total += s.split(".").length;
        }
        total
    "#;

    let mut engine = JSEngine::new();
    let chunk = engine.compile(source).expect("workload must compile");
    let result = engine.execute(&chunk).expect("workload must run");

    #[cfg(feature = "dhat-heap")]
    {
        let stats = dhat::HeapStats::get();
        println!(
            "[dhat] total: {} blocks / {} bytes",
            stats.total_blocks, stats.total_bytes
        );
        println!(
            "[dhat] max live: {} blocks / {} bytes",
            stats.max_blocks, stats.max_bytes
        );
        drop(_profiler);
    }

    println!("workload result: {result}");
}
