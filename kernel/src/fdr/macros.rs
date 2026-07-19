//! `fdr/macros.rs` — the `tracing`-grammar subset, as kernel-owned `macro_rules!`.
//!
//! Blueprint §4.1 decision: keep `tracing`'s exact macro grammar (fields-then-message,
//! `%`=Display / `?`=Debug sigils, span-name-first) so every call-site change is a
//! path-prefix rename (`tracing::info_span!` → `crate::fdr::info_span!`), NOT a semantic
//! rewrite. Rejected: a `[patch]` facade literally named `tracing` (hides the cutover from
//! `grep`); a hand-rolled `#[instrument]` proc-macro (would itself pull `syn`/`quote`,
//! defeating the item).
//!
//! Macros are `#[macro_export]`ed (hoisted to crate root as `fdr_*`) and re-exported under
//! the `fdr::` path by `mod.rs` (`pub use crate::fdr_info_span as info_span;` …), so both
//! `crate::fdr::info_span!` (internal) and `dowiz_kernel::fdr::info_span!` (integration
//! tests) resolve. The `fdr_*` root names are an implementation detail; call sites use the
//! `fdr::` path.
//!
//! Accepted grammar losses (blueprint §5 step 3, nothing in the repo uses them): the
//! `RUST_LOG` per-target filter grammar (only level-filtering via `DOWIZ_LOG`), span
//! hierarchy/context propagation (the incumbent layer was already single-span).

/// Internal field-list muncher (do not call directly). Accumulates `k = %v` / `k = ?v` /
/// `k = v` fields into `$vec`, then emits with the trailing message expr. Separate literal
/// `%`/`?` arms (not `$(:tt)?`) avoid the local-ambiguity the optional-sigil form hits.
#[macro_export]
#[doc(hidden)]
macro_rules! __fdr_munch {
    ($lvl:expr, $vec:ident, [$k:ident = % $val:expr, $($rest:tt)*]) => {
        $vec.push((stringify!($k), format!("{}", $val)));
        $crate::__fdr_munch!($lvl, $vec, [$($rest)*]);
    };
    ($lvl:expr, $vec:ident, [$k:ident = ? $val:expr, $($rest:tt)*]) => {
        $vec.push((stringify!($k), format!("{:?}", $val)));
        $crate::__fdr_munch!($lvl, $vec, [$($rest)*]);
    };
    ($lvl:expr, $vec:ident, [$k:ident = $val:expr, $($rest:tt)*]) => {
        $vec.push((stringify!($k), format!("{}", $val)));
        $crate::__fdr_munch!($lvl, $vec, [$($rest)*]);
    };
    // Terminal: trailing message expression.
    ($lvl:expr, $vec:ident, [$msg:expr $(,)?]) => {
        $crate::fdr::emit_event($lvl, $msg, &$vec);
    };
    // Terminal: no message (fields-only, or empty).
    ($lvl:expr, $vec:ident, []) => {
        $crate::fdr::emit_event($lvl, "", &$vec);
    };
}

/// `fdr::event!(level, k = v, …, "message")` — the general event macro. Evaluates its
/// arguments and emits ONLY when a sink is installed and `level` passes (the disabled
/// fast path is a single relaxed atomic load — matching `tracing`'s dispatch-check cost —
/// and takes NO clock, so the `wasm32` `Instant`/`SystemTime` panic is never reached).
#[macro_export]
macro_rules! fdr_event {
    ($lvl:expr, $($rest:tt)*) => {{
        if $crate::fdr::event_enabled($lvl) {
            #[allow(unused_mut)]
            let mut __fdr_v: ::std::vec::Vec<(&'static str, ::std::string::String)> =
                ::std::vec::Vec::new();
            $crate::__fdr_munch!($lvl, __fdr_v, [$($rest)*]);
        }
    }};
}

#[macro_export]
macro_rules! fdr_error {
    ($($rest:tt)*) => { $crate::fdr_event!($crate::fdr::Level::Error, $($rest)*) };
}
#[macro_export]
macro_rules! fdr_warn {
    ($($rest:tt)*) => { $crate::fdr_event!($crate::fdr::Level::Warn, $($rest)*) };
}
#[macro_export]
macro_rules! fdr_info {
    ($($rest:tt)*) => { $crate::fdr_event!($crate::fdr::Level::Info, $($rest)*) };
}
#[macro_export]
macro_rules! fdr_debug {
    ($($rest:tt)*) => { $crate::fdr_event!($crate::fdr::Level::Debug, $($rest)*) };
}
#[macro_export]
macro_rules! fdr_trace {
    ($($rest:tt)*) => { $crate::fdr_event!($crate::fdr::Level::Trace, $($rest)*) };
}

/// `fdr::info_span!(name, k = %v, …)` → a [`crate::fdr::SpanHandle`]; `.entered()` yields a
/// guard whose `Drop` reports the span's wall-clock duration to the observer (and, if a
/// ring sink is installed, writes a `span_close` FDR record). Span FIELDS have no machine
/// consumer (only the span NAME reaches `metric.jsonl`) and are intentionally not captured
/// here — this keeps the hot path (`place_order`) allocation-free and takes NO clock until
/// `.entered()`, where the `Instant` is gated off `wasm32` entirely.
#[macro_export]
macro_rules! fdr_info_span {
    ($name:expr $(, $($rest:tt)*)?) => {{
        $crate::fdr::SpanHandle::new($name)
    }};
}
