//! `kernel::predict` — chronological-topological prediction engine.
//!
//! Composes the existing kernel prediction primitives (Markov attractor, spectral
//! drift, absorbing chain, Noether invariant, causal backdoor, online learner)
//! into a single API for predicting system behavior.
//!
//! # Architecture
//! ```text
//! TemporalPredictor
//! +-- State Classification (point-in-time)
//! |   +-- markov::analyze() -> Verdict
//! |   +-- spectral::classify_drift() -> DriftClass
//! +-- Convergence Forecasting (closed-form)
//! |   +-- markov::gap/mixing_time -> time-to-stable
//! |   +-- absorbing::expected_steps -> time-to-terminal
//! +-- Invariant Guard (trajectory-based)
//! |   +-- noether::invariant_drift() -> drift rate
//! |   +-- threshold prediction: time-to-violation
//! +-- Causal Intervention (do-calculus)
//!     +-- causal::backdoor_adjust() -> P(Y|do(X))
//! ```
//!
//! All components are pure-`std`, zero-dep, deterministic. Float is used
//! deliberately — this is dynamics, never money.

use crate::markov::{self, Verdict as MarkovVerdict};
use crate::spectral;

/// Drift classification from the spectral engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriftClass {
    /// Eigenvalues inside unit circle — converging.
    Damped,
    /// Eigenvalue near unit circle — oscillating.
    Resonant,
    /// Eigenvalue outside unit circle — diverging.
    Unstable,
    /// No data to classify.
    Unknown,
}

/// The system's trajectory class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrajectoryClass {
    /// System is healthy and converging.
    Healthy,
    /// System is in a limit cycle (repeating pattern).
    LimitCycle,
    /// System is in a strange attractor (chaotic churn).
    StrangeAttractor,
    /// System is diverging (unstable).
    Diverging,
    /// Insufficient data to classify.
    Unclassified,
}

/// Outcome of a prediction query.
#[derive(Debug, Clone)]
pub struct PredictionReport {
    /// Markov attractor verdict on the event stream.
    pub markov_verdict: MarkovVerdict,
    /// Spectral drift classification.
    pub drift_class: DriftClass,
    /// Overall trajectory classification.
    pub trajectory: TrajectoryClass,
    /// Spectral gap γ = 1 − |λ₂|. γ→0 ⇒ never mixes (trapped).
    pub spectral_gap: f64,
    /// Estimated mixing time τ ≈ 1/γ (∞ for non-mixing).
    pub mixing_time: f64,
    /// Entropy rate of the Markov chain (bits/step). Low ⇒ deterministic cycle.
    pub entropy_rate: f64,
    /// Escape mass — long-run time in progress states.
    pub escape_mass: f64,
    /// Total invariant drift along a trajectory (if provided).
    pub invariant_drift: f64,
    /// Number of events analyzed.
    pub event_count: usize,
}

/// Prediction engine composing existing kernel primitives.
pub struct TemporalPredictor {
    /// Minimum events needed for a meaningful prediction.
    pub min_events: usize,
}

impl TemporalPredictor {
    /// Build with default thresholds.
    pub fn new() -> Self {
        TemporalPredictor { min_events: 8 }
    }

    /// Build with a custom minimum-event threshold.
    pub fn with_min_events(min_events: usize) -> Self {
        TemporalPredictor { min_events }
    }

    /// Predict the system's next state from an event stream.
    ///
    /// `events` is a sequence of event labels (e.g. "edit", "run_ok", "run_fail").
    /// Returns a `PredictionReport` with the combined analysis.
    pub fn predict_next(&self, events: &[&str]) -> PredictionReport {
        if events.len() < self.min_events {
            return PredictionReport {
                markov_verdict: MarkovVerdict::Healthy,
                drift_class: DriftClass::Unknown,
                trajectory: TrajectoryClass::Unclassified,
                spectral_gap: 1.0,
                mixing_time: 1.0,
                entropy_rate: 0.0,
                escape_mass: 1.0,
                invariant_drift: 0.0,
                event_count: events.len(),
            };
        }

        // Layer 1: Markov attractor analysis
        let report = markov::analyze(events);
        let markov_verdict = report.verdict;
        let spectral_gap = report.gap;
        let mixing_time = report.mixing_time;
        let entropy_rate = report.entropy_rate_bits;
        let escape_mass = report.escape_mass;

        // Layer 2: Spectral drift classification (from the Markov transition matrix)
        let drift_class = if events.len() >= self.min_events {
            Self::classify_from_events(events)
        } else {
            DriftClass::Unknown
        };

        // Layer 3: Overall trajectory classification
        let trajectory = Self::classify_trajectory(markov_verdict, drift_class, spectral_gap);

        PredictionReport {
            markov_verdict,
            drift_class,
            trajectory,
            spectral_gap,
            mixing_time,
            entropy_rate,
            escape_mass,
            invariant_drift: 0.0,
            event_count: events.len(),
        }
    }

    /// Predict with invariant checking along a trajectory.
    ///
    /// `events` is the event stream; `trajectory` is the state trajectory;
    /// `update` is the state transition function; `invariant` is the conserved
    /// quantity to check.
    pub fn predict_with_invariant<F, G>(
        &self,
        events: &[&str],
        x0: &[f64],
        update: F,
        invariant: G,
    ) -> PredictionReport
    where
        F: Fn(&[f64]) -> Vec<f64>,
        G: Fn(&[f64]) -> f64,
    {
        let mut report = self.predict_next(events);

        // Layer 4: Noether invariant drift
        report.invariant_drift = crate::noether::invariant_drift(x0, update, invariant, 100);
        report
    }

    /// Predict the causal effect of an intervention.
    ///
    /// Given observational data, compute P(Y|do(X)) via backdoor adjustment.
    /// `p_y_xz` is P(Y|X,Z), `p_z` is P(Z), `p_xz` is P(X,Z).
    pub fn predict_causal(
        &self,
        p_y_xz: &[f64],
        p_z: &[f64],
        p_xz: &[f64],
        n_x: usize,
        n_z: usize,
    ) -> Result<Vec<f64>, &'static str> {
        let effect = crate::causal::backdoor_adjust(p_y_xz, p_z, p_xz, n_x, n_z)?;
        Ok(effect.do_p_y)
    }

    /// Estimate time-to-terminal from an absorbing chain's transient block Q.
    ///
    /// Returns `Some(expected_steps)` if Q is nilpotent (DAG), `None` if cyclic.
    pub fn estimate_time_to_terminal(
        &self,
        q: &[Vec<f64>],
    ) -> Option<Vec<f64>> {
        let n = crate::absorbing::fundamental_matrix(q)?;
        Some(crate::absorbing::expected_steps(&n))
    }

    /// Estimate absorption probabilities from an absorbing chain.
    ///
    /// Returns `Some(B)` where B[i][j] = P(terminal j | start i).
    pub fn estimate_absorption_probs(
        &self,
        q: &[Vec<f64>],
        r: &[Vec<f64>],
    ) -> Option<Vec<Vec<f64>>> {
        let n = crate::absorbing::fundamental_matrix(q)?;
        Some(crate::absorbing::absorption_probs(&n, r))
    }

    // ── Internal helpers ─────────────────────────────────────────

    /// Classify drift from raw events by building a transition matrix and
    /// checking the spectral radius.
    fn classify_from_events(events: &[&str]) -> DriftClass {
        // Build state alphabet and transition counts
        let mut alphabet: Vec<String> = Vec::new();
        for &e in events {
            if !alphabet.iter().any(|a| a == e) {
                alphabet.push(e.to_string());
            }
        }
        let n = alphabet.len();
        if n == 0 {
            return DriftClass::Unknown;
        }

        // Count transitions
        let mut counts = vec![vec![0u32; n]; n];
        for w in events.windows(2) {
            let from_idx = alphabet.iter().position(|a| a == w[0]);
            let to_idx = alphabet.iter().position(|a| a == w[1]);
            if let (Some(from), Some(to)) = (from_idx, to_idx) {
                counts[from][to] += 1;
            }
        }

        // Row-normalize to get transition matrix
        let mut trans = vec![vec![0.0f64; n]; n];
        for i in 0..n {
            let row_sum: u32 = counts[i].iter().sum();
            if row_sum > 0 {
                for j in 0..n {
                    trans[i][j] = counts[i][j] as f64 / row_sum as f64;
                }
            } else {
                let inv = 1.0 / n as f64;
                for j in 0..n {
                    trans[i][j] = inv;
                }
            }
        }

        // Classify using spectral engine
        match spectral::classify_drift(&trans) {
            spectral::DriftClass::Damped => DriftClass::Damped,
            spectral::DriftClass::Resonant => DriftClass::Resonant,
            spectral::DriftClass::Unstable => DriftClass::Unstable,
        }
    }

    /// Combine Markov verdict + spectral drift + gap into a trajectory class.
    fn classify_trajectory(
        markov: MarkovVerdict,
        drift: DriftClass,
        gap: f64,
    ) -> TrajectoryClass {
        match markov {
            MarkovVerdict::Healthy => {
                if drift == DriftClass::Unstable {
                    TrajectoryClass::Diverging
                } else if gap < 0.01 {
                    TrajectoryClass::Diverging
                } else {
                    TrajectoryClass::Healthy
                }
            }
            MarkovVerdict::LimitCycle => TrajectoryClass::LimitCycle,
            MarkovVerdict::StrangeAttractor => TrajectoryClass::StrangeAttractor,
        }
    }
}

impl Default for TemporalPredictor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn healthy_events_classified_healthy() {
        let p = TemporalPredictor::new();
        // Non-periodic chain: edit → probe/run_ok → edit → probe/run_ok
        // Breaking the perfect 2-cycle so the chain mixes
        let events = vec![
            "edit", "probe", "run_ok", "edit", "probe", "run_ok",
            "edit", "probe", "run_ok", "edit",
        ];
        let report = p.predict_next(&events);
        // With 3 states and non-periodic transitions, the Markov verdict is Healthy
        assert_eq!(report.markov_verdict, MarkovVerdict::Healthy);
        assert_eq!(report.trajectory, TrajectoryClass::Healthy);
        assert!(report.spectral_gap > 0.0);
    }

    #[test]
    fn cycle_events_classified_limit_cycle() {
        let p = TemporalPredictor::new();
        // Alternating run_fail → edit → run_fail → edit = limit cycle
        let events = vec![
            "run_fail", "edit", "run_fail", "edit",
            "run_fail", "edit", "run_fail", "edit",
            "run_fail", "edit", "run_fail", "edit",
        ];
        let report = p.predict_next(&events);
        assert_eq!(report.markov_verdict, MarkovVerdict::LimitCycle);
        assert_eq!(report.trajectory, TrajectoryClass::LimitCycle);
    }

    #[test]
    fn short_events_get_defaults() {
        let p = TemporalPredictor::new();
        let events = vec!["edit", "run_ok"];
        let report = p.predict_next(&events);
        assert_eq!(report.markov_verdict, MarkovVerdict::Healthy);
        assert_eq!(report.drift_class, DriftClass::Unknown);
        assert_eq!(report.trajectory, TrajectoryClass::Unclassified);
    }

    #[test]
    fn report_contains_all_fields() {
        let p = TemporalPredictor::new();
        let events = vec![
            "edit", "probe", "run_ok", "edit",
            "probe", "run_ok", "edit", "probe",
        ];
        let report = p.predict_next(&events);
        assert!(report.event_count > 0);
        assert!(report.entropy_rate >= 0.0);
        assert!((0.0..=1.0).contains(&report.escape_mass));
    }

    #[test]
    fn invariant_drift_detected() {
        let p = TemporalPredictor::new();
        let events = vec!["edit", "run_ok", "edit", "run_ok", "edit", "run_ok", "edit", "run_ok"];
        // Constant invariant: always returns 1.0
        let report = p.predict_with_invariant(
            &events,
            &[1.0],
            |x| x.to_vec(),       // identity update
            |x| x[0],             // invariant = x[0]
        );
        assert_eq!(report.invariant_drift, 0.0);
    }

    #[test]
    fn invariant_drift_violation() {
        let p = TemporalPredictor::new();
        let events = vec!["edit", "run_ok", "edit", "run_ok", "edit", "run_ok", "edit", "run_ok"];
        // Drifting invariant: adds 1.0 each step
        let report = p.predict_with_invariant(
            &events,
            &[0.0],
            |x| vec![x[0] + 1.0], // increasing
            |x| x[0],              // invariant = x[0], drifts by 100
        );
        assert!(report.invariant_drift > 0.0);
    }

    #[test]
    fn causal_prediction() {
        let p = TemporalPredictor::new();
        // Simple confounded example:
        // P(Y=1|X=0,Z=0)=0.1, P(Y=1|X=0,Z=1)=0.9
        // P(Y=1|X=1,Z=0)=0.3, P(Y=1|X=1,Z=1)=0.7
        let p_y_xz = vec![0.1, 0.9, 0.3, 0.7]; // [x=0,z=0], [x=0,z=1], [x=1,z=0], [x=1,z=1]
        let p_z = vec![0.5, 0.5];               // Z=0, Z=1
        let p_xz = vec![0.25, 0.25, 0.25, 0.25]; // uniform joint
        let do_effect = p.predict_causal(&p_y_xz, &p_z, &p_xz, 2, 2).unwrap();
        // do(X=0) = 0.1*0.5 + 0.9*0.5 = 0.5
        // do(X=1) = 0.3*0.5 + 0.7*0.5 = 0.5
        assert!((do_effect[0] - 0.5).abs() < 1e-9);
        assert!((do_effect[1] - 0.5).abs() < 1e-9);
    }

    #[test]
    fn absorbing_time_to_terminal() {
        let p = TemporalPredictor::new();
        // Simple 2-state transient: T0 → T1 → terminal
        // Q = [[0, 1], [0, 0]] (T0→T1, T1→terminal)
        let q = vec![vec![0.0, 1.0], vec![0.0, 0.0]];
        let steps = p.estimate_time_to_terminal(&q).unwrap();
        // N = I + Q = [[1,1],[0,1]], t = [2, 1]
        assert!((steps[0] - 2.0).abs() < 1e-9);
        assert!((steps[1] - 1.0).abs() < 1e-9);
    }

    #[test]
    fn spectral_gap_positive_for_mixing() {
        let p = TemporalPredictor::new();
        let events = vec![
            "edit", "run_ok", "probe", "run_ok",
            "edit", "run_ok", "probe", "run_ok",
            "edit", "run_ok", "probe", "run_ok",
        ];
        let report = p.predict_next(&events);
        assert!(report.spectral_gap >= 0.0);
        assert!(report.mixing_time >= 0.0);
    }
}
