//! 文字列操作（連結・`String.prototype.split`）のベンチマーク。
//!
//! 共有文字列（`StrSlice` / `Rc<str>` ベース共有）経路の性能回帰を検知するための
//! ワークロード。`examples/profiling.rs` と同じ形状（連結 → split）を使う。
//!
//! 確保バイト数の回帰検知は `tests/string_alloc_regression.rs`（dhat 予算テスト）が
//! 担当する。このベンチは wall-time を測るもので、CI 上では参考値として記録する。

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use pixi_byte::JSEngine;

/// 連結セグメント。`split(".")` で 3 要素 + 末尾の空文字列になる。
const SEGMENT: &str = "foo.bar.baz.";
const SEGMENT_LEN: usize = SEGMENT.len();

/// 連結回数 → split 回数の組み合わせ。
const SPLIT_CASES: &[(usize, usize)] = &[(2_000, 100), (10_000, 500)];

fn concat_source(loops: usize) -> String {
    format!(
        r#"
        let s = "";
        for (let i = 0; i < {loops}; i++) {{ s += "{SEGMENT}"; }}
        s.length
        "#
    )
}

fn split_source(concat_loops: usize, split_loops: usize) -> String {
    format!(
        r#"
        let s = "";
        for (let i = 0; i < {concat_loops}; i++) {{ s += "{SEGMENT}"; }}
        let total = 0;
        for (let i = 0; i < {split_loops}; i++) {{
            total += s.split(".").length;
        }}
        total
        "#
    )
}

/// ワークロードの期待値。`"foo.bar.baz."` の split は 3 セパレータ + 末尾空文字列
/// で `3 * concat_loops + 1` 要素になる。
fn split_expected(concat_loops: usize, split_loops: usize) -> String {
    ((concat_loops * 3 + 1) * split_loops).to_string()
}

fn benchmark_string_concat(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_concat");
    for &loops in &[2_000usize, 10_000] {
        group.throughput(Throughput::Elements((loops * SEGMENT_LEN) as u64));
        group.bench_with_input(BenchmarkId::new("build", loops), &loops, |b, &loops| {
            let source = concat_source(loops);
            let mut engine = JSEngine::new();
            let chunk = engine.compile(&source).expect("workload must compile");
            let expected = (loops * SEGMENT_LEN).to_string();
            b.iter(|| {
                let result = black_box(engine.execute(&chunk)).expect("workload must run");
                assert_eq!(result.to_string(), expected, "workload result mismatch");
            });
        });
    }
    group.finish();
}

fn benchmark_string_split(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_split");
    // 大きいケースは 1 反復が 1 秒超えのため、サンプル数と計測時間を抑える。
    group.sample_size(16);
    group.measurement_time(std::time::Duration::from_secs(4));
    for &(concat_loops, split_loops) in SPLIT_CASES {
        group.throughput(Throughput::Elements(
            (concat_loops * SEGMENT_LEN * split_loops) as u64,
        ));
        group.bench_with_input(
            BenchmarkId::new(
                "split",
                format!("{concat_loops}x{split_loops}"),
            ),
            &(concat_loops, split_loops),
            |b, &(concat_loops, split_loops)| {
                let source = split_source(concat_loops, split_loops);
                let mut engine = JSEngine::new();
                let chunk = engine.compile(&source).expect("workload must compile");
                let expected = split_expected(concat_loops, split_loops);
                b.iter(|| {
                    let result = black_box(engine.execute(&chunk)).expect("workload must run");
                    assert_eq!(result.to_string(), expected, "workload result mismatch");
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, benchmark_string_concat, benchmark_string_split);
criterion_main!(benches);
