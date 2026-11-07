use pixi_byte::{JSEngine, JSValue};

/// Verifies the host can register an eval'd function expression on the global
/// object and invoke it from Rust.
fn rust_invokes_js_function_from_global() {
    let mut engine = JSEngine::new();

    let add = engine
        .eval("(function add(a, b) { return a + b; })")
        .unwrap();
    engine
        .global_mut()
        .borrow_mut()
        .set("add".to_string(), add.clone());

    let result = engine
        .call(
            add,
            JSValue::undefined(),
            vec![JSValue::from_number(1.0), JSValue::from_number(2.0)],
        )
        .unwrap();

    assert_eq!(result, JSValue::from_number(3.0));
}

fn rust_invokes_js_function_returning_object() {
    let mut engine = JSEngine::new();

    let make = engine
        .eval("(function make() { return { x: 10 }; })")
        .unwrap();

    let result = engine.call(make, JSValue::undefined(), Vec::new()).unwrap();

    if let Some(obj_ref) = result.as_object() {
        assert_eq!(obj_ref.borrow().get("x"), JSValue::from_number(10.0));
    } else {
        panic!("expected object, got {:?}", result);
    }
}

fn rust_invokes_function_with_this() {
    let mut engine = JSEngine::new();

    let get_x = engine.eval("(function getX() { return this.x; })").unwrap();

    let mut this_obj = pixi_byte::value::jsobject::JSObject::new();
    this_obj.set("x".to_string(), JSValue::from_number(7.0));
    let this_rc = std::rc::Rc::new(std::cell::RefCell::new(this_obj));

    let result = engine
        .call(get_x, JSValue::from_object(this_rc), Vec::new())
        .unwrap();

    assert_eq!(result, JSValue::from_number(7.0));
}

fn rust_invokes_native_function() {
    let mut engine = JSEngine::new();

    // natives receive `[this, ...args]`, so skip the first element when summing
    let native = JSValue::from_native_function(|_vm, args| {
        let sum: f64 = args.iter().skip(1).map(JSValue::to_number).sum();
        Ok(JSValue::from_number(sum))
    });

    engine
        .global_mut()
        .borrow_mut()
        .set("sumNums".to_string(), native);

    let sum_nums = engine.global_mut().borrow().get("sumNums");

    let result = engine
        .call(
            sum_nums,
            JSValue::undefined(),
            vec![
                JSValue::from_number(1.0),
                JSValue::from_number(2.0),
                JSValue::from_number(3.0),
            ],
        )
        .unwrap();

    assert_eq!(result, JSValue::from_number(6.0));
}

fn call_with_non_function_returns_error() {
    let mut engine = JSEngine::new();

    let result = engine.call(JSValue::from_number(42.0), JSValue::undefined(), Vec::new());

    assert!(result.is_err());
}

fn rest_parameters_collect_remaining_arguments() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function collect(first, ...rest) { return first + ":" + rest.join(","); }
            const arrow = (...values) => values.length;
            collect("a", "b", "c") === "a:b,c" && arrow(1, 2, 3) === 3;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn function_calls_expand_spread_arguments() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                function add(a, b, c) { return a + b + c; }
                const middle = [2];
                add(1, ...middle, 3);
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_number(6.0));
}

fn method_calls_expand_spread_arguments_and_preserve_receiver() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const target = {
                    base: 1,
                    add(a, b) { return this.base + a + b; }
                };
                target.add(...[2, 3]);
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_number(6.0));
}

fn test_simple_closure() {
    let mut engine = JSEngine::new();
    let src = r#"
        function makeAdder(x) {
            return function(y) {
                return x + y;
            };
        }
        var add5 = makeAdder(5);
        add5(3);
    "#;

    let res = engine.eval(src).expect("eval failed");
    match res.as_number() {
        Some(n) => assert_eq!(n, 8.0),
        _ => panic!("unexpected result: {}", res),
    }
}

fn test_closure_capture_after_mutation() {
    let mut engine = JSEngine::new();
    let src = r#"
        function counter() {
            var c = 0;
            return function() {
                c = c + 1;
                return c;
            };
        }
        var inc = counter();
        inc();
        inc();
    "#;

    let res = engine.eval(src).expect("eval failed");
    match res.as_number() {
        Some(n) => assert_eq!(n, 2.0),
        _ => panic!("unexpected result: {}", res),
    }
}

fn arrow_functions_support_expression_and_block_bodies() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const add = (a, b) => a + b;
            const double = value => { return value * 2; };
            double(add(2, 3));
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_number(10.0));
}

fn zero_parameter_arrow_captures_outer_bindings() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let value = 4;
            const read = () => value;
            value = 7;
            read();
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_number(7.0));
}

fn arrows_work_as_promise_reactions() {
    let mut engine = JSEngine::new();
    engine
        .eval(
            r#"
            let result = 0;
            Promise.resolve(20).then(value => value + 1).then(value => { result = value; });
            "#,
        )
        .unwrap();
    engine.run_jobs().unwrap();

    assert_eq!(engine.eval("result").unwrap(), JSValue::from_number(21.0));
}

fn arrow_function_keeps_lexical_this_through_call() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function makeReader() { return () => this.value; }
            const holder = { value: 42, makeReader: makeReader };
            const reader = holder.makeReader();
            reader.call({ value: 1 });
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_number(42.0));
}

fn function_declarations_are_available_before_their_source_position() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const result = later(20, 22);
            function later(left, right) {
                return left + right;
            }
            result;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(42.0));
}

fn function_body_declarations_are_hoisted_per_call() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function outer() {
                return inner();
                function inner() {
                    return "ready";
                }
            }
            outer();
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("ready".to_string()));
}

fn function_clones_retain_reference_identity() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const original = function () { return 1; };
            const alias = original;
            const distinct = function () { return 1; };
            original === alias && original !== distinct;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn bound_function_clones_retain_reference_identity() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function target() {}
            const first = target.bind(null);
            const alias = first;
            const second = target.bind(null);
            first === alias && first !== second;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn repeated_closure_creation_produces_distinct_identities() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function create() {
                return function () {};
            }
            const first = create();
            const second = create();
            first !== second;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn assigning_an_array_index_grows_length() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const queue = [];
            queue[0] = "entry";
            queue[2] = "later";
            queue.length;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(3.0));
}

fn object_keys_enumerates_function_properties() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const hooks = () => {};
            hooks.j = id => id === 2076;
            const keys = Object.keys(hooks);
            keys.length === 1 && keys[0] === "j";
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn minified_webpack_deferred_entry_runs() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let called = 0;
            const runtime = {};
            var queue = [];
            runtime.O = (result, ids, callback, priority) => {
                if (!ids) {
                    var ceiling = 1 / 0;
                    for (outer = 0; outer < queue.length; outer++) {
                        for (var [ids, callback, priority] = queue[outer], ready = true, inner = 0;
                             inner < ids.length;
                             inner++)
                            (!1 & priority || ceiling >= priority) &&
                                Object.keys(runtime.O).every(key => runtime.O[key](ids[inner]))
                                ? ids.splice(inner--, 1)
                                : ready = false;
                        if (ready) {
                            queue.splice(outer--, 1);
                            callback();
                        }
                    }
                    return result;
                }
                priority = priority || 0;
                for (var outer = queue.length;
                     outer > 0 && queue[outer - 1][2] > priority;
                     outer--)
                    queue[outer] = queue[outer - 1];
                queue[outer] = [ids, callback, priority];
            };
            runtime.O.j = id => id === 2076;
            runtime.O(undefined, [2076], () => { called = 1; });
            runtime.O();
            called;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(1.0));
}

fn function_constructor_is_callable_and_inherits_function_methods() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const has = Function.call.bind(Object.prototype.hasOwnProperty);
            typeof Function === "function" &&
                typeof Function("return 1") === "function" &&
                has({answer: 42}, "answer");
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn bound_functions_can_store_own_properties() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function checker(required) { return required; }
            const optional = checker.bind(null, false);
            optional.isRequired = checker.bind(null, true);
            optional() === false && optional.isRequired() === true;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn functions_inherit_object_prototype_methods() {
    let mut engine = JSEngine::new();
    assert_eq!(
        engine
            .eval("(function example() {}).propertyIsEnumerable('prototype')")
            .unwrap(),
        JSValue::from_bool(false)
    );
}

fn functions_keep_own_properties_and_constructor_prototypes() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function Box(value) {
                this.value = value;
            }
            Box.category = "container";
            Box.prototype.kind = "box";
            const instance = new Box(42);
            Box.category + ":" + instance.kind + ":" + instance.value + ":" +
                (instance.constructor === Box) + ":" + (instance instanceof Box);
            "#,
        )
        .unwrap();
    assert_eq!(
        result,
        JSValue::from_string("container:box:42:true:true".to_string())
    );
}

fn prototype_assignment_supports_react_style_inheritance_setup() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function BasePrototype() {
                this.base = "ready";
            }
            function Component() {}
            function PureComponent() {}
            PureComponent.prototype = Component.prototype = new BasePrototype();
            PureComponent.prototype.constructor = PureComponent;
            const instance = new PureComponent();
            instance.base + ":" + (instance.constructor === PureComponent);
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("ready:true".to_string()));
}

fn closure_own_properties_are_isolated() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function create() { return function () {}; }
            const first = create();
            const second = create();
            first.marker = "first";
            first.marker + ":" + (second.marker === undefined);
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("first:true".to_string()));
}

fn chained_prototype_method_assignment_updates_each_constructor() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function First() {}
            function Second() {}
            Second.prototype.render = First.prototype.render = function () {
                return "ready";
            };
            const first = new First();
            const second = new Second();
            first.render() + ":" + second.render();
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("ready:ready".to_string()));
}

fn nested_constructor_keeps_its_assigned_prototype_methods() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function createInstance() {
                function Constructor(value) { this.value = value; }
                Constructor.prototype.render = function () { return this.value; };
                return new Constructor("ready");
            }
            createInstance().render();
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("ready".to_string()));
}

fn callable_builtin_objects_inherit_function_prototype_methods() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            String.call(null, 42) + ":" + Number.call(null, "3");
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("42:3".to_string()));
}

fn test_simple_function() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
        function f() { return 5; }
        f();
    "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(5.0));
}

fn test_function_with_args() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
        function add(a, b) { return a + b; }
        add(2, 3);
    "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(5.0));
}

fn function_without_return_produces_undefined() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval("function update() { const value = {}; value.current = {}; } update();")
        .unwrap();
    assert_eq!(result, JSValue::undefined());
}

fn parameter_shadows_the_function_name() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval("(function value(value) { return value; })(42);")
        .unwrap();
    assert_eq!(result, JSValue::from_number(42.0));
}

fn local_declaration_does_not_overwrite_an_outer_binding() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            var value = 1;
            function read() {
                var value = 2;
                return value;
            }
            read() + ":" + value;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("2:1".to_string()));
}

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

fn functions_expose_their_arguments_by_index_and_length() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function inspect() {
                return arguments[0] + arguments[1] + arguments.length;
            }
            inspect(10, 20);
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_number(32.0));
}

fn array_slice_can_copy_an_arguments_object() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function collect() {
                return Array.prototype.slice.call(arguments, 1).join("-");
            }
            collect("skip", "a", "b");
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_string("a-b".to_string()));
}

fn arrow_functions_use_the_enclosing_arguments_binding() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function outer(value) {
                const read = () => arguments[0];
                return read("ignored");
            }
            outer("outer");
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_string("outer".to_string()));
}

#[test]
fn suite_call_1() {
    rust_invokes_js_function_from_global();
    rust_invokes_js_function_returning_object();
    rust_invokes_function_with_this();
    rust_invokes_native_function();
}

#[test]
fn suite_call_2() {
    call_with_non_function_returns_error();
}

#[test]
fn suite_call_spread() {
    rest_parameters_collect_remaining_arguments();
    function_calls_expand_spread_arguments();
    method_calls_expand_spread_arguments_and_preserve_receiver();
}

#[test]
fn suite_closure() {
    test_simple_closure();
    test_closure_capture_after_mutation();
}

#[test]
fn suite_arrow_function() {
    arrow_functions_support_expression_and_block_bodies();
    zero_parameter_arrow_captures_outer_bindings();
    arrows_work_as_promise_reactions();
    arrow_function_keeps_lexical_this_through_call();
}

#[test]
fn suite_function_hoisting() {
    function_declarations_are_available_before_their_source_position();
    function_body_declarations_are_hoisted_per_call();
}

#[test]
fn suite_function_identity_1() {
    function_clones_retain_reference_identity();
    bound_function_clones_retain_reference_identity();
    repeated_closure_creation_produces_distinct_identities();
    assigning_an_array_index_grows_length();
}

#[test]
fn suite_function_identity_2() {
    object_keys_enumerates_function_properties();
    minified_webpack_deferred_entry_runs();
}

#[test]
fn suite_function_properties_1() {
    function_constructor_is_callable_and_inherits_function_methods();
    bound_functions_can_store_own_properties();
    functions_inherit_object_prototype_methods();
    functions_keep_own_properties_and_constructor_prototypes();
}

#[test]
fn suite_function_properties_2() {
    prototype_assignment_supports_react_style_inheritance_setup();
    closure_own_properties_are_isolated();
    chained_prototype_method_assignment_updates_each_constructor();
    nested_constructor_keeps_its_assigned_prototype_methods();
}

#[test]
fn suite_function_properties_3() {
    callable_builtin_objects_inherit_function_prototype_methods();
}

#[test]
fn suite_function_tests_1() {
    test_simple_function();
    test_function_with_args();
    function_without_return_produces_undefined();
    parameter_shadows_the_function_name();
}

#[test]
fn suite_function_tests_2() {
    local_declaration_does_not_overwrite_an_outer_binding();
}

#[test]
fn suite_function_object_semantics_1() {
    reachable_function_properties_survive_sweeps();
    prototype_reachable_after_dropping_the_function_keeps_the_entry();
    escaping_closures_keep_their_scope();
    escaping_named_function_expression_stays_recursive();
}

#[test]
fn suite_function_object_semantics_2() {
    escaping_arguments_keep_their_scope();
    escaping_instance_keeps_constructor_properties();
    function_objects_are_per_instance();
    decorated_function_survives_when_stored_in_an_object();
}

#[test]
fn suite_arguments_object() {
    functions_expose_their_arguments_by_index_and_length();
    array_slice_can_copy_an_arguments_object();
    arrow_functions_use_the_enclosing_arguments_binding();
}
