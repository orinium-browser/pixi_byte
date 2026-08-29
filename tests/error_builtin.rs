use pixi_byte::{JSEngine, JSValue};

#[test]
fn error_is_callable_and_exposes_standard_properties() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const error = Error("failed");
            error.name === "Error" &&
            error.message === "failed" &&
            error.toString() === "Error: failed" &&
            error.stack.includes("failed");
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

#[test]
fn thrown_errors_are_preserved_by_catch() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let message = "";
            try {
                throw Error("boom");
            } catch (error) {
                message = error.message;
            }
            message;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("boom".to_string()));
}

#[test]
fn native_error_subtypes_are_callable_and_constructible() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const type = new TypeError("wrong");
            const range = RangeError("outside");
            type.name === "TypeError" &&
                type.message === "wrong" &&
                type.toString() === "TypeError: wrong" &&
                range.name === "RangeError" &&
                range.toString() === "RangeError: outside";
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

#[test]
fn native_error_constructors_have_function_type_and_call_method() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const error = Error.call(null, "called");
            typeof Error === "function" &&
                typeof TypeError === "function" &&
                error.message === "called";
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

#[test]
fn function_call_preserves_thrown_error_objects() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let name = "";
            try {
                (function () { throw new TypeError("broken"); }).call(null);
            } catch (error) {
                name = error.name + ": " + error.message;
            }
            name;
            "#,
        )
        .unwrap();
    assert_eq!(
        result,
        JSValue::from_string("TypeError: broken".to_string())
    );
}
