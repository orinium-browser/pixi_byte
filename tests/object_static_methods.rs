use pixi_byte::{JSEngine, JSValue};

#[test]
fn object_is_uses_same_value_semantics() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            "Object.is(NaN, NaN) && !Object.is(0, -0) && Object.is(-0, -0) && Object.is('a', 'a');",
        )
        .unwrap();

    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn object_static_methods_do_not_treat_the_constructor_as_an_argument() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const prototype = { inherited: true };
            const target = Object.create(prototype);
            Object.defineProperty(target, "own", {
                value: 7,
                enumerable: true,
                configurable: true,
                writable: true
            });
            Object.getPrototypeOf(target) === prototype && target.own === 7;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn get_prototype_of_accepts_functions_and_boxable_primitives() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function value() {}
            Object.getPrototypeOf(value) === Function.prototype &&
                Object.getPrototypeOf("text") === String.prototype &&
                Object.getPrototypeOf(1) === Number.prototype;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn ordinary_and_explicit_null_prototypes_are_distinct() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const ordinary = {};
            const bare = Object.create(null);
            Object.getPrototypeOf(ordinary) === Object.prototype &&
                Object.getPrototypeOf(Object.getPrototypeOf(ordinary)) === null &&
                Object.getPrototypeOf(bare) === null &&
                bare.toString === undefined;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn get_own_property_names_includes_non_enumerable_properties() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const target = {};
            Object.defineProperty(target, "hidden", { value: 1 });
            const names = Object.getOwnPropertyNames(target);
            names.length + ":" + names[0] + ":" +
                Object.getOwnPropertySymbols(target).length;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::String("1:hidden:0".to_string()));
}

#[test]
fn property_descriptor_methods_accept_functions() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function target() {}
            const returned = Object.defineProperty(target, "hidden", { value: 7 });
            returned === target &&
                Object.getOwnPropertyDescriptor(target, "hidden").value === 7;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn bound_set_prototype_of_forwards_arguments() {
    let mut engine = pixi_byte::JSEngine::new();
    let result = engine
        .eval(
            r#"
            function inherit(value, base) {
                return inherit = Object.setPrototypeOf.bind(), inherit(value, base);
            }
            function Base() {}
            function Derived() {}
            Base.marker = 42;
            inherit(Derived, Base);
            Derived.marker === 42;
            "#,
        )
        .unwrap();
    assert_eq!(result, pixi_byte::JSValue::Boolean(true));
}

#[test]
fn object_assign_accepts_function_targets_and_sources() {
    let mut engine = pixi_byte::JSEngine::new();
    let result = engine
        .eval(
            r#"
            function target() {}
            function source() {}
            source.answer = 42;
            Object.assign(target, source) === target && target.answer === 42;
            "#,
        )
        .unwrap();
    assert_eq!(result, pixi_byte::JSValue::Boolean(true));
}

#[test]
fn entries_values_and_from_entries_round_trip_enumerable_properties() {
    let mut engine = pixi_byte::JSEngine::new();
    let result = engine
        .eval(
            r#"
            const source = { first: 1, second: 2 };
            const entries = Object.entries(source);
            const rebuilt = Object.fromEntries(entries.map(([key, value]) => [key, value * 2]));
            Object.values(source).length === 2
                && Object.values(source).includes(1)
                && Object.values(source).includes(2)
                && rebuilt.first === 2
                && rebuilt.second === 4;
            "#,
        )
        .unwrap();
    assert_eq!(result, pixi_byte::JSValue::Boolean(true));
}
