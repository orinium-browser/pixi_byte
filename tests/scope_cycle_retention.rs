//! Regression test for end-of-scope cycle reclamation.
//!
//! A closure created in a scope captures that scope's environment, and the
//! environment's bindings hold the closure, so `Rc` alone can never free the
//! pair. Before the VM broke that cycle at scope exit, every function call that
//! created a closure leaked its whole scope: environments, closure values, the
//! objects decorating them and the promises their callbacks settled on.
//!
//! A representative hot loop has a helper create a never-settling promise
//! while the caller decorates two throwaway functions. Before the cycle was
//! broken, this shape grew live memory until the process was killed.
//!
//! dhat allows one profiler per process and the census counters are global, so
//! the measurement lives in a single test in its own binary.
//!
//! ```sh
//! cargo test --test scope_cycle_retention
//! ```

use dhat::{HeapStats, Profiler};
use pixi_byte::JSEngine;

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

/// Workload units executed per round. Every unit creates a scope, two
/// decorated closures and one pending promise.
const UNITS_PER_ROUND: usize = 2000;
/// Rounds to run; the second half is compared against the first half.
const ROUNDS: usize = 6;
/// Callable objects allowed to remain cached after a round.
const MAX_CACHED: usize = 200;
/// Live-byte headroom between the warm-up peak and the steady-state peak.
const MAX_GROWTH_BYTES: usize = 400_000;
/// Live environment/closure growth allowed between the steady-state rounds.
/// One leaked scope per unit would add `UNITS_PER_ROUND` per round.
const MAX_GROWTH_COUNT: usize = 200;

/// Reads `key=<number>` out of the `live_census()` string.
fn count(census: &str, key: &str) -> usize {
    census
        .split_whitespace()
        .find_map(|field| field.strip_prefix(key)?.strip_prefix('='))
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| panic!("census has no `{key}=` field: {census}"))
}

/// A representative hot unit: a function call that creates closures inside its
/// own scope, decorates them and creates a promise that never settles. Nothing
/// outlives the call.
fn closure_promise_unit() -> String {
    format!(
        r#"
        function makePromise() {{
          return new Promise(function () {{}});
        }}
        function unit() {{
          var a = function () {{ return 1; }};
          a.tag = 1;
          var b = function () {{ return 2; }};
          b.tag = 2;
          a();
          b();
          return makePromise();
        }}
        (function () {{
          var sink = [];
          for (var i = 0; i < {UNITS_PER_ROUND}; i++) {{ sink.push(unit()); }}
          sink.length = 0;
        }})();
        "#
    )
}

/// Reactions registered on a promise that settles afterwards, while the
/// promise itself stays reachable: the settled promise must not keep the
/// handlers (and their scopes) alive forever.
fn settled_promise_reactions() -> String {
    format!(
        r#"
        var settler;
        var kept = new Promise(function (resolve) {{ settler = resolve; }});
        for (var i = 0; i < {UNITS_PER_ROUND}; i++) {{
          var handler = function () {{ return i; }};
          kept.then(handler);
        }}
        settler(1);
        "#
    )
}

struct Sample {
    live_bytes: usize,
    objects: usize,
    environments: usize,
    functions: usize,
    arrow_functions: usize,
    cached: usize,
}

#[test]
fn closure_scopes_and_settled_reactions_are_reclaimed() {
    let _profiler = Profiler::new_heap();
    let mut engine = JSEngine::new();

    let measure = |engine: &mut JSEngine| Sample {
        live_bytes: HeapStats::get().curr_bytes,
        objects: count(&pixi_byte::live_census(), "objects"),
        environments: count(&pixi_byte::live_census(), "environments"),
        functions: count(&pixi_byte::live_census(), "functions"),
        arrow_functions: count(&pixi_byte::live_census(), "arrow_functions"),
        cached: engine.diagnostics().callable_objects,
    };

    let unit = engine
        .compile(&closure_promise_unit())
        .expect("closure workload unit must compile");
    let mut samples = Vec::new();
    for round in 0..ROUNDS {
        engine
            .execute(&unit)
            .expect("closure workload unit must run");
        engine.run_jobs().expect("jobs must run");
        let sample = measure(&mut engine);
        println!(
            "round {}: live {} B objects={} environments={} functions={} \
             arrow_functions={} cached={}",
            round + 1,
            sample.live_bytes,
            sample.objects,
            sample.environments,
            sample.functions,
            sample.arrow_functions,
            sample.cached
        );
        assert!(
            sample.cached <= MAX_CACHED,
            "round {}: {} callable objects cached (limit {MAX_CACHED})",
            round + 1,
            sample.cached
        );
        samples.push(sample);
    }

    let split = samples.len() / 2;
    let peak = |window: &[Sample]| window.iter().map(|sample| sample.live_bytes).max().unwrap();
    let warmup_peak = peak(&samples[..split]);
    let steady_peak = peak(&samples[split..]);
    assert!(
        steady_peak <= warmup_peak + MAX_GROWTH_BYTES,
        "steady-state peak {steady_peak} B exceeds warm-up peak {warmup_peak} B by more than \
         {MAX_GROWTH_BYTES} B; closure scopes are being retained"
    );

    for (window, label) in [
        (&samples[..split], "warm-up"),
        (&samples[split..], "steady-state"),
    ] {
        let first = window.first().unwrap();
        let last = window.last().unwrap();
        for (name, start, end) in [
            ("objects", first.objects, last.objects),
            ("environments", first.environments, last.environments),
            ("functions", first.functions, last.functions),
            (
                "arrow_functions",
                first.arrow_functions,
                last.arrow_functions,
            ),
        ] {
            assert!(
                end <= start + MAX_GROWTH_COUNT,
                "{label} {name} grew from {start} to {end} over {} rounds \
                 ({UNITS_PER_ROUND} units each); each call leaks its scope",
                window.len() - 1
            );
        }
    }

    // Settled promises must drop their reaction handlers and child promises.
    let reactions = engine
        .compile(&settled_promise_reactions())
        .expect("reaction workload must compile");
    engine
        .execute(&reactions)
        .expect("reaction workload must run");
    // One drain stops after its batch cap; keep going until the queue is empty.
    for _ in 0..64 {
        if !engine.has_pending_jobs() {
            break;
        }
        engine.run_jobs().expect("jobs must run");
    }
    let after_reactions = measure(&mut engine);
    let before_reactions = samples.last().unwrap();
    println!(
        "after settled reactions: live {} B (before {} B) objects={} (before {})",
        after_reactions.live_bytes,
        before_reactions.live_bytes,
        after_reactions.objects,
        before_reactions.objects
    );
    assert!(
        after_reactions.objects <= before_reactions.objects + MAX_GROWTH_COUNT,
        "settling promises retained their reactions: objects {} -> {}",
        before_reactions.objects,
        after_reactions.objects
    );
    assert!(
        after_reactions.live_bytes <= before_reactions.live_bytes + MAX_GROWTH_BYTES,
        "settling promises retained their handlers: live {} B -> {} B",
        before_reactions.live_bytes,
        after_reactions.live_bytes
    );
    assert!(
        after_reactions.environments <= before_reactions.environments + MAX_GROWTH_COUNT,
        "settling promises retained handler scopes: environments {} -> {}",
        before_reactions.environments,
        after_reactions.environments
    );
}
