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
    assert_eq!(result, JSValue::from_number(9.0));
}

#[test]
fn proxy_internal_properties_do_not_leak_through_enumeration() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const proxy = new Proxy({}, {});
                const copy = { ...proxy };
                Object.keys(proxy).length === 0 && Object.keys(copy).length === 0;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

#[test]
fn callable_proxy_forwards_calls_and_apply_traps() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const target = (left, right) => left + right;
                const direct = new Proxy(target, {});
                const trapped = new Proxy(target, {
                    apply(fn, receiver, args) { return fn(args[0] * 2, args[1]); }
                });
                direct(2, 3) + ":" + trapped(2, 3);
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("5:7".to_string()));
}

#[test]
fn callable_proxy_forwards_function_properties_and_class_construction() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                class Box {
                    constructor(value) { this.value = value; }
                    read() { return this.value; }
                }
                Box.prototype.isComponent = true;
                const WrappedBox = new Proxy(Box, {});
                const box = new WrappedBox(13);
                typeof WrappedBox + ":" + WrappedBox.prototype.isComponent + ":" +
                    (box instanceof Box) + ":" + box.read();
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_string("function:true:true:13".to_string()));
}
