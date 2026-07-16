//! telemetry.rs — native harvest-ledger aggregation (H1 EV loop, zero extra deps).
//!
//! The `Dispatcher` already writes one JSON row per dispatch to the harvest ledger
//! (`track_record.jsonl`, gov_route-compatible schema `{model,task,success,value,cost,...}`).
//! This module reads that ledger and folds it into per-model telemetry: dispatch count, success
//! rate, mean token cost, total cost — the same aggregates `gov_route` consumes, but as a
//! in-process struct the harness can surface on demand (no new telemetry channel, no network).
//!
//! Fail-closed: an empty/missing ledger yields `Telemetry::default()` (all-zeros), never a panic.
//! The fold is deterministic (sorted by model id) so two runs over the same ledger are byte-identical.

use crate::dispatch::TrackRecord;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Aggregated telemetry for ONE model.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ModelStats {
    /// Number of dispatches recorded for this model.
    pub dispatches: u64,
    /// Number of successful (backend-returned-ok) dispatches.
    pub successes: u64,
    /// Total tokens consumed (sum of `cost`/total_tokens across rows).
    pub total_tokens: u64,
    /// Sum of outcome `value` (EV numerator) across rows.
    pub total_value: f64,
}

impl ModelStats {
    /// Success rate in [0.0, 1.0]. 0.0 when no dispatches (fail-closed).
    pub fn success_rate(&self) -> f64 {
        if self.dispatches == 0 {
            0.0
        } else {
            self.successes as f64 / self.dispatches as f64
        }
    }
    /// Mean token cost per dispatch (0.0 when none).
    pub fn mean_tokens(&self) -> f64 {
        if self.dispatches == 0 {
            0.0
        } else {
            self.total_tokens as f64 / self.dispatches as f64
        }
    }
}

/// A full telemetry fold over a harvest ledger.
#[derive(Debug, Clone, Default)]
pub struct Telemetry {
    /// Per-model aggregates, sorted by model id (deterministic).
    pub by_model: BTreeMap<String, ModelStats>,
    /// Total dispatches across all models.
    pub total_dispatches: u64,
}

impl Telemetry {
    /// Fold a single `TrackRecord` into the aggregate (call once per ledger row).
    pub fn ingest(&mut self, rec: &TrackRecord) {
        self.total_dispatches += 1;
        let s = self.by_model.entry(rec.model_id.clone()).or_default();
        s.dispatches += 1;
        if rec.success {
            s.successes += 1;
        }
        s.total_tokens += rec.cost.round() as u64;
        s.total_value += rec.value;
    }

    /// Read and fold a harvest ledger file (one JSON object per line). Fail-closed:
    /// a missing/unreadable file yields an empty `Telemetry` rather than an error.
    pub fn from_ledger(path: &Path) -> Self {
        let mut t = Telemetry::default();
        let text = match fs::read_to_string(path) {
            Ok(t) => t,
            Err(_) => return t, // missing ledger → empty telemetry
        };
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            // Reuse the same decode the Dispatcher uses for its harvest row.
            if let Ok(rec) = crate::dispatch::decode_track_record(line) {
                t.ingest(&rec);
            }
        }
        t
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch::TrackRecord;

    fn rec(model: &str, success: bool, tokens: u64, value: f64) -> TrackRecord {
        TrackRecord {
            backend_id: "ollama".into(),
            model_id: model.into(),
            total_tokens: tokens,
            ms: 100,
            task: "chat".into(),
            success,
            value,
            cost: tokens as f64,
        }
    }

    // Native telemetry must fold the same ledger gov_route reads — fail-closed + deterministic.
    #[test]
    fn green_folds_per_model_and_success_rate() {
        let mut t = Telemetry::default();
        t.ingest(&rec("qwen2.5-coder:7b", true, 33, 0.0)); // ok
        t.ingest(&rec("qwen2.5-coder:7b", true, 40, 0.0)); // ok
        t.ingest(&rec("qwen2.5-coder:7b", false, 0, 0.0)); // fail (no tokens)
        t.ingest(&rec("llama3.1:8b", true, 50, 1.0)); // different model

        assert_eq!(t.total_dispatches, 4);
        let q = t.by_model.get("qwen2.5-coder:7b").unwrap();
        assert_eq!(q.dispatches, 3);
        assert_eq!(q.successes, 2);
        assert_eq!(q.success_rate(), 2.0 / 3.0);
        assert_eq!(q.total_tokens, 73);
        assert_eq!(q.mean_tokens(), 73.0 / 3.0);
        // BTreeMap ⇒ deterministic ordering: llama3.1 sorts before qwen2.5.
        let first = t.by_model.keys().next().unwrap();
        assert_eq!(first, "llama3.1:8b");
        // value aggregates per model only.
        assert_eq!(t.by_model.get("llama3.1:8b").unwrap().total_value, 1.0);
    }

    // Fail-closed: missing ledger ⇒ empty telemetry (never panic).
    #[test]
    fn green_missing_ledger_is_empty() {
        let t = Telemetry::from_ledger(Path::new("/nonexistent/track_record.jsonl"));
        assert_eq!(t.total_dispatches, 0);
        assert!(t.by_model.is_empty());
    }
}
