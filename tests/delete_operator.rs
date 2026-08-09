use pixi_byte::{JSEngine, JSValue};

#[test]
fn delete_removes_an_own_property() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const target = { keep: 1, remove: 2 };
            const deleted = delete target.remove;
            deleted && !("remove" in target) && target.keep === 1;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn deleting_a_missing_or_inherited_property_succeeds() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const parent = { inherited: true };
            const child = Object.create(parent);
            (delete child.missing) && (delete child.inherited) && ("inherited" in child);
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn deleting_an_identifier_does_not_remove_its_binding() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let value = 3;
            const deleted = delete value;
            !deleted && value === 3;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Boolean(true));
}
