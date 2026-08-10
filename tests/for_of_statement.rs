use pixi_byte::{JSEngine, JSValue};

#[test]
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

    assert_eq!(result, JSValue::String("abc".to_string()));
}

#[test]
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

    assert_eq!(result, JSValue::Number(7.0));
}
