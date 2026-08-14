//! span.rs — the no_std span seam for the eigenvalue family's observability
//! hook (the `Clock`-style seam the ledger calls for).
//!
//! In `dowiz-kernel` the same call sites resolve to `crate::fdr::info_span!`
//! (a `SpanHandle` whose `Drop` reports wall-clock duration to the FDR
//! observer when one is installed). In no_std `dowiz-core` there is no
//! observer, so this is a zero-cost no-op: `info_span!("name").entered()`
//! returns a guard whose `Drop` does nothing. The eigenvalue spans are
//! observability-only (never a decision input), so losing their telemetry in
//! kernel-space is acceptable and documented.
//!
//! The macro keeps `tracing`'s exact span grammar (name-first, optional
//! fields) so a call-site change is a path-prefix rename
//! (`crate::fdr::info_span!` → `crate::span::info_span!`), never a semantic
//! rewrite.

/// A span handle. In no_std it is just the `&'static str` name — no clock is
/// taken, no observer is touched.
pub struct SpanHandle(&'static str);

impl SpanHandle {
    #[inline]
    pub fn new(name: &'static str) -> Self {
        SpanHandle(name)
    }

    /// Enter the span, yielding a guard whose `Drop` is a no-op in no_std.
    #[inline]
    pub fn entered(&self) -> SpanGuard {
        SpanGuard { _name: self.0 }
    }
}

/// The entered-span guard. `Drop` reports nothing in no_std (no observer).
pub struct SpanGuard {
    _name: &'static str,
}

/// `span::info_span!(name, k = %v, …)` → a no-op [`SpanHandle`]. The optional
/// field list is accepted for grammar parity with `fdr::info_span!` and
/// ignored (span fields have no machine consumer).
#[macro_export]
macro_rules! span_info_span {
    ($name:expr $(, $($rest:tt)*)?) => {{
        $crate::span::SpanHandle::new($name)
    }};
}

pub use crate::span_info_span as info_span;
