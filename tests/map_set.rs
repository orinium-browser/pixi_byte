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
            const copy = new Set(values);
            values.has("a") && values.has("c") && deleted && !values.has("b") &&
                values.size === 2 && copy.size === 2 && copy.has("c");
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
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
            const copy = new Map(values);
            values.get(key) === 1 && values.get(other) === 2 && !values.has({}) &&
                values.size === 2 && copy.get(other) === 2;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

#[test]
fn map_keeps_large_bigint_keys_distinct() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
        const values = new Map;
        values.set(1n << 70n, "large");
        values.set(1n << 6n, "small");
        values.size === 2 && values.get(1n << 70n) === "large";
    "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
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
    assert_eq!(result, JSValue::from_string("set:aatrueset:bbtrue".to_string()));
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
    assert_eq!(result, JSValue::from_string("a1trueb2true".to_string()));
}

#[test]
fn map_iteration_can_update_existing_entries_without_extending_iteration() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = new Map([["a", [1]], ["b", [2]], ["c", [3]]]);
            let visits = 0;
            for (const [key, value] of values.entries()) {
                values.set(key, value.map(item => item + 1));
                visits++;
            }
            visits + ":" + values.get("a")[0] + values.get("c")[0];
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("3:24".to_string()));
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
    assert_eq!(result, JSValue::from_string("abfalsetrue".to_string()));
}

#[test]
fn for_of_consumes_set_values() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let joined = "";
            for (const value of new Set(["a", "b"])) joined += value;
            joined;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("ab".to_string()));
}

#[test]
fn array_spread_consumes_set_values() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(r#"[...new Set(["a", "b", "a"])].join("");"#)
        .unwrap();
    assert_eq!(result, JSValue::from_string("ab".to_string()));
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
    assert_eq!(result, JSValue::from_string("a1a1".to_string()));
}

#[test]
fn weak_map_supports_object_keys_without_iteration() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const first = {};
            const second = function () {};
            const values = new WeakMap([[first, 1]]);
            values.set(second, 2);
            values.get(first) === 1 && values.get(second) === 2 && values.has(first);
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}
