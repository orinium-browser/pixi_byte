use pixi_byte::error::JSError;
use pixi_byte::vm::VM;
use pixi_byte::{JSEngine, JSValue};
use std::cell::RefCell;
use std::rc::Rc;

fn test_jsvalue_to_string() {
    assert_eq!(JSValue::undefined().to_string(), "undefined");
    assert_eq!(JSValue::null().to_string(), "null");
    assert_eq!(JSValue::from_bool(true).to_string(), "true");
}
fn test_jsvalue_to_number() {
    assert!(JSValue::undefined().to_number().is_nan());
    assert_eq!(JSValue::null().to_number(), 0.0);
    assert_eq!(JSValue::from_bool(true).to_number(), 1.0);
}
fn test_jsvalue_to_boolean() {
    assert!(!JSValue::undefined().to_boolean());
    assert!(!JSValue::null().to_boolean());
    assert!(JSValue::from_bool(true).to_boolean());
}
fn test_jsvalue_strict_equals() {
    assert!(JSValue::undefined().strict_equals(&JSValue::undefined()));
    assert!(JSValue::null().strict_equals(&JSValue::null()));
}
fn test_jsvalue_abstract_equals() {
    assert!(JSValue::null().abstract_equals(&JSValue::undefined()));
    assert!(JSValue::from_number(42.0).abstract_equals(&JSValue::from_string("42".to_string())));
}

fn test_basic_arithmetic() {
    let mut engine = JSEngine::new();

    let result = engine.eval("2 + 3").unwrap();
    assert_eq!(result.to_number(), 5.0);

    let result = engine.eval("10 - 4").unwrap();
    assert_eq!(result.to_number(), 6.0);

    let result = engine.eval("3 * 4").unwrap();
    assert_eq!(result.to_number(), 12.0);

    let result = engine.eval("15 / 3").unwrap();
    assert_eq!(result.to_number(), 5.0);
}

fn test_complex_expressions() {
    let mut engine = JSEngine::new();

    let result = engine.eval("(1 + 2) * 3").unwrap();
    assert_eq!(result.to_number(), 9.0);

    let result = engine.eval("10 / 2 + 3 * 4").unwrap();
    assert_eq!(result.to_number(), 17.0);
}

fn test_variables() {
    let mut engine = JSEngine::new();

    engine.eval("let x = 10").unwrap();
    let result = engine.eval("x + 5").unwrap();
    assert_eq!(result.to_number(), 15.0);

    engine.eval("let y = 20").unwrap();
    let result = engine.eval("x + y").unwrap();
    assert_eq!(result.to_number(), 30.0);
}

fn test_string_operations() {
    let mut engine = JSEngine::new();

    let result = engine.eval(r#""hello" + " " + "world""#).unwrap();
    assert_eq!(result.to_string(), "hello world");

    let result = engine.eval(r#""value: " + 42"#).unwrap();
    assert_eq!(result.to_string(), "value: 42");
}

fn test_boolean_operations() {
    let mut engine = JSEngine::new();

    let result = engine.eval("true && true").unwrap();
    assert_eq!(result.to_boolean(), true);

    let result = engine.eval("true && false").unwrap();
    assert_eq!(result.to_boolean(), false);

    let result = engine.eval("false || true").unwrap();
    assert_eq!(result.to_boolean(), true);

    let result = engine.eval("!true").unwrap();
    assert_eq!(result.to_boolean(), false);
}

fn test_comparisons() {
    let mut engine = JSEngine::new();

    let result = engine.eval("5 > 3").unwrap();
    assert_eq!(result.to_boolean(), true);

    let result = engine.eval("2 < 1").unwrap();
    assert_eq!(result.to_boolean(), false);

    let result = engine.eval("5 === 5").unwrap();
    assert_eq!(result.to_boolean(), true);

    let result = engine.eval("5 === '5'").unwrap();
    assert_eq!(result.to_boolean(), false);

    let result = engine.eval("5 == '5'").unwrap();
    assert_eq!(result.to_boolean(), true);
}

fn test_typeof() {
    let mut engine = JSEngine::new();

    let result = engine.eval("typeof 42").unwrap();
    assert_eq!(result.to_string(), "number");

    let result = engine.eval(r#"typeof "hello""#).unwrap();
    assert_eq!(result.to_string(), "string");

    let result = engine.eval("typeof true").unwrap();
    assert_eq!(result.to_string(), "boolean");

    let result = engine.eval("typeof undefined").unwrap();
    assert_eq!(result.to_string(), "undefined");
}

fn test_vm_execute_literal() {
    let mut engine = JSEngine::new();
    let result = engine.eval("42").unwrap();
    assert_eq!(result, JSValue::from_number(42.0));
}
fn test_vm_execute_addition() {
    let mut engine = JSEngine::new();
    let result = engine.eval("1 + 2").unwrap();
    assert_eq!(result, JSValue::from_number(3.0));
}
fn test_vm_execute_string_concat() {
    let mut engine = JSEngine::new();
    let source = r#""hello" + " " + "world""#;
    let result = engine.eval(source).unwrap();
    assert_eq!(result, JSValue::from_string("hello world".to_string()));
}
fn test_vm_execute_comparison() {
    let mut engine = JSEngine::new();
    let result = engine.eval("5 > 3").unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}
fn test_vm_execute_variable() {
    let mut engine = JSEngine::new();
    let result = engine.eval("let x = 10; x + 5").unwrap();
    assert_eq!(result, JSValue::from_number(15.0));
}

fn test_vm_execute_add_function() {
    let source = r#"
        function add(a,b) { return a + b; }

        add(098,2)
        "#;
    let mut engine = JSEngine::new();
    let result = engine.eval(source).unwrap();
    assert_eq!(result, JSValue::from_number(100.0));
}

/// Test stub for state held by the host (the embedding app)
#[derive(Debug, PartialEq)]
struct JsHost {
    count: u32,
}

fn native_can_read_host_data() {
    let host: Rc<RefCell<dyn std::any::Any>> = Rc::new(RefCell::new(JsHost { count: 42 }));

    let mut engine = JSEngine::new();
    engine.set_host(host);

    // native function that downcasts vm.host and reads a value from it
    let native = JSValue::from_native_function(|vm: &mut VM, _args| {
        let host = vm
            .host
            .as_ref()
            .ok_or_else(|| JSError::InternalError("host is not set".to_string()))?;
        let borrowed = host.borrow();
        let js_host = (&*borrowed as &dyn std::any::Any)
            .downcast_ref::<JsHost>()
            .ok_or_else(|| JSError::InternalError("wrong host type".to_string()))?;
        Ok(JSValue::from_number(js_host.count as f64))
    });

    engine
        .global_mut()
        .borrow_mut()
        .set("readHost".to_string(), native);

    let result = engine.eval("readHost()").unwrap();
    assert_eq!(result, JSValue::from_number(42.0));
}

fn host_is_none_by_default() {
    let mut engine = JSEngine::new();

    // when host is not set, the native returns false
    let native = JSValue::from_native_function(|vm: &mut VM, _args| match &vm.host {
        Some(_) => Ok(JSValue::from_bool(true)),
        None => Ok(JSValue::from_bool(false)),
    });

    engine
        .global_mut()
        .borrow_mut()
        .set("hasHost".to_string(), native);

    let result = engine.eval("hasHost()").unwrap();
    assert_eq!(result, JSValue::from_bool(false));
}

fn host_data_can_be_mutated_by_native() {
    let host: Rc<RefCell<dyn std::any::Any>> = Rc::new(RefCell::new(JsHost { count: 0 }));

    let mut engine = JSEngine::new();
    engine.set_host(host);

    let native = JSValue::from_native_function(|vm: &mut VM, _args| {
        let host = vm
            .host
            .as_ref()
            .ok_or_else(|| JSError::InternalError("host is not set".to_string()))?;
        let mut borrowed = host.borrow_mut();
        let js_host = (&mut *borrowed as &mut dyn std::any::Any)
            .downcast_mut::<JsHost>()
            .ok_or_else(|| JSError::InternalError("wrong host type".to_string()))?;
        js_host.count += 1;
        Ok(JSValue::from_number(js_host.count as f64))
    });

    engine
        .global_mut()
        .borrow_mut()
        .set("bumpHost".to_string(), native);

    assert_eq!(
        engine.eval("bumpHost()").unwrap(),
        JSValue::from_number(1.0)
    );
    assert_eq!(
        engine.eval("bumpHost()").unwrap(),
        JSValue::from_number(2.0)
    );
}

fn enqueue_nested(vm: &mut pixi_byte::vm::VM, _args: Vec<JSValue>) -> pixi_byte::JSResult<JSValue> {
    let nested = vm.global_object.borrow().get("nested");
    vm.enqueue_job(nested, JSValue::undefined(), Vec::new());
    Ok(JSValue::undefined())
}

fn jobs_run_in_fifo_order_and_drain_nested_jobs() {
    let mut engine = JSEngine::new();
    engine.eval(r#"let order = "";"#).unwrap();
    let first = engine
        .eval(r#"(function () { order = order + "first"; })"#)
        .unwrap();
    let nested = engine
        .eval(r#"(function () { order = order + "-nested"; })"#)
        .unwrap();
    let second = engine
        .eval(r#"(function () { order = order + "-second"; })"#)
        .unwrap();
    engine
        .global_mut()
        .borrow_mut()
        .set("nested".to_string(), nested);
    engine.enqueue_job(first, JSValue::undefined(), Vec::new());
    engine.enqueue_job(
        JSValue::from_native_function(enqueue_nested),
        JSValue::undefined(),
        Vec::new(),
    );
    engine.enqueue_job(second, JSValue::undefined(), Vec::new());

    engine.run_jobs().unwrap();

    assert_eq!(
        engine.eval("order").unwrap().to_console_string(),
        "first-second-nested"
    );
}

fn inline_string_values_drop_cleanly_in_debug_builds() {
    let mut engine = JSEngine::new();
    // インライン文字列（<=5 バイト）がスタックに push/pop され、
    // 通常の drop 経路を通るワークロード。
    let result = engine
        .eval(
            r#"
            let s = "foo.bar.baz.";
            let parts = s.split(".");
            let total = "";
            for (const p of parts) { total += p; }
            let c = s[0];
            let t = c + "x";
            t.length + parts.length
        "#,
        )
        .unwrap();
    // t = "f" + "x" → length 2、split 結果は 4 要素（末尾の空文字列含む）。
    assert_eq!(result.to_string(), "6");
}

#[test]
fn suite_value_tests_1() {
    test_jsvalue_to_string();
    test_jsvalue_to_number();
    test_jsvalue_to_boolean();
    test_jsvalue_strict_equals();
}

#[test]
fn suite_value_tests_2() {
    test_jsvalue_abstract_equals();
}

#[test]
fn suite_integration_tests_1() {
    test_basic_arithmetic();
    test_complex_expressions();
    test_variables();
    test_string_operations();
}

#[test]
fn suite_integration_tests_2() {
    test_boolean_operations();
    test_comparisons();
    test_typeof();
}

#[test]
fn suite_vm_tests_1() {
    test_vm_execute_literal();
    test_vm_execute_addition();
    test_vm_execute_string_concat();
    test_vm_execute_comparison();
}

#[test]
fn suite_vm_tests_2() {
    test_vm_execute_variable();
    test_vm_execute_add_function();
}

#[test]
fn suite_host() {
    native_can_read_host_data();
    host_is_none_by_default();
    host_data_can_be_mutated_by_native();
}

#[test]
fn suite_jobs() {
    jobs_run_in_fifo_order_and_drain_nested_jobs();
}

#[test]
fn suite_inline_str_debug_drop() {
    inline_string_values_drop_cleanly_in_debug_builds();
}
