use criterion::{Criterion, black_box, criterion_group, criterion_main};
use pixi_byte::JSEngine;

/// 単純な算術演算のベンチマーク
fn benchmark_arithmetic(c: &mut Criterion) {
    c.bench_function("simple addition", |b| {
        let mut engine = JSEngine::new();
        b.iter(|| {
            engine.eval(black_box("1 + 2")).unwrap();
        });
    });

    c.bench_function("complex expression", |b| {
        let mut engine = JSEngine::new();
        b.iter(|| {
            engine.eval(black_box("(1 + 2) * 3 - 4 / 2")).unwrap();
        });
    });
}

/// 変数の割り当てと使用のベンチマーク
fn benchmark_variables(c: &mut Criterion) {
    c.bench_function("variable assignment", |b| {
        let mut engine = JSEngine::new();
        b.iter(|| {
            engine.eval(black_box("let x = 42; x + 1")).unwrap();
        });
    });
}

criterion_group!(benches, benchmark_arithmetic, benchmark_variables);
criterion_main!(benches);
