//! telemetry/obs.rs — P83 Layer 1: per-function production observability.
//!
//! BLUEPRINT P83 / SYNTHESIS PERFORMANCE AUDIT 2026-07-18 §3.3-C4 (the `C4` row).
//! ZERO new dependencies — reuses the already-linked `tracing` (0.1) + `tracing-subscriber`
//! (0.3) and the three `tracing` spans already placed over the verified hot functions
//! (`place_order`, `place_order_priced`, `fold_transitions`). The remaining five of the
//! eight verified functions (`route`, `commit_after_decide`, `decide_settlement`,
//! `cap::verify_chain`, and `mldsa verify` behind `pq`) are wrapped by the Layer-1
//! `#[instrument(...)]` macros (see `instrument.rs`), gated to this feature so the
//! shipping build is perf-neutral.
//!
//! What this module does:
//!   * `SpanMetricsLayer` — a hand-rolled `tracing_subscriber` Layer that, on every
//!     span `close`, records the wall-clock duration into a LOG-BUCKET histogram
//!     (powers-of-two microsecond buckets — a no-allocation, deterministic summary).
//!   * `metric.jsonl` — every `flush`/record of a span appends ONE JSON line to
//!     `metric.jsonl` (NDJSON), std-only writer (no serde / network / RNG).
//!   * `alert.jsonl` — the Layer-2 load breach artifact (see `breach.rs`); this module
//!     owns the shared std-only NDJSON writer helper used by both layers.
//!
//! Explicitly EXCLUDED (SYNTHESIS §6-E18): `assert_transition` inner loop is NOT
//! instrumented — the `fold_transitions` span + the Layer-2 sampler cover it.
//!
//! Safety: the writer is BEST-EFFORT. A failed open/write never poisons the caller
//! (the layer is observability, not a trust boundary). State is plain `std`; no `Rng`,
//! no network, no `serde`.

use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use tracing::span::{Attributes, Id, Record};
use tracing::{Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;

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
    let nproc = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
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

    /// Deterministic NDJSON row for `metric.jsonl`. Hand-rolled (no serde).
    pub fn to_jsonl(&self, span: &str) -> String {
        // Buckets emitted as a compact "i:count" map, lexicographically sorted by bin.
        let mut parts: Vec<String> = Vec::with_capacity(NUM_BUCKETS);
        for (i, c) in self.buckets.iter().enumerate() {
            if *c > 0 {
                parts.push(format!("{}:{}", i, c));
            }
        }
        let hist = parts.join(",");
        format!(
            "{{\"metric\":\"span_latency_us\",\"span\":{:?},\"count\":{},\"sum_us\":{},\"min_us\":{},\"max_us\":{},\"mean_us\":{:.3},\"hist\":[{}]}}\n",
            span, self.count, self.sum_us, self.min_us, self.max_us, self.mean_us(), hist
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
        let mut g = match self.inner.lock() {
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

/// The `tracing_subscriber` Layer. Measures wall-clock span duration via `on_enter`/
/// `on_close` and forwards it to the shared `SpanMetrics`.
pub struct SpanMetricsLayer {
    metrics: SpanMetrics,
}

impl SpanMetricsLayer {
    pub fn new(dir: Option<PathBuf>) -> Self {
        SpanMetricsLayer {
            metrics: SpanMetrics::new(dir),
        }
    }

    /// The shared registry (exposed for `flush`/tests).
    pub fn metrics(&self) -> &SpanMetrics {
        &self.metrics
    }
}

/// Enter timestamp + span name of the *currently-entered* span on this thread, kept
/// in thread-locals instead of in the span's `Extensions`.
///
/// WHY A THREAD-LOCAL (not `Extensions::insert`/`get`): `tracing::Span::entered()`
/// takes the span's per-span `Extensions` **read** lock while it runs
/// `Layer::on_enter`; calling `span.extensions_mut()` from inside `on_enter`/`on_close`
/// then tries to take the SAME span's `Extensions` **write** lock → self-deadlock (the
/// prior agent's code did exactly this and the test hung forever; confirmed via gdb:
/// blocked in `sharded::extensions_mut` → `RwLock::write`). A thread-local avoids
/// touching the registry's locks entirely during the callbacks. The immediately-entered
/// span is always on the current thread, so a thread-local is correct for the
/// single-thread entered case this layer measures.
use std::cell::RefCell;

thread_local! {
    static ENTER_AT: RefCell<Option<Instant>> = const { RefCell::new(None) };
    static CURRENT_SPAN_NAME: RefCell<Option<String>> = const { RefCell::new(None) };
}

impl<S> tracing_subscriber::Layer<S> for SpanMetricsLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, _attrs: &Attributes<'_>, _id: &Id, _ctx: Context<'_, S>) {
        // No per-span state is stamped here. `on_enter` records the real enter time
        // (below); a span that is created but never entered simply has no enter
        // timestamp, and is skipped on close (correct — it did no work).
    }

    fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
        // Stamp the enter time for the currently-entered span (always on this thread),
        // and remember its name so `on_close` can record under the right name WITHOUT
        // re-locking the registry's per-span Extensions (see note above). `name()` reads
        // only the span metadata (a `&'static` str) and does NOT take the per-span
        // extensions lock, so it is deadlock-free here. Update-on-reenter keeps the
        // latest enter stamp, so the measured duration is the final enter→close interval.
        if let Some(span) = ctx.span(id) {
            let name = span.name().to_string();
            ENTER_AT.with(|t| *t.borrow_mut() = Some(Instant::now()));
            CURRENT_SPAN_NAME.with(|n| *n.borrow_mut() = Some(name));
        }
    }

    fn on_close(&self, _id: Id, _ctx: Context<'_, S>) {
        let (t0, name) = {
            let t0 = ENTER_AT.with(|t| t.borrow_mut().take());
            let name = CURRENT_SPAN_NAME.with(|n| n.borrow_mut().take());
            (t0, name)
        };
        let t0 = match t0 {
            Some(t0) => t0,
            None => return, // never entered (e.g. span created but not entered) → skip.
        };
        let name = match name {
            Some(n) => n,
            None => return,
        };
        let dur_us = t0.elapsed().as_micros().min(u64::MAX as u128) as u64;
        self.metrics.record(&name, dur_us);
    }

    // `on_record` — we do not read fields; span names are enough for the histogram.
    fn on_record(&self, _id: &Id, _values: &Record<'_>, _ctx: Context<'_, S>) {}
}

/// Hidden-field probe helper (used by `init` + tests): the `record` path is reachable
/// only when the `telemetry` feature is compiled, so this file is cfg-gated at the
/// `mod telemetry` site in `lib.rs`. No-op shim kept OUT (the layer is never referenced
/// in a non-telemetry build).

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
}
