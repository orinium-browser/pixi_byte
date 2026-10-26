use pixi_byte::{JSEngine, JSValue};

#[test]
fn numbers_support_radix_to_string() {
    let mut engine = JSEngine::new();
    assert_eq!(
        engine.eval("(255).toString(16)").unwrap(),
        JSValue::from_string("ff".to_string())
    );
    assert_eq!(
        engine.eval("(35).toString(36)").unwrap(),
        JSValue::from_string("z".to_string())
    );
}

#[test]
fn math_exposes_functions_used_by_react() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            Math.min(8, 3, 5) === 3 &&
            Math.max(8, 3, 5) === 8 &&
            Math.abs(-4) === 4 &&
            Math.pow(2, 3) === 8 &&
            Math.PI > 3.14 &&
            Math.clz32(1) === 31 &&
            Math.ceil(1.2) === 2 &&
            Math.floor(1.8) === 1 &&
            Math.log(8) / Math.LN2 > 2.99 &&
            Math.log(8) / Math.LN2 < 3.01;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

#[test]
fn math_random_returns_a_fraction() {
    let mut engine = JSEngine::new();
    let result = engine.eval("Math.random()").unwrap();
    let Some(value) = result.as_number() else {
        panic!("Math.random did not return a number");
    };
    assert!((0.0..1.0).contains(&value));
}

#[test]
fn number_formatting_and_static_helpers() {
    let mut engine = pixi_byte::JSEngine::new();
    let result = engine
        .eval(
            r#"
            [
                (1234.5678).toFixed(2),
                (12345).toExponential(2),
                (123.456).toPrecision(4),
                Number.isInteger(4),
                Number.isSafeInteger(2 ** 53),
                Number.MAX_SAFE_INTEGER,
                Number.isNaN(Number.NaN)
            ].join("|");
            "#,
        )
        .unwrap();
    assert_eq!(
        result,
        pixi_byte::JSValue::from_string(
            "1234.57|1.23e+4|123.5|true|false|9007199254740991|true".to_string()
        )
    );
}

#[test]
fn string_unicode_and_padding_helpers() {
    let mut engine = pixi_byte::JSEngine::new();
    let result = engine
        .eval(
            r#"
            [
                "\u{1F600}".codePointAt(0),
                "abc".padStart(5, "xy"),
                "abc".padEnd(5, "xy"),
                "ab".repeat(3),
                "hello".at(-1),
                "  hi  ".trimStart(),
                String.fromCodePoint(0x1F600).length
            ].join("|");
            "#,
        )
        .unwrap();
    assert_eq!(
        result,
        pixi_byte::JSValue::from_string("128512|xyabc|abcxy|ababab|o|hi  |2".to_string())
    );
}

#[test]
fn array_fill_copy_within_and_find_last() {
    let mut engine = pixi_byte::JSEngine::new();
    let result = engine
        .eval(
            r#"
            const a = [1, 2, 3, 4];
            a.fill(9, 1, 3);
            const b = [1, 2, 3, 4, 5];
            b.copyWithin(0, 3);
            const c = [5, 6, 7, 8];
            [
                a.join(","),
                b.join(","),
                c.findLast(function (n) { return n % 2 === 0; }),
                c.findLastIndex(function (n) { return n % 2 === 0; })
            ].join("|");
            "#,
        )
        .unwrap();
    assert_eq!(
        result,
        pixi_byte::JSValue::from_string("1,9,9,4|4,5,3,4,5|8|3".to_string())
    );
}
