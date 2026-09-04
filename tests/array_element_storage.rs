//! Regression tests for the dense element storage inside `JSObject`.
//!
//! Arrays keep their elements in a dedicated dense `Vec<Option<JSValue>>`
//! instead of expanding each index into the string-keyed property map.
//! These tests lock down the observable invariants of that storage:
//! `Some(value)` == present element (even when the value is `undefined`),
//! `None` == hole (after `delete`), and property-map overrides still win.

use pixi_byte::{JSEngine, JSValue};

#[test]
fn element_storage_reads_and_writes() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = ["a", "b", "c"];
            values[1] = "B";
            values[0] + values[1] + values[2] + ":" + values.length;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("aBc:3".to_string()));
}

#[test]
fn setting_undefined_is_still_a_present_element() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = [1, 2];
            values[0] = undefined;
            (0 in values) && values[0] === undefined && values.length === 2;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

#[test]
fn delete_creates_a_hole() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = ["a", "b", "c"];
            const removed = delete values[1];
            const before = 1 in values;
            const after = values[1];
            removed && !before && after === undefined && values.length === 3
                && Object.keys(values).join(",") === "0,2";
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

#[test]
fn sparse_write_beyond_length_preserves_holes() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = ["a"];
            values[4] = "e";
            values.length === 5
                && !(2 in values)
                && values[2] === undefined
                && values[4] === "e"
                && Object.keys(values).join(",") === "0,4";
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

#[test]
fn property_map_overrides_element_storage() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = ["a", "b"];
            Object.defineProperty(values, "1", { value: "B", writable: false });
            values[1] === "B";
            const readOnly = Object.getOwnPropertyDescriptor(values, "1");
            // 既存要素の再定義なので configurable/enumerable は既存値 (true) を継承
            readOnly.writable === false && readOnly.configurable === true
                && readOnly.enumerable === true
                && Object.getOwnPropertyDescriptor(values, "0").writable === true;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

#[test]
fn delete_after_define_property_does_not_resurrect_element() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = ["a", "b", "c"];
            Object.defineProperty(values, "1", { value: "B", configurable: true, writable: true, enumerable: true });
            const removed = delete values[1];
            removed && !(1 in values) && values[1] === undefined;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

#[test]
fn seal_and_freeze_still_block_element_writes() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const frozen = [1, 2, 3];
            Object.freeze(frozen);
            frozen[0] = 99;
            frozen[3] = 4;
            const frozenBlocked = frozen[0] === 1 && frozen[3] === undefined && !(3 in frozen);
            const frozenDesc = Object.getOwnPropertyDescriptor(frozen, "0");
            const frozenHard = frozenDesc.writable === false && frozenDesc.configurable === false;

            const sealed = [1, 2, 3];
            Object.seal(sealed);
            sealed[0] = 42;
            sealed[3] = 4;
            const sealedOk = sealed[0] === 42 && sealed[3] === undefined && !(3 in sealed);
            const sealedDesc = Object.getOwnPropertyDescriptor(sealed, "0");
            const sealedHard = sealedDesc.writable === true && sealedDesc.configurable === false;

            frozenBlocked && frozenHard && sealedOk && sealedHard;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

#[test]
fn large_arrays_keep_element_order_in_keys() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = new Array(1000);
            for (let i = 0; i < values.length; i += 1) {
                values[i] = i;
            }
            const keys = Object.keys(values);
            keys.length === 1000 && keys[0] === "0" && keys[999] === "999";
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

#[test]
fn iterator_and_methods_skip_holes() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = ["a", "b", "c"];
            delete values[1];
            let visited = "";
            values.forEach(function (value, index) {
                visited += index + value;
            });
            visited === "0a2c" && values.join("-") === "a--c";
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}
