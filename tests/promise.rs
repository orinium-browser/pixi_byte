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
        JSValue::from_string("sync".into())
    );
    engine.run_jobs().unwrap();
    assert_eq!(
        engine.eval("result").unwrap(),
        JSValue::from_string("sync-first-21".into())
    );
}

#[test]
fn babel_generator_async_helper_advances_generator_promises() {
    let mut engine = JSEngine::new();
    engine
        .eval(
            r#"
            let result = "pending";
            function step(generator, resolve, reject, next, fail, key, value) {
                let info;
                try { info = generator[key](value); } catch (error) { reject(error); return; }
                if (info.done) resolve(info.value);
                else Promise.resolve(info.value).then(next, fail);
            }
            function run() {
                const body = function* () {
                    yield Promise.resolve(1);
                    yield Promise.resolve(2);
                };
                return new Promise(function (resolve, reject) {
                    const generator = body();
                    function next(value) { step(generator, resolve, reject, next, fail, "next", value); }
                    function fail(error) { step(generator, resolve, reject, next, fail, "throw", error); }
                    next(undefined);
                });
            }
            run().then(function () { result = "done"; }).catch(function (error) {
                result = "error:" + error;
            });
            "#,
        )
        .unwrap();
    engine.run_jobs().unwrap();
    assert_eq!(
        engine.eval("result").unwrap(),
        JSValue::from_string("done".to_string())
    );
}

#[test]
fn generator_switch_eagerly_evaluates_babel_chunk_yields() {
    let mut engine = JSEngine::new();
    engine
        .eval(
            r#"
            let calls = 0;
            function load() { calls = calls + 1; return Promise.resolve(); }
            const body = function* (locale) {
                switch (locale.toLowerCase().split("-")[0]) {
                    case "ja": yield load(); break;
                    case "en": default:
                        yield load(); yield load(); yield load(); yield load();
                }
            };
            body("en");
            "#,
        )
        .unwrap();
    assert_eq!(engine.eval("calls").unwrap(), JSValue::from_number(4.0));
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
        JSValue::from_string("failed".into())
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
        JSValue::from_string("first".into())
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
        JSValue::from_string("yes".into())
    );
    assert_eq!(
        engine.eval("rejected").unwrap(),
        JSValue::from_string("no".into())
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
        JSValue::from_string("first-second".into())
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
    assert_eq!(result, JSValue::from_number(7.0));
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
    assert_eq!(result, JSValue::from_number(4.0));
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
    assert_eq!(result, JSValue::from_string("body{color:red}".to_string()));
}

#[test]
fn eager_generator_evaluates_every_webpack_style_chunk_request() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let requested = [];
            function load(id) {
                requested.push(id);
                return new Promise(function (resolve) {});
            }
            function* locale(locale) {
                if (true) switch (
                    (yield load(5263).then(function () {})),
                    requested.push(999),
                    locale.toLowerCase().split("-")[0]
                ) {
                    case "en":
                        yield load(7674).then(function () {});
                        yield load(5199).then(function () {});
                        yield load(2299).then(function () {});
                        yield load(2123).then(function () {});
                        break;
                }
            }
            const iterator = locale("en-US");
            requested.join(",") + ":" + (typeof iterator.next);
            "#,
        )
        .unwrap();

    assert_eq!(
        result,
        JSValue::from_string("5263,999,7674,5199,2299,2123:function".to_string())
    );
}
