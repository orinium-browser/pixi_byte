use pixi_byte::{JSEngine, JSValue};

#[test]
fn proxy_get_and_set_traps_are_invoked() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const target = { value: 2 };
                const proxy = new Proxy(target, {
                    get(object, key) { return object[key] + 1; },
                    set(object, key, value) { object[key] = value * 2; return true; }
                });
                const read = proxy.value;
                proxy.value = 3;
                read + target.value;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Number(9.0));
}
