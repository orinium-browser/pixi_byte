use pixi_byte::{JSEngine, JSValue};

#[test]
fn object_static_methods_do_not_treat_the_constructor_as_an_argument() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const prototype = { inherited: true };
            const target = Object.create(prototype);
            Object.defineProperty(target, "own", {
                value: 7,
                enumerable: true,
                configurable: true,
                writable: true
            });
            Object.getPrototypeOf(target) === prototype && target.own === 7;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Boolean(true));
}
