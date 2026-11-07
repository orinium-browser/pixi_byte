# getOwnPropertyDescriptor spec compliance fix — 2026-09-09

## Problem
`Object.getOwnPropertyDescriptor` deviated from the spec in three ways:
1. Primitive receivers got an empty `Object.prototype`-proto object instead of a real wrapper — `Object.getOwnPropertyDescriptor("abc", "length")` returned `undefined`.
2. `prop_name.to_string()` was used instead of `ToPropertyKey` (no `@@toPrimitive`/`toString`/`valueOf` coercion).
3. The function carried its own ad-hoc conversion logic instead of delegating to VM-level abstract operations.

## Changes
### `src/vm/mod.rs` — new abstract operations on VM
- `VM::to_object(value) -> JSResult<Rc<RefCell<JSObject>>>` (ES §7.3.2):
  - Object → as-is; user Function/ArrowFunction → `user_function_object` (fallback `ensure_callable_object`); Native/BoundFunction → empty object with `Function.prototype`.
  - String → wrapper with `String.prototype` + own `length` and indexed properties, all non-writable/non-enumerable/non-configurable (built via `define_property`, not `set`).
  - Number → `number_prototype`; Boolean/BigInt → `object_prototype` (no dedicated wrappers yet).
  - Null/Undefined → `TypeError: cannot convert null or undefined to object`.
- `VM::to_property_key(value) -> JSResult<String>`: routes through `to_primitive(PrimitiveHint::Default)`. Note: this engine represents Symbol values as `@@...` string keys (no distinct `JsValueKind::Symbol`), so a `PropertyKey` enum is unnecessary today; `to_property_key` is the single choke point to change when real Symbols land.

### `src/builtins/object.rs`
- `object_get_own_property_descriptor` rewritten as the thin §19.1.2.8 algorithm: `to_object` + `to_property_key` + `get_property_descriptor` (confirmed own-property-only, no prototype walk) + descriptor construction.
- New `create_property_descriptor(Property)` helper (accessor → `get`/`set`, data → `value`/`writable`, both + `enumerable`/`configurable`).
- `object_define_property` now uses `vm.to_property_key` instead of `to_string()`.
- Arguments read via `let [obj_val, prop_name] = args.as_slice()` instead of `args.remove(0)`.

### `tests/objects.rs` (getOwnPropertyDescriptor coverage)
Covers: string primitive `length`/index descriptors (incl. attribute flags), `Object.keys(new Object("ab"))` is empty, null receiver throws, own-property-only lookup (no prototype-chain leak), accessor descriptor shape, symbol-like keys, missing-arg TypeError, function receiver with `prototype` property.

## Verification
`cargo test` — 81/81 suites pass (incl. the 10 new tests). `cargo clippy` — no new warnings (the `to_*` `wrong_self_convention` notes match the existing `to_string_value`/`to_primitive` house convention).

## Known pre-existing quirk (not fixed, out of scope)
`JSError`s thrown from Rust natives are delivered to JS `catch` clauses as stringified values (`redirect_exception` in `src/vm/mod.rs` line ~376), so `e instanceof TypeError` and `e.message` do not work for native-origin errors. Error objects would need to be constructed for non-`Thrown` variants to fix this engine-wide.
