//! telemetry/obs.rs — P83 Layer 1: per-function production observability.
//!
//! BLUEPRINT P83 / SYNTHESIS PERFORMANCE AUDIT 2026-07-18 §3.3-C4 (the `C4` row).
//! ZERO external dependencies — the span-timing *value* was always hand-rolled; only the
//! *hook* used to be `tracing`-shaped. Roadmap items 4+29 retired the `tracing` pair, so
//! the hook is now the kernel-owned `fdr::SpanObserver` trait (see `SpanMetricsObserver`
//! below). Everything else — `LogBucket` histograms, `JsonlWriter`, the `metric.jsonl` row
//! format, `normalized_load1()` — carries over UNCHANGED and byte-identical.
//!
//! Pure no_std items (LogBucket, bucket_index, diagnostic_row, consts) live in
//! `dowiz_core::span_metrics::obs` and are re-exported below. This shim keeps only the
//! std parts: `normalized_load1` (/proc/loadavg), `JsonlWriter` (file I/O),
//! `SpanMetrics`/`SpanMetricsObserver` (global observer backed by `fdr::SpanObserver`).
//!
//! What this module does:
//!   * `SpanMetricsObserver` — implements `fdr::SpanObserver`; on every span close, records
//!     the wall-clock duration into a LOG-BUCKET histogram (powers-of-two microsecond
//!     buckets — a no-allocation, deterministic summary). Replaces the retired
//!     `SpanMetricsLayer` (a `tracing_subscriber::Layer`), and with it the thread-local
//!     deadlock workaround the Layer needed (no registry/Extensions locks exist to deadlock
//!     on) AND the incumbent's outer-span-dropped-under-nesting bug — each `fdr::SpanGuard`
//!     now owns its own start stamp, so nested spans are measured correctly.
//!   * `metric.jsonl` — every recorded span appends ONE JSON line (NDJSON), std-only writer
//!     (no serde / network / RNG). Span-name escaping routes through the single
//!     `fdr::json::escape_into` authority (was `{:?}` — byte-identical for the 8 `[a-z_]`
//!     span names; golden-pinned below).
//!   * `alert.jsonl` — the Layer-2 load breach artifact (see `breach.rs`); this module owns
//!     the shared std-only NDJSON writer helper used by both layers.
//!
//! Explicitly EXCLUDED (SYNTHESIS §6-E18): `assert_transition` inner loop is NOT
//! instrumented — the `fold_transitions` span + the Layer-2 sampler cover it.
//!
//! Safety: the writer is BEST-EFFORT. A failed open/write never poisons the caller (the
//! observer is observability, not a trust boundary). State is plain `std`; no `Rng`, no
//! network, no `serde`.

// Re-export the pure no_std items from dowiz-core.
pub use dowiz_core::span_metrics::obs::*;

use alloc::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

/// Compute the normalized 1-minute load: `load1 / nproc` (Linux `/proc/loadavg` +
/// `available_parallelism`; degrades-closed to `None` off-Linux or on read failure).
#[cfg(target_os = "linux")]
pub fn normalized_load1() -> Option<f64> {
    let s = crate::vfs::read_to_string("/proc/loadavg").ok()?;
    let l1: f64 = s.split_whitespace().next()?.parse().ok()?;
    let nproc = crate::kthread::available_parallelism();
    Some(l1 / nproc as f64)
}

/// Non-Linux: no `/proc/loadavg` → no breach can be detected here (Layer-2 is a
/// Linux-system-wide profiler, by design). Degrades closed.
#[cfg(not(target_os = "linux"))]
pub fn normalized_load1() -> Option<f64> {
    None
}

/// Shared, process-global append writer for the `.jsonl` artifacts. Best-effort:
/// a single poisoned/contended lock or a failed write is swallowed (observability
/// must never crash the shipping path).
pub struct JsonlWriter {
    dir: Option<PathBuf>,
}

impl JsonlWriter {
    /// `dir = None` disables the writer (no file is opened; `append` is a silent no-op).
    pub fn new(dir: Option<PathBuf>) -> Self {
        JsonlWriter { dir }
    }

    /// Append one line to `name` inside the writer's directory. Returns false if the
    /// writer is disabled or the open/append failed (best-effort — caller ignores it).
    pub fn append(&self, name: &str, line: &str) -> bool {
        let dir = match &self.dir {
            Some(d) => d,
            None => return false,
        };
        let path = dir.join(name);
        crate::vfs::append(&path, line.as_bytes()).is_ok()
    }
}

/// The registry of per-span log-bucket histograms + the shared jsonl writer.
pub struct SpanMetrics {
    inner: Mutex<Inner>,
}

struct Inner {
    /// Per-span histogram, keyed by span name (BTreeMap ⇒ deterministic iteration order).
    hist: BTreeMap<String, LogBucket>,
    writer: JsonlWriter,
}

impl SpanMetrics {
    pub fn new(dir: Option<PathBuf>) -> Self {
        SpanMetrics {
            inner: Mutex::new(Inner {
                hist: BTreeMap::new(),
                writer: JsonlWriter::new(dir),
            }),
        }
    }

    /// Record a completed span's duration (microseconds). Appends a `metric.jsonl` row
    /// immediately (one row per span close — streamable, no buffering required).
    pub fn record(&self, span: &str, dur_us: u64) {
        let mut g = match self.inner.lock() {
            Ok(g) => g,
            Err(_) => return, // poisoned lock: drop the sample, never panic.
        };
        let b = g.hist.entry(span.to_string()).or_default();
        b.record(dur_us);
        let line = b.to_jsonl(span);
        g.writer.append(METRIC_JSONL, &line);
    }

    /// Snapshot the current histograms as one `metric.jsonl` row per span (used by tests
    /// / explicit flush; the per-close path already streams rows). Returns rows written.
    pub fn flush(&self) -> usize {
        let g = match self.inner.lock() {
            Ok(g) => g,
            Err(_) => return 0,
        };
        let mut n = 0;
        for (span, b) in g.hist.iter() {
            g.writer.append(METRIC_JSONL, &b.to_jsonl(span));
            n += 1;
        }
        n
    }
}

/// The kernel-owned span observer (replaces the retired `tracing_subscriber` Layer). On
/// every span close, folds the wall-clock duration into the shared `SpanMetrics`. Wired via
/// `fdr::set_global_observer` / `fdr::set_scoped_observer` (see `span_metrics::init`).
pub struct SpanMetricsObserver {
    metrics: SpanMetrics,
}

impl SpanMetricsObserver {
    pub fn new(dir: Option<PathBuf>) -> Self {
        SpanMetricsObserver {
            metrics: SpanMetrics::new(dir),
        }
    }

    /// The shared registry (exposed for `flush`/tests).
    pub fn metrics(&self) -> &SpanMetrics {
        &self.metrics
    }
}

impl crate::fdr::SpanObserver for SpanMetricsObserver {
    fn on_span_close(&self, name: &'static str, dur_us: u64) {
        self.metrics.record(name, dur_us);
    }
}

/// Convenience: is `normalized_load1()` past the breach threshold?
pub fn is_load_breach() -> bool {
    match normalized_load1() {
        Some(l) => l >= LOAD_BREACH_THRESHOLD,
        None => false,
    }
}

// ── Unit tests: std-only writer + observer reachability ──
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn green_writer_disabled_is_noop() {
        let w = JsonlWriter::new(None);
        assert!(!w.append(METRIC_JSONL, "x\n"));
    }

    #[test]
    fn green_writer_appends_to_dir() {
        let dir = std::env::temp_dir().join(format!("p83_writer_{}", std::process::id()));
        let _ = crate::vfs::create_dir_all(&dir);
        let w = JsonlWriter::new(Some(dir.clone()));
        assert!(w.append(METRIC_JSONL, "{\"a\":1}\n"));
        let p = dir.join(METRIC_JSONL);
        let contents = crate::vfs::read_to_string(&p).unwrap();
        assert!(contents.contains("{\"a\":1}"));
        let _ = crate::vfs::remove_dir_all(&dir);
    }

    #[test]
    fn green_spanmetrics_records_and_flushes() {
        let dir = std::env::temp_dir().join(format!("p83_metrics_{}", std::process::id()));
        let _ = crate::vfs::create_dir_all(&dir);
        let m = SpanMetrics::new(Some(dir.clone()));
        m.record("fold_transitions", 10);
        m.record("fold_transitions", 20);
        // flushes one row per known span
        assert_eq!(m.flush(), 1);
        let p = dir.join(METRIC_JSONL);
        let contents = crate::vfs::read_to_string(&p).unwrap();
        assert!(contents.contains("\"span\":\"fold_transitions\""));
        assert!(contents.contains("\"count\":2"));
        let _ = crate::vfs::remove_dir_all(&dir);
    }

    /// The observer path (fdr::SpanObserver) records to metric.jsonl — proves the ported
    /// hook is wired to the same histogram/writer chain as the direct `record` API.
    #[test]
    fn green_observer_records_span_close() {
        use crate::fdr::SpanObserver;
        let dir = std::env::temp_dir().join(format!("p83_obs_{}", std::process::id()));
        let _ = crate::vfs::create_dir_all(&dir);
        let obs = SpanMetricsObserver::new(Some(dir.clone()));
        obs.on_span_close("route", 33);
        let p = dir.join(METRIC_JSONL);
        let contents = crate::vfs::read_to_string(&p).unwrap();
        assert!(contents.contains("\"span\":\"route\""));
        let _ = crate::vfs::remove_dir_all(&dir);
    }
}