use pixi_byte::{JSEngine, JSValue};

#[test]
fn function_clones_retain_reference_identity() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const original = function () { return 1; };
            const alias = original;
            const distinct = function () { return 1; };
            original === alias && original !== distinct;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

#[test]
fn bound_function_clones_retain_reference_identity() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function target() {}
            const first = target.bind(null);
            const alias = first;
            const second = target.bind(null);
            first === alias && first !== second;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

#[test]
fn repeated_closure_creation_produces_distinct_identities() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function create() {
                return function () {};
            }
            const first = create();
            const second = create();
            first !== second;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

#[test]
fn assigning_an_array_index_grows_length() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const queue = [];
            queue[0] = "entry";
            queue[2] = "later";
            queue.length;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(3.0));
}

#[test]
fn object_keys_enumerates_function_properties() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const hooks = () => {};
            hooks.j = id => id === 2076;
            const keys = Object.keys(hooks);
            keys.length === 1 && keys[0] === "j";
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

#[test]
fn minified_webpack_deferred_entry_runs() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let called = 0;
            const runtime = {};
            var queue = [];
            runtime.O = (result, ids, callback, priority) => {
                if (!ids) {
                    var ceiling = 1 / 0;
                    for (outer = 0; outer < queue.length; outer++) {
                        for (var [ids, callback, priority] = queue[outer], ready = true, inner = 0;
                             inner < ids.length;
                             inner++)
                            (!1 & priority || ceiling >= priority) &&
                                Object.keys(runtime.O).every(key => runtime.O[key](ids[inner]))
                                ? ids.splice(inner--, 1)
                                : ready = false;
                        if (ready) {
                            queue.splice(outer--, 1);
                            callback();
                        }
                    }
                    return result;
                }
                priority = priority || 0;
                for (var outer = queue.length;
                     outer > 0 && queue[outer - 1][2] > priority;
                     outer--)
                    queue[outer] = queue[outer - 1];
                queue[outer] = [ids, callback, priority];
            };
            runtime.O.j = id => id === 2076;
            runtime.O(undefined, [2076], () => { called = 1; });
            runtime.O();
            called;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(1.0));
}
