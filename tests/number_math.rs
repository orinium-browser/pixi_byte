use pixi_byte::{JSEngine, JSValue};

#[test]
fn numbers_support_radix_to_string() {
    let mut engine = JSEngine::new();
    assert_eq!(
        engine.eval("(255).toString(16)").unwrap(),
        JSValue::from_string("ff".to_string())
    );
    assert_eq!(
        engine.eval("(35).toString(36)").unwrap(),
        JSValue::from_string("z".to_string())
    );
}

#[test]
fn math_exposes_functions_used_by_react() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            Math.min(8, 3, 5) === 3 &&
            Math.max(8, 3, 5) === 8 &&
            Math.abs(-4) === 4 &&
            Math.pow(2, 3) === 8 &&
            Math.PI > 3.14 &&
            Math.clz32(1) === 31 &&
            Math.ceil(1.2) === 2 &&
            Math.floor(1.8) === 1 &&
            Math.log(8) / Math.LN2 > 2.99 &&
            Math.log(8) / Math.LN2 < 3.01;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

#[test]
fn math_random_returns_a_fraction() {
    let mut engine = JSEngine::new();
    let result = engine.eval("Math.random()").unwrap();
    let Some(value) = result.as_number() else {
        panic!("Math.random did not return a number");
    };
    assert!((0.0..1.0).contains(&value));
}
