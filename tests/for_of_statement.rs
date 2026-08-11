use pixi_byte::{JSEngine, JSValue};

#[test]
fn for_of_iterates_array_values() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                let joined = "";
                for (let value of ["a", "b", "c"]) {
                    joined = joined + value;
                }
                joined;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::String("abc".to_string()));
}

#[test]
fn for_of_supports_var_binding_and_continue() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                let total = 0;
                for (var value of [1, 2, 3]) {
                    if (value == 2) continue;
                    total = total + value;
                }
                value + total;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Number(7.0));
}

#[test]
fn for_of_let_closures_capture_each_iteration_binding() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const callbacks = [];
                for (let [name, value] of [["px", "horizontal"], ["py", "vertical"]]) {
                    callbacks.push(() => name + ":" + value);
                }
                callbacks[0]() + "," + callbacks[1]();
            "#,
        )
        .unwrap();

    assert_eq!(
        result,
        JSValue::String("px:horizontal,py:vertical".to_string())
    );
}

#[test]
fn for_of_let_scope_is_restored_by_continue_and_break() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const callbacks = [];
                for (let value of [1, 2, 3]) {
                    callbacks.push(() => value);
                    if (value === 1) continue;
                    if (value === 2) break;
                }
                callbacks[0]() * 10 + callbacks[1]();
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Number(12.0));
}

#[test]
fn nested_callbacks_capture_bindings_from_their_for_of_iteration() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const handlers = {};
                const groups = [
                    ["p", ["padding"]],
                    [["px", ["padding-left", "padding-right"]], ["py", ["padding-top", "padding-bottom"]]],
                    [["pt", ["padding-top"]], ["pr", ["padding-right"]]]
                ];
                for (let group of groups) {
                    const entries = Array.isArray(group[0]) ? group : [group];
                    Object.assign(handlers, entries.reduce(
                        (result, [name, properties]) => Object.assign(result, {
                            [name]: value => properties.reduce(
                                (declarations, property) => Object.assign(declarations, { [property]: value }),
                                {}
                            )
                        }),
                        {}
                    ));
                }
                const result = handlers.py("0.5rem");
                result["padding-top"] + "," + result["padding-bottom"] + "," + result["padding-left"];
            "#,
        )
        .unwrap();

    assert_eq!(
        result,
        JSValue::String("0.5rem,0.5rem,undefined".to_string())
    );
}

#[test]
fn classic_for_let_closures_capture_each_iteration_binding() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const callbacks = [];
                for (let index = 0; index < 3; index++) {
                    callbacks.push(() => index);
                }
                callbacks[0]() * 100 + callbacks[1]() * 10 + callbacks[2]();
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Number(12.0));
}

#[test]
fn classic_for_let_scope_is_restored_by_break() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                let index = 9;
                for (let index = 0; index < 3; index++) {
                    if (index === 1) break;
                }
                index;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Number(9.0));
}

#[test]
fn for_in_let_closures_capture_each_iteration_binding() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const callbacks = {};
                for (let name in { px: 1, py: 2 }) {
                    callbacks[name] = () => name;
                }
                callbacks.px() + "," + callbacks.py();
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::String("px,py".to_string()));
}

#[test]
fn for_in_let_scope_is_restored_by_continue_and_break() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                let name = "outer";
                const callbacks = [];
                for (let name in { first: 1, second: 2, third: 3 }) {
                    callbacks.push(() => name);
                    if (callbacks.length === 1) continue;
                    break;
                }
                name + ":" + callbacks[0]() + ":" + callbacks[1]();
            "#,
        )
        .unwrap();

    let JSValue::String(value) = result else {
        panic!("expected string");
    };
    assert!(value.starts_with("outer:"));
    let parts: Vec<_> = value.split(':').collect();
    assert_ne!(parts[1], parts[2]);
}
