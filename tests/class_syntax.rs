use pixi_byte::{JSEngine, JSValue};

#[test]
fn class_expression_constructs_instances_and_calls_methods() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const Box = class {
                    constructor(value = 3) { this.value = value; }
                    double() { return this.value * 2; }
                    static label() { return "box"; }
                };
                const box = new Box();
                Box.label() + ":" + box.double();
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::String("box:6".to_string()));
}

#[test]
fn class_extends_and_super_constructor_preserve_prototype_chain() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                class Base {
                    constructor(value) { this.value = value; }
                    read() { return this.value; }
                }
                class Child extends Base {
                    constructor(value) { super(value + 1); }
                }
                const child = new Child(4);
                (child instanceof Child) + ":" + (child instanceof Base) + ":" + child.read();
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::String("true:true:5".to_string()));
}

#[test]
fn anonymous_class_expression_can_extend_a_base_class() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const Base = class { read() { return 4; } };
                const Child = class extends Base {};
                new Child().read();
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Number(4.0));
}

#[test]
fn async_method_and_await_are_accepted_in_synchronous_mode() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                class Reader {
                    async read(value) { return await value; }
                }
                new Reader().read(4);
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Number(4.0));
}

#[test]
fn async_function_expression_is_accepted_in_synchronous_mode() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                const read = async function(value) { return await value; };
                read(4);
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Number(4.0));
}

#[test]
fn generator_method_collects_yielded_values() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                class Values {
                    *items() {
                        yield 1;
                        yield 2;
                    }
                }
                let total = 0;
                for (let value of new Values().items()) total = total + value;
                total;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Number(3.0));
}

#[test]
fn generator_function_collects_yielded_values() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                function* values() {
                    yield 2;
                    yield 3;
                }
                let total = 0;
                for (let value of values()) total = total + value;
                total;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Number(5.0));
}

#[test]
fn generator_can_yield_a_regular_expression_literal() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                function* values() { yield /a/; }
                values()[0].test("a");
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn method_parameters_support_destructuring_and_defaults() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                class Cache {
                    read({ value: value = 4 } = {}) { return value; }
                }
                const cache = new Cache();
                cache.read() + cache.read({ value: 3 });
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Number(7.0));
}

#[test]
fn class_accessors_are_invoked_as_properties() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                class Box {
                    constructor() { this.value = 2; }
                    get doubled() { return this.value * 2; }
                    set doubled(value) { this.value = value / 2; }
                    static get label() { return "box"; }
                }
                const box = new Box();
                box.doubled = 10;
                Box.label + ":" + box.value + ":" + box.doubled;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::String("box:5:10".to_string()));
}
