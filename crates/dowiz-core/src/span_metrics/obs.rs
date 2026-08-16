//! telemetry/obs.rs — P83 Layer 1 pure histogram core (no_std).
//!
//! Pure, no-std parts: log-bucket histogram buckets, bucket-index computation,
//! and diagnostic row formatting. The std parts (normalized_load1, JsonlWriter,
//! SpanMetrics, SpanMetricsObserver) stay in the kernel shim at
//! `kernel/src/span_metrics/obs.rs`.

use alloc::string::String;
use alloc::vec::Vec;

/// Number of log-bucket histogram bins. Powers-of-two in microseconds, capped at
/// `2^(NUM_BUCKETS-1)` µs (≈ 2.7 s) — anything slower lands in the overflow bin.
pub const NUM_BUCKETS: usize = 22;

/// P83 feature flag (mirrored in `Cargo.toml`). Everything in this file is only
/// compiled when the caller builds with `--features telemetry`.
pub const TELEMETRY_FEATURE: &str = "telemetry";

/// Load-breach threshold: `load1 / nproc >= LOAD_BREACH_THRESHOLD` (SYNTHESIS §3.3-C4
/// "load1/nproc >= 4" → normalized load ≥ 4.0 — i.e. >4 runnable tasks per core sustained).
pub const LOAD_BREACH_THRESHOLD: f64 = 4.0;

/// The canonical artifact filenames (NDJSON / `.jsonl`).
pub const METRIC_JSONL: &str = "metric.jsonl";
pub const ALERT_JSONL: &str = "alert.jsonl";

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
    /// Record one sample (duration in microseconds). `pub` so the kernel's
    /// `SpanMetrics` (std) can drive it from behind the Mutex.
    pub fn record(&mut self, dur_us: u64) {
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

/// Build a `metric.jsonl` row for an arbitrary synthetic sample (test/diagnostic only).
pub fn diagnostic_row(span: &str, samples_us: &[u64]) -> String {
    let mut b = LogBucket::default();
    for &s in samples_us {
        b.record(s);
    }
    b.to_jsonl(span)
}

// ── Unit tests: log-bucket math (pure, no std needed) ──
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
}