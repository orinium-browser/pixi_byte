use pixi_byte::{JSEngine, JSValue};

#[test]
fn for_in_enumerates_own_and_inherited_properties() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const parent = { inherited: 1 };
            const target = Object.create(parent);
            target.own = 2;
            let total = 0;
            for (const key in target) {
                total += target[key];
            }
            total;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_number(3.0));
}

#[test]
fn for_in_supports_existing_bindings_break_and_continue() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const target = { first: 1, skip: 2, stop: 3, after: 4 };
            let key = "";
            let total = 0;
            for (key in target) {
                if (key === "skip") continue;
                total += target[key];
            }
            let iterations = 0;
            for (key in target) {
                iterations += 1;
                break;
            }
            total === 8 && iterations === 1;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_bool(true));
}
