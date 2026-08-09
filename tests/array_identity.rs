use pixi_byte::{JSEngine, JSValue};

#[test]
fn array_literals_are_identified_as_arrays() {
    let mut engine = JSEngine::new();
    assert_eq!(
        engine.eval("Array.isArray([])").unwrap(),
        JSValue::Boolean(true)
    );
    assert_eq!(
        engine.eval("Array.isArray({ length: 0 })").unwrap(),
        JSValue::Boolean(false)
    );
}

#[test]
fn array_literals_inherit_array_prototype_methods() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = [1];
            values.push(2, 3);
            values.length === 3 && values.pop() === 3;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Boolean(true));
}
