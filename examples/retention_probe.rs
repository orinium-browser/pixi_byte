//! Heap-retention probe for the JS engine.
//!
//! Runs one representative workload scenario per invocation in rounds and
//! prints live bytes plus VM bookkeeping between rounds, then reports what is
//! left after dropping the engine. Used to attribute linear growth to a
//! specific retention path.
//!
//! ```sh
//! cargo run --profile dhat --features dhat-heap --example retention_probe -- fnprops
//! ```

use pixi_byte::JSEngine;

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

/// One scenario is `(name, source)`. Each round executes the source once.
fn scenarios() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "fnprops",
            r#"
            var sink = [];
            for (var i = 0; i < 200; i++) {
              var f = function named(a, b) { return a + b; };
              f.extra = i;
              f.counter = function () { return i; };
              sink.push(f(i, 1));
            }
            sink.length = 0;
            "#,
        ),
        (
            "fnpropsnested",
            r#"
            (function () {
              var sink = [];
              for (var i = 0; i < 200; i++) {
                var f = function named(a, b) { return a + b; };
                f.extra = i;
                sink.push(f(i, 1));
              }
            })();
            "#,
        ),
        (
            "fnread",
            r#"
            var sink = [];
            for (var i = 0; i < 200; i++) {
              var f = function named(a, b) { return a + b; };
              sink.push(f.name + f.length);
              f(i, 1);
            }
            sink.length = 0;
            "#,
        ),
        (
            "fnproto",
            r#"
            var sink = [];
            for (var i = 0; i < 200; i++) {
              var f = function named() { return 1; };
              sink.push(f.prototype);
              f.prototype.marker = i;
            }
            sink.length = 0;
            "#,
        ),
        (
            "closures",
            r#"
            var sink = [];
            for (var i = 0; i < 200; i++) {
              var makeCounter = function () { var n = 0; return function () { return ++n; }; };
              var counter = makeCounter();
              sink.push(counter());
            }
            sink.length = 0;
            "#,
        ),
        (
            "bind",
            r#"
            var sink = [];
            for (var i = 0; i < 200; i++) {
              var f = function named(a) { return a; };
              var g = f.bind(null, i);
              sink.push(g(1));
              sink.push(f.call(null, i));
              sink.push(f.apply(null, [i]));
            }
            sink.length = 0;
            "#,
        ),
        (
            "promise",
            r#"
            var sink = [];
            for (var i = 0; i < 200; i++) {
              var p = new Promise(function (resolve) { resolve(i); });
              p.then(function (v) { return v + 1; });
            }
            sink.length = 0;
            "#,
        ),
        (
            "promisechain",
            r#"
            (function () {
              var p = Promise.resolve(0);
              for (var i = 0; i < 200; i++) {
                p = p.then(function (v) { return v + 1; });
              }
            })();
            "#,
        ),
        (
            "promiseretry",
            r#"
            (function () {
              function step(v) {
                if (v >= 200) { return v; }
                return Promise.resolve(v + 1).then(step);
              }
              step(0);
            })();
            "#,
        ),
        (
            "promisethens",
            r#"
            (function () {
              for (var i = 0; i < 200; i++) {
                var p = Promise.resolve(i);
                p.then(function (v) { return v; }).then(function (v) { return v; });
              }
            })();
            "#,
        ),
        (
            "promisefin",
            r#"
            (function () {
              for (var i = 0; i < 200; i++) {
                var p = Promise.resolve(i);
                p.finally(function () { return i; });
                p.catch(function () { return i; });
              }
            })();
            "#,
        ),
        (
            "promisepending",
            r#"
            var sink = [];
            for (var i = 0; i < 200; i++) {
              var p = new Promise(function (resolve) { sink.push(resolve); });
              p.then(function (v) { return v; });
              sink.length = 0;
            }
            "#,
        ),
        (
            // A helper creates a never-settling promise while the caller
            // decorates two throwaway functions. The sink is cleared every
            // round, so nothing should survive.
            "decoratedclosures",
            r#"
            function makePromise() {
              return new Promise(function () {});
            }
            function unit() {
              var a = function () { return 1; };
              a.tag = 1;
              var b = function () { return 2; };
              b.tag = 2;
              return makePromise();
            }
            (function () {
              var sink = [];
              for (var i = 0; i < 200; i++) { sink.push(unit()); }
              sink.length = 0;
            })();
            "#,
        ),
        (
            "arrays",
            r#"
            var sink = [];
            for (var i = 0; i < 200; i++) {
              var arr = [1, 2, 3, i];
              arr.push(i);
              arr.map(function (v) { return v * 2; });
              sink.push(arr.length);
            }
            sink.length = 0;
            "#,
        ),
        (
            "objects",
            r#"
            var sink = [];
            for (var i = 0; i < 200; i++) {
              var o = { a: 1, b: "x", c: i };
              o.d = i;
              sink.push(o.a + o.d);
            }
            sink.length = 0;
            "#,
        ),
    ]
}

fn report(label: &str, engine: &JSEngine) {
    let d = engine.diagnostics();
    #[cfg(feature = "dhat-heap")]
    {
        let stats = dhat::HeapStats::get();
        println!(
            "{label}: live {} B / {} blocks | callable_objects={} jobs={} frames={} stack={} | {}",
            stats.curr_bytes,
            stats.curr_blocks,
            d.callable_objects,
            d.pending_jobs,
            d.frames,
            d.stack,
            pixi_byte::live_census()
        );
    }
    #[cfg(not(feature = "dhat-heap"))]
    {
        let _ = label;
        println!(
            "{label}: callable_objects={} jobs={} frames={} stack={}",
            d.callable_objects, d.pending_jobs, d.frames, d.stack
        );
    }
}

fn main() {
    #[cfg(feature = "dhat-heap")]
    let profiler = dhat::Profiler::new_heap();

    let selected = std::env::args().nth(1).filter(|name| !name.is_empty());
    let rounds: usize = std::env::args()
        .nth(2)
        .and_then(|r| r.parse().ok())
        .unwrap_or(4);

    for (name, source) in scenarios() {
        if let Some(selected) = &selected
            && selected != name
        {
            continue;
        }
        println!("=== {name} ===");
        let mut engine = JSEngine::new();
        report("baseline", &engine);
        let chunk = engine.compile(source).expect("scenario must compile");
        for round in 1..=rounds {
            engine.execute(&chunk).expect("scenario must run");
            engine.run_jobs().expect("jobs must run");
            report(&format!("round {round}"), &engine);
        }
        drop(engine);
        #[cfg(feature = "dhat-heap")]
        {
            let stats = dhat::HeapStats::get();
            println!(
                "after drop(engine): live {} B / {} blocks",
                stats.curr_bytes, stats.curr_blocks
            );
        }
    }

    #[cfg(feature = "dhat-heap")]
    drop(profiler);
}
