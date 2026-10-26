//! Regression test for the lifetime of materialized function objects.
//!
//! A function value only gets a JS-visible object when something observes it
//! (an own property write, `.prototype`, ...). That object holds the function
//! value through `__call__` and `prototype.constructor`, while the VM holds the
//! object, so a long-running script that decorates throwaway functions (core-js
//! and bundled polyfills do this constantly) used to retain every decorated
//! function forever. The VM now sweeps entries that nothing but the cache still
//! references; this test pins the reclamation side. Semantics preservation for
//! reachable functions lives in `function_object_semantics.rs`.
//!
//! dhat allows a single profiler per process, so the measurement test has its
//! own test binary.
//!
//! ```sh
//! cargo test --test callable_object_retention
//! ```

use dhat::{HeapStats, Profiler};
use pixi_byte::JSEngine;

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

/// Functions decorated per round. Must exceed the sweep floor (1024) so a
/// single round reclaims what the previous round left behind.
const PER_ROUND: usize = 1_500;
/// Rounds to run. The first half is the warm-up window, the second half the
/// steady-state window compared against it.
const ROUNDS: usize = 8;

/// Cached callable objects allowed to remain after a round. The sweep runs at
/// the floor, so at most one round's worth of post-sweep garbage can remain;
/// an unbounded cache would hold `PER_ROUND * round` entries.
const MAX_CACHED_AFTER_ROUND: usize = 1_600;

/// Live-byte headroom for the steady-state half over the warm-up half.
///
/// Live memory oscillates with the reclamation sweep (it climbs until the cache
/// hits the sweeep threshold, then drops), so the comparison is peak-to-peak
/// rather than round-to-round. Each leaked function object costs roughly 1 KB,
/// so an unbounded cache would add several MB per round; 400 KB of headroom
/// covers the oscillation and the engine's own bookkeeping.
const MAX_STEADY_STATE_GROWTH: usize = 400_000;

/// Decorates throwaway functions inside a function scope, so the functions
/// themselves become unreachable when the scope ends.
fn decorate_throwaways() -> String {
    format!(
        r#"
        (function () {{
          for (var i = 0; i < {PER_ROUND}; i++) {{
            var f = function decorated(a) {{ return a; }};
            f.tag = i;
            f.helper = function () {{ return i; }};
            f(i);
          }}
        }})();
        "#
    )
}

struct Delta {
    live_bytes: usize,
}

fn run(engine: &mut JSEngine, source: &str) -> pixi_byte::JSValue {
    let chunk = engine.compile(source).expect("workload must compile");
    engine.execute(&chunk).expect("workload must run")
}

/// Executes one round and reports the live heap afterwards.
fn round(engine: &mut JSEngine) -> Delta {
    run(engine, &decorate_throwaways());
    engine.run_jobs().expect("jobs must run");
    Delta {
        live_bytes: HeapStats::get().curr_bytes,
    }
}

#[test]
fn decorated_throwaway_functions_are_reclaimed() {
    let _profiler = Profiler::new_heap();
    let mut engine = JSEngine::new();
    let baseline = HeapStats::get().curr_bytes;

    let mut deltas = Vec::new();
    for round_index in 0..ROUNDS {
        let delta = round(&mut engine);
        let cached = engine.diagnostics().callable_objects;
        println!(
            "round {}: live {} B (baseline {baseline}), cached callable objects {cached}",
            round_index + 1,
            delta.live_bytes
        );
        assert!(
            cached <= MAX_CACHED_AFTER_ROUND,
            "round {}: {cached} callable objects cached (limit {MAX_CACHED_AFTER_ROUND}); \
             unreachable function objects are being retained",
            round_index + 1
        );
        deltas.push(delta.live_bytes);
    }

    // Compare peak live bytes of the steady-state half with the warm-up half.
    // A leak adds a fixed amount per round, so the late peak would run away;
    // the reclamation sawtooth keeps both peaks at the same height.
    let split = deltas.len() / 2;
    let warmup_peak = deltas[..split].iter().copied().max().unwrap();
    let steady_peak = deltas[split..].iter().copied().max().unwrap();
    assert!(
        steady_peak <= warmup_peak + MAX_STEADY_STATE_GROWTH,
        "steady-state peak {steady_peak} B exceeds warm-up peak {warmup_peak} B by more than \
         {MAX_STEADY_STATE_GROWTH} B; per-round growth indicates retained function objects"
    );
}
