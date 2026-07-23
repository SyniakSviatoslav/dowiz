//! `kernel::delta` — delta calculus replaces classic comparisons.
//!
//! In a spectral/fractal system, classic comparisons (a > b, x == y) lose
//! information. Delta calculus compares the CHANGE between states, not the
//! absolute values. This is the core comparison engine for trinary+trig+eigen.
//!
//! # Delta replaces comparison
//!   Classic:  if a > b { act }
//!   Delta:    if delta(a, b).magnitude > threshold { act }
//!   Classic:  assert_eq!(x, y)
//!   Delta:    assert!(delta(x, y).is_stable(threshold))
//!
//! # Operations
//!   Δv = v₁ - v₀          → vector delta (direction + magnitude)
//!   ∂t = t₁ - t₀          → time delta
//!   rate = Δv / ∂t        → rate of change (velocity)
//!   accel = Δ(rate) / ∂t  → acceleration (second derivative)
//!   drift = cumulative Σ|Δv| → total system drift
//!
//! ZERO deps. Uses eigen, trig, trinary.

use crate::eigen::{EigenDecomp, decompose};
use crate::trig::{Xyz, PhaseVector};

// ─── Delta — the fundamental change primitive ─────────────────────────────

/// A delta between two vectors. Carries magnitude, direction, and rate.
#[derive(Debug, Clone)]
pub struct Delta {
    pub components: Vec<f64>,  // v₁ - v₀
    pub magnitude: f64,        // ||Δv||
    pub ts_from: u64,
    pub ts_to: u64,
    pub rate: f64,             // magnitude / (t₁-t₀) in units/ms
}

impl Delta {
    /// Compute delta between two vectors at two timestamps.
    pub fn between(v0: &[f64], ts0: u64, v1: &[f64], ts1: u64) -> Self {
        let n = v0.len().min(v1.len());
        let mut components = vec![0.0f64; n];
        let mut mag_sq = 0.0f64;
        for i in 0..n {
            components[i] = v1[i] - v0[i];
            mag_sq += components[i] * components[i];
        }
        let magnitude = mag_sq.sqrt();
        let dt = (ts1.saturating_sub(ts0) as f64).max(1.0);
        Delta { components, magnitude, ts_from: ts0, ts_to: ts1, rate: magnitude / dt }
    }

    /// Is this delta significant? (magnitude > threshold).
    pub fn is_significant(&self, threshold: f64) -> bool {
        self.magnitude > threshold
    }

    /// Is the system stable? (rate < threshold — change is slowing down).
    pub fn is_stable(&self, rate_threshold: f64) -> bool {
        self.rate < rate_threshold
    }

    /// Normalized delta components: unit direction vector.
    pub fn direction(&self) -> Vec<f64> {
        if self.magnitude < 1e-15 { return vec![0.0; self.components.len()]; }
        self.components.iter().map(|&c| c / self.magnitude).collect()
    }

    /// Project delta onto a target direction: how much of the change is in this direction.
    pub fn project(&self, direction: &[f64]) -> f64 {
        let n = self.components.len().min(direction.len());
        let mut dot = 0.0f64;
        let mut dir_norm = 0.0f64;
        for i in 0..n {
            dot += self.components[i] * direction[i];
            dir_norm += direction[i] * direction[i];
        }
        if dir_norm < 1e-15 { return 0.0; }
        dot / dir_norm.sqrt()
    }
}

// ─── EigenDelta — compare eigen decompositions ────────────────────────────

/// Delta between two eigen decompositions. Measures spectral distance.
pub struct EigenDelta {
    pub spectral_distance: f64,   // ||λ₁ - λ₀|| between dominant eigenvalues
    pub mode_count_delta: isize,  // change in number of unstable modes
    pub dominant_shift: f64,      // angle between dominant eigenvectors
}

impl EigenDelta {
    /// Compare two eigen decompositions.
    pub fn between(d0: &EigenDecomp, d1: &EigenDecomp) -> Self {
        let r0 = d0.spectral_radius();
        let r1 = d1.spectral_radius();
        let spectral_distance = (r1 - r0).abs();
        let mode_count_delta = d1.unstable_count() as isize - d0.unstable_count() as isize;

        let dominant_shift = match (d0.dominant(), d1.dominant()) {
            (Some(a), Some(b)) => {
                let n = a.dim().min(b.dim());
                let mut dot = 0.0; let mut na = 0.0; let mut nb = 0.0;
                for i in 0..n {
                    dot += a.vector[i] * b.vector[i];
                    na += a.vector[i] * a.vector[i];
                    nb += b.vector[i] * b.vector[i];
                }
                if na < 1e-15 || nb < 1e-15 { 0.0 }
                else { (dot / (na.sqrt() * nb.sqrt())).acos() }
            }
            _ => 0.0,
        };

        EigenDelta { spectral_distance, mode_count_delta, dominant_shift }
    }

    /// Is this a significant spectral change?
    pub fn is_significant(&self, dist_threshold: f64, shift_threshold: f64) -> bool {
        self.spectral_distance > dist_threshold || self.dominant_shift > shift_threshold
    }

    /// Are new unstable modes appearing? (system becoming less stable).
    pub fn is_destabilizing(&self) -> bool { self.mode_count_delta > 0 }
}

// ─── PhaseDelta — compare phase-space states ──────────────────────────────

/// Delta in phase space between two PhaseVectors or Xyz states.
pub struct PhaseDelta {
    pub angle_delta: f64,      // total angular drift (sum of phase differences)
    pub xyz_distance: f64,     // Euclidean distance in xyz space
    pub coherence: f64,        // dot-product coherence [0,1]: 1 = identical, 0 = orthogonal
}

impl PhaseDelta {
    pub fn between_phases(a: &PhaseVector, b: &PhaseVector) -> Self {
        let n = a.phases.len().min(b.phases.len());
        let mut angle_sum = 0.0f64;
        for i in 0..n {
            angle_sum += a.phases[i].distance(&b.phases[i]);
        }
        let angle_delta = angle_sum / n.max(1) as f64;
        let coherence = a.dot(b).clamp(0.0, 1.0);
        PhaseDelta { angle_delta, xyz_distance: 0.0, coherence }
    }

    pub fn between_xyz(a: &Xyz, b: &Xyz) -> Self {
        PhaseDelta {
            angle_delta: 0.0,
            xyz_distance: a.distance(b),
            coherence: 1.0 - a.dist_norm(b),
        }
    }

    /// Significant if coherence drops below threshold.
    pub fn is_significant(&self, coherence_threshold: f64) -> bool {
        self.coherence < coherence_threshold
    }

    /// Phase drift rate: angle_delta / time.
    pub fn drift_rate(&self, dt_ms: f64) -> f64 {
        if dt_ms < 1.0 { return 0.0; }
        self.angle_delta / dt_ms
    }
}

// ─── DeltaTracker — cumulative drift monitoring ───────────────────────────

/// Tracks cumulative changes between successive states. Alerts on regime change.
#[derive(Debug, Clone)]
pub struct DeltaTracker {
    pub history: Vec<Delta>,
    pub cumulative_drift: f64,
    pub max_rate: f64,
    pub alarm_threshold: f64,
    pub alarm_rate: f64,
}

impl DeltaTracker {
    pub fn new(alarm_threshold: f64, alarm_rate: f64) -> Self {
        DeltaTracker { history: Vec::new(), cumulative_drift: 0.0, max_rate: 0.0,
            alarm_threshold, alarm_rate }
    }

    /// Push a new delta observation.
    pub fn observe(&mut self, delta: Delta) {
        self.cumulative_drift += delta.magnitude;
        self.max_rate = self.max_rate.max(delta.rate);
        self.history.push(delta);
    }

    /// Observe a state transition (computes delta internally).
    pub fn observe_transition(&mut self, v0: &[f64], ts0: u64, v1: &[f64], ts1: u64) {
        let d = Delta::between(v0, ts0, v1, ts1);
        self.observe(d);
    }

    /// Is the system in alarm? (recent drift exceeds threshold OR rate too high).
    pub fn is_alarming(&self, window: usize) -> bool {
        let recent: f64 = self.history.iter().rev().take(window)
            .map(|d| d.magnitude).sum();
        let recent_rate: f64 = self.history.iter().rev().take(window)
            .map(|d| d.rate).fold(0.0, f64::max);
        recent > self.alarm_threshold || recent_rate > self.alarm_rate
    }

    /// Reset cumulative drift (after handling alarm).
    pub fn reset(&mut self) {
        self.cumulative_drift = 0.0;
        self.max_rate = 0.0;
        self.history.clear();
    }

    pub fn len(&self) -> usize { self.history.len() }
}

// ─── Comparison replaces std::cmp ─────────────────────────────────────────

/// Delta-based comparison result: Greater/Lesser/Equal replaced by significant drifts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeltaComparison {
    Growing,     //  Δ > threshold  — state is significantly increasing
    Shrinking,   // -Δ > threshold  — state is significantly decreasing
    Stable,      // |Δ| ≤ threshold — no significant change
    Oscillating, // alternating signs in recent window
}

/// Compare two states using delta calculus instead of classic comparison.
pub fn compare(v0: &[f64], ts0: u64, v1: &[f64], ts1: u64, threshold: f64) -> DeltaComparison {
    let d = Delta::between(v0, ts0, v1, ts1);
    if d.magnitude <= threshold { return DeltaComparison::Stable; }
    // Check direction: sum of component signs
    let direction: f64 = d.components.iter().sum();
    if direction > threshold { DeltaComparison::Growing }
    else if direction < -threshold { DeltaComparison::Shrinking }
    else { DeltaComparison::Oscillating }
}

/// Detect oscillation in a sequence of deltas.
pub fn is_oscillating(deltas: &[Delta], window: usize) -> bool {
    if deltas.len() < window + 1 { return false; }
    let recent = &deltas[deltas.len() - window..];
    let signs: Vec<f64> = recent.iter().map(|d| d.components.iter().sum::<f64>().signum()).collect();
    // Alternating signs = oscillation
    let mut flips = 0;
    for i in 1..signs.len() {
        if signs[i] * signs[i-1] < 0.0 { flips += 1; }
    }
    flips >= 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delta_between_vectors() {
        let v0 = vec![0.0, 0.0, 0.0];
        let v1 = vec![3.0, 4.0, 0.0];
        let d = Delta::between(&v0, 1000, &v1, 2000);
        assert!((d.magnitude - 5.0).abs() < 1e-10);
        assert!(d.rate > 0.0);
    }

    #[test]
    fn delta_direction_unit_vector() {
        let v0 = vec![0.0, 0.0];
        let v1 = vec![1.0, 0.0];
        let d = Delta::between(&v0, 0, &v1, 1);
        let dir = d.direction();
        assert!((dir[0] - 1.0).abs() < 1e-10);
        assert!(dir[1].abs() < 1e-10);
    }

    #[test]
    fn eigen_delta_destabilizing() {
        let stable = decompose(&[0.5, 0.3], 2);
        let unstable = decompose(&[1.5, 0.3], 2);
        let ed = EigenDelta::between(&stable, &unstable);
        assert!(ed.spectral_distance > 0.5);
        assert!(ed.is_destabilizing());
    }

    #[test]
    fn phase_delta_coherence() {
        let a = PhaseVector::from_scalars(&[0.0, 0.0]);
        let b = PhaseVector::from_scalars(&[0.0, 0.0]);
        let pd = PhaseDelta::between_phases(&a, &b);
        assert!((pd.coherence - 1.0).abs() < 1e-10);
    }

    #[test]
    fn compare_growing() {
        let v0 = vec![0.0, 0.0];
        let v1 = vec![10.0, 0.0];
        assert_eq!(compare(&v0, 0, &v1, 1, 1.0), DeltaComparison::Growing);
    }

    #[test]
    fn compare_stable() {
        let v = vec![0.0, 0.0];
        assert_eq!(compare(&v, 0, &v, 1, 1.0), DeltaComparison::Stable);
    }

    #[test]
    fn delta_tracker_alarm() {
        let mut dt = DeltaTracker::new(5.0, 100.0);
        let v0 = vec![0.0; 10];
        let v1: Vec<f64> = (0..10).map(|i| i as f64 * 2.0).collect();
        dt.observe_transition(&v0, 0, &v1, 1000);
        assert!(dt.is_alarming(1));
    }

    #[test]
    fn oscillation_detection() {
        let v = vec![0.0];
        let d1 = Delta::between(&v, 0, &[1.0], 1);
        let d2 = Delta::between(&[1.0], 1, &[-1.0], 2);
        let d3 = Delta::between(&[-1.0], 2, &[1.0], 3);
        let d4 = Delta::between(&[1.0], 3, &[-1.0], 4);
        assert!(is_oscillating(&[d1, d2, d3, d4], 3));
    }
}
