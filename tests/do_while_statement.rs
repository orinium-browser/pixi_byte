use pixi_byte::{JSEngine, JSValue};

#[test]
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

#[test]
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
