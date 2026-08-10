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
    assert_eq!(result, JSValue::Number(9.0));
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
    assert_eq!(result, JSValue::Number(5.0));
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
    assert_eq!(result, JSValue::Number(5.0));
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
    assert_eq!(result, JSValue::Number(7.0));
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
    assert_eq!(result, JSValue::Number(5.0));
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
    assert_eq!(result, JSValue::Number(5.0));
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
    assert_eq!(result, JSValue::Number(5.0));
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
    assert_eq!(result, JSValue::Number(5.0));
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
    assert_eq!(result, JSValue::Number(5.0));
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
    assert_eq!(result, JSValue::Number(42.0));
}
