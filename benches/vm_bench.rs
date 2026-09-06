use criterion::{Criterion, criterion_group, criterion_main};
use pixi_byte::JSEngine;
use std::hint::black_box;

fn bench_execution(c: &mut Criterion, name: &str, source: &str, iterations: usize) {
    let mut engine = JSEngine::new();
    let chunk = engine.compile(source).unwrap();
    c.bench_function(name, |b| {
        let mut engine = black_box(JSEngine::new());
        b.iter(|| {
            for _ in 0..iterations {
                black_box(engine.execute(&chunk)).unwrap();
            }
        });
    });
}

/// 単純な算術演算のベンチマーク
fn benchmark_arithmetic(c: &mut Criterion) {
    bench_execution(c, "simple addition x1000", "1 + 2", 1000);
    bench_execution(c, "complex expression x1000", "(1 + 2) * 3 - 4 / 2", 1000);
}

/// 変数の割り当てと使用のベンチマーク
fn benchmark_variables(c: &mut Criterion) {
    bench_execution(c, "variable assignment x1000", "let x = 42; x + 1", 1000);
}

/// 実際のワークロード: ループ内の累積加算
fn benchmark_loop(c: &mut Criterion) {
    let source = "let s = 0; for (let i = 0; i < 1000; i++) { s = s + i; } s";
    bench_execution(c, "loop 1000 iterations", source, 1);
}

/// 実際のワークロード: 関数呼び出し
fn benchmark_function_call(c: &mut Criterion) {
    let source = "function add(a, b) { return a + b; } let s = 0; for (let i = 0; i < 100; i++) { s = add(s, i); } s";
    bench_execution(c, "function call loop 100", source, 1);
}

/// 実際のワークロード: 配列要素の読み書き（プロパティキー生成がホット）
fn benchmark_array_access(c: &mut Criterion) {
    let source = "let a = []; for (let i = 0; i < 1000; i++) { a[i] = i * 2; } let s = 0; for (let i = 0; i < 1000; i++) { s += a[i]; } s";
    bench_execution(c, "array element access 2000 ops", source, 1);
}

/// 実際のワークロード: 配列メソッドの要素走査（get_index がホット）
fn benchmark_array_map(c: &mut Criterion) {
    let source = "let a = []; for (let i = 0; i < 100; i++) { a.push(i); } let s = 0; a.map(x => x * 2).forEach(x => { s += x; }); s";
    bench_execution(c, "array map+forEach 100 elems", source, 1);
}

criterion_group!(
    benches,
    benchmark_arithmetic,
    benchmark_variables,
    benchmark_loop,
    benchmark_function_call,
    benchmark_array_access,
    benchmark_array_map
);
criterion_main!(benches);
