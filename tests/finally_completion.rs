use pixi_byte::{JSEngine, JSValue};

#[test]
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

#[test]
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

#[test]
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
