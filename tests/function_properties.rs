use pixi_byte::{JSEngine, JSValue};

#[test]
fn function_constructor_is_callable_and_inherits_function_methods() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const has = Function.call.bind(Object.prototype.hasOwnProperty);
            typeof Function === "function" &&
                typeof Function("return 1") === "function" &&
                has({answer: 42}, "answer");
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn bound_functions_can_store_own_properties() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function checker(required) { return required; }
            const optional = checker.bind(null, false);
            optional.isRequired = checker.bind(null, true);
            optional() === false && optional.isRequired() === true;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::Boolean(true));
}

#[test]
fn functions_inherit_object_prototype_methods() {
    let mut engine = JSEngine::new();
    assert_eq!(
        engine
            .eval("(function example() {}).propertyIsEnumerable('prototype')")
            .unwrap(),
        JSValue::Boolean(false)
    );
}

#[test]
fn functions_keep_own_properties_and_constructor_prototypes() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function Box(value) {
                this.value = value;
            }
            Box.category = "container";
            Box.prototype.kind = "box";
            const instance = new Box(42);
            Box.category + ":" + instance.kind + ":" + instance.value + ":" +
                (instance.constructor === Box) + ":" + (instance instanceof Box);
            "#,
        )
        .unwrap();
    assert_eq!(
        result,
        JSValue::String("container:box:42:true:true".to_string())
    );
}

#[test]
fn prototype_assignment_supports_react_style_inheritance_setup() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function BasePrototype() {
                this.base = "ready";
            }
            function Component() {}
            function PureComponent() {}
            PureComponent.prototype = Component.prototype = new BasePrototype();
            PureComponent.prototype.constructor = PureComponent;
            const instance = new PureComponent();
            instance.base + ":" + (instance.constructor === PureComponent);
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::String("ready:true".to_string()));
}

#[test]
fn closure_own_properties_are_isolated() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function create() { return function () {}; }
            const first = create();
            const second = create();
            first.marker = "first";
            first.marker + ":" + (second.marker === undefined);
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::String("first:true".to_string()));
}

#[test]
fn chained_prototype_method_assignment_updates_each_constructor() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function First() {}
            function Second() {}
            Second.prototype.render = First.prototype.render = function () {
                return "ready";
            };
            const first = new First();
            const second = new Second();
            first.render() + ":" + second.render();
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::String("ready:ready".to_string()));
}

#[test]
fn nested_constructor_keeps_its_assigned_prototype_methods() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            function createInstance() {
                function Constructor(value) { this.value = value; }
                Constructor.prototype.render = function () { return this.value; };
                return new Constructor("ready");
            }
            createInstance().render();
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::String("ready".to_string()));
}

#[test]
fn callable_builtin_objects_inherit_function_prototype_methods() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            String.call(null, 42) + ":" + Number.call(null, "3");
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::String("42:3".to_string()));
}
