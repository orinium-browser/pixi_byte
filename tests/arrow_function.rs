use pixi_byte::{JSEngine, JSValue};

#[test]
fn arrow_functions_support_expression_and_block_bodies() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const add = (a, b) => a + b;
            const double = value => { return value * 2; };
            double(add(2, 3));
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Number(10.0));
}

#[test]
fn zero_parameter_arrow_captures_outer_bindings() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let value = 4;
            const read = () => value;
            value = 7;
            read();
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Number(7.0));
}

#[test]
fn arrows_work_as_promise_reactions() {
    let mut engine = JSEngine::new();
    engine
        .eval(
            r#"
            let result = 0;
            Promise.resolve(20).then(value => value + 1).then(value => { result = value; });
            "#,
        )
        .unwrap();
    engine.run_jobs().unwrap();

    assert_eq!(engine.eval("result").unwrap(), JSValue::Number(21.0));
}

#[test]
fn arrow_function_keeps_lexical_this_through_call() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function makeReader() { return () => this.value; }
            const holder = { value: 42, makeReader: makeReader };
            const reader = holder.makeReader();
            reader.call({ value: 1 });
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Number(42.0));
}
