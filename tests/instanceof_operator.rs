use pixi_byte::value::jsobject::{HOST_HAS_INSTANCE, JSObject};
use pixi_byte::{JSEngine, JSValue};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn instanceof_checks_the_constructor_prototype_chain() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const prototype = {};
            const Constructor = { prototype: prototype };
            const instance = Object.create(prototype);
            instance instanceof Constructor;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn instanceof_supports_host_constructor_checks() {
    let mut engine = JSEngine::new();
    let mut constructor = JSObject::new();
    constructor.set(
        HOST_HAS_INSTANCE.to_string(),
        JSValue::NativeFunction(|_vm, args| {
            let matches = match args.get(1) {
                Some(JSValue::Number(value)) => *value == 42.0,
                _ => false,
            };
            Ok(JSValue::Boolean(matches))
        }),
    );
    engine.global_mut().borrow_mut().set(
        "Answer".to_string(),
        JSValue::Object(Rc::new(RefCell::new(constructor))),
    );

    assert_eq!(
        engine.eval("42 instanceof Answer").unwrap(),
        JSValue::Boolean(true)
    );
    assert_eq!(
        engine.eval("41 instanceof Answer").unwrap(),
        JSValue::Boolean(false)
    );
}
