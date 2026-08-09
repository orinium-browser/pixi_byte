use pixi_byte::{JSEngine, JSValue};

#[test]
fn date_now_returns_epoch_milliseconds() {
    let mut engine = JSEngine::new();
    let result = engine.eval("Date.now()").unwrap();
    let JSValue::Number(milliseconds) = result else {
        panic!("Date.now() must return a number");
    };
    assert!(milliseconds > 0.0);
}
