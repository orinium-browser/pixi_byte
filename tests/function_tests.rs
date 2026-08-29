use pixi_byte::{JSEngine, JSValue};

#[test]
fn test_simple_function() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
        function f() { return 5; }
        f();
    "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(5.0));
}

#[test]
fn test_function_with_args() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
        function add(a, b) { return a + b; }
        add(2, 3);
    "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(5.0));
}

#[test]
fn function_without_return_produces_undefined() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval("function update() { const value = {}; value.current = {}; } update();")
        .unwrap();
    assert_eq!(result, JSValue::undefined());
}

#[test]
fn parameter_shadows_the_function_name() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval("(function value(value) { return value; })(42);")
        .unwrap();
    assert_eq!(result, JSValue::from_number(42.0));
}

#[test]
fn local_declaration_does_not_overwrite_an_outer_binding() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            var value = 1;
            function read() {
                var value = 2;
                return value;
            }
            read() + ":" + value;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("2:1".to_string()));
}
