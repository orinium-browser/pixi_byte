use pixi_byte::{JSEngine, JSValue};

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
