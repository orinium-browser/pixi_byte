use pixi_byte::{JSEngine, JSValue};

#[test]
fn exposes_ecmascript_numeric_globals() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            Infinity === 1 / 0 && isNaN(NaN) &&
                parseInt("  -0x10px", 0) === -16 &&
                Number.parseInt("11", 2) === 3 &&
                parseFloat("  3.25px") === 3.25 &&
                Number.parseFloat("1.5e2rest") === 150 &&
                BigInt(4) === 4n && typeof 4n === "bigint";
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_bool(true));
}

#[test]
fn bigint_preserves_arbitrary_precision_operations() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
        let value = 1n << 70n;
        const old = value++;
        old === 1180591620717411303424n &&
            value === 1180591620717411303425n &&
            (value & 255n) === 1n &&
            (1n << 70n) > (1n << 69n);
    "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}
