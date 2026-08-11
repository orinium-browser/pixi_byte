use pixi_byte::{JSEngine, JSValue};

#[test]
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
    assert_eq!(result, JSValue::Number(2.0));
}

#[test]
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
    assert_eq!(result, JSValue::Number(3.0));
}

#[test]
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
    assert_eq!(result, JSValue::String("outer".to_string()));
}

#[test]
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
    assert_eq!(result, JSValue::String("outer".to_string()));
}
