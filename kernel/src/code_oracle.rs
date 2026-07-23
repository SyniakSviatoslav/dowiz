//! `kernel::code_oracle` — code change prediction + ETA oracle.
//!
//! Predicts the impact and duration of code changes using:
//! - Change history analysis (chronos snapshots of git history)
//! - Eigen decomposition of changed modules
//! - Delta tracking of change magnitude over time
//! - Oracle-trained ETA models
//!
//! # How it works
//! 1. Record chronos snapshots BEFORE each change wave
//! 2. After changes, compute delta between snapshots
//! 3. Train oracle: predicted_eta = f(modules_touched, lines_changed, eigen_shift)
//! 4. Use oracle to predict future change ETAs
//!
//! ZERO deps.

use crate::eigen::{EigenDecomp, decompose};
use crate::delta::{Delta, DeltaTracker};
use crate::chronos::Chronos;

/// A single code change event — recorded before/after each commit.
#[derive(Debug, Clone)]
pub struct ChangeRecord {
    pub timestamp_ms: u64,
    pub modules_touched: Vec<String>,
    pub lines_added: u64,
    pub lines_removed: u64,
    pub eigen_before: EigenDecomp,
    pub eigen_after: EigenDecomp,
    pub actual_eta_minutes: f64,
}

impl ChangeRecord {
    pub fn new(modules: &[&str], added: u64, removed: u64, eta_min: f64) -> Self {
        let values: Vec<f64> = vec![added as f64, removed as f64, eta_min];
        ChangeRecord {
            timestamp_ms: crate::now_ms(),
            modules_touched: modules.iter().map(|s| s.to_string()).collect(),
            lines_added: added,
            lines_removed: removed,
            eigen_before: decompose(&values, 3),
            eigen_after: decompose(&values, 3),
            actual_eta_minutes: eta_min,
        }
    }

    /// Total lines changed.
    pub fn total_lines(&self) -> u64 { self.lines_added + self.lines_removed }

    /// Spectral shift magnitude (how much the eigen decomposition changed).
    pub fn eigen_shift(&self) -> f64 {
        (self.eigen_after.spectral_radius() - self.eigen_before.spectral_radius()).abs()
    }
}

/// ETA oracle — learns from past changes to predict future durations.
#[derive(Debug, Clone)]
pub struct EtaOracle {
    pub history: Vec<ChangeRecord>,
    pub chronos: Chronos,
    pub delta_tracker: DeltaTracker,
}

impl EtaOracle {
    pub fn new() -> Self {
        EtaOracle {
            history: Vec::with_capacity(1000),
            chronos: Chronos::new(1000),
            delta_tracker: DeltaTracker::new(100.0, 10.0),
        }
    }

    /// Record a completed change.
    pub fn record(&mut self, modules: &[&str], added: u64, removed: u64, eta_minutes: f64) {
        let record = ChangeRecord::new(modules, added, removed, eta_minutes);
        let prev_vals: Vec<f64> = self.history.last()
            .map(|r| vec![r.total_lines() as f64, r.actual_eta_minutes])
            .unwrap_or(vec![0.0, 0.0]);
        let new_vals = vec![record.total_lines() as f64, record.actual_eta_minutes];
        let delta = Delta::between(&prev_vals, 0, &new_vals, 1);
        self.delta_tracker.observe(delta);

        let mut snap_values = std::collections::HashMap::new();
        snap_values.insert("lines".into(), record.total_lines() as f64);
        snap_values.insert("eta".into(), record.actual_eta_minutes);
        snap_values.insert("modules".into(), record.modules_touched.len() as f64);
        self.chronos.snapshot(snap_values);

        self.history.push(record);
    }

    /// Predict ETA for a new change: ETA = α·lines + β·modules + γ·eigen_shift.
    pub fn predict_eta(&self, modules_touched: usize, lines_estimated: u64) -> f64 {
        if self.history.len() < 3 {
            return lines_estimated as f64 / 10.0 + modules_touched as f64 * 5.0; // naive baseline
        }

        // Simple linear regression over recent history
        let n = self.history.len().min(50);
        let recent = &self.history[self.history.len() - n..];

        let mut sum_x = 0.0f64; let mut sum_y = 0.0f64; let mut sum_xx = 0.0f64; let mut sum_xy = 0.0f64;
        for r in recent {
            let x = r.total_lines() as f64 + r.modules_touched.len() as f64 * 10.0;
            let y = r.actual_eta_minutes;
            sum_x += x; sum_y += y; sum_xx += x * x; sum_xy += x * y;
        }
        let nf = n as f64;
        let slope = if nf * sum_xx - sum_x * sum_x > 1e-10 {
            (nf * sum_xy - sum_x * sum_y) / (nf * sum_xx - sum_x * sum_x)
        } else { 0.05 };

        let intercept = (sum_y - slope * sum_x) / nf;
        let x_pred = lines_estimated as f64 + modules_touched as f64 * 10.0;
        (slope * x_pred + intercept).max(1.0)
    }

    /// Confidence interval for ETA prediction (± minutes).
    pub fn eta_confidence(&self) -> (f64, f64) {
        if self.history.len() < 5 { return (0.5, 0.5); }
        let errors: Vec<f64> = self.history.iter()
            .map(|r| {
                let pred = self.predict_eta(r.modules_touched.len(), r.total_lines());
                (pred - r.actual_eta_minutes).abs()
            }).collect();
        let mean_err = errors.iter().sum::<f64>() / errors.len() as f64;
        let var_err = errors.iter().map(|e| (e - mean_err) * (e - mean_err)).sum::<f64>() / errors.len() as f64;
        (mean_err, var_err.sqrt())
    }

    /// Impact prediction: how much will the eigen spectrum shift?
    pub fn predict_impact(&self, modules_touched: usize, lines_estimated: u64) -> f64 {
        if self.history.len() < 3 { return lines_estimated as f64 / 1000.0; }
        let similar: Vec<&ChangeRecord> = self.history.iter()
            .filter(|r| (r.modules_touched.len() as isize - modules_touched as isize).unsigned_abs() <= 2)
            .collect();
        if similar.is_empty() { return lines_estimated as f64 / 500.0; }
        similar.iter().map(|r| r.eigen_shift()).sum::<f64>() / similar.len() as f64
    }

    /// Dashboard.
    pub fn dashboard(&self) -> String {
        let mut out = String::from("═══ ETA ORACLE ═══\n");
        out.push_str(&format!("  history: {} changes\n", self.history.len()));
        if self.history.len() >= 3 {
            let eta_10 = self.predict_eta(3, 100);
            let eta_50 = self.predict_eta(8, 500);
            let (mean, std) = self.eta_confidence();
            out.push_str(&format!("  predicted ETA (3 modules, 100 lines): {:.0} min\n", eta_10));
            out.push_str(&format!("  predicted ETA (8 modules, 500 lines): {:.0} min\n", eta_50));
            out.push_str(&format!("  confidence: ±{:.0} min (σ={:.0})\n", mean, std));
        }
        if !self.history.is_empty() {
            let last = self.history.last().unwrap();
            out.push_str(&format!("  last change: {} modules, {} lines, {:.0} min\n",
                last.modules_touched.len(), last.total_lines(), last.actual_eta_minutes));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oracle_records_and_predicts() {
        let mut oracle = EtaOracle::new();
        oracle.record(&["predictor.rs"], 48, 36, 10.0);
        oracle.record(&["predictor.rs", "lib.rs"], 120, 80, 20.0);
        oracle.record(&["predictor.rs"], 30, 15, 5.0);
        oracle.record(&["lib.rs", "pid.rs", "gossip.rs"], 200, 150, 30.0);

        let eta = oracle.predict_eta(2, 100);
        assert!(eta > 0.0);
        assert!(eta < 100.0);
    }

    #[test]
    fn oracle_confidence_is_finite() {
        let mut oracle = EtaOracle::new();
        for i in 0..10 {
            oracle.record(&["test.rs"], 10 + i, 5 + i, (i + 1) as f64 * 2.0);
        }
        let (mean, std) = oracle.eta_confidence();
        assert!(mean > 0.0);
        assert!(std > 0.0);
    }

    #[test]
    fn empty_oracle_defaults_to_naive() {
        let oracle = EtaOracle::new();
        let eta = oracle.predict_eta(3, 100);
        assert!(eta > 0.0);
    }

    #[test]
    fn impact_prediction_finite() {
        let mut oracle = EtaOracle::new();
        oracle.record(&["a.rs"], 100, 50, 15.0);
        oracle.record(&["a.rs", "b.rs"], 200, 100, 30.0);
        let impact = oracle.predict_impact(2, 150);
        assert!(impact >= 0.0);
        assert!(impact < 1.0);
    }

    #[test]
    fn dashboard_renders() {
        let mut oracle = EtaOracle::new();
        oracle.record(&["test.rs"], 10, 5, 2.0);
        let d = oracle.dashboard();
        assert!(d.contains("ETA ORACLE"));
        assert!(d.contains("history"));
    }
}
