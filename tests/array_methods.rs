use pixi_byte::{JSEngine, JSValue};

#[test]
fn array_from_supports_array_like_iterables_and_mapping() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const arrayLike = { 0: "a", 1: "b", length: 2 };
            Array.from(arrayLike).join("") === "ab"
                && Array.from(new Set([1, 2]), value => value * 2).join(",") === "2,4";
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn array_mutation_methods_update_values_and_length() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = [2, 3];
            values.unshift(1);
            const shifted = values.shift();
            const removed = values.splice(1, 1, 4, 5);
            shifted === 1 && removed[0] === 3 && values.join("-") === "2-4-5";
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn array_read_methods_return_expected_results() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = [1, 2, 3];
            const copy = values.slice(1).concat([4], 5);
            copy.join(",") === "2,3,4,5" && copy.indexOf(4) === 2 && copy.includes(5);
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn array_for_each_calls_back_in_index_order() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let result = "";
            ["a", "b", "c"].forEach(function (value, index) {
                result += index + value;
            });
            result;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::String("0a1b2c".to_string()));
}

#[test]
fn array_iteration_methods_support_component_data_transforms() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = [1, 2, 3, 4];
            const mapped = values.map(function (value, index) { return value + index; });
            const filtered = mapped.filter(function (value) { return value > 3; });
            const total = filtered.reduce(function (sum, value) { return sum + value; }, 0);
            mapped.join(",") + ":" + filtered.join(",") + ":" + total
                + ":" + values.some(function (value) { return value === 3; })
                + ":" + values.every(function (value) { return value > 0; });
            "#,
        )
        .unwrap();

    assert_eq!(
        result,
        JSValue::String("1,3,5,7:5,7:12:true:true".to_string())
    );
}
