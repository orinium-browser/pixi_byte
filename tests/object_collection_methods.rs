use pixi_byte::{JSEngine, JSValue};

#[test]
fn object_keys_returns_enumerable_own_keys() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const prototype = { inherited: true };
            const target = Object.create(prototype);
            target.first = 1;
            target.second = 2;
            const keys = Object.keys(target);
            keys.length === 2 && "0" in keys && "1" in keys && keys[0] !== "inherited";
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn object_assign_copies_sources_from_left_to_right() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const target = { preserved: true };
            const returned = Object.assign(target, { value: 1 }, null, { value: 2 });
            returned === target && target.preserved && target.value === 2;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn extracted_object_assign_uses_its_first_argument_as_the_target() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const assign = Object.assign;
            const target = {};
            const returned = assign(target, { value: 42 });
            returned === target && target.value === 42 && typeof globalThis.value === "undefined";
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Boolean(true));
}
