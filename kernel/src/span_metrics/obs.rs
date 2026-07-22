//! telemetry/obs.rs — P83 Layer 1: per-function production observability.
//!
//! BLUEPRINT P83 / SYNTHESIS PERFORMANCE AUDIT 2026-07-18 §3.3-C4 (the `C4` row).
//! ZERO external dependencies — the span-timing *value* was always hand-rolled; only the
//! *hook* used to be `tracing`-shaped. Roadmap items 4+29 retired the `tracing` pair, so
//! the hook is now the kernel-owned `fdr::SpanObserver` trait (see `SpanMetricsObserver`
//! below). Everything else — `LogBucket` histograms, `JsonlWriter`, the `metric.jsonl` row
//! format, `normalized_load1()` — carries over UNCHANGED and byte-identical.
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

use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

/// Number of log-bucket histogram bins. Powers-of-two in microseconds, capped at
/// `2^(NUM_BUCKETS-1)` µs (≈ 2.7 s) — anything slower lands in the overflow bin.
const NUM_BUCKETS: usize = 22;

/// P83 feature flag (mirrored in `Cargo.toml`). Everything in this file is only
/// compiled when the caller builds with `--features telemetry`.
pub const TELEMETRY_FEATURE: &str = "telemetry";

/// Load-breach threshold: `load1 / nproc >= LOAD_BREACH_THRESHOLD` (SYNTHESIS §3.3-C4
/// "load1/nproc >= 4" → normalized load ≥ 4.0 — i.e. >4 runnable tasks per core sustained).
pub const LOAD_BREACH_THRESHOLD: f64 = 4.0;

/// The canonical artifact filenames (NDJSON / `.jsonl`).
pub const METRIC_JSONL: &str = "metric.jsonl";
pub const ALERT_JSONL: &str = "alert.jsonl";

/// Compute the normalized 1-minute load: `load1 / nproc` (Linux `/proc/loadavg` +
/// `available_parallelism`; degrades-closed to `None` off-Linux or on read failure).
#[cfg(target_os = "linux")]
pub fn normalized_load1() -> Option<f64> {
    let s = std::fs::read_to_string("/proc/loadavg").ok()?;
    let l1: f64 = s.split_whitespace().next()?.parse().ok()?;
    let nproc = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    Some(l1 / nproc as f64)
}

/// Non-Linux: no `/proc/loadavg` → no breach can be detected here (Layer-2 is a
/// Linux-system-wide profiler, by design). Degrades closed.
#[cfg(not(target_os = "linux"))]
pub fn normalized_load1() -> Option<f64> {
    None
}

/// A single function's log-bucket histogram: counts per µs power-of-two bin.
#[derive(Default, Clone)]
pub struct LogBucket {
    /// `buckets[i]` = number of samples with `2^i <= dur_us < 2^(i+1)` (i < NUM_BUCKETS-1).
    /// `buckets[NUM_BUCKETS-1]` is the overflow bin (>= `2^(NUM_BUCKETS-1)` µs).
    pub buckets: [u64; NUM_BUCKETS],
    pub count: u64,
    pub sum_us: u128,
    pub min_us: u64,
    pub max_us: u64,
}

impl LogBucket {
    fn record(&mut self, dur_us: u64) {
        let i = bucket_index(dur_us);
        self.buckets[i] += 1;
        self.count += 1;
        self.sum_us += dur_us as u128;
        if self.count == 1 {
            self.min_us = dur_us;
        } else {
            self.min_us = self.min_us.min(dur_us);
        }
        self.max_us = self.max_us.max(dur_us);
    }

    /// Mean duration in microseconds (0 when empty — fail-closed, never NaN/div-by-zero).
    pub fn mean_us(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum_us as f64 / self.count as f64
        }
    }

    /// Deterministic NDJSON row for `metric.jsonl`. Hand-rolled (no serde). The span name
    /// is escaped through the single `fdr::json` authority (was Rust `{:?}`) — byte-identical
    /// for the 8 real `[a-z_]` span names (escaping never fires); golden-pinned in tests.
    pub fn to_jsonl(&self, span: &str) -> String {
        // Buckets emitted as a compact "i:count" map, lexicographically sorted by bin.
        let mut parts: Vec<String> = Vec::with_capacity(NUM_BUCKETS);
        for (i, c) in self.buckets.iter().enumerate() {
            if *c > 0 {
                parts.push(format!("{}:{}", i, c));
            }
        }
        let hist = parts.join(",");
        let mut span_json = String::with_capacity(span.len() + 2);
        crate::fdr::json::quote_into(&mut span_json, span);
        format!(
            "{{\"metric\":\"span_latency_us\",\"span\":{},\"count\":{},\"sum_us\":{},\"min_us\":{},\"max_us\":{},\"mean_us\":{:.3},\"hist\":[{}]}}\n",
            span_json, self.count, self.sum_us, self.min_us, self.max_us, self.mean_us(), hist
        )
    }
}

/// Map a duration in microseconds to its log-bucket index.
fn bucket_index(dur_us: u64) -> usize {
    if dur_us == 0 {
        return 0;
    }
    let mut i = 0;
    let mut p = 2u64;
    while p <= dur_us && i < NUM_BUCKETS - 1 {
        p <<= 1;
        i += 1;
    }
    i.min(NUM_BUCKETS - 1)
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
        match OpenOptions::new().create(true).append(true).open(&path) {
            Ok(mut f) => f.write_all(line.as_bytes()).is_ok(),
            Err(_) => false,
        }
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

/// Build a `metric.jsonl` row for an arbitrary synthetic sample (test/diagnostic only).
pub fn diagnostic_row(span: &str, samples_us: &[u64]) -> String {
    let mut b = LogBucket::default();
    for &s in samples_us {
        b.record(s);
    }
    b.to_jsonl(span)
}

/// Convenience: is `normalized_load1()` past the breach threshold?
pub fn is_load_breach() -> bool {
    match normalized_load1() {
        Some(l) => l >= LOAD_BREACH_THRESHOLD,
        None => false,
    }
}

// ── Unit tests: log-bucket math + writer reachability (P83 probe per AGENTS.md) ──
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn green_bucket_index_powers_of_two() {
        assert_eq!(bucket_index(0), 0);
        assert_eq!(bucket_index(1), 0); // 2^0 <= 1 < 2^1
        assert_eq!(bucket_index(2), 1);
        assert_eq!(bucket_index(3), 1);
        assert_eq!(bucket_index(4), 2);
        assert_eq!(bucket_index(1023), 9); // 2^9=512 <= 1023 < 1024
        assert_eq!(bucket_index(1024), 10);
        // overflow bin:
        assert_eq!(bucket_index(u64::MAX), NUM_BUCKETS - 1);
        assert_eq!(bucket_index(1 << (NUM_BUCKETS - 1)), NUM_BUCKETS - 1);
    }

    #[test]
    fn green_logbucket_record_and_stats() {
        let mut b = LogBucket::default();
        assert_eq!(b.count, 0);
        assert_eq!(b.mean_us(), 0.0);
        b.record(1);
        b.record(3);
        b.record(500);
        assert_eq!(b.count, 3);
        assert_eq!(b.sum_us, 504);
        assert_eq!(b.min_us, 1);
        assert_eq!(b.max_us, 500);
        assert!((b.mean_us() - 168.0).abs() < 1e-6);
        // buckets: 1→0, 3→1, 500→8  (2^8=256 <= 500 < 512)
        assert_eq!(b.buckets[0], 1);
        assert_eq!(b.buckets[1], 1);
        assert_eq!(b.buckets[8], 1);
    }

    #[test]
    fn green_metric_row_is_valid_jsonl_shape() {
        let row = diagnostic_row("place_order", &[1, 2, 4, 8]);
        // ends with newline, starts with '{'
        assert!(row.starts_with('{'));
        assert!(row.ends_with('\n'));
        // contains the span name and the metric tag
        assert!(row.contains("\"span\":\"place_order\""));
        assert!(row.contains("\"metric\":\"span_latency_us\""));
    }

    /// GOLDEN byte-compat: the EXACT `metric.jsonl` row bytes for a fixed sample. This is
    /// the items-4+29 proof that routing the span name through `fdr::json` (was `{:?}`)
    /// changed NOTHING for a real `[a-z_]` span name. If this row ever changes, the
    /// `tools/telemetry` / governance parsers break — so it is pinned to the byte.
    #[test]
    fn golden_metric_row_exact_bytes() {
        // samples [1,2,4,8] → buckets 0,1,2,3; count 4; sum 15; min 1; max 8; mean 3.750.
        let row = diagnostic_row("place_order", &[1, 2, 4, 8]);
        assert_eq!(
            row,
            "{\"metric\":\"span_latency_us\",\"span\":\"place_order\",\"count\":4,\"sum_us\":15,\"min_us\":1,\"max_us\":8,\"mean_us\":3.750,\"hist\":[0:1,1:1,2:1,3:1]}\n"
        );
    }

    #[test]
    fn green_writer_disabled_is_noop() {
        let w = JsonlWriter::new(None);
        assert!(!w.append(METRIC_JSONL, "x\n"));
    }

    #[test]
    fn green_writer_appends_to_dir() {
        let dir = std::env::temp_dir().join(format!("p83_writer_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let w = JsonlWriter::new(Some(dir.clone()));
        assert!(w.append(METRIC_JSONL, "{\"a\":1}\n"));
        let p = dir.join(METRIC_JSONL);
        let contents = std::fs::read_to_string(&p).unwrap();
        assert!(contents.contains("{\"a\":1}"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn green_spanmetrics_records_and_flushes() {
        let dir = std::env::temp_dir().join(format!("p83_metrics_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let m = SpanMetrics::new(Some(dir.clone()));
        m.record("fold_transitions", 10);
        m.record("fold_transitions", 20);
        // flushes one row per known span
        assert_eq!(m.flush(), 1);
        let p = dir.join(METRIC_JSONL);
        let contents = std::fs::read_to_string(&p).unwrap();
        assert!(contents.contains("\"span\":\"fold_transitions\""));
        assert!(contents.contains("\"count\":2"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The observer path (fdr::SpanObserver) records to metric.jsonl — proves the ported
    /// hook is wired to the same histogram/writer chain as the direct `record` API.
    #[test]
    fn green_observer_records_span_close() {
        use crate::fdr::SpanObserver;
        let dir = std::env::temp_dir().join(format!("p83_obs_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let obs = SpanMetricsObserver::new(Some(dir.clone()));
        obs.on_span_close("route", 33);
        let p = dir.join(METRIC_JSONL);
        let contents = std::fs::read_to_string(&p).unwrap();
        assert!(contents.contains("\"span\":\"route\""));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
