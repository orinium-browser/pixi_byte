use pixi_byte::JSEngine;
use pixi_byte::error::JSError;
use pixi_byte::value::JSValue;
use pixi_byte::vm::VM;
use std::cell::RefCell;
use std::rc::Rc;

/// Test stub for state held by the host (the embedding app)
#[derive(Debug, PartialEq)]
struct JsHost {
    count: u32,
}

#[test]
fn native_can_read_host_data() {
    let host: Rc<RefCell<dyn std::any::Any>> = Rc::new(RefCell::new(JsHost { count: 42 }));

    let mut engine = JSEngine::new();
    engine.set_host(host);

    // native function that downcasts vm.host and reads a value from it
    let native = JSValue::NativeFunction(|vm: &mut VM, _args| {
        let host = vm
            .host
            .as_ref()
            .ok_or_else(|| JSError::InternalError("host is not set".to_string()))?;
        let borrowed = host.borrow();
        let js_host = (&*borrowed as &dyn std::any::Any)
            .downcast_ref::<JsHost>()
            .ok_or_else(|| JSError::InternalError("wrong host type".to_string()))?;
        Ok(JSValue::Number(js_host.count as f64))
    });

    engine
        .global_mut()
        .borrow_mut()
        .set("readHost".to_string(), native);

    let result = engine.eval("readHost()").unwrap();
    assert_eq!(result, JSValue::Number(42.0));
}

#[test]
fn host_is_none_by_default() {
    let mut engine = JSEngine::new();

    // when host is not set, the native returns false
    let native = JSValue::NativeFunction(|vm: &mut VM, _args| match &vm.host {
        Some(_) => Ok(JSValue::Boolean(true)),
        None => Ok(JSValue::Boolean(false)),
    });

    engine
        .global_mut()
        .borrow_mut()
        .set("hasHost".to_string(), native);

    let result = engine.eval("hasHost()").unwrap();
    assert_eq!(result, JSValue::Boolean(false));
}

#[test]
fn host_data_can_be_mutated_by_native() {
    let host: Rc<RefCell<dyn std::any::Any>> = Rc::new(RefCell::new(JsHost { count: 0 }));

    let mut engine = JSEngine::new();
    engine.set_host(host);

    let native = JSValue::NativeFunction(|vm: &mut VM, _args| {
        let host = vm
            .host
            .as_ref()
            .ok_or_else(|| JSError::InternalError("host is not set".to_string()))?;
        let mut borrowed = host.borrow_mut();
        let js_host = (&mut *borrowed as &mut dyn std::any::Any)
            .downcast_mut::<JsHost>()
            .ok_or_else(|| JSError::InternalError("wrong host type".to_string()))?;
        js_host.count += 1;
        Ok(JSValue::Number(js_host.count as f64))
    });

    engine
        .global_mut()
        .borrow_mut()
        .set("bumpHost".to_string(), native);

    assert_eq!(engine.eval("bumpHost()").unwrap(), JSValue::Number(1.0));
    assert_eq!(engine.eval("bumpHost()").unwrap(), JSValue::Number(2.0));
}
