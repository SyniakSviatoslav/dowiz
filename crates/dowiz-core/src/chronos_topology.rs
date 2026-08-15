#![allow(unused)]
//! `dowiz_core::chronos_topology` — chrono-topological foundation (pure core).
//!
//! The root substrate for ALL state management. Every module writes its state
//! here as a TriMatrix indexed by timestamp. The engine maintains three views:
//!
//!   PAST     = snapshot at t₀    (what WAS)
//!   PRESENT  = snapshot at t₁    (what IS)
//!   PREDICTED = extrapolated t₂  (what WILL BE)
//!
//! Prediction = PRESENT + (PRESENT - PAST) + spectral drift correction.
//! All three matrices compared via delta, drift, and stability metrics.
//!
//! # Chrono-topological navigation
//! - Time axis: timestamp-indexed history (chronos)
//! - Space axes: TriMatrix (x=rows, y=cols, z=time depth)
//! - Topology: connectivity between states (which past→present→predicted paths exist)
//! - Navigation: jump to any (row, col, timestamp) and observe the system
//!
//! ZERO deps. Uses trinary, trig, wave, chronos.
//!
//! The wall-clock-stamped entry points (`TemporalTrinity::new`/`advance`,
//! `ChronoTopology::register`/`update`/`navigate`) take `now_ms` explicitly so
//! this module stays `no_std`; the kernel shim wraps them as free functions.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use crate::trinary::{Tri, TriMatrix, Rgb, DeltaChain};
use crate::trig::Xyz;
use crate::wave::{InterferenceField, spectral_fingerprint};

/// A 4D point in chrono-topological space: (row, col, timestamp_ms, xyz).
#[derive(Debug, Clone, Copy)]
pub struct CT4 {
    pub row: u32,         // matrix row (system dimension)
    pub col: u32,         // matrix col (metric index)
    pub ts: u64,          // timestamp (universal index)
    pub xyz: Xyz,         // phase-space encoding
}

impl CT4 {
    pub fn new(row: u32, col: u32, ts: u64, xyz: Xyz) -> Self {
        CT4 { row, col, ts, xyz }
    }
    /// Phase distance between two 4D points (xyz distance, normalized).
    pub fn dist(&self, other: &CT4) -> f64 {
        self.xyz.dist_norm(&other.xyz)
    }
}

/// The three temporal views: past, present, predicted.
#[derive(Debug, Clone)]
pub struct TemporalTrinity {
    pub past: TriMatrix,
    pub past_ts: u64,
    pub present: TriMatrix,
    pub present_ts: u64,
    pub predicted: TriMatrix,
    pub predicted_ts: u64,
    /// Confidence in prediction [0, 1].
    pub prediction_confidence: f64,
}

impl TemporalTrinity {
    pub fn new(rows: usize, cols: usize, now_ms: u64) -> Self {
        TemporalTrinity {
            past: TriMatrix::new(rows, cols),
            past_ts: now_ms,
            present: TriMatrix::new(rows, cols),
            present_ts: now_ms,
            predicted: TriMatrix::new(rows, cols),
            predicted_ts: now_ms + 1000,
            prediction_confidence: 0.0,
        }
    }

    /// Advance: present → past, predicted → present, new prediction computed.
    pub fn advance(&mut self, new_present: TriMatrix, now_ms: u64) {
        self.past = core::mem::replace(&mut self.present, new_present);
        self.past_ts = self.present_ts;
        self.present_ts = now_ms;
        self.predict_next();
    }

    /// Predict the next state using Kalman-like update with adaptive gain.
    ///
    /// Gain is adapted based on prior prediction error: higher error → higher
    /// gain (react faster to changes); lower error → lower gain (smoother).
    pub fn predict_next(&mut self) {
        // Adaptive gain: bounded sigmoid over prediction error.
        let err = self.prediction_error();
        let gain = 1.0 / (1.0 + crate::math::exp(-10.0 * (err - 0.3))); // steep around 0.3 error
        // Clamp into [0.05, 0.95] so extreme values don't lock the filter.
        let gain = gain.clamp(0.05, 0.95);

        self.predicted = self.present.kalman_predict(&self.past, gain);
        self.predicted_ts = self.present_ts + (self.present_ts - self.past_ts).max(100);
        self.prediction_confidence = 1.0 - err;
    }

    /// Delta between past and present (how much changed).
    pub fn delta_past_present(&self) -> (f64, usize) {
        let mut delta_sum = 0.0f64;
        let mut changes = 0usize;
        for r in 0..self.present.rows {
            for c in 0..self.present.cols {
                let past_rgb = Rgb::from_tri(self.past.get(r, c));
                let pres_rgb = Rgb::from_tri(self.present.get(r, c));
                if past_rgb != pres_rgb {
                    delta_sum += past_rgb.delta_norm(&pres_rgb);
                    changes += 1;
                }
            }
        }
        (delta_sum, changes)
    }

    /// Delta between predicted and actual present (prediction error).
    pub fn prediction_error(&self) -> f64 {
        let mut err = 0.0f64;
        let mut count = 0usize;
        for r in 0..self.present.rows {
            for c in 0..self.present.cols {
                let pred_rgb = Rgb::from_tri(self.predicted.get(r, c));
                let pres_rgb = Rgb::from_tri(self.present.get(r, c));
                err += pred_rgb.delta_norm(&pres_rgb);
                count += 1;
            }
        }
        if count == 0 { 0.0 } else { err / count as f64 }
    }

    /// RGB bitmap comparison: show past/present/predicted side by side.
    pub fn comparison_bitmap(&self) -> Vec<(Rgb, Rgb, Rgb)> {
        let n = self.present.rows.min(self.past.rows);
        (0..n).map(|r| (
            self.past.row_rgb(r),
            self.present.row_rgb(r),
            self.predicted.row_rgb(r),
        )).collect()
    }

    /// Stability score: fraction of cells unchanged from past to present.
    pub fn stability(&self) -> f64 {
        let (_, changes) = self.delta_past_present();
        let total = self.present.rows * self.present.cols;
        if total == 0 { 1.0 } else { 1.0 - changes as f64 / total as f64 }
    }
}

/// Chrono-topological engine — the root state substrate.
#[derive(Debug, Clone)]
pub struct ChronoTopology {
    /// Named temporal trinities (one per subsystem: "orders", "security", "enrichment"...)
    pub subsystems: BTreeMap<String, TemporalTrinity>,
    /// Global delta chain (system-wide drift tracking).
    pub drift: DeltaChain,
    /// Interference field (spectral superposition of all waves).
    pub field: InterferenceField,
    /// Navigation history: visited 4D points.
    pub nav_history: Vec<CT4>,
}

impl ChronoTopology {
    pub fn new() -> Self {
        ChronoTopology {
            subsystems: BTreeMap::new(),
            drift: DeltaChain::new(),
            field: InterferenceField::new(),
            nav_history: Vec::new(),
        }
    }

    /// Register a subsystem with given matrix dimensions (stamped at `now_ms`).
    pub fn register(&mut self, name: &str, rows: usize, cols: usize, now_ms: u64) {
        self.subsystems.insert(name.to_string(), TemporalTrinity::new(rows, cols, now_ms));
    }

    /// Update a subsystem's present state (advances past→present→predicted).
    pub fn update(&mut self, name: &str, matrix: TriMatrix, now_ms: u64) {
        if let Some(trinity) = self.subsystems.get_mut(name) {
            trinity.advance(matrix, now_ms);
            // Emit a spectral fingerprint for this state change
            let (delta, _) = trinity.delta_past_present();
            let wave = spectral_fingerprint(name, delta, now_ms);
            self.field.add_wave(wave);
        }
    }

    /// Navigate to a specific 4D point.
    pub fn navigate(&mut self, row: u32, col: u32, ts: u64, now_ms: u64) {
        let xyz = self.field.xyz_state(now_ms);
        self.nav_history.push(CT4::new(row, col, ts, xyz));
        self.drift.push(Rgb::from_tri(Tri::Unknown)); // placeholder
    }

    /// Global system state as a single TriMatrix (concatenated rows from all subsystems).
    pub fn global_matrix(&self) -> TriMatrix {
        let mut rows = 0usize;
        let cols = self.subsystems.values().next().map(|t| t.present.cols).unwrap_or(0);
        for t in self.subsystems.values() { rows += t.present.rows; }
        let mut global = TriMatrix::new(rows, cols);
        let mut offset = 0;
        for t in self.subsystems.values() {
            for r in 0..t.present.rows {
                for c in 0..t.present.cols {
                    global.set(offset + r, c, t.present.get(r, c));
                }
            }
            offset += t.present.rows;
        }
        global
    }

    /// Dashboard: per-subsystem status.
    pub fn dashboard(&self) -> String {
        let mut out = String::from("═══ CHRONO-TOPOLOGY ═══\n");
        for (name, t) in &self.subsystems {
            let (delta, changes) = t.delta_past_present();
            let pred_err = t.prediction_error();
            let stab = t.stability();
            out.push_str(&format!("  {}: Δ={:.3} chg={} err={:.3} stab={:.3}\n",
                name, delta, changes, pred_err, stab));
        }
        out.push_str(&format!("  Global drift: {:.3}\n", self.drift.total_drift()));
        out.push_str(&format!("  Active waves: {}\n", self.field.active_count()));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temporal_trinity_advances() {
        let mut tt = TemporalTrinity::new(2, 2, 1000);
        let mut m = TriMatrix::new(2, 2);
        m.set(0, 0, Tri::True);
        m.set(0, 1, Tri::False);
        tt.advance(m.clone(), 1000);
        // After advance: present = m, predicted computed
        assert_eq!(tt.present.get(0, 0), Tri::True);
        assert!(tt.prediction_confidence >= 0.0);
    }

    #[test]
    fn temporal_trinity_stability() {
        let mut tt = TemporalTrinity::new(2, 2, 1000);
        let mut m1 = TriMatrix::new(2, 2);
        m1.set(0, 0, Tri::True);
        tt.advance(m1.clone(), 1000);
        tt.advance(m1.clone(), 2000); // same state twice
        assert!((tt.stability() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn temporal_trinity_delta() {
        let mut tt = TemporalTrinity::new(2, 2, 1000);
        let mut m1 = TriMatrix::new(2, 2);
        m1.set(0, 0, Tri::True);
        let mut m2 = TriMatrix::new(2, 2);
        m2.set(0, 0, Tri::False);
        tt.advance(m1, 1000);
        tt.advance(m2, 2000);
        let (delta, changes) = tt.delta_past_present();
        assert!(delta > 0.0);
        assert_eq!(changes, 1);
    }

    #[test]
    fn chrono_topology_register_and_update() {
        let mut ct = ChronoTopology::new();
        ct.register("test", 2, 3, 1000);
        let mut m = TriMatrix::new(2, 3);
        m.set(0, 0, Tri::True);
        ct.update("test", m, 1000);
        assert_eq!(ct.subsystems.len(), 1);
    }

    #[test]
    fn chrono_topology_dashboard() {
        let mut ct = ChronoTopology::new();
        ct.register("orders", 2, 2, 1000);
        ct.register("security", 1, 2, 1000);
        let d = ct.dashboard();
        assert!(d.contains("orders"));
        assert!(d.contains("security"));
    }
}
