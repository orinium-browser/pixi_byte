use pixi_byte::JSEngine;
use pixi_byte::value::JSValue;

/// Verifies the host can register an eval'd function expression on the global
/// object and invoke it from Rust.
#[test]
fn rust_invokes_js_function_from_global() {
    let mut engine = JSEngine::new();

    let add = engine
        .eval("(function add(a, b) { return a + b; })")
        .unwrap();
    engine
        .global_mut()
        .borrow_mut()
        .set("add".to_string(), add.clone());

    let result = engine
        .call(
            add,
            JSValue::undefined(),
            vec![JSValue::from_number(1.0), JSValue::from_number(2.0)],
        )
        .unwrap();

    assert_eq!(result, JSValue::from_number(3.0));
}

#[test]
fn rust_invokes_js_function_returning_object() {
    let mut engine = JSEngine::new();

    let make = engine
        .eval("(function make() { return { x: 10 }; })")
        .unwrap();

    let result = engine.call(make, JSValue::undefined(), Vec::new()).unwrap();

    if let Some(obj_ref) = result.as_object() {
        assert_eq!(obj_ref.borrow().get("x"), JSValue::from_number(10.0));
    } else {
        panic!("expected object, got {:?}", result);
    }
}

#[test]
fn rust_invokes_function_with_this() {
    let mut engine = JSEngine::new();

    let get_x = engine.eval("(function getX() { return this.x; })").unwrap();

    let mut this_obj = pixi_byte::value::jsobject::JSObject::new();
    this_obj.set("x".to_string(), JSValue::from_number(7.0));
    let this_rc = std::rc::Rc::new(std::cell::RefCell::new(this_obj));

    let result = engine
        .call(get_x, JSValue::from_object(this_rc), Vec::new())
        .unwrap();

    assert_eq!(result, JSValue::from_number(7.0));
}

#[test]
fn rust_invokes_native_function() {
    let mut engine = JSEngine::new();

    // natives receive `[this, ...args]`, so skip the first element when summing
    let native = JSValue::from_native_function(|_vm, args| {
        let sum: f64 = args.iter().skip(1).map(JSValue::to_number).sum();
        Ok(JSValue::from_number(sum))
    });

    engine
        .global_mut()
        .borrow_mut()
        .set("sumNums".to_string(), native);

    let sum_nums = engine.global_mut().borrow().get("sumNums");

    let result = engine
        .call(
            sum_nums,
            JSValue::undefined(),
            vec![
                JSValue::from_number(1.0),
                JSValue::from_number(2.0),
                JSValue::from_number(3.0),
            ],
        )
        .unwrap();

    assert_eq!(result, JSValue::from_number(6.0));
}

#[test]
fn call_with_non_function_returns_error() {
    let mut engine = JSEngine::new();

    let result = engine.call(JSValue::from_number(42.0), JSValue::undefined(), Vec::new());

    assert!(result.is_err());
}
