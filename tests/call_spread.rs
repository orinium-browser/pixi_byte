use pixi_byte::{JSEngine, JSValue};

#[test]
fn rest_parameters_collect_remaining_arguments() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function collect(first, ...rest) { return first + ":" + rest.join(","); }
            const arrow = (...values) => values.length;
            collect("a", "b", "c") === "a:b,c" && arrow(1, 2, 3) === 3;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn function_calls_expand_spread_arguments() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                function add(a, b, c) { return a + b + c; }
                const middle = [2];
                add(1, ...middle, 3);
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Number(6.0));
}

#[test]
fn method_calls_expand_spread_arguments_and_preserve_receiver() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const target = {
                    base: 1,
                    add(a, b) { return this.base + a + b; }
                };
                target.add(...[2, 3]);
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Number(6.0));
}
