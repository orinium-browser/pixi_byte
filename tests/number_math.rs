use pixi_byte::{JSEngine, JSValue};

#[test]
fn numbers_support_radix_to_string() {
    let mut engine = JSEngine::new();
    assert_eq!(
        engine.eval("(255).toString(16)").unwrap(),
        JSValue::String("ff".to_string())
    );
    assert_eq!(
        engine.eval("(35).toString(36)").unwrap(),
        JSValue::String("z".to_string())
    );
}

#[test]
fn math_exposes_functions_used_by_react() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            Math.min(8, 3, 5) === 3 &&
            Math.clz32(1) === 31 &&
            Math.ceil(1.2) === 2 &&
            Math.floor(1.8) === 1 &&
            Math.log(8) / Math.LN2 > 2.99 &&
            Math.log(8) / Math.LN2 < 3.01;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn math_random_returns_a_fraction() {
    let mut engine = JSEngine::new();
    let result = engine.eval("Math.random()").unwrap();
    let JSValue::Number(value) = result else {
        panic!("Math.random did not return a number");
    };
    assert!((0.0..1.0).contains(&value));
}
