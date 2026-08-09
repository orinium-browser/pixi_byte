use pixi_byte::{JSEngine, JSValue};
use std::cell::RefCell;
use std::rc::Rc;

fn object_constructor(
    _vm: &mut pixi_byte::vm::VM,
    _args: Vec<JSValue>,
) -> pixi_byte::JSResult<JSValue> {
    let mut result = pixi_byte::value::jsobject::JSObject::new();
    result.set("value".to_string(), JSValue::Number(9.0));
    Ok(JSValue::Object(Rc::new(RefCell::new(result))))
}

#[test]
fn new_calls_constructor_with_a_fresh_this_object() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function Box(value) { this.value = value; }
            let box = new Box(42);
            box.value;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Number(42.0));
}

#[test]
fn constructor_object_return_value_replaces_this() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function Factory() { return { value: 7 }; }
            new Factory().value;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Number(7.0));
}

#[test]
fn object_can_expose_an_internal_constructor_entry_point() {
    let mut engine = JSEngine::new();
    let mut constructor = pixi_byte::value::jsobject::JSObject::new();
    constructor.set(
        "__construct__".to_string(),
        JSValue::NativeFunction(object_constructor),
    );
    engine.global_mut().borrow_mut().set(
        "ObjectConstructor".to_string(),
        JSValue::Object(Rc::new(RefCell::new(constructor))),
    );

    assert_eq!(
        engine.eval("new ObjectConstructor().value").unwrap(),
        JSValue::Number(9.0)
    );
}

#[test]
fn new_accepts_member_expression_constructors() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const namespace = {
                Constructor: function (value) { this.value = value; }
            };
            const instance = new namespace.Constructor(42);
            instance.value;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Number(42.0));
}

#[test]
fn implicit_object_expression_does_not_replace_constructor_this() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function Wrapper(value) { this.value = value; }
            const internal = { marker: "internal" };
            const wrapper = new Wrapper(internal);
            wrapper === internal ? "replaced" : wrapper.value.marker;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::String("internal".to_string()));
}

#[test]
fn arrow_function_is_not_a_constructor() {
    let mut engine = JSEngine::new();
    assert!(engine.eval("new (() => 1)()").is_err());
}
