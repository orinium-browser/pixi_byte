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
fn bound_class_construction_uses_the_target_prototype_and_new_instance() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                class Box {
                    constructor(value) { this.value = value; }
                    read() { return this.value; }
                }
                const wrongReceiver = {};
                const BoundBox = Box.bind(wrongReceiver, 7);
                const box = new BoundBox();
                (box instanceof Box) + ":" + box.read() + ":" + wrongReceiver.value;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::String("true:7:undefined".to_string()));
}

#[test]
fn callable_object_constructor_uses_the_inner_function_prototype() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                class Box {
                    constructor(value) { this.value = value; }
                    read() { return this.value; }
                }
                const WrappedBox = { __construct__: Box };
                const box = new WrappedBox(9);
                (box instanceof Box) + ":" + box.read();
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::String("true:9".to_string()));
}

#[test]
fn callable_object_with_bound_constructor_preserves_class_construction() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                class Box {
                    constructor(value) { this.value = value; }
                    read() { return this.value; }
                }
                const wrongReceiver = {};
                const WrappedBox = {
                    __construct__: Box.bind(wrongReceiver, 11),
                    prototype: Object.create(null)
                };
                const box = new WrappedBox();
                (box instanceof Box) + ":" + box.read() + ":" + wrongReceiver.value;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::String("true:11:undefined".to_string()));
}

#[test]
fn callable_object_exposes_inner_class_properties_and_is_constructible() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                class Box {
                    constructor(value) { this.value = value; }
                    read() { return this.value; }
                }
                Box.prototype.isComponent = true;
                const WrappedBox = { __call__: Box };
                const box = new WrappedBox(15);
                WrappedBox.name + ":" + WrappedBox.prototype.isComponent + ":" +
                    (box instanceof Box) + ":" + box.read();
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::String("Box:true:true:15".to_string()));
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
fn implicit_derived_constructor_forwards_arguments_to_base_class() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                class Base {
                    constructor(value) { this.value = value; }
                }
                class Middle extends Base {}
                class Child extends Middle {
                    constructor(value) { super(value); }
                }
                new Child("ready").value;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::String("ready".to_string()));
}

#[test]
fn super_method_calls_use_the_derived_instance_as_receiver() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                class Base {
                    read(suffix) { return this.value + suffix; }
                }
                class Child extends Base {
                    constructor() { super(); this.value = "child"; }
                    read(...suffix) { return super.read(...suffix); }
                }
                new Child().read("-base");
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::String("child-base".to_string()));
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
fn generator_functions_return_iterator_result_objects() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
                function* values() { yield 2; yield 3; }
                const iterator = values();
                const first = iterator.next();
                const second = iterator.next();
                const done = iterator.next();
                first.value + second.value + ":" + first.done + ":" + done.done;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::String("5:false:true".to_string()));
}

#[test]
fn generator_function_expressions_return_iterators() {
    let mut engine = JSEngine::new();
    assert_eq!(
        engine
            .eval("function run() { const values = function* () { yield 4; }; const iterator = values(); const key = 'next'; return iterator[key]().value; } run();")
            .unwrap(),
        JSValue::Number(4.0)
    );
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
