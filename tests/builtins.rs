use pixi_byte::{JSEngine, JSValue};

fn string_primitives_expose_common_prototype_methods() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const parts = " Alpha,Beta ".trim().toLowerCase().split(",");
            parts[0] === "alpha" && parts[1] === "beta" &&
                "hello".substring(1, 4) === "ell" &&
                "tailwind".startsWith("wind", 4) &&
                "tailwind.css".endsWith("wind", 8);
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn concat_appends_each_argument_after_string_conversion() {
    let mut engine = JSEngine::new();
    let result = engine.eval(r#""Scratch".concat(" ", 3, true)"#).unwrap();
    assert_eq!(result, JSValue::from_string("Scratch 3true".to_string()));
}

fn replace_supports_global_regexes_templates_and_callbacks() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const escaped = "a=b:c".replace(/[=:]/g, "[$&]");
            const callback = "a1b2".replace(/[0-9]/g, function (match) {
                return "-" + match;
            });
            escaped === "a[=]b[:]c" && callback === "a-1b-2";
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn string_constructor_and_from_char_code_are_callable() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(r#"String(42) + String.fromCharCode(65, 66)"#)
        .unwrap();
    assert_eq!(result, JSValue::from_string("42AB".to_string()));
}

fn char_code_at_uses_utf16_code_units() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(r#""A😀".charCodeAt(0) === 65 && "A😀".charCodeAt(1) === 55357"#)
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn last_index_of_honors_the_search_position() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            "text-blue-600".lastIndexOf("-") === 9 &&
                "text-blue-600".lastIndexOf("-", 8) === 4 &&
                "A😀B😀".lastIndexOf("😀") === 4 &&
                "abc".lastIndexOf("", 2) === 2;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn split_with_shared_slices_handles_multibyte_and_limits() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const a = "カタカナ,ひらがな,漢字".split(",");
            const b = "A😀B😀C".split("");
            const c = "x,y,z".split(",", 2);
            const d = "🐱🐶🐭".split("");
            a[0] === "カタカナ" && a[1] === "ひらがな" && a[2] === "漢字" &&
                b.join("-") === "A-😀-B-😀-C" &&
                c.length === 2 && c[0] === "x" && c[1] === "y" &&
                d.join("|") === "🐱|🐶|🐭";
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn split_with_shared_slices_matches_regex_path() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const parts = "a1b22c333".split(/[0-9]+/);
            parts.join("-") === "a-b-c-";
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

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

fn math_random_returns_a_fraction() {
    let mut engine = JSEngine::new();
    let result = engine.eval("Math.random()").unwrap();
    let Some(value) = result.as_number() else {
        panic!("Math.random did not return a number");
    };
    assert!((0.0..1.0).contains(&value));
}

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

fn boolean_converts_values_using_javascript_truthiness() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval("Boolean(0) === false && Boolean('') === false && Boolean('x') === true")
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn symbol_for_returns_stable_registry_values() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            Symbol.for("react.element") === Symbol.for("react.element") &&
            Symbol.for("react.element") !== Symbol.for("react.portal") &&
            Symbol.iterator === "@@iterator";
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn symbol_call_returns_unique_values_and_well_known_symbols() {
    let mut engine = pixi_byte::JSEngine::new();
    let result = engine
        .eval(
            r#"
            const first = Symbol("key");
            const second = Symbol("key");
            first !== second && Symbol.toStringTag === "@@toStringTag";
            "#,
        )
        .unwrap();
    assert_eq!(result, pixi_byte::JSValue::from_bool(true));
}

fn error_is_callable_and_exposes_standard_properties() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const error = Error("failed");
            error.name === "Error" &&
            error.message === "failed" &&
            error.toString() === "Error: failed" &&
            error.stack.includes("failed");
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn thrown_errors_are_preserved_by_catch() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let message = "";
            try {
                throw Error("boom");
            } catch (error) {
                message = error.message;
            }
            message;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("boom".to_string()));
}

fn native_error_subtypes_are_callable_and_constructible() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const type = new TypeError("wrong");
            const range = RangeError("outside");
            type.name === "TypeError" &&
                type.message === "wrong" &&
                type.toString() === "TypeError: wrong" &&
                range.name === "RangeError" &&
                range.toString() === "RangeError: outside";
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn native_error_constructors_have_function_type_and_call_method() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const error = Error.call(null, "called");
            typeof Error === "function" &&
                typeof TypeError === "function" &&
                error.message === "called";
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn function_call_preserves_thrown_error_objects() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let name = "";
            try {
                (function () { throw new TypeError("broken"); }).call(null);
            } catch (error) {
                name = error.name + ": " + error.message;
            }
            name;
            "#,
        )
        .unwrap();
    assert_eq!(
        result,
        JSValue::from_string("TypeError: broken".to_string())
    );
}

fn bigint_literals_are_accepted_by_bitwise_expressions() {
    let mut engine = JSEngine::new();
    let result = engine.eval("((1n | 2n) & 3n) === 3n;").unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn global_is_nan_applies_number_conversion() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            isNaN("not a number") &&
            !isNaN("42") &&
            isNaN(undefined);
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn encode_uri_component_uses_utf8_percent_encoding() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(r#"encodeURIComponent("a b/日本!~*'()");"#)
        .unwrap();

    assert_eq!(
        result,
        JSValue::from_string("a%20b%2F%E6%97%A5%E6%9C%AC!~*'()".to_string())
    );
}

fn json_stringify_quotes_strings_for_selectors() {
    let mut engine = JSEngine::new();
    let result = engine.eval(r#"JSON.stringify("a\"b")"#).unwrap();
    assert_eq!(result, JSValue::from_string(r#""a\"b""#.to_string()));
}

fn json_stringify_serializes_arrays_and_objects() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(r#"JSON.stringify({ value: 1, items: [true, null, "x"] })"#)
        .unwrap();
    let Some(value) = result.as_string() else {
        panic!("JSON.stringify must return a string");
    };
    assert!(value.starts_with('{'));
    assert!(value.ends_with('}'));
    assert!(value.contains(r#""value":1"#));
    assert!(value.contains(r#""items":[true,null,"x"]"#));
}

fn json_parse_builds_nested_values_and_decodes_escapes() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const value = JSON.parse('{"items":[1,true,null,{"name":"Scratch\\n日本"}]}');
            value.items.length + ":" + value.items[0] + ":" +
                value.items[1] + ":" + (value.items[2] === null) + ":" +
                value.items[3].name;
            "#,
        )
        .unwrap();
    assert_eq!(
        result,
        JSValue::from_string("4:1:true:true:Scratch\n日本".to_string())
    );
}

fn json_parse_handles_unicode_pairs_and_strict_number_grammar() {
    let mut engine = JSEngine::new();
    assert_eq!(
        engine.eval(r#"JSON.parse('"\\ud83d\\ude00"')"#).unwrap(),
        JSValue::from_string("😀".to_string())
    );
    assert!(engine.eval("JSON.parse('01')").is_err());
    assert!(engine.eval("JSON.parse('1.')").is_err());
}

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
    assert_eq!(
        result,
        JSValue::from_string("1970:0:1:2:86400000".to_string())
    );
}

fn date_now_returns_epoch_milliseconds() {
    let mut engine = JSEngine::new();
    let result = engine.eval("Date.now()").unwrap();
    let Some(milliseconds) = result.as_number() else {
        panic!("Date.now() must return a number");
    };
    assert!(milliseconds > 0.0);
}

fn date_is_constructible_and_exposes_its_epoch_time() {
    let mut engine = JSEngine::new();
    let result = engine.eval("new Date(1234).getTime()").unwrap();
    assert_eq!(result, JSValue::from_number(1234.0));
}

fn date_value_of_returns_the_constructed_time() {
    let mut engine = JSEngine::new();
    let result = engine.eval("new Date(42).valueOf()").unwrap();
    assert_eq!(result, JSValue::from_number(42.0));
}

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
    assert_eq!(result, JSValue::from_string("1999:2024".to_string()));
}

fn date_to_utc_string_uses_http_date_shape() {
    let mut engine = JSEngine::new();
    assert_eq!(
        engine.eval("new Date(0).toUTCString()").unwrap(),
        JSValue::from_string("Thu, 01 Jan 1970 00:00:00 GMT".to_string())
    );
}

fn date_parse_is_present_and_rejects_unsupported_text_with_nan() {
    let mut engine = JSEngine::new();
    assert_eq!(
        engine.eval("Date.parse('1234')").unwrap(),
        JSValue::from_number(1234.0)
    );
    let Some(value) = engine.eval("Date.parse('not a date')").unwrap().as_number() else {
        panic!("Date.parse must return a number");
    };
    assert!(value.is_nan());
}

fn date_exposes_getters_setters_and_iso_formatting() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const date = new Date(Date.UTC(2020, 0, 2, 3, 4, 5, 6));
            const fields = [
                date.getDay(), date.getHours(), date.getMinutes(),
                date.getSeconds(), date.getMilliseconds(), date.getUTCDay()
            ].join(":");
            const iso = date.toISOString();
            date.setUTCFullYear(2021);
            const year = date.getFullYear();
            date.setMilliseconds(7);
            fields + "|" + iso + "|" + year + "|" + date.getMilliseconds();
            "#,
        )
        .unwrap();
    assert_eq!(
        result,
        JSValue::from_string("4:3:4:5:6:4|2020-01-02T03:04:05.006Z|2021|7".to_string())
    );
}

fn date_utc_constructs_timestamp_from_components() {
    let mut engine = JSEngine::new();
    let result = engine.eval("Date.UTC(1970, 0, 2, 3, 4, 5, 6)").unwrap();
    assert_eq!(result, JSValue::from_number(97_445_006.0));
}

fn set_constructs_from_arrays_and_mutates_membership() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = new Set(["a", "b", "a"]);
            values.add("c");
            const deleted = values.delete("b");
            const copy = new Set(values);
            values.has("a") && values.has("c") && deleted && !values.has("b") &&
                values.size === 2 && copy.size === 2 && copy.has("c");
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn map_supports_identity_keys_and_chained_set() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const key = {};
            const other = {};
            const values = new Map;
            values.set(key, 1).set(other, 2);
            const copy = new Map(values);
            values.get(key) === 1 && values.get(other) === 2 && !values.has({}) &&
                values.size === 2 && copy.get(other) === 2;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn map_keeps_large_bigint_keys_distinct() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
        const values = new Map;
        values.set(1n << 70n, "large");
        values.set(1n << 6n, "small");
        values.size === 2 && values.get(1n << 70n) === "large";
    "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn set_for_each_uses_web_compatible_arguments_and_this_value() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = new Set(["a", "b"]);
            const receiver = { prefix: "set:" };
            let result = "";
            values.forEach(function (value, key, collection) {
                result += this.prefix + value + key + (collection === values);
            }, receiver);
            result;
            "#,
        )
        .unwrap();
    assert_eq!(
        result,
        JSValue::from_string("set:aatrueset:bbtrue".to_string())
    );
}

fn map_for_each_visits_values_and_keys_in_insertion_order() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = new Map([["a", 1], ["b", 2]]);
            let result = "";
            values.forEach(function (value, key, collection) {
                result += key + value + (collection === values);
            });
            result;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("a1trueb2true".to_string()));
}

fn map_iteration_can_update_existing_entries_without_extending_iteration() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = new Map([["a", [1]], ["b", [2]], ["c", [3]]]);
            let visits = 0;
            for (const [key, value] of values.entries()) {
                values.set(key, value.map(item => item + 1));
                visits++;
            }
            visits + ":" + values.get("a")[0] + values.get("c")[0];
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("3:24".to_string()));
}

fn set_exposes_the_iterator_protocol() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const iterator = new Set(["a", "b"])[Symbol.iterator]();
            const first = iterator.next();
            const second = iterator.next();
            const end = iterator.next();
            first.value + second.value + first.done + end.done;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("abfalsetrue".to_string()));
}

fn for_of_consumes_set_values() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            let joined = "";
            for (const value of new Set(["a", "b"])) joined += value;
            joined;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("ab".to_string()));
}

fn array_spread_consumes_set_values() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(r#"[...new Set(["a", "b", "a"])].join("");"#)
        .unwrap();
    assert_eq!(result, JSValue::from_string("ab".to_string()));
}

fn map_iterators_expose_keys_values_and_entries() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const values = new Map([["a", 1], ["b", 2]]);
            const key = values.keys().next().value;
            const value = values.values().next().value;
            const entry = values[Symbol.iterator]().next().value;
            key + value + entry[0] + entry[1];
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_string("a1a1".to_string()));
}

fn weak_map_supports_object_keys_without_iteration() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const first = {};
            const second = function () {};
            const values = new WeakMap([[first, 1]]);
            values.set(second, 2);
            values.get(first) === 1 && values.get(second) === 2 && values.has(first);
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn regexp_literal_can_start_with_equals() {
    let mut engine = JSEngine::new();
    assert_eq!(
        engine.eval(r#"/=+$/.test("===")"#).unwrap(),
        JSValue::from_bool(true)
    );
}

fn regexp_literals_support_flags_and_test() {
    let mut engine = JSEngine::new();
    assert_eq!(
        engine.eval(r#"/^hello/i.test("Hello world")"#).unwrap(),
        JSValue::from_bool(true)
    );
}

fn regexp_exec_returns_captures_and_match_index() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const match = /(a)(b)/.exec("--ab--");
            match[0] === "ab" && match[1] === "a" && match.index === 2;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn division_is_not_tokenized_as_a_regexp() {
    let mut engine = JSEngine::new();
    assert_eq!(
        engine.eval("12 / 3 / 2").unwrap(),
        JSValue::from_number(2.0)
    );
}

fn javascript_unicode_escapes_are_supported_in_regexp_literals() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            /^[\u00C0-\u00D6]+$/.test("ÀÖ")
                && "a\0b�c".replace(/\u0000|\uFFFD/g, "-") === "a-b-c";
            "#,
        )
        .unwrap();

    assert_eq!(result, JSValue::from_bool(true));
}

fn regexp_constructor_exposes_source_flags_and_test() {
    let mut engine = pixi_byte::JSEngine::new();
    let result = engine
        .eval("const r = new RegExp('^a+$', 'i'); [r.source, r.flags, r.ignoreCase, r.test('AAA')].join('|')")
        .unwrap();
    assert_eq!(result.to_string(), "^a+$|i|true|true");
}

fn regexp_is_callable_without_new() {
    let mut engine = pixi_byte::JSEngine::new();
    let result = engine.eval("RegExp('^ok$').test('ok')").unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn regexp_values_support_instanceof() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(r#"/ok/ instanceof RegExp && new RegExp("ok") instanceof RegExp && !({} instanceof RegExp)"#)
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn javascript_character_classes_allow_a_literal_open_bracket() {
    let mut engine = pixi_byte::JSEngine::new();
    let result = engine.eval(r#"/[\\^$.*+?()[\]{}|]/g.test("[")"#).unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn legacy_character_class_ranges_allow_class_escapes_as_endpoints() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(r#"/^[\w-_]+$/.test("min-h-") && /[a-\s]/.test(" ")"#)
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn lookarounds_are_supported_via_fancy_fallback() {
    // Lookarounds are not supported by the `regex` crate, so the engine falls
    // back to `fancy-regex` and evaluates them for real. `" add ,"` satisfies
    // the positive lookahead and the negative lookahead, so the pattern matches.
    let mut engine = pixi_byte::JSEngine::new();
    let result = engine
        .eval(r#"/\s+(add|subtract)\b(?!\))\s*(?=[,])/.test(" add ,")"#)
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));

    // `" add)"` fails the negative lookahead `(?!\))`.
    let result = engine
        .eval(r#"/\s+(add|subtract)\b(?!\))\s*(?=[,])/.test(" add) ,")"#)
        .unwrap();
    assert_eq!(result, JSValue::from_bool(false));
}

fn invalid_quantifier_braces_are_treated_as_literals() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(r#"/{([^,]*?)}/.test("{value}") && /a{2,3}/.test("aaa")"#)
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn global_regexp_test_and_exec_update_last_index() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const testPattern = /[; ]/g;
            testPattern.lastIndex = 1;
            const tested = testPattern.test("@tailwind base;");
            const afterTest = testPattern.lastIndex;
            const execPattern = /x/g;
            execPattern.lastIndex = 2;
            const match = execPattern.exec("abxxy");
            tested && afterTest === 10 && match.index === 2 &&
                execPattern.lastIndex === 3;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn sticky_regexp_requires_a_match_at_last_index_and_uses_utf16_offsets() {
    let mut engine = JSEngine::new();
    let result = engine
        .eval(
            r#"
            const pattern = /a/y;
            pattern.lastIndex = 2;
            const matched = pattern.test("😀a");
            const end = pattern.lastIndex;
            pattern.lastIndex = 2;
            matched && end === 3 && !pattern.test("😀ba") &&
                pattern.lastIndex === 0;
            "#,
        )
        .unwrap();
    assert_eq!(result, JSValue::from_bool(true));
}

fn eval(code: &str) -> String {
    let mut engine = JSEngine::new();
    match engine.eval(code) {
        Ok(v) => v.to_string(),
        Err(e) => format!("ERR: {e:?}"),
    }
}

fn repl_dup_flag() {
    // The dedupe helper `([\s\S])(?=[\s\S]*\1)` uses a lookahead + backreference,
    // which the linear-time `regex` crate cannot compile. The `fancy-regex`
    // fallback supports it, so duplicate flag characters are actually removed
    // (each character that recurs later in the string is dropped).
    let out = eval(
        "JSON.stringify(['g','gg','ggi','gmi'].map(function(s){return s.replace(/([\\s\\S])(?=[\\s\\S]*\\1)/g,'')}))",
    );
    let expected = "[\"g\",\"g\",\"gi\",\"gmi\"]";
    let ok = out == expected;
    println!("dup1 => {out} (ok={ok})");
    assert!(ok, "got {out}");
}

fn repl_dup_flag2() {
    let out =
        eval("JSON.stringify(String.prototype.replace.call('g',/([\\s\\S])(?=[\\s\\S]*\\1)/g,''))");
    println!("dup2 => {out}");
    assert_eq!(out, "\"g\"");
}

#[test]
fn suite_string_builtin_1() {
    string_primitives_expose_common_prototype_methods();
    concat_appends_each_argument_after_string_conversion();
    replace_supports_global_regexes_templates_and_callbacks();
    string_constructor_and_from_char_code_are_callable();
}

#[test]
fn suite_string_builtin_2() {
    char_code_at_uses_utf16_code_units();
    last_index_of_honors_the_search_position();
    split_with_shared_slices_handles_multibyte_and_limits();
    split_with_shared_slices_matches_regex_path();
}

#[test]
fn suite_number_math_1() {
    numbers_support_radix_to_string();
    math_exposes_functions_used_by_react();
    math_random_returns_a_fraction();
    number_formatting_and_static_helpers();
}

#[test]
fn suite_number_math_2() {
    string_unicode_and_padding_helpers();
    array_fill_copy_within_and_find_last();
}

#[test]
fn suite_numeric_globals() {
    exposes_ecmascript_numeric_globals();
    bigint_preserves_arbitrary_precision_operations();
}

#[test]
fn suite_boolean_builtin() {
    boolean_converts_values_using_javascript_truthiness();
}

#[test]
fn suite_symbol_builtin() {
    symbol_for_returns_stable_registry_values();
    symbol_call_returns_unique_values_and_well_known_symbols();
}

#[test]
fn suite_error_builtin_1() {
    error_is_callable_and_exposes_standard_properties();
    thrown_errors_are_preserved_by_catch();
    native_error_subtypes_are_callable_and_constructible();
    native_error_constructors_have_function_type_and_call_method();
}

#[test]
fn suite_error_builtin_2() {
    function_call_preserves_thrown_error_objects();
}

#[test]
fn suite_bigint_syntax() {
    bigint_literals_are_accepted_by_bitwise_expressions();
}

#[test]
fn suite_global_is_nan() {
    global_is_nan_applies_number_conversion();
}

#[test]
fn suite_uri() {
    encode_uri_component_uses_utf8_percent_encoding();
}

#[test]
fn suite_json() {
    json_stringify_quotes_strings_for_selectors();
    json_stringify_serializes_arrays_and_objects();
    json_parse_builds_nested_values_and_decodes_escapes();
    json_parse_handles_unicode_pairs_and_strict_number_grammar();
}

#[test]
fn suite_date_1() {
    date_calendar_fields_and_set_date_use_epoch_milliseconds();
    date_now_returns_epoch_milliseconds();
    date_is_constructible_and_exposes_its_epoch_time();
    date_value_of_returns_the_constructed_time();
}

#[test]
fn suite_date_2() {
    date_set_year_and_set_full_year_update_the_calendar_year();
    date_to_utc_string_uses_http_date_shape();
    date_parse_is_present_and_rejects_unsupported_text_with_nan();
    date_exposes_getters_setters_and_iso_formatting();
}

#[test]
fn suite_date_3() {
    date_utc_constructs_timestamp_from_components();
}

#[test]
fn suite_map_set_1() {
    set_constructs_from_arrays_and_mutates_membership();
    map_supports_identity_keys_and_chained_set();
    map_keeps_large_bigint_keys_distinct();
    set_for_each_uses_web_compatible_arguments_and_this_value();
}

#[test]
fn suite_map_set_2() {
    map_for_each_visits_values_and_keys_in_insertion_order();
    map_iteration_can_update_existing_entries_without_extending_iteration();
    set_exposes_the_iterator_protocol();
    for_of_consumes_set_values();
}

#[test]
fn suite_map_set_3() {
    array_spread_consumes_set_values();
    map_iterators_expose_keys_values_and_entries();
    weak_map_supports_object_keys_without_iteration();
}

#[test]
fn suite_regexp_literal_1() {
    regexp_literal_can_start_with_equals();
    regexp_literals_support_flags_and_test();
    regexp_exec_returns_captures_and_match_index();
    division_is_not_tokenized_as_a_regexp();
}

#[test]
fn suite_regexp_literal_2() {
    javascript_unicode_escapes_are_supported_in_regexp_literals();
    regexp_constructor_exposes_source_flags_and_test();
    regexp_is_callable_without_new();
    regexp_values_support_instanceof();
}

#[test]
fn suite_regexp_literal_3() {
    javascript_character_classes_allow_a_literal_open_bracket();
    legacy_character_class_ranges_allow_class_escapes_as_endpoints();
    lookarounds_are_supported_via_fancy_fallback();
    invalid_quantifier_braces_are_treated_as_literals();
}

#[test]
fn suite_regexp_literal_4() {
    global_regexp_test_and_exec_update_last_index();
    sticky_regexp_requires_a_match_at_last_index_and_uses_utf16_offsets();
}

#[test]
fn suite_dup_flag_repro() {
    repl_dup_flag();
    repl_dup_flag2();
}
