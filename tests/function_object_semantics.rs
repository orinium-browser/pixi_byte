//! Semantics of function-object materialization across cache reclamation.
//!
//! The VM periodically drops cached callable objects that nothing but the cache
//! still references (see `callable_object_retention.rs`). These tests pin the
//! other side of that behaviour: every object JavaScript can still reach must
//! keep its own properties, its `prototype`, `prototype.constructor` identity
//! and its function value.

use pixi_byte::JSEngine;

/// Runs `source` and returns the string form of its value.
fn eval(engine: &mut JSEngine, source: &str) -> String {
    let chunk = engine.compile(source).expect("source must compile");
    engine.execute(&chunk).expect("source must run").to_string()
}

/// Materializes enough throwaway function objects to trigger several
/// reclamation sweeps, without touching the values under test.
fn force_sweeps(engine: &mut JSEngine) {
    let source = r#"
        (function () {
          for (var i = 0; i < 1500; i++) {
            var f = function throwaway(a) { return a; };
            f.tag = i;
            f(i);
          }
        })();
    "#;
    for _ in 0..3 {
        eval(engine, source);
    }
}

#[test]
fn reachable_function_properties_survive_sweeps() {
    let mut engine = JSEngine::new();
    eval(
        &mut engine,
        r#"
        var kept = function keptName(a) { return a + 1; };
        kept.marker = 41;
        kept.prototype.formarker = "proto";
        Math.floor.tag = "native";
        "#,
    );

    force_sweeps(&mut engine);

    let result = eval(
        &mut engine,
        r#"
        kept.marker + "|" + kept(1) + "|" + kept.prototype.formarker + "|" +
          (kept.prototype.constructor === kept) + "|" + Math.floor.tag + "|" +
          kept.name + "|" + kept.length
        "#,
    );
    assert_eq!(result, "41|2|proto|true|native|keptName|1");
}

#[test]
fn prototype_reachable_after_dropping_the_function_keeps_the_entry() {
    let mut engine = JSEngine::new();
    eval(
        &mut engine,
        r#"
        // Only the prototype object survives the IIFE: `fn` itself is gone, but
        // `proto.constructor` must still be the same function value, with the
        // properties written through both paths intact.
        var proto = (function () {
          var fn = function probe() {};
          fn.marker = "from-function";
          return fn.prototype;
        })();
        proto.constructor.note = "via-prototype";
        "#,
    );

    force_sweeps(&mut engine);

    let result = eval(
        &mut engine,
        r#"
        proto.constructor.note + "|" + proto.constructor.marker + "|" +
          (proto.constructor === proto.constructor) + "|" + (proto.constructor.prototype === proto)
        "#,
    );
    assert_eq!(result, "via-prototype|from-function|true|true");
}

/// Every closure that escapes a call must keep the scope it captured, even
/// though the VM reclaims scopes whose closures do not escape.
#[test]
fn escaping_closures_keep_their_scope() {
    let mut engine = JSEngine::new();
    let result = eval(
        &mut engine,
        r#"
        function makeCounter() {
          var count = 0;
          var step = function () { count = count + 1; return count; };
          var read = function () { return count; };
          return { step: step, read: read };
        }
        var c = makeCounter();
        c.step(); c.step();
        var d = makeCounter();
        d.step();
        c.read() + "|" + d.read() + "|" + c.step()
        "#,
    );
    assert_eq!(result, "2|1|3");
}

/// A named function expression stores itself in its own scope: the recursive
/// call must still resolve after the creating call returns.
#[test]
fn escaping_named_function_expression_stays_recursive() {
    let mut engine = JSEngine::new();
    let result = eval(
        &mut engine,
        r#"
        function make() {
          return function fact(n) { return n <= 1 ? 1 : n * fact(n - 1); };
        }
        var f = make();
        f.marker = "kept";
        f(5) + "|" + f.marker + "|" + f.name
        "#,
    );
    assert_eq!(result, "120|kept|fact");
}

/// An escaping `arguments` object keeps the scope it was created in.
#[test]
fn escaping_arguments_keep_their_scope() {
    let mut engine = JSEngine::new();
    let result = eval(
        &mut engine,
        r#"
        function capture(a, b) {
          var args = arguments;
          return function () { return args[0] + args[1] + a + b; };
        }
        capture(1, 2)()
        "#,
    );
    assert_eq!(result, "6");
}

/// An instance escaping a constructor call keeps the prototype object, and
/// through it the constructor's own properties.
#[test]
fn escaping_instance_keeps_constructor_properties() {
    let mut engine = JSEngine::new();
    let result = eval(
        &mut engine,
        r#"
        function make() {
          var Ctor = function Ctor(v) { this.v = v; };
          Ctor.marker = "from-function";
          Ctor.prototype.kind = "proto";
          return new Ctor(7);
        }
        var instance = make();
        instance.v + "|" + instance.kind + "|" + instance.constructor.marker
        "#,
    );
    assert_eq!(result, "7|proto|from-function");
}

#[test]
fn function_objects_are_per_instance() {
    let mut engine = JSEngine::new();
    let result = eval(
        &mut engine,
        r#"
        function make() { return function inner() { return 1; }; }
        var a = make();
        var b = make();
        a.marker = "a";
        b.marker = "b";
        (a.marker + b.marker) + "|" + (a === b) + "|" + (a.prototype === b.prototype)
        "#,
    );
    assert_eq!(result, "ab|false|false");
}

#[test]
fn decorated_function_survives_when_stored_in_an_object() {
    let mut engine = JSEngine::new();
    eval(
        &mut engine,
        r#"
        var registry = {};
        (function () {
          for (var i = 0; i < 10; i++) {
            var handler = function handlerFn() { return i; };
            handler.id = i;
            registry["h" + i] = handler;
          }
        })();
        "#,
    );

    force_sweeps(&mut engine);

    let result = eval(
        &mut engine,
        r#"
        var sum = 0;
        for (var key in registry) { sum += registry[key].id; }
        sum + "|" + registry.h3.id + "|" + (registry.h3 === registry.h3)
        "#,
    );
    assert_eq!(result, "45|3|true");
}
