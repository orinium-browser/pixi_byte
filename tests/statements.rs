use pixi_byte::value::jsobject::{HOST_HAS_INSTANCE, JSObject};
use pixi_byte::{JSEngine, JSValue};
use std::cell::RefCell;
use std::rc::Rc;

fn executes_if_and_else_branches() {
    let mut engine = JSEngine::new();

    let truthy = engine
        .eval("let truthy = 0; if (true) { truthy = 1; } else { truthy = 2; } truthy;")
        .unwrap();
    assert_eq!(truthy.to_number(), 1.0);

    let falsy = engine
        .eval("let falsy = 0; if (false) { falsy = 1; } else { falsy = 2; } falsy;")
        .unwrap();
    assert_eq!(falsy.to_number(), 2.0);
}

fn supports_else_if_and_single_statement_bodies() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval("let value = 0; if (false) value = 1; else if (true) value = 2; value;")
        .unwrap();
    assert_eq!(result.to_number(), 2.0);
}

fn returns_from_inside_an_if_statement() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval("function choose(value) { if (value) { return 10; } return 20; } choose(true);")
        .unwrap();
    assert_eq!(result.to_number(), 10.0);
}

fn executes_while_loop_with_updates() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval("let index = 0; let total = 0; while (index < 5) { total += index; index++; } total;")
        .unwrap();
    assert_eq!(result.to_number(), 10.0);
}

fn break_exits_and_continue_restarts_the_loop() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval(
            "let index = 0; let total = 0; while (index < 10) { index++; if (index === 2) { continue; } if (index === 5) { break; } total += index; } total;",
        )
        .unwrap();
    assert_eq!(result.to_number(), 8.0);
}

fn supports_single_statement_while_body() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval("let index = 0; while (index < 3) index++; index;")
        .unwrap();
    assert_eq!(result.to_number(), 3.0);
}

fn do_while_runs_before_testing_its_condition() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let count = 0;
            do {
                count += 1;
            } while (false);
            count;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_number(1.0));
}

fn do_while_supports_break_and_continue() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let index = 0;
            let total = 0;
            do {
                index += 1;
                if (index === 2) {
                    continue;
                }
                total += index;
                if (index === 4) {
                    break;
                }
            } while (index < 10);
            total;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_number(8.0));
}

fn executes_classic_for_loop() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval("let total = 0; for (let index = 0; index < 5; index++) { total += index; } total;")
        .unwrap();
    assert_eq!(result.to_number(), 10.0);
}

fn accepts_multiple_initializer_bindings() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval("let total = 0; for (var left = 0, right = 4; left < right; left++, right--) { total += left + right; } total;")
        .unwrap();
    assert_eq!(result.to_number(), 8.0);
}

fn continue_runs_update_and_break_exits() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval(
            "let total = 0; for (let index = 0; index < 10; index++) { if (index === 1) { continue; } if (index === 4) { break; } total += index; } total;",
        )
        .unwrap();
    assert_eq!(result.to_number(), 5.0);
}

fn supports_expression_initializer_and_single_statement_body() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval("let index = 0; let total = 0; for (index = 1; index < 4; index++) total += index; total;")
        .unwrap();
    assert_eq!(result.to_number(), 6.0);
}

fn supports_an_empty_statement_as_the_loop_body() {
    let mut engine = JSEngine::new();
    let result = engine.eval("let i = 0; for (; i < 3; i++); i;").unwrap();
    assert_eq!(result.to_number(), 3.0);
}

fn for_in_enumerates_own_and_inherited_properties() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const parent = { inherited: 1 };
            const target = Object.create(parent);
            target.own = 2;
            let total = 0;
            for (const key in target) {
                total += target[key];
            }
            total;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_number(3.0));
}

fn for_in_supports_existing_bindings_break_and_continue() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const target = { first: 1, skip: 2, stop: 3, after: 4 };
            let key = "";
            let total = 0;
            for (key in target) {
                if (key === "skip") continue;
                total += target[key];
            }
            let iterations = 0;
            for (key in target) {
                iterations += 1;
                break;
            }
            total === 8 && iterations === 1;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_bool(true));
}

fn for_of_iterates_array_values() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                let joined = "";
                for (let value of ["a", "b", "c"]) {
                    joined = joined + value;
                }
                joined;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_string("abc".to_string()));
}

fn for_of_supports_var_binding_and_continue() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                let total = 0;
                for (var value of [1, 2, 3]) {
                    if (value == 2) continue;
                    total = total + value;
                }
                value + total;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_number(7.0));
}

fn for_of_let_closures_capture_each_iteration_binding() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const callbacks = [];
                for (let [name, value] of [["px", "horizontal"], ["py", "vertical"]]) {
                    callbacks.push(() => name + ":" + value);
                }
                callbacks[0]() + "," + callbacks[1]();
            "#,
        )
        .unwrap();

    assert_eq!(
        result,
        JSValue::from_string("px:horizontal,py:vertical".to_string())
    );
}

fn for_of_let_scope_is_restored_by_continue_and_break() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const callbacks = [];
                for (let value of [1, 2, 3]) {
                    callbacks.push(() => value);
                    if (value === 1) continue;
                    if (value === 2) break;
                }
                callbacks[0]() * 10 + callbacks[1]();
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_number(12.0));
}

fn nested_callbacks_capture_bindings_from_their_for_of_iteration() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const handlers = {};
                const groups = [
                    ["p", ["padding"]],
                    [["px", ["padding-left", "padding-right"]], ["py", ["padding-top", "padding-bottom"]]],
                    [["pt", ["padding-top"]], ["pr", ["padding-right"]]]
                ];
                for (let group of groups) {
                    const entries = Array.isArray(group[0]) ? group : [group];
                    Object.assign(handlers, entries.reduce(
                        (result, [name, properties]) => Object.assign(result, {
                            [name]: value => properties.reduce(
                                (declarations, property) => Object.assign(declarations, { [property]: value }),
                                {}
                            )
                        }),
                        {}
                    ));
                }
                const result = handlers.py("0.5rem");
                result["padding-top"] + "," + result["padding-bottom"] + "," + result["padding-left"];
            "#,
        )
        .unwrap();

    assert_eq!(
        result,
        JSValue::from_string("0.5rem,0.5rem,undefined".to_string())
    );
}

fn classic_for_let_closures_capture_each_iteration_binding() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const callbacks = [];
                for (let index = 0; index < 3; index++) {
                    callbacks.push(() => index);
                }
                callbacks[0]() * 100 + callbacks[1]() * 10 + callbacks[2]();
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_number(12.0));
}

fn classic_for_let_scope_is_restored_by_break() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                let index = 9;
                for (let index = 0; index < 3; index++) {
                    if (index === 1) break;
                }
                index;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_number(9.0));
}

fn for_in_let_closures_capture_each_iteration_binding() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const callbacks = {};
                for (let name in { px: 1, py: 2 }) {
                    callbacks[name] = () => name;
                }
                callbacks.px() + "," + callbacks.py();
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_string("px,py".to_string()));
}

fn for_in_let_scope_is_restored_by_continue_and_break() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                let name = "outer";
                const callbacks = [];
                for (let name in { first: 1, second: 2, third: 3 }) {
                    callbacks.push(() => name);
                    if (callbacks.length === 1) continue;
                    break;
                }
                name + ":" + callbacks[0]() + ":" + callbacks[1]();
            "#,
        )
        .unwrap();

    let Some(value) = result.as_string() else {
        panic!("expected string");
    };
    assert!(value.starts_with("outer:"));
    let parts: Vec<_> = value.split(':').collect();
    assert_ne!(parts[1], parts[2]);
}

fn switch_matches_cases_and_falls_through() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let result = "";
            switch (2) {
                case 1:
                    result = "one";
                    break;
                case 2:
                    result = "two";
                case 3:
                    result += "-three";
                    break;
                default:
                    result = "default";
            }
            result;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_string("two-three".to_string()));
}

fn switch_uses_default_only_when_no_case_matches() {
    let mut engine = JSEngine::new();

    assert_eq!(
        engine
            .eval(
                r#"
                let result = "";
                switch (4) {
                    default:
                        result = "default";
                        break;
                    case 4:
                        result = "matched";
                }
                result;
                "#,
            )
            .unwrap(),
        JSValue::from_string("matched".to_string())
    );
}

fn switch_break_does_not_exit_an_enclosing_loop() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let iterations = 0;
            for (let index = 0; index < 3; index++) {
                switch (index) {
                    case 1:
                        break;
                    default:
                        iterations += 1;
                }
                iterations += 10;
            }
            iterations;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_number(32.0));
}

fn labeled_break_exits_blocks_and_switches() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let value = 0;
            outer: {
                value = 1;
                switch (value) {
                    case 1:
                        value = 2;
                        break outer;
                    default:
                        value = 3;
                }
                value = 4;
            }
            value;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(2.0));
}

fn labeled_continue_targets_the_named_loop() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let count = 0;
            outer: for (let row = 0; row < 3; row++) {
                for (let column = 0; column < 3; column++) {
                    if (column === 1) continue outer;
                    count++;
                }
            }
            count;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(3.0));
}

fn labeled_break_unwinds_the_target_loop_lexical_environment() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let value = "outer";
            outer: for (let value = 0; value < 1; value++) {
                break outer;
            }
            value;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("outer".to_string()));
}

fn labeled_continue_unwinds_inner_lexical_environments() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let value = "outer";
            outer: for (let row = 0; row < 1; row++) {
                for (let value of [1]) {
                    continue outer;
                }
            }
            value;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("outer".to_string()));
}

fn catch_receives_the_thrown_value() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let result = "";
            try {
                throw "boom";
            } catch (error) {
                result = error;
            }
            result;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_string("boom".to_string()));
}

fn finally_runs_after_normal_and_exceptional_completion() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let trace = "";
            try {
                trace += "try";
                throw "boom";
            } catch (error) {
                trace += "-catch";
            } finally {
                trace += "-finally";
            }
            try {
                trace += "-normal";
            } finally {
                trace += "-finally";
            }
            trace;
            "#,
        )
        .unwrap();

    assert_eq!(
        result,
        JSValue::from_string("try-catch-finally-normal-finally".to_string())
    );
}

fn finally_rethrows_an_unhandled_value() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let finalized = false;
            let caught = "";
            try {
                try {
                    throw "boom";
                } finally {
                    finalized = true;
                }
            } catch (error) {
                caught = error;
            }
            finalized && caught === "boom";
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_bool(true));
}

fn catch_binding_is_optional() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let caught = false;
            try {
                throw 1;
            } catch {
                caught = true;
            }
            caught;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_bool(true));
}

fn catch_binding_does_not_overwrite_an_outer_binding() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function preserve(value) {
                try {
                    throw "temporary";
                } catch (value) {}
                return value;
            }
            preserve("outer");
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_string("outer".to_string()));
}

fn exception_unwinds_loop_lexical_environment_before_catch() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let value = "outer";
            try {
                for (let value of [1]) {
                    throw "stop";
                }
            } catch (error) {}
            value;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_string("outer".to_string()));
}

fn return_runs_finally_before_leaving_the_function() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let trace = "";
            function run() {
                try {
                    trace += "try";
                    return 42;
                } finally {
                    trace += "-finally";
                }
            }
            const value = run();
            trace + ":" + value;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("try-finally:42".to_string()));
}

fn return_unwinds_nested_finally_blocks() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let trace = "";
            function run() {
                try {
                    try {
                        return "done";
                    } finally {
                        trace += "inner";
                    }
                } finally {
                    trace += "-outer";
                }
            }
            run() + ":" + trace;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("done:inner-outer".to_string()));
}

fn return_inside_finally_overrides_an_earlier_return() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function run() {
                try {
                    return "try";
                } finally {
                    return "finally";
                }
            }
            run();
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("finally".to_string()));
}

fn declares_multiple_bindings_in_one_statement() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval("var first = 1, second = first + 2, third; third = 4; first + second + third;")
        .unwrap();
    assert_eq!(result.to_number(), 8.0);
}

fn supports_multiple_let_and_const_bindings() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval("let first = 2, second = 3; const third = 4, fourth = 5; first + second + third + fourth;")
        .unwrap();
    assert_eq!(result.to_number(), 14.0);
}

fn sequence_expression_evaluates_left_to_right_and_returns_the_last_value() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let value = 0;
            const result = (value = 1, value += 2, value * 4);
            result;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_number(12.0));
}

fn commas_still_separate_function_arguments_and_array_elements() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function combine(first, second) {
                return first * 10 + second;
            }
            const values = [2, 3];
            combine(values[0], values[1]);
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_number(23.0));
}

fn evaluates_only_the_selected_conditional_branch() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval("let side = 0; let value = true ? (side = 1) : (side = 2); value * 10 + side;")
        .unwrap();
    assert_eq!(result.to_number(), 11.0);
}

fn conditional_expression_is_right_associative() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval("let first = false, second = true; first ? 1 : second ? 2 : 3;")
        .unwrap();
    assert_eq!(result.to_number(), 2.0);
}

fn conditional_expression_can_feed_an_assignment() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval("let enabled = false; let value = enabled ? 'on' : 'off'; value;")
        .unwrap();
    assert_eq!(result.to_string(), "off");
}

fn logical_and_skips_a_falsy_left_hand_side() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let calls = 0;
            function hit() {
                calls += 1;
                return true;
            }
            false && hit();
            calls;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_number(0.0));
}

fn logical_or_skips_a_truthy_left_hand_side() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let calls = 0;
            function hit() {
                calls += 1;
                return false;
            }
            true || hit();
            calls;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_number(0.0));
}

fn logical_expressions_preserve_operand_values() {
    let mut engine = JSEngine::new();

    assert_eq!(
        engine.eval(r#"0 || "fallback""#).unwrap(),
        JSValue::from_string("fallback".to_string())
    );
    assert_eq!(
        engine.eval(r#""left" && "right""#).unwrap(),
        JSValue::from_string("right".to_string())
    );
}

fn bitwise_and_shift_expressions_follow_javascript_precedence() {
    let mut engine = JSEngine::new();
    for source in [
        "(12 & 10) === 8",
        "(12 | 3) === 15",
        "(12 ^ 10) === 6",
        "(1 << 4) === 16",
        "(16 >> 2) === 4",
        "(-1 >>> 1) === 2147483647",
        "(1 | 2 & 4) === 1",
    ] {
        assert_eq!(
            engine.eval(source).unwrap(),
            JSValue::from_bool(true),
            "failed expression: {source}"
        );
    }
}

fn exponentiation_is_right_associative() {
    let mut engine = JSEngine::new();
    assert_eq!(
        engine.eval("2 ** 3 ** 2").unwrap(),
        JSValue::from_number(512.0)
    );
}

fn bitwise_compound_assignments_update_identifiers() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let value = 5;
            value |= 2;
            value &= 6;
            value ^= 3;
            value <<= 2;
            value >>= 1;
            value >>>= 1;
            value;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(5.0));
}

fn bitwise_compound_assignments_update_members() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const state = { flags: 1 };
            state.flags |= 4;
            state.flags &= 5;
            state.flags;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(5.0));
}

fn prefix_and_postfix_updates_preserve_their_result_values() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval("let value = 1; let old = value++; let current = ++value; old * 100 + current;")
        .unwrap();
    assert_eq!(result.to_number(), 103.0);
}

fn decrement_updates_identifier_bindings() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval("let value = 3; value--; --value; value;")
        .unwrap();
    assert_eq!(result.to_number(), 1.0);
}

fn compound_assignments_use_the_corresponding_operator() {
    let mut engine = JSEngine::new();

    let result = engine
        .eval("let value = 5; value += 3; value *= 2; value -= 4; value /= 2; value %= 5; value;")
        .unwrap();
    assert_eq!(result.to_number(), 1.0);
}

fn member_updates_preserve_prefix_and_postfix_results() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const target = { count: 1 };
            const old = target.count++;
            const current = ++target.count;
            old === 1 && current === 3 && target.count === 3;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_bool(true));
}

fn member_assignment_returns_the_assigned_value() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const target = {};
            const assigned = (target.value = 42);
            assigned === 42 && target.value === 42;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_bool(true));
}

fn delete_removes_an_own_property() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const target = { keep: 1, remove: 2 };
            const deleted = delete target.remove;
            deleted && !("remove" in target) && target.keep === 1;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_bool(true));
}

fn deleting_a_missing_or_inherited_property_succeeds() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const parent = { inherited: true };
            const child = Object.create(parent);
            (delete child.missing) && (delete child.inherited) && ("inherited" in child);
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_bool(true));
}

fn deleting_an_identifier_does_not_remove_its_binding() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let value = 3;
            const deleted = delete value;
            !deleted && value === 3;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_bool(true));
}

fn in_operator_checks_own_and_inherited_properties() {
    let mut engine = JSEngine::new();

    assert_eq!(
        engine.eval(r#""own" in { own: 1 }"#).unwrap(),
        JSValue::from_bool(true)
    );
    assert_eq!(
        engine.eval(r#""missing" in { own: 1 }"#).unwrap(),
        JSValue::from_bool(false)
    );
    assert_eq!(
        engine
            .eval(
                r#"
                const parent = { inherited: true };
                const child = Object.create(parent);
                "inherited" in child;
                "#,
            )
            .unwrap(),
        JSValue::from_bool(true)
    );
}

fn in_operator_rejects_a_primitive_right_hand_side() {
    let mut engine = JSEngine::new();
    assert!(engine.eval(r#""length" in 1"#).is_err());
}

fn instanceof_checks_the_constructor_prototype_chain() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const prototype = {};
            const Constructor = { prototype: prototype };
            const instance = Object.create(prototype);
            instance instanceof Constructor;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_bool(true));
}

fn instanceof_supports_host_constructor_checks() {
    let mut engine = JSEngine::new();
    let mut constructor = JSObject::new();
    constructor.set(
        HOST_HAS_INSTANCE.to_string(),
        JSValue::from_native_function(|_vm, args| {
            let matches = match args.get(1) {
                Some(value) => value.as_number() == Some(42.0),
                _ => false,
            };
            Ok(JSValue::from_bool(matches))
        }),
    );
    engine.global_mut().borrow_mut().set(
        "Answer".to_string(),
        JSValue::from_object(Rc::new(RefCell::new(constructor))),
    );

    assert_eq!(
        engine.eval("42 instanceof Answer").unwrap(),
        JSValue::from_bool(true)
    );
    assert_eq!(
        engine.eval("41 instanceof Answer").unwrap(),
        JSValue::from_bool(false)
    );
}

#[test]
fn suite_if_statement() {
    executes_if_and_else_branches();
    supports_else_if_and_single_statement_bodies();
    returns_from_inside_an_if_statement();
}

#[test]
fn suite_while_statement() {
    executes_while_loop_with_updates();
    break_exits_and_continue_restarts_the_loop();
    supports_single_statement_while_body();
}

#[test]
fn suite_do_while_statement() {
    do_while_runs_before_testing_its_condition();
    do_while_supports_break_and_continue();
}

#[test]
fn suite_for_statement_1() {
    executes_classic_for_loop();
    accepts_multiple_initializer_bindings();
    continue_runs_update_and_break_exits();
    supports_expression_initializer_and_single_statement_body();
}

#[test]
fn suite_for_statement_2() {
    supports_an_empty_statement_as_the_loop_body();
}

#[test]
fn suite_for_in_statement() {
    for_in_enumerates_own_and_inherited_properties();
    for_in_supports_existing_bindings_break_and_continue();
}

#[test]
fn suite_for_of_statement_1() {
    for_of_iterates_array_values();
    for_of_supports_var_binding_and_continue();
    for_of_let_closures_capture_each_iteration_binding();
    for_of_let_scope_is_restored_by_continue_and_break();
}

#[test]
fn suite_for_of_statement_2() {
    nested_callbacks_capture_bindings_from_their_for_of_iteration();
    classic_for_let_closures_capture_each_iteration_binding();
    classic_for_let_scope_is_restored_by_break();
    for_in_let_closures_capture_each_iteration_binding();
}

#[test]
fn suite_for_of_statement_3() {
    for_in_let_scope_is_restored_by_continue_and_break();
}

#[test]
fn suite_switch_statement() {
    switch_matches_cases_and_falls_through();
    switch_uses_default_only_when_no_case_matches();
    switch_break_does_not_exit_an_enclosing_loop();
}

#[test]
fn suite_labeled_statements() {
    labeled_break_exits_blocks_and_switches();
    labeled_continue_targets_the_named_loop();
    labeled_break_unwinds_the_target_loop_lexical_environment();
    labeled_continue_unwinds_inner_lexical_environments();
}

#[test]
fn suite_try_statement_1() {
    catch_receives_the_thrown_value();
    finally_runs_after_normal_and_exceptional_completion();
    finally_rethrows_an_unhandled_value();
    catch_binding_is_optional();
}

#[test]
fn suite_try_statement_2() {
    catch_binding_does_not_overwrite_an_outer_binding();
    exception_unwinds_loop_lexical_environment_before_catch();
}

#[test]
fn suite_finally_completion() {
    return_runs_finally_before_leaving_the_function();
    return_unwinds_nested_finally_blocks();
    return_inside_finally_overrides_an_earlier_return();
}

#[test]
fn suite_variable_declaration() {
    declares_multiple_bindings_in_one_statement();
    supports_multiple_let_and_const_bindings();
}

#[test]
fn suite_sequence_expression() {
    sequence_expression_evaluates_left_to_right_and_returns_the_last_value();
    commas_still_separate_function_arguments_and_array_elements();
}

#[test]
fn suite_conditional_expression() {
    evaluates_only_the_selected_conditional_branch();
    conditional_expression_is_right_associative();
    conditional_expression_can_feed_an_assignment();
}

#[test]
fn suite_logical_expression() {
    logical_and_skips_a_falsy_left_hand_side();
    logical_or_skips_a_truthy_left_hand_side();
    logical_expressions_preserve_operand_values();
}

#[test]
fn suite_bitwise_expression() {
    bitwise_and_shift_expressions_follow_javascript_precedence();
    exponentiation_is_right_associative();
}

#[test]
fn suite_bitwise_assignment() {
    bitwise_compound_assignments_update_identifiers();
    bitwise_compound_assignments_update_members();
}

#[test]
fn suite_update_expression() {
    prefix_and_postfix_updates_preserve_their_result_values();
    decrement_updates_identifier_bindings();
    compound_assignments_use_the_corresponding_operator();
}

#[test]
fn suite_member_update() {
    member_updates_preserve_prefix_and_postfix_results();
    member_assignment_returns_the_assigned_value();
}

#[test]
fn suite_delete_operator() {
    delete_removes_an_own_property();
    deleting_a_missing_or_inherited_property_succeeds();
    deleting_an_identifier_does_not_remove_its_binding();
}

#[test]
fn suite_in_operator() {
    in_operator_checks_own_and_inherited_properties();
    in_operator_rejects_a_primitive_right_hand_side();
}

#[test]
fn suite_instanceof_operator() {
    instanceof_checks_the_constructor_prototype_chain();
    instanceof_supports_host_constructor_checks();
}
