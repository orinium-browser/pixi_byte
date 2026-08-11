use pixi_byte::{JSEngine, JSValue};

#[test]
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

    assert_eq!(result, JSValue::String("boom".to_string()));
}

#[test]
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
        JSValue::String("try-catch-finally-normal-finally".to_string())
    );
}

#[test]
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

    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
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

    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
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

    assert_eq!(result, JSValue::String("outer".to_string()));
}
