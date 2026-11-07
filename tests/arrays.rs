use pixi_byte::JSEngine;
use pixi_byte::value::{JSArray, JSValue};
use pixi_byte::vm::VM;

fn test_array_create() {
    let arr = JSArray::new();
    assert_eq!(arr.length(), 0);
}
fn test_array_push_pop() {
    let mut arr = JSArray::new();
    arr.push(JSValue::from_number(1.0));
    arr.push(JSValue::from_number(2.0));
    arr.push(JSValue::from_number(3.0));
    assert_eq!(arr.length(), 3);
    assert_eq!(arr.pop(), JSValue::from_number(3.0));
    assert_eq!(arr.length(), 2);
}
fn test_array_get_set() {
    let mut arr = JSArray::new();
    arr.set(0, JSValue::from_string("first".to_string()));
    arr.set(2, JSValue::from_string("third".to_string()));
    assert_eq!(arr.get(0), JSValue::from_string("first".to_string()));
    assert_eq!(arr.get(1), JSValue::undefined());
    assert_eq!(arr.get(2), JSValue::from_string("third".to_string()));
}
fn test_array_shift_unshift() {
    let mut arr = JSArray::from_vec(vec![JSValue::from_number(2.0), JSValue::from_number(3.0)]);
    arr.unshift(JSValue::from_number(1.0));
    assert_eq!(arr.length(), 3);
    let first = arr.shift();
    assert_eq!(first, JSValue::from_number(1.0));
}
fn test_array_from_vec() {
    let arr = JSArray::from_vec(vec![
        JSValue::from_string("a".to_string()),
        JSValue::from_string("b".to_string()),
    ]);
    assert_eq!(arr.length(), 2);
}

fn array_for_each_is_generic_through_function_call() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let total = 0;
            const forEach = [].forEach;
            forEach.call([1, 2, 3], function (value) { total += value; });
            total;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(6.0));
}

fn array_for_each_call_keeps_receiver_before_callback_this_arg() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let seen = 0;
            const forEach = [].forEach;
            forEach.call([7], function (value) { seen = value; }, "callback this");
            seen;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_number(7.0));
}

fn array_for_each_is_generic_over_strings() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let joined = "";
            [].forEach.call("abc", function (value) { joined += value; });
            joined;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("abc".to_string()));
}

fn array_from_supports_array_like_iterables_and_mapping() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const arrayLike = { 0: "a", 1: "b", length: 2 };
            Array.from(arrayLike).join("") === "ab"
                && Array.from(new Set([1, 2]), value => value * 2).join(",") === "2,4";
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn array_order_search_and_flattening_methods_work() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = [3, 1, 2];
            values.sort((left, right) => left - right);
            const flattened = [1, [2, [3]]].flat(2);
            const mapped = [1, 2].flatMap(value => [value, value * 10]);
            values.join(",") === "1,2,3"
                && values.find(value => value > 1) === 2
                && values.findIndex(value => value === 3) === 2
                && flattened.join(",") === "1,2,3"
                && mapped.join(",") === "1,10,2,20"
                && values.reverse().at(-1) === 1;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn array_mutation_methods_update_values_and_length() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = [2, 3];
            values.unshift(1);
            const shifted = values.shift();
            const removed = values.splice(1, 1, 4, 5);
            shifted === 1 && removed[0] === 3 && values.join("-") === "2-4-5";
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_bool(true));
}

fn array_read_methods_return_expected_results() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = [1, 2, 3];
            const copy = values.slice(1).concat([4], 5);
            copy.join(",") === "2,3,4,5" && copy.indexOf(4) === 2 && copy.includes(5);
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_bool(true));
}

fn array_for_each_calls_back_in_index_order() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let result = "";
            ["a", "b", "c"].forEach(function (value, index) {
                result += index + value;
            });
            result;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_string("0a1b2c".to_string()));
}

fn array_iteration_methods_support_component_data_transforms() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = [1, 2, 3, 4];
            const mapped = values.map(function (value, index) { return value + index; });
            const filtered = mapped.filter(function (value) { return value > 3; });
            const total = filtered.reduce(function (sum, value) { return sum + value; }, 0);
            const reversed = values.reduceRight(function (text, value) {
                return text + value;
            }, "");
            mapped.join(",") + ":" + filtered.join(",") + ":" + total
                + ":" + values.some(function (value) { return value === 3; })
                + ":" + values.every(function (value) { return value > 0; })
                + ":" + reversed;
            "#,
        )
        .unwrap();

    assert_eq!(
        result,
        JSValue::from_string("1,3,5,7:5,7:12:true:true:4321".to_string())
    );
}

fn array_iterators_expose_keys_values_and_entries() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = ["a", "b"];
            const entries = [];
            for (const [index, value] of values.entries()) {
                entries.push(index + value);
            }
            entries.join(",") === "0a,1b" &&
                values.keys().next().value === 0 &&
                values.values().next().value === "a" &&
                values[Symbol.iterator]().next().value === "a";
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn array_prototype_methods_are_not_enumerable() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = ["first"];
            const keys = [];
            for (const key in values) keys.push(key);
            function argumentKeys() {
                const result = [];
                for (const key in arguments) result.push(key);
                return result.join(",");
            }
            keys.join(",") + ":" + argumentKeys("value");
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("0:0".to_string()));
}

fn array_join_uses_custom_object_to_string() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const selector = { toString() { return ".py-2"; } };
            [selector, "padding:0.5rem"].join("{") + "}";
            "#,
        )
        .unwrap();
    assert_eq!(
        result,
        JSValue::from_string(".py-2{padding:0.5rem}".to_string())
    );
}

fn array_to_string_delegates_to_join() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const selector = { toString() { return ".py-2"; } };
            [selector, "padding:0.5rem"].toString();
            "#,
        )
        .unwrap();
    assert_eq!(
        result,
        JSValue::from_string(".py-2,padding:0.5rem".to_string())
    );
}

fn array_to_string_uses_an_overridden_join_method() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = [1, 2];
            values.join = function() { return "custom"; };
            values.toString();
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("custom".to_string()));
}

fn array_to_string_falls_back_when_join_is_not_callable() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = [1, 2];
            values.join = 1;
            values.toString();
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("[object Array]".to_string()));
}

fn reduce_callbacks_capture_each_invocation_parameters() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const callbacks = [["px", "horizontal"], ["py", "vertical"]].reduce(
                (result, [name, value]) => Object.assign(result, { [name]: () => name + ":" + value }),
                {}
            );
            callbacks.px() + "," + callbacks.py();
            "#,
        )
        .unwrap();
    assert_eq!(
        result,
        JSValue::from_string("px:horizontal,py:vertical".to_string())
    );
}

fn array_constructor_creates_a_requested_length() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = Array(3);
            Array.isArray(values) && values.length === 3;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn array_constructor_accepts_elements_and_new() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const first = Array("a", "b");
            const second = new Array(1, 2);
            first.join("") === "ab" && second.join("") === "12";
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn array_literals_are_identified_as_arrays() {
    let mut engine = JSEngine::new();
    assert_eq!(
        engine.eval("Array.isArray([])").unwrap(),
        JSValue::from_bool(true)
    );
    assert_eq!(
        engine.eval("Array.isArray({ length: 0 })").unwrap(),
        JSValue::from_bool(false)
    );
}

fn array_literals_inherit_array_prototype_methods() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = [1];
            values.push(2, 3);
            values.length === 3 && values.pop() === 3;
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_bool(true));
}

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

fn test_array_prototype_push_pop_native() {
    let mut vm = VM::new();
    let global = vm.global_object.clone();

    // Array が登録されている
    let array_ctor = global.borrow().get("Array");
    let ctor_ref = array_ctor
        .as_object()
        .unwrap_or_else(|| panic!("Array global not object"));
    // prototype がある
    let proto = ctor_ref.borrow().get("prototype");
    let proto_ref = proto
        .as_object()
        .unwrap_or_else(|| panic!("Array.prototype not object"));
    // push/pop がネイティブで登録されている
    let push = proto_ref.borrow().get("push");
    let pop = proto_ref.borrow().get("pop");
    assert!(push.is_native_function(), "push not native");
    assert!(pop.is_native_function(), "pop not native");

    // 簡単な配列オブジェクトを作り、push/pop を呼び出す
    let mut arr = pixi_byte::value::jsobject::JSObject::new();
    arr.set("length".to_string(), JSValue::from_number(0.0));
    let arr_obj = JSValue::from_object(std::rc::Rc::new(std::cell::RefCell::new(arr)));

    // CallMethod semantics expect stack: ..., object, property, arg1,arg2..., and opcode will inject receiver.
    // We'll invoke the native function directly via CallMethod path simulation: fetch method and call with receiver.
    if let Some(f) = proto_ref.borrow().get("push").as_native_function() {
        let res = f(&mut vm, vec![arr_obj.clone(), JSValue::from_number(10.0)]).unwrap();
        assert_eq!(res, JSValue::from_number(1.0));
    } else {
        panic!("push not callable");
    }

    if let Some(f) = proto_ref.borrow().get("pop").as_native_function() {
        let res = f(&mut vm, vec![arr_obj.clone()]).unwrap();
        assert_eq!(res, JSValue::from_number(10.0));
    } else {
        panic!("pop not callable");
    }
}

#[test]
fn suite_array_tests_1() {
    test_array_create();
    test_array_push_pop();
    test_array_get_set();
    test_array_shift_unshift();
}

#[test]
fn suite_array_tests_2() {
    test_array_from_vec();
}

#[test]
fn suite_array_methods_1() {
    array_for_each_is_generic_through_function_call();
    array_for_each_call_keeps_receiver_before_callback_this_arg();
    array_for_each_is_generic_over_strings();
    array_from_supports_array_like_iterables_and_mapping();
}

#[test]
fn suite_array_methods_2() {
    array_order_search_and_flattening_methods_work();
    array_mutation_methods_update_values_and_length();
    array_read_methods_return_expected_results();
    array_for_each_calls_back_in_index_order();
}

#[test]
fn suite_array_methods_3() {
    array_iteration_methods_support_component_data_transforms();
    array_iterators_expose_keys_values_and_entries();
    array_prototype_methods_are_not_enumerable();
    array_join_uses_custom_object_to_string();
}

#[test]
fn suite_array_methods_4() {
    array_to_string_delegates_to_join();
    array_to_string_uses_an_overridden_join_method();
    array_to_string_falls_back_when_join_is_not_callable();
    reduce_callbacks_capture_each_invocation_parameters();
}

#[test]
fn suite_array_constructor() {
    array_constructor_creates_a_requested_length();
    array_constructor_accepts_elements_and_new();
}

#[test]
fn suite_array_identity() {
    array_literals_are_identified_as_arrays();
    array_literals_inherit_array_prototype_methods();
}

#[test]
fn suite_array_element_storage_1() {
    element_storage_reads_and_writes();
    setting_undefined_is_still_a_present_element();
    delete_creates_a_hole();
    sparse_write_beyond_length_preserves_holes();
}

#[test]
fn suite_array_element_storage_2() {
    property_map_overrides_element_storage();
    delete_after_define_property_does_not_resurrect_element();
    seal_and_freeze_still_block_element_writes();
    large_arrays_keep_element_order_in_keys();
}

#[test]
fn suite_array_element_storage_3() {
    iterator_and_methods_skip_holes();
}

#[test]
fn suite_array_builtin_tests() {
    test_array_prototype_push_pop_native();
}
