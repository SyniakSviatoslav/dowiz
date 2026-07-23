//! `kernel::telemetry_harvest` — deterministic, zero-dep harvest ledger.
//!
//! Every hot path MUST emit a deterministic, zero-dep telemetry record.
//! Every new capability MUST be probe-able.
//!
//! The harvest ledger is gov_route-compatible: records serialize as JSONL lines
//! in the same `{model,task,success,value,cost}` schema as the dispatcher's
//! `track_record.jsonl`, so the EV loop can price every operation uniformly.
//!
//! # Reference pattern
//! - `track_record.jsonl` at the kernel root: `{model,task,success,value,cost,...}`
//! - The `Dispatcher` records every dispatch; this ledger extends that pattern
//!   into the kernel-native hot paths (enrich, intent detect, trinary math).

use crate::now_ms;

/// Deterministic, zero-dep telemetry record for the harvest ledger.
/// Compatible with gov_route-compatible dispatch schema.
#[derive(Debug, Clone)]
pub struct HarvestRecord {
    pub model: String,
    pub task: String,
    pub success: bool,
    pub value: f64,
    pub cost: f64,
    pub timestamp_ms: u64,
}

impl HarvestRecord {
    pub fn new(model: &str, task: &str, success: bool, value: f64, cost: f64) -> Self {
        HarvestRecord {
            model: model.to_string(),
            task: task.to_string(),
            success,
            value: crate::sanitize_f64(value),
            cost: crate::sanitize_f64(cost).max(0.0),
            timestamp_ms: now_ms(),
        }
    }

    /// Single JSON object line — gov_route-compatible schema with optional timestamp.
    pub fn to_jsonl_line(&self) -> String {
        format!(
            r#"{{"model":"{}","task":"{}","success":{},"value":{},"cost":{},"timestamp_ms":{}}}"#,
            self.model, self.task, self.success,
            crate::sanitize_f64(self.value),
            crate::sanitize_f64(self.cost),
            self.timestamp_ms,
        )
    }
}

/// Summary statistics over the harvest ledger.
#[derive(Debug, Clone)]
pub struct HarvestReport {
    pub total: usize,
    pub success_rate: f64,
    pub ev_score: f64,
    pub avg_value: f64,
    pub avg_cost: f64,
}

/// Bounded, append-only harvest ledger.
///
/// Records carry `{model, task, success, value, cost, timestamp_ms}`.
/// EV scoring: `ev_score = avg(value * success) / avg(cost + 1)`.
/// JSONL output is gov_route-compatible (same field names as the dispatcher's
/// `track_record.jsonl`).
pub struct HarvestLedger {
    records: Vec<HarvestRecord>,
    max_records: usize,
}

impl HarvestLedger {
    pub fn new(max_records: usize) -> Self {
        let init_cap = max_records.min(1024);
        HarvestLedger {
            records: Vec::with_capacity(init_cap),
            max_records: max_records.max(1),
        }
    }

    /// Record a telemetry event. Returns the created record.
    /// The ledger acts as a ring buffer: once full, oldest records are evicted.
    pub fn record(
        &mut self,
        model: &str,
        task: &str,
        success: bool,
        value: f64,
        cost: f64,
    ) -> HarvestRecord {
        let rec = HarvestRecord::new(model, task, success, value, cost);
        if self.records.len() >= self.max_records {
            self.records.remove(0);
        }
        self.records.push(rec.clone());
        rec
    }

    /// Expected-value score: avg(value * success) / avg(cost + 1).
    /// Higher is better (more value per unit cost). Returns 0.0 if empty.
    pub fn ev_score(&self) -> f64 {
        let n = self.records.len();
        if n == 0 {
            return 0.0;
        }
        let sum_value_success: f64 = self
            .records
            .iter()
            .map(|r| r.value * if r.success { 1.0 } else { 0.0 })
            .sum();
        let sum_cost_plus_one: f64 = self.records.iter().map(|r| r.cost + 1.0).sum();
        if sum_cost_plus_one == 0.0 {
            return 0.0;
        }
        sum_value_success / sum_cost_plus_one
    }

    /// Produce a summary report over all records.
    pub fn report(&self) -> HarvestReport {
        let n = self.records.len();
        let success_rate = if n > 0 {
            self.records.iter().filter(|r| r.success).count() as f64 / n as f64
        } else {
            0.0
        };
        let avg_value = if n > 0 {
            self.records.iter().map(|r| r.value).sum::<f64>() / n as f64
        } else {
            0.0
        };
        let avg_cost = if n > 0 {
            self.records.iter().map(|r| r.cost).sum::<f64>() / n as f64
        } else {
            0.0
        };
        HarvestReport {
            total: n,
            success_rate,
            ev_score: self.ev_score(),
            avg_value,
            avg_cost,
        }
    }

    /// Emit all records as gov_route-compatible JSONL (one object per line).
    pub fn track_to_jsonl(&self) -> String {
        self.records
            .iter()
            .map(|r| r.to_jsonl_line())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn records(&self) -> &[HarvestRecord] {
        &self.records
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── HarvestRecord roundtrip ──────────────────────────────────────────

    #[test]
    fn test_harvest_record_roundtrip() {
        let rec = HarvestRecord::new("enrich", "lookup", true, 0.85, 1.2);
        assert_eq!(rec.model, "enrich");
        assert_eq!(rec.task, "lookup");
        assert!(rec.success);
        assert!((rec.value - 0.85).abs() < 1e-10);
        assert!((rec.cost - 1.2).abs() < 1e-10);
        assert!(rec.timestamp_ms > 0, "timestamp must be populated");
    }

    #[test]
    fn test_harvest_record_sanitize_nan_value() {
        let rec = HarvestRecord::new("test", "nan_check", false, f64::NAN, 5.0);
        assert_eq!(rec.value, 0.0, "NaN value must be sanitized to 0.0");
    }

    #[test]
    fn test_harvest_record_sanitize_negative_cost() {
        let rec = HarvestRecord::new("test", "neg_cost", true, 0.5, -3.0);
        assert_eq!(rec.cost, 0.0, "negative cost must be sanitized to 0.0");
    }

    #[test]
    fn test_harvest_record_jsonl_line_format() {
        let rec = HarvestRecord::new("enrich", "lookup", true, 0.85, 1.2);
        let line = rec.to_jsonl_line();
        assert!(line.starts_with("{"), "must start with {{");
        assert!(line.ends_with("}"), "must end with }}");
        assert!(line.contains(r#""model":"#), "must contain model field");
        assert!(line.contains(r#""task":"#), "must contain task field");
        assert!(line.contains(r#""success":"#), "must contain success field");
        assert!(line.contains(r#""value":"#), "must contain value field");
        assert!(line.contains(r#""cost":"#), "must contain cost field");
        assert!(line.contains(r#""timestamp_ms":"#), "must contain timestamp_ms field");
    }

    // ── EV score monotonic ───────────────────────────────────────────────

    #[test]
    fn test_ev_score_monotonic() {
        let mut ledger = HarvestLedger::new(100);
        // 10 good records: high value, low cost
        for _ in 0..10 {
            ledger.record("test", "good", true, 0.9, 1.0);
        }
        let good_score = ledger.ev_score();
        assert!(good_score > 0.0, "good records must produce positive EV");

        // 1 bad record: zero value, high cost
        ledger.record("test", "bad", false, 0.0, 100.0);
        let after_bad = ledger.ev_score();
        assert!(
            after_bad < good_score,
            "EV must drop after adding bad record: {good_score} → {after_bad}"
        );
    }

    #[test]
    fn test_ev_score_empty_ledger() {
        let ledger = HarvestLedger::new(100);
        assert_eq!(ledger.ev_score(), 0.0, "empty ledger EV must be 0.0");
    }

    #[test]
    fn test_ev_score_all_failures() {
        let mut ledger = HarvestLedger::new(100);
        for _ in 0..5 {
            ledger.record("test", "fail", false, 1.0, 1.0);
        }
        assert_eq!(ledger.ev_score(), 0.0, "all-fail EV must be 0.0");
    }

    #[test]
    fn test_ev_score_zero_cost() {
        let mut ledger = HarvestLedger::new(100);
        ledger.record("test", "free", true, 1.0, 0.0);
        // ev = (1.0 * 1.0) / (0.0 + 1.0) = 1.0
        assert!((ledger.ev_score() - 1.0).abs() < 1e-10);
    }

    // ── JSONL format ─────────────────────────────────────────────────────

    #[test]
    fn test_track_to_jsonl_format() {
        let mut ledger = HarvestLedger::new(100);
        ledger.record("enrich", "lookup", true, 0.85, 1.2);
        ledger.record("intent", "detect", true, 0.92, 0.5);
        let jsonl = ledger.track_to_jsonl();

        let lines: Vec<&str> = jsonl.lines().collect();
        assert_eq!(lines.len(), 2, "must have exactly 2 lines");

        for line in &lines {
            assert!(line.starts_with("{"), "each line must be a JSON object");
            assert!(line.ends_with("}"), "each line must be a JSON object");
            assert!(line.contains(r#""success":true"#) || line.contains(r#""success":false"#),
                "must contain a boolean success field");
        }
    }

    #[test]
    fn test_track_to_jsonl_empty() {
        let ledger = HarvestLedger::new(100);
        assert!(ledger.track_to_jsonl().is_empty(), "empty ledger → empty JSONL");
    }

    // ── Truncation ───────────────────────────────────────────────────────

    #[test]
    fn test_harvest_truncation() {
        let max = 50;
        let mut ledger = HarvestLedger::new(max);
        for i in 0..(max + 100) {
            ledger.record("test", "overflow", i % 2 == 0, i as f64 * 0.01, 1.0);
        }
        assert_eq!(
            ledger.len(),
            max,
            "after adding {total} records, count must = max {max}",
            total = max + 100
        );
    }

    #[test]
    fn test_harvest_truncation_fifo_order() {
        let max = 3;
        let mut ledger = HarvestLedger::new(max);
        ledger.record("m", "t", true, 1.0, 1.0); // evicted
        ledger.record("m", "t", true, 2.0, 1.0); // evicted
        ledger.record("m", "t", true, 3.0, 1.0); // kept
        ledger.record("m", "t", true, 4.0, 1.0); // kept
        ledger.record("m", "t", true, 5.0, 1.0); // kept
        assert_eq!(ledger.len(), 3);
        // The three kept records should have values 3.0, 4.0, 5.0
        assert!((ledger.records[0].value - 3.0).abs() < 1e-10);
        assert!((ledger.records[1].value - 4.0).abs() < 1e-10);
        assert!((ledger.records[2].value - 5.0).abs() < 1e-10);
    }

    // ── Report ───────────────────────────────────────────────────────────

    #[test]
    fn test_harvest_report_empty() {
        let ledger = HarvestLedger::new(100);
        let report = ledger.report();
        assert_eq!(report.total, 0);
        assert_eq!(report.success_rate, 0.0);
        assert_eq!(report.ev_score, 0.0);
        assert_eq!(report.avg_value, 0.0);
        assert_eq!(report.avg_cost, 0.0);
    }

    #[test]
    fn test_harvest_report_mixed() {
        let mut ledger = HarvestLedger::new(100);
        ledger.record("a", "x", true, 0.8, 2.0);
        ledger.record("a", "x", false, 0.0, 10.0);
        let report = ledger.report();
        assert_eq!(report.total, 2);
        assert!((report.success_rate - 0.5).abs() < 1e-10);
        assert!((report.avg_value - 0.4).abs() < 1e-10);
        assert!((report.avg_cost - 6.0).abs() < 1e-10);
    }

    // ── Wiring probes (fail if telemetry is broken) ─────────────────────

    /// Probe: enrich path MUST emit a record.
    #[test]
    fn probe_enrich_path_emits_record() {
        let mut ledger = HarvestLedger::new(100);
        let engine = crate::prompt_enrich::PromptEnrichEngine::new();
        let _ = engine.enrich_report_with_telemetry("fix the compilation bug", &mut ledger);
        assert!(!ledger.is_empty(), "enrich path must emit at least one record");
    }

    /// Probe: trinary path MUST emit a record.
    #[test]
    fn probe_trinary_path_emits_record() {
        use crate::trinary::{TriMatrix, Tri};
        let mut ledger = HarvestLedger::new(100);
        let mut a = TriMatrix::new(2, 2);
        a.set(0, 0, Tri::True);
        a.set(0, 1, Tri::False);
        a.set(1, 0, Tri::Unknown);
        a.set(1, 1, Tri::True);
        let mut b = TriMatrix::new(2, 2);
        b.set(0, 0, Tri::True);
        b.set(0, 1, Tri::Unknown);
        b.set(1, 0, Tri::False);
        b.set(1, 1, Tri::True);
        let _ = a.mul_with_telemetry(&b, &mut ledger);
        assert!(!ledger.is_empty(), "trinary path must emit at least one record");
        assert_eq!(ledger.len(), 1, "exactly one record for one mul call");
    }

    /// Probe: intent detection path MUST emit a record.
    #[test]
    fn probe_intent_path_emits_record() {
        let mut ledger = HarvestLedger::new(100);
        let intents = crate::prompt_enrich::detect_all_intents_with_telemetry(
            "write a report on the architecture",
            &mut ledger,
        );
        assert!(!intents.is_empty(), "must detect at least one intent");
        assert!(!ledger.is_empty(), "intent path must emit at least one record");
    }
}
