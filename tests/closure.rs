use pixi_byte::JSEngine;

#[test]
fn test_simple_closure() {
    let mut engine = JSEngine::new();
    let src = r#"
        function makeAdder(x) {
            return function(y) {
                return x + y;
            };
        }
        var add5 = makeAdder(5);
        add5(3);
    "#;

    let res = engine.eval(src).expect("eval failed");
    match res.as_number() {
        Some(n) => assert_eq!(n, 8.0),
        _ => panic!("unexpected result: {}", res),
    }
}

#[test]
fn test_closure_capture_after_mutation() {
    let mut engine = JSEngine::new();
    let src = r#"
        function counter() {
            var c = 0;
            return function() {
                c = c + 1;
                return c;
            };
        }
        var inc = counter();
        inc();
        inc();
    "#;

    let res = engine.eval(src).expect("eval failed");
    match res.as_number() {
        Some(n) => assert_eq!(n, 2.0),
        _ => panic!("unexpected result: {}", res),
    }
}
