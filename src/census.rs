//! Live-value census (diagnostics).
//!
//! Counts the JavaScript values and engine structures that currently exist, so
//! a memory investigation can tell *which* kind of value is accumulating
//! instead of only seeing process RSS. The counters are plain atomics updated
//! on allocation/drop; they are cheap enough to leave in normal builds and are
//! intended for diagnostics and memory-regression tests.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

macro_rules! counters {
    ($($name:ident => $label:literal),* $(,)?) => {
        $(
            static $name: AtomicUsize = AtomicUsize::new(0);
        )*

        /// Snapshot of the counters, one line per kind.
        pub fn report() -> String {
            let mut out = String::new();
            $(
                out.push_str(&format!("{}={} ", $label, $name.load(Ordering::Relaxed)));
            )*
            out.trim_end().to_string()
        }
    };
}

counters! {
    OBJECTS => "objects",
    FUNCTIONS => "functions",
    ARROW_FUNCTIONS => "arrow_functions",
    NATIVE_FUNCTIONS => "native_functions",
    BOUND_FUNCTIONS => "bound_functions",
    STRINGS => "strings",
    BIGINTS => "bigints",
    ENVIRONMENTS => "environments",
    PROMISES => "promises_created",
    PROMISES_PENDING => "promises_unsettled",
    PROMISES_SETTLED => "promises_settled",
    REACTIONS => "reactions",
}

pub(crate) fn object_created() {
    OBJECTS.fetch_add(1, Ordering::Relaxed);
}

pub(crate) fn object_dropped() {
    OBJECTS.fetch_sub(1, Ordering::Relaxed);
}

/// A new promise object was created (it starts pending).
///
/// Promise objects are ordinary `JSObject`s, so there is no drop hook for them:
/// `promises_created` and `promises_unsettled` are monotonic creation counters,
/// not live-object counts. Live promise objects are tracked by `objects`.
pub(crate) fn promise_created() {
    PROMISES.fetch_add(1, Ordering::Relaxed);
    PROMISES_PENDING.fetch_add(1, Ordering::Relaxed);
}

/// A promise moved from pending to settled (settling is idempotent).
pub(crate) fn promise_settled() {
    PROMISES_PENDING.fetch_sub(1, Ordering::Relaxed);
    PROMISES_SETTLED.fetch_add(1, Ordering::Relaxed);
}

/// A `.then` reaction was registered on a still-pending promise.
///
/// The implementation stores each reaction as three own properties and does not
/// remove them when the promise settles, so this counter measures how much
/// reaction bookkeeping the engine is retaining.
pub(crate) fn reaction_registered() {
    REACTIONS.fetch_add(1, Ordering::Relaxed);
}

pub(crate) fn environment_created() {
    ENVIRONMENTS.fetch_add(1, Ordering::Relaxed);
}

pub(crate) fn environment_dropped() {
    ENVIRONMENTS.fetch_sub(1, Ordering::Relaxed);
}

pub(crate) fn value_created(kind: crate::value::jsvalue::JsValueKind) {
    use crate::value::jsvalue::JsValueKind;
    let counter = match kind {
        JsValueKind::Function => &FUNCTIONS,
        JsValueKind::ArrowFunction => &ARROW_FUNCTIONS,
        JsValueKind::NativeFunction => &NATIVE_FUNCTIONS,
        JsValueKind::BoundFunction => &BOUND_FUNCTIONS,
        JsValueKind::String => &STRINGS,
        JsValueKind::BigInt => &BIGINTS,
        _ => return,
    };
    counter.fetch_add(1, Ordering::Relaxed);
}

pub(crate) fn value_dropped(kind: crate::value::jsvalue::JsValueKind) {
    use crate::value::jsvalue::JsValueKind;
    let counter = match kind {
        JsValueKind::Function => &FUNCTIONS,
        JsValueKind::ArrowFunction => &ARROW_FUNCTIONS,
        JsValueKind::NativeFunction => &NATIVE_FUNCTIONS,
        JsValueKind::BoundFunction => &BOUND_FUNCTIONS,
        JsValueKind::String => &STRINGS,
        JsValueKind::BigInt => &BIGINTS,
        _ => return,
    };
    counter.fetch_sub(1, Ordering::Relaxed);
}

/// Whether the attribution counters are enabled (`PIXI_LEAK_DEBUG=1`).
///
/// Callers that would have to build a `String` for [`note`] must check this
/// first, so the disabled path stays allocation-free.
pub fn attribution_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("PIXI_LEAK_DEBUG").is_some())
}

/// Attribution counters: `(tag, detail) -> count`.
///
/// Used to find out *which* property read materializes a callable object and
/// *where* the never-settling promises are created. Only maintained when
/// `PIXI_LEAK_DEBUG` is set, because it allocates a `String` per note.
fn attribution() -> &'static Mutex<std::collections::HashMap<(String, String), u64>> {
    static COUNTS: OnceLock<Mutex<std::collections::HashMap<(String, String), u64>>> =
        OnceLock::new();
    COUNTS.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

/// Records one attribution event (no-op unless `PIXI_LEAK_DEBUG` is set).
pub fn note(tag: &str, detail: &str) {
    if !attribution_enabled() {
        return;
    }
    let mut counts = attribution().lock().unwrap();
    *counts
        .entry((tag.to_string(), detail.to_string()))
        .or_insert(0) += 1;
}

/// The `limit` most frequent attribution events, as one string per line.
pub fn attribution_report(limit: usize) -> String {
    if !attribution_enabled() {
        return String::new();
    }
    let counts = attribution().lock().unwrap();
    let mut entries: Vec<_> = counts.iter().collect();
    entries.sort_by(|a, b| b.1.cmp(a.1));
    entries
        .into_iter()
        .take(limit)
        .map(|((tag, detail), count)| format!("    {tag} {detail} -> {count}"))
        .collect::<Vec<_>>()
        .join("\n")
}
