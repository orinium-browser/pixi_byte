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

#[test]
fn await_unwraps_fulfilled_promises_and_synchronous_lazy_results() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            async function read() {
                const lazy = { sync() { return 4; } };
                return (await Promise.resolve(3)) + (await lazy);
            }
            read();
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Number(7.0));
}

#[test]
fn await_drains_pending_promise_reactions() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            async function read() {
                return await Promise.resolve(3).then(value => value + 1);
            }
            read();
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Number(4.0));
}

#[test]
fn await_stringifies_synchronous_lazy_result_css() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            async function compile() {
                const result = {};
                const lazy = {
                    sync() { return result; },
                    toString() { return "body{color:red}"; }
                };
                const { css } = await lazy;
                return css;
            }
            compile();
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::String("body{color:red}".to_string()));
}
