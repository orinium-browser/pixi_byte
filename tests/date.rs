use pixi_byte::{JSEngine, JSValue};

#[test]
fn date_calendar_fields_and_set_date_use_epoch_milliseconds() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const date = new Date(0);
            const before = date.getFullYear() + ":" + date.getMonth() + ":" + date.getDate();
            date.setDate(2);
            before + ":" + date.getDate() + ":" + date.getTime();
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::String("1970:0:1:2:86400000".to_string()));
}

#[test]
fn date_now_returns_epoch_milliseconds() {
    let mut engine = JSEngine::new();
    let result = engine.eval("Date.now()").unwrap();
    let JSValue::Number(milliseconds) = result else {
        panic!("Date.now() must return a number");
    };
    assert!(milliseconds > 0.0);
}

#[test]
fn date_is_constructible_and_exposes_its_epoch_time() {
    let mut engine = JSEngine::new();
    let result = engine.eval("new Date(1234).getTime()").unwrap();
    assert_eq!(result, JSValue::Number(1234.0));
}

#[test]
fn date_value_of_returns_the_constructed_time() {
    let mut engine = JSEngine::new();
    let result = engine.eval("new Date(42).valueOf()").unwrap();
    assert_eq!(result, JSValue::Number(42.0));
}

#[test]
fn date_set_year_and_set_full_year_update_the_calendar_year() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const legacy = new Date(0);
            legacy.setYear(99);
            const modern = new Date(0);
            modern.setFullYear(2024);
            legacy.getFullYear() + ":" + modern.getFullYear();
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::String("1999:2024".to_string()));
}

#[test]
fn date_to_utc_string_uses_http_date_shape() {
    let mut engine = JSEngine::new();
    assert_eq!(
        engine.eval("new Date(0).toUTCString()").unwrap(),
        JSValue::String("Thu, 01 Jan 1970 00:00:00 GMT".to_string())
    );
}

#[test]
fn date_parse_is_present_and_rejects_unsupported_text_with_nan() {
    let mut engine = JSEngine::new();
    assert_eq!(
        engine.eval("Date.parse('1234')").unwrap(),
        JSValue::Number(1234.0)
    );
    let JSValue::Number(value) = engine.eval("Date.parse('not a date')").unwrap() else {
        panic!("Date.parse must return a number");
    };
    assert!(value.is_nan());
}
