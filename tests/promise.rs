use pixi_byte::{JSEngine, JSValue};

#[test]
fn promise_then_runs_asynchronously_and_chains_values() {
    let mut engine = JSEngine::new();
    engine
        .eval(
            r#"
            let result = "sync";
            new Promise(function (resolve) {
                resolve(20);
            }).then(function (value) {
                result = result + "-first";
                return value + 1;
            }).then(function (value) {
                result = result + "-" + value;
            });
            "#,
        )
        .unwrap();

    assert_eq!(
        engine.eval("result").unwrap(),
        JSValue::String("sync".into())
    );
    engine.run_jobs().unwrap();
    assert_eq!(
        engine.eval("result").unwrap(),
        JSValue::String("sync-first-21".into())
    );
}

#[test]
fn promise_catch_handles_rejection() {
    let mut engine = JSEngine::new();
    engine
        .eval(
            r#"
            let reason = "pending";
            new Promise(function (_resolve, reject) {
                reject("failed");
            }).catch(function (value) {
                reason = value;
            });
            "#,
        )
        .unwrap();

    engine.run_jobs().unwrap();
    assert_eq!(
        engine.eval("reason").unwrap(),
        JSValue::String("failed".into())
    );
}

#[test]
fn promise_ignores_later_settlement_attempts() {
    let mut engine = JSEngine::new();
    engine
        .eval(
            r#"
            let result = "pending";
            new Promise(function (resolve, reject) {
                resolve("first");
                reject("second");
            }).then(function (value) {
                result = value;
            });
            "#,
        )
        .unwrap();

    engine.run_jobs().unwrap();
    assert_eq!(
        engine.eval("result").unwrap(),
        JSValue::String("first".into())
    );
}

#[test]
fn promise_static_resolve_and_reject_create_settled_promises() {
    let mut engine = JSEngine::new();
    engine
        .eval(
            r#"
            let fulfilled = "pending";
            let rejected = "pending";
            Promise.resolve("yes").then(function (value) { fulfilled = value; });
            Promise.reject("no").catch(function (reason) { rejected = reason; });
            "#,
        )
        .unwrap();

    engine.run_jobs().unwrap();
    assert_eq!(
        engine.eval("fulfilled").unwrap(),
        JSValue::String("yes".into())
    );
    assert_eq!(
        engine.eval("rejected").unwrap(),
        JSValue::String("no".into())
    );
}

#[test]
fn promise_all_preserves_input_order() {
    let mut engine = JSEngine::new();
    engine
        .eval(
            r#"
            let combined = "pending";
            Promise.all([Promise.resolve("first"), "second"]).then(function (values) {
                combined = values[0] + "-" + values[1];
            });
            "#,
        )
        .unwrap();

    engine.run_jobs().unwrap();
    assert_eq!(
        engine.eval("combined").unwrap(),
        JSValue::String("first-second".into())
    );
}
