//! telemetry/breach.rs — P83 Layer 2 pure breach types (no_std).
//!
//! The pure data types for load-breach detection. The std parts (perf capture,
//! file I/O, /proc loadavg) stay in the kernel shim at `kernel/src/span_metrics/breach.rs`.

/// Default wall-clock duration the system-wide `perf record` is allowed to run.
/// Bounded so it can never hang the host (R6 "must not hang").
pub const PERF_CAPTURE_SECS: u64 = 5;

/// Default sampling frequency for `perf record -F` (SYNTHESIS §3.3-C4: `-F 99`).
pub const PERF_FREQ: u64 = 99;

/// The outcome of a breach evaluation — drives whether `perf` is invoked and
/// what `alert.jsonl` records.
pub enum BreachAction {
    /// `normalized_load1()` was below threshold → no capture, no alert.
    NoBreach { load: f64 },
    /// Breach detected; `perf record -a -g -F 99 -- sleep N` ran (or was attempted).
    /// `captured` = true if `perf.data` was produced; `fallback` = true if we took
    /// the `pprof` feature-gated no-op path instead of shelling out.
    Captured {
        load: f64,
        captured: bool,
        fallback: bool,
        detail: alloc::string::String,
    },
}