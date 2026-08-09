use pixi_byte::{JSEngine, JSValue};

fn enqueue_nested(vm: &mut pixi_byte::vm::VM, _args: Vec<JSValue>) -> pixi_byte::JSResult<JSValue> {
    let nested = vm.global_object.borrow().get("nested");
    vm.enqueue_job(nested, JSValue::Undefined, Vec::new());
    Ok(JSValue::Undefined)
}

#[test]
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
    engine.enqueue_job(first, JSValue::Undefined, Vec::new());
    engine.enqueue_job(
        JSValue::NativeFunction(enqueue_nested),
        JSValue::Undefined,
        Vec::new(),
    );
    engine.enqueue_job(second, JSValue::Undefined, Vec::new());

    engine.run_jobs().unwrap();

    assert_eq!(
        engine.eval("order").unwrap().to_console_string(),
        "first-second-nested"
    );
}
