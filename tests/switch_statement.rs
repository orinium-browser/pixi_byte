use pixi_byte::{JSEngine, JSValue};

#[test]
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

#[test]
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

#[test]
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
