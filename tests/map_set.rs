use pixi_byte::{JSEngine, JSValue};

#[test]
fn set_constructs_from_arrays_and_mutates_membership() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = new Set(["a", "b", "a"]);
            values.add("c");
            const deleted = values.delete("b");
            values.has("a") && values.has("c") && deleted && !values.has("b");
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn map_supports_identity_keys_and_chained_set() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const key = {};
            const other = {};
            const values = new Map;
            values.set(key, 1).set(other, 2);
            values.get(key) === 1 && values.get(other) === 2 && !values.has({});
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn set_for_each_uses_web_compatible_arguments_and_this_value() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = new Set(["a", "b"]);
            const receiver = { prefix: "set:" };
            let result = "";
            values.forEach(function (value, key, collection) {
                result += this.prefix + value + key + (collection === values);
            }, receiver);
            result;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::String("set:aatrueset:bbtrue".to_string()));
}

#[test]
fn map_for_each_visits_values_and_keys_in_insertion_order() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = new Map([["a", 1], ["b", 2]]);
            let result = "";
            values.forEach(function (value, key, collection) {
                result += key + value + (collection === values);
            });
            result;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::String("a1trueb2true".to_string()));
}

#[test]
fn set_exposes_the_iterator_protocol() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const iterator = new Set(["a", "b"])[Symbol.iterator]();
            const first = iterator.next();
            const second = iterator.next();
            const end = iterator.next();
            first.value + second.value + first.done + end.done;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::String("abfalsetrue".to_string()));
}

#[test]
fn map_iterators_expose_keys_values_and_entries() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = new Map([["a", 1], ["b", 2]]);
            const key = values.keys().next().value;
            const value = values.values().next().value;
            const entry = values[Symbol.iterator]().next().value;
            key + value + entry[0] + entry[1];
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::String("a1a1".to_string()));
}
