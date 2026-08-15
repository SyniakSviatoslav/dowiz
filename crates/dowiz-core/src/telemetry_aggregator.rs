//! telemetry_aggregator.rs — Kernel-native telemetry aggregation.
//!
//! Replaces the bash script `hydra_metrics_sender.sh` and provides a unified
//! kernel-native interface to the existing telemetry primitives:
//! - `typed_metrics` — CPU/mem/GPU sampling
//! - `telemetry` — self-improvement pattern surface (trigram-based)
//! - `telemetry_harvest` — deterministic JSONL harvest ledger
//! - `span_metrics` — P83 span instrumentation (feature-gated)
//!
//! # What this replaces
//! The bash script `hydra_metrics_sender.sh` read JSONL artifacts from
//! `tools/telemetry/logs/`, aggregated them, and wrote reports. The kernel
//! now provides this natively:
//! - `HarvestLedger` (telemetry_harvest) replaces JSONL log aggregation
//! - `MetricSample` (typed_metrics) replaces /proc sampling in bash
//! - `surface_recurring_patterns` (telemetry) replaces pattern analysis
//! - `SpanMetricsObserver` (span_metrics) replaces span tracking
//!
//! # Design
//! - Pure Rust, zero external dependencies
//! - Combines existing kernel primitives into a single aggregation interface
//! - Produces structured reports (not raw text dumps)
//! - Feature-gated behind `telemetry` (same as span_metrics)

use crate::telemetry::{surface_recurring_patterns, PatternSurface};
use crate::telemetry_harvest::{HarvestLedger, HarvestReport};
use crate::typed_metrics::{MemSample, ProcCpuSample};
use alloc::collections::VecDeque;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// A unified telemetry snapshot — aggregates all available kernel metrics
/// into a single structured report.
#[derive(Debug, Clone)]
pub struct TelemetrySnapshot {
    /// Timestamp of the snapshot.
    pub timestamp_us: u64,
    /// CPU sample (if available from /proc).
    pub cpu: Option<ProcCpuSample>,
    /// Memory sample (if available from /proc).
    pub mem: Option<MemSample>,
    /// Harvest ledger summary (aggregate of all recorded events).
    pub harvest_report: Option<HarvestReport>,
    /// Recurring pattern surface (top-k trigrams from tool outcomes).
    pub pattern_surface: Option<PatternSurface>,
    /// Total events recorded in the harvest ledger.
    pub total_events: usize,
    /// SHA3-256 of the canonical snapshot bytes (for integrity).
    pub snapshot_hash: [u8; 32],
}

/// The telemetry aggregator — combines all kernel telemetry primitives.
pub struct TelemetryAggregator {
    /// The harvest ledger for event recording.
    harvest_ledger: HarvestLedger,
    /// Tool outcome tokens for pattern surface (self-improvement loop).
    outcome_tokens: alloc::collections::VecDeque<String>,
    /// Maximum outcome tokens to retain (sliding window).
    max_tokens: usize,
    /// Harvest ledger capacity, retained so the ledger can be reset.
    max_records: usize,
}

impl TelemetryAggregator {
    /// Create a new aggregator with a harvest ledger capped at `max_records`.
    pub fn new(max_records: usize) -> Self {
        let max_records = max_records.max(1);
        TelemetryAggregator {
            harvest_ledger: HarvestLedger::new(max_records),
            outcome_tokens: alloc::collections::VecDeque::new(),
            max_tokens: 1000,
            max_records,
        }
    }

    /// Record a tool outcome event in the harvest ledger. `now_ms` is caller-supplied
    /// monotonic milliseconds (the no_std form — the host stamps its own clock).
    pub fn record_event(
        &mut self,
        model: &str,
        task: &str,
        success: bool,
        value: f64,
        cost: f64,
        outcome_token: &str,
        now_ms: u64,
    ) {
        self.harvest_ledger
            .record(model, task, success, value, cost, now_ms);

        // Track outcome token for pattern surface (O(1) ring eviction).
        self.outcome_tokens.push_back(outcome_token.to_string());
        if self.outcome_tokens.len() > self.max_tokens {
            self.outcome_tokens.pop_front();
        }
    }

    /// Take a full telemetry snapshot from caller-supplied samples + timestamp (the
    /// no_std form — the host samples `/proc` and stamps its own wall clock).
    pub fn snapshot(
        &mut self,
        timestamp_us: u64,
        cpu: Option<ProcCpuSample>,
        mem: Option<MemSample>,
    ) -> TelemetrySnapshot {
        let harvest_report = self.harvest_ledger.report();
        let pattern_surface = if self.outcome_tokens.is_empty() {
            None
        } else {
            let tokens: Vec<&str> = self.outcome_tokens.iter().map(|s| s.as_str()).collect();
            Some(surface_recurring_patterns(&tokens, 5))
        };

        let total_events = self.harvest_ledger.len();

        let mut snapshot = TelemetrySnapshot {
            timestamp_us,
            cpu,
            mem,
            harvest_report: Some(harvest_report),
            pattern_surface,
            total_events,
            snapshot_hash: [0u8; 32], // filled below
        };

        // Compute hash of canonical bytes.
        let mut buf = Vec::with_capacity(128);
        buf.extend_from_slice(&timestamp_us.to_le_bytes());
        buf.extend_from_slice(&(snapshot.cpu.is_some() as u8).to_le_bytes());
        buf.extend_from_slice(&(snapshot.mem.is_some() as u8).to_le_bytes());
        if let Some(ref report) = snapshot.harvest_report {
            buf.extend_from_slice(&report.total.to_le_bytes());
            buf.extend_from_slice(&report.success_rate.to_le_bytes());
            buf.extend_from_slice(&report.ev_score.to_le_bytes());
        }
        buf.extend_from_slice(&total_events.to_le_bytes());
        snapshot.snapshot_hash = crate::event_log::sha3_256(&buf);

        snapshot
    }

    /// Get the harvest ledger for direct access.
    pub fn ledger(&self) -> &HarvestLedger {
        &self.harvest_ledger
    }

    /// Get the number of events recorded.
    pub fn event_count(&self) -> usize {
        self.harvest_ledger.len()
    }

    /// Get the configured harvest ledger capacity.
    pub fn max_records(&self) -> usize {
        self.max_records
    }

    /// Clear all recorded events (for reset/testing).
    pub fn clear(&mut self) {
        self.harvest_ledger = HarvestLedger::new(self.max_records);
        self.outcome_tokens.clear();
    }

    /// ASCII report — human-readable summary of the current telemetry state.
    pub fn ascii_report(&self) -> String {
        let mut out = String::from("=== Kernel Telemetry Report ===\n");

        out.push_str(&format!("Events recorded: {}\n", self.event_count()));

        let report = self.harvest_ledger.report();
        out.push_str(&format!(
            "  Success rate: {:.2}%\n",
            report.success_rate * 100.0
        ));
        out.push_str(&format!("  EV score: {:.4}\n", report.ev_score));
        out.push_str(&format!("  Avg value: {:.4}\n", report.avg_value));
        out.push_str(&format!("  Avg cost: {:.4}\n", report.avg_cost));

        if !self.outcome_tokens.is_empty() {
            let tokens: Vec<&str> = self.outcome_tokens.iter().map(|s| s.as_str()).collect();
            let surface = surface_recurring_patterns(&tokens, 3);
            if !surface.top.is_empty() {
                out.push_str("\nTop recurring patterns:\n");
                for (tri, count) in &surface.top {
                    out.push_str(&format!("  {:?} × {}\n", tri, count));
                }
            }
        }

        out.push_str("\n=== End Report ===\n");
        out
    }

    /// Glyph report — the value series rendered as a sparkline (Phase C/F:
    /// numeric telemetry series render via pixel glyphs, not raw ASCII).
    pub fn glyph_report(&self) -> String {
        let series = self.harvest_ledger.value_series();
        if series.is_empty() {
            return String::from("(no telemetry events)");
        }
        let spark = crate::glyph_dashboard::render_sparkline(&series);
        format!("value sparkline: {spark}")
    }
}

impl Default for TelemetryAggregator {
    fn default() -> Self {
        Self::new(1024)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_aggregator_has_no_events() {
        let agg = TelemetryAggregator::new(100);
        assert_eq!(agg.event_count(), 0);
    }

    #[test]
    fn record_event_increments_count() {
        let mut agg = TelemetryAggregator::new(100);
        agg.record_event("test-model", "test-task", true, 1.0, 0.5, "success", 0);
        assert_eq!(agg.event_count(), 1);
    }

    #[test]
    fn glyph_report_empty_and_nonempty() {
        let agg = TelemetryAggregator::new(100);
        assert_eq!(agg.glyph_report(), "(no telemetry events)");

        let mut agg2 = TelemetryAggregator::new(100);
        agg2.record_event("m", "t1", true, 0.1, 0.2, "ok", 0);
        agg2.record_event("m", "t2", false, 0.9, 0.3, "fail", 0);
        let g = agg2.glyph_report();
        assert!(g.contains("value sparkline:"));
        assert!(g.chars().any(|c| ('\u{2581}'..='\u{2588}').contains(&c)));
    }

    #[test]
    fn snapshot_contains_harvest_report() {
        let mut agg = TelemetryAggregator::new(100);
        agg.record_event("m", "t", true, 2.0, 1.0, "ok", 0);
        agg.record_event("m", "t", false, 0.0, 1.0, "fail", 0);

        let snap = agg.snapshot(0, None, None);
        assert!(snap.harvest_report.is_some());
        let report = snap.harvest_report.unwrap();
        assert_eq!(report.total, 2);
    }

    #[test]
    fn clear_resets_state() {
        let mut agg = TelemetryAggregator::new(100);
        agg.record_event("m", "t", true, 1.0, 0.5, "ok", 0);
        assert_eq!(agg.event_count(), 1);

        agg.clear();
        assert_eq!(agg.event_count(), 0);
    }

    #[test]
    fn ascii_report_format() {
        let agg = TelemetryAggregator::new(100);
        let report = agg.ascii_report();
        assert!(report.contains("Kernel Telemetry Report"));
        assert!(report.contains("Events recorded: 0"));
    }

    #[test]
    fn snapshot_hash_is_computed() {
        let mut agg = TelemetryAggregator::new(100);
        agg.record_event("m", "t", true, 1.0, 0.5, "ok", 0);
        let snap = agg.snapshot(0, None, None);

        assert_eq!(snap.snapshot_hash.len(), 32);
        assert!(!snap.snapshot_hash.iter().all(|&b| b == 0));
    }

    #[test]
    fn multiple_events_aggregates() {
        let mut agg = TelemetryAggregator::new(100);
        for i in 0..10 {
            agg.record_event(
                "model",
                &format!("task-{}", i),
                i % 2 == 0,
                1.0,
                0.1,
                "event",
                0,
            );
        }
        assert_eq!(agg.event_count(), 10);

        let report = agg.ledger().report();
        assert_eq!(report.total, 10);
        // 5 successes out of 10
        assert!((report.success_rate - 0.5).abs() < 1e-6);
    }

    #[test]
    fn default_aggregator_has_reasonable_cap() {
        let agg = TelemetryAggregator::default();
        // Default cap should be 1024
        assert_eq!(agg.max_records(), 1024);
    }
}
