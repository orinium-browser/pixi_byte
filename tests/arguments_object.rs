use pixi_byte::{JSEngine, JSValue};

#[test]
fn functions_expose_their_arguments_by_index_and_length() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function inspect() {
                return arguments[0] + arguments[1] + arguments.length;
            }
            inspect(10, 20);
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Number(32.0));
}

#[test]
fn array_slice_can_copy_an_arguments_object() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function collect() {
                return Array.prototype.slice.call(arguments, 1).join("-");
            }
            collect("skip", "a", "b");
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::String("a-b".to_string()));
}

#[test]
fn arrow_functions_use_the_enclosing_arguments_binding() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function outer(value) {
                const read = () => arguments[0];
                return read("ignored");
            }
            outer("outer");
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::String("outer".to_string()));
}
