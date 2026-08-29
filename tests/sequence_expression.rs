use pixi_byte::{JSEngine, JSValue};

#[test]
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

#[test]
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
