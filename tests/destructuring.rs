use pixi_byte::{JSEngine, JSValue};

#[test]
fn array_and_object_declarations_bind_values() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const [first, second] = [2, 3];
                const { value: third } = { value: 4 };
                first + second + third;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(9.0));
}

#[test]
fn for_of_supports_array_binding_patterns() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                let total = 0;
                for (let [key, value] of [["a", 2], ["b", 3]]) {
                    total = total + value;
                }
                total;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(5.0));
}

#[test]
fn declaration_list_can_mix_identifiers_and_patterns() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                let pair = [2, 3], [left, right] = pair;
                left + right;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(5.0));
}

#[test]
fn array_destructuring_assignment_updates_existing_bindings() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                let left = 0, right = 0;
                const assigned = ([left, right] = [2, 3]);
                left + right + assigned[0];
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(7.0));
}

#[test]
fn destructuring_assignment_supports_member_targets() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const target = {};
                [target.left, target.right] = [2, 3];
                target.left + target.right;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(5.0));
}

#[test]
fn function_parameters_support_binding_patterns() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                function add({ left: left, right: right = 3 }) {
                    return left + right;
                }
                add({ left: 2 });
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(5.0));
}

#[test]
fn arrow_parameters_support_binding_patterns() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const add = ([left, right]) => left + right;
                add([2, 3]);
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(5.0));
}

#[test]
fn async_arrow_supports_binding_patterns_in_synchronous_mode() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const add = async ([left, right]) => await left + right;
                add([2, 3]);
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(5.0));
}

#[test]
fn object_binding_patterns_support_rest() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const { first: first, ...rest } = { first: 2, second: 3 };
                first + rest.second;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(5.0));
}

#[test]
fn object_binding_patterns_accept_string_keys() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const { "min-width": minimum } = { "min-width": 42 };
                minimum;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(42.0));
}

#[test]
fn computed_key_bindings_exclude_the_key_from_rest() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const key = "second";
                const source = { first: 1, second: 2, third: 3 };
                const { [key]: value, ...rest } = source;
                value + ":" + rest.first + ":" + rest.third + ":" + ("second" in rest);
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("2:1:3:false".to_string()));
}

#[test]
fn numeric_computed_keys_bind_like_property_keys() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const { [117]: action, [4]: amount } = { 117: "load", 4: 9 };
                action + ":" + amount;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("load:9".to_string()));
}

#[test]
fn computed_key_bindings_support_defaults_and_nesting() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const key = "item";
                const { [key]: { level = 5 } = {} } = {};
                level;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(5.0));
}

#[test]
fn computed_key_expression_runs_once_per_binding() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                let calls = 0;
                function key() { calls = calls + 1; return "value"; }
                const { [key()]: bound, ...rest } = { value: 7, other: 1 };
                bound + ":" + calls + ":" + ("value" in rest);
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("7:1:false".to_string()));
}

#[test]
fn computed_keys_work_in_for_of_and_parameter_patterns() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const key = "n";
                function pick({ [key]: value }) { return value; }
                let total = 0;
                for (const { [key]: n } of [{ n: 1 }, { n: 2 }]) { total = total + n; }
                total + ":" + pick({ n: 4 });
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("3:4".to_string()));
}

#[test]
fn computed_keys_work_in_destructuring_assignments() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                let value = 0;
                const key = "v";
                ({ [key]: value } = { v: 4 });
                value;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(4.0));
}

#[test]
fn object_destructuring_assignment_supports_defaults() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                let a = 9, b = 9;
                const assigned = ({ a: a = 1, b: b = 2 } = { a: 5 });
                a + ":" + b + ":" + assigned.a;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("5:2:5".to_string()));
}

#[test]
fn array_destructuring_assignment_skips_elisions() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                let first = 0, third = 0;
                [first, , third] = [1, 2, 3];
                first + ":" + third;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("1:3".to_string()));
}
