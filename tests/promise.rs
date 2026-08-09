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
