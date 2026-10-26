use pixi_byte::{JSEngine, JSValue};

#[test]
fn nullish_coalescing_assignment_falls_back_when_nullish() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                let a = null;
                let b = 1;
                a ??= 10;
                b ??= 20;
                a + ":" + b;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_string("10:1".to_string()));
}

#[test]
fn nullish_coalescing_assignment_is_a_token_together() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const cache = { value: null };
                cache.value ??= (async () => 42)();
                String(cache.value);
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_string("42".to_string()));
}

#[test]
fn for_await_of_parses_and_iterates_synchronously() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const collected = [];
                async function run() {
                    for await (const item of [1, 2, 3]) {
                        collected.push(item);
                    }
                    return collected.join(",");
                }
                collected.join("?");
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_string("".to_string()));
}
#[test]
fn logical_assignment_operators_short_circuit_on_truthiness() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                let a = 0;
                let b = "keep";
                a ||= 5;
                b &&= b + "!";
                a + ":" + b;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_string("5:keep!".to_string()));
}

#[test]
fn spread_arguments_work_with_new_and_optional_member_calls() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                class Point {
                    constructor(x, y) { this.x = x; this.y = y; }
                    sum() { return this.x + this.y; }
                }
                const args = [2, 3];
                const point = new Point(...args);
                const maybe = point;
                const optional = maybe?.sum(...args);
                point.sum() + ":" + optional;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_string("5:5".to_string()));
}

#[test]
fn spread_arguments_forward_through_super() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                class Base {
                    constructor(a, b) { this.v = a + b; }
                }
                class Child extends Base {
                    constructor(...args) { super(...args); }
                }
                String(new Child(2, 3).v);
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_string("5".to_string()));
}
