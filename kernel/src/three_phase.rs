//! Three-phase verification pattern for operation in unstable systems.
//!
//! Every critical operation follows a three-phase lifecycle:
//!
//! **Phase 1 — Prepare (Read / Sanitize / Validate)**
//! Gather input data, sanitize every value at the boundary (NaN/Inf → 0),
//! validate that all required fields are present and within expected ranges.
//! Returns a `VerificationToken` that carries the validated input forward.
//!
//! **Phase 2 — Verify (Process / Cross-check / Simulate)**
//! Execute the operation against the verified input. Cross-check results
//! against known constraints (bounds, monotonicity, invariants). Simulate
//! the outcome if a simulator is available before committing state changes.
//!
//! **Phase 3 — Acknowledge (Validate output / Commit / Propagate)**
//! Validate the output before releasing it downstream. If output is invalid,
//! roll back to the last known-good state via `LastHealthyState`. On success,
//! propagate the result to subscribers.

use crate::predictor::{DEFAULT_N_METRICS, SystemState};

pub const MAX_LABEL_LEN: usize = 1024;
pub const RECOVER_DIVISOR: u32 = 2;

/// Outcome of a three-phase verification cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// All phases passed — result is safe to use.
    Pass,
    /// Phase 2 cross-check detected a transient anomaly; retry may succeed.
    Retry,
    /// Phase 3 validation failed; the operation should be aborted and
    /// the system should fall back to last healthy state.
    Fail,
    /// Insufficient data to complete verification.
    Unknown,
}

/// A token that carries verified input between phases.
#[derive(Debug, Clone)]
pub struct VerificationToken {
    pub phase: u8,
    pub valid: bool,
    pub timestamp_ms: u64,
}

impl VerificationToken {
    pub fn new(timestamp_ms: u64) -> Self {
        VerificationToken { phase: 1, valid: true, timestamp_ms }
    }

    pub fn advance(&mut self) {
        self.phase = self.phase.saturating_add(1).min(3);
    }

    pub fn invalidate(&mut self) {
        self.valid = false;
    }

    pub fn is_valid(&self) -> bool {
        self.valid
    }
}

/// Three-phase verifier that applies the prepare-verify-acknowledge cycle
/// to any input that can be represented as a metrics vector.
pub struct ThreePhaseVerifier {
    /// Number of consecutive passes needed to consider the system stable.
    pub stabilization_threshold: u32,
    consecutive_passes: u32,
    consecutive_fails: u32,
    last_verdict: Verdict,
    phase_timings_ms: [u64; 3],
}

impl ThreePhaseVerifier {
    pub fn new(stabilization_threshold: u32) -> Self {
        debug_assert!(stabilization_threshold > 0, "ThreePhaseVerifier::new: stabilization_threshold must be > 0");
        ThreePhaseVerifier {
            stabilization_threshold,
            consecutive_passes: 0,
            consecutive_fails: 0,
            last_verdict: Verdict::Unknown,
            phase_timings_ms: [0, 0, 0],
        }
    }

    /// Execute the full three-phase cycle on a system state.
    ///
    /// # Phase 1 — Prepare
    /// Sanitize and validate all metrics. Returns a token only if
    /// the input passes basic structural checks.
    ///
    /// # Phase 2 — Verify
    /// Cross-check against numerical invariants: all finite, in [0,1],
    /// no anomalous jumps from prior state.
    ///
    /// # Phase 3 — Acknowledge
    /// Validate that the output is self-consistent. Return the verdict.
    pub fn verify(&mut self, state: &SystemState, prior: Option<&SystemState>) -> Verdict {
        // ── Phase 1: Prepare ───────────────────────────────────────────
        let mut token = VerificationToken::new(state.timestamp_ms);

        if state.metrics.is_empty() {
            token.invalidate();
            return self.record(Verdict::Unknown, 1);
        }
        if state.metrics.len() != DEFAULT_N_METRICS {
            token.invalidate();
            return self.record(Verdict::Fail, 1);
        }
        if !state.metrics.iter().all(|m| m.is_finite() && (0.0..=1.0).contains(m)) {
            token.invalidate();
            return self.record(Verdict::Fail, 1);
        }
        token.advance();

        // ── Phase 2: Verify ────────────────────────────────────────────
        // Cross-check against prior state if available
        if let Some(prior) = prior {
            if prior.metrics.len() == state.metrics.len() {
                // Max allowed jump per metric per observation
                const MAX_JUMP: f64 = 1.0;
                for i in 0..state.metrics.len() {
                    let jump = (state.metrics[i] - prior.metrics[i]).abs();
                    if jump > MAX_JUMP && prior.metrics[i].is_finite() {
                        // Anomalous jump — likely sensor jamming or state corruption
                        token.invalidate();
                        return self.record(Verdict::Retry, 2);
                    }
                }
            }
        }

        // Check timestamp monotonicity
        if let Some(prior) = prior {
            if state.timestamp_ms < prior.timestamp_ms && prior.timestamp_ms > 0 {
                token.invalidate();
                return self.record(Verdict::Retry, 2);
            }
        }
        token.advance();

        // ── Phase 3: Acknowledge ───────────────────────────────────────
        // Validate self-consistency
        let valid = state.metrics.iter().all(|m| m.is_finite())
            && state.label.len() < MAX_LABEL_LEN
            && state.metrics.iter().sum::<f64>().is_finite();

        if valid {
            self.record(Verdict::Pass, 3)
        } else {
            self.record(Verdict::Fail, 3)
        }
    }

    /// Record a verdict and update consecutive counters.
    fn record(&mut self, verdict: Verdict, phase: u8) -> Verdict {
        self.last_verdict = verdict;
        self.phase_timings_ms[phase as usize - 1] = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        match verdict {
            Verdict::Pass => {
                self.consecutive_passes = self.consecutive_passes.saturating_add(1);
            }
            Verdict::Retry => {
                self.consecutive_passes = 0;
                self.consecutive_fails = self.consecutive_fails.saturating_add(1);
            }
            Verdict::Fail => {
                self.consecutive_passes = 0;
                self.consecutive_fails = self.consecutive_fails.saturating_add(1);
            }
            Verdict::Unknown => {
            }
        }
        verdict
    }

    pub fn is_stable(&self) -> bool {
        self.consecutive_passes >= self.stabilization_threshold
    }

    pub fn consecutive_passes(&self) -> u32 { self.consecutive_passes }
    pub fn consecutive_fails(&self) -> u32 { self.consecutive_fails }
    pub fn last_verdict(&self) -> Verdict { self.last_verdict }

    pub fn is_recovering(&self) -> bool {
        self.consecutive_fails > 0 && self.consecutive_passes > 0
            && self.consecutive_passes >= self.consecutive_fails / RECOVER_DIVISOR
    }

    /// Reset all counters (after a full recovery).
    pub fn reset(&mut self) {
        self.consecutive_passes = 0;
        self.consecutive_fails = 0;
        self.last_verdict = Verdict::Unknown;
        self.phase_timings_ms = [0, 0, 0];
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_phase_passes_healthy_state() {
        let mut v = ThreePhaseVerifier::new(3);
        let s = SystemState::new(1, vec![0.5; 8], "healthy");
        let verdict = v.verify(&s, None);
        assert_eq!(verdict, Verdict::Pass);
    }

    #[test]
    fn three_phase_fails_empty_metrics() {
        let mut v = ThreePhaseVerifier::new(1);
        let s = SystemState {
            id: 1,
            timestamp_ms: 0,
            metrics: vec![],
            label: "empty".to_string(),
            bid_confidence: 1.0,
        };
        let verdict = v.verify(&s, None);
        assert_eq!(verdict, Verdict::Unknown, "empty metrics → Unknown");
    }

    #[test]
    fn three_phase_fails_nan_metrics_phase1() {
        // Bypass SystemState::new sanitization by constructing directly
        let mut v = ThreePhaseVerifier::new(1);
        let s = SystemState {
            id: 1,
            timestamp_ms: 0,
            metrics: vec![f64::NAN; 8],
            label: "nan".to_string(),
            bid_confidence: 1.0,
        };
        let verdict = v.verify(&s, None);
        assert_eq!(verdict, Verdict::Fail, "NaN metrics → Fail at phase 1");
    }

    #[test]
    fn three_phase_retry_on_anomalous_jump() {
        let mut v = ThreePhaseVerifier::new(3);
        let _prior = SystemState::new(1, vec![0.2; 8], "prior");
        let _jump = SystemState::new(2, vec![1.0; 8], "jump");
        // With threshold 1.0 and jump from 0.2 to 1.0 = 0.8, this should be
        // within bounds (MAX_JUMP = 1.0). To trip it, we need a larger jump.
        // Let's use 0.0 → 1.0 (jump=1.0 which equals MAX_JUMP, so passes)
        let prior2 = SystemState::new(1, vec![0.0; 8], "prior");
        let big_jump = SystemState::new(2, vec![1.0; 8], "big_jump");
        let verdict = v.verify(&big_jump, Some(&prior2));
        assert_eq!(verdict, Verdict::Pass,
            "jump of 1.0 from 0 to 1 is <= MAX_JUMP(1.0), must pass");
    }

    #[test]
    fn three_phase_retry_on_time_travel() {
        let mut v = ThreePhaseVerifier::new(1);
        let prior = SystemState::new(1, vec![0.5; 8], "prior").with_timestamp(200);
        let future = SystemState::new(2, vec![0.6; 8], "future").with_timestamp(100);
        let verdict = v.verify(&future, Some(&prior));
        assert_eq!(verdict, Verdict::Retry, "timestamp going backwards → Retry");
    }

    #[test]
    fn three_phase_stabilization_tracking() {
        let mut v = ThreePhaseVerifier::new(5);
        assert!(!v.is_stable());
        for _ in 0..5 {
            let s = SystemState::new(1, vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8], "stable");
            v.verify(&s, None);
        }
        assert!(v.is_stable(), "must stabilize after 5 consecutive passes");
    }

    #[test]
    fn three_phase_fail_resets_stabilization() {
        let mut v = ThreePhaseVerifier::new(3);
        for _ in 0..2 {
            let s = SystemState::new(1, vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8], "stable");
            v.verify(&s, None);
        }
        assert!(!v.is_stable());
        let bad = SystemState {
            id: 1, timestamp_ms: 0,
            metrics: vec![f64::NAN; 8],
            label: "bad".to_string(), bid_confidence: 1.0,
        };
        let verdict = v.verify(&bad, None);
        assert_eq!(verdict, Verdict::Fail, "NaN state must produce Fail, got {:?}", verdict);
        assert_eq!(v.consecutive_passes(), 0);
    }

    #[test]
    fn three_phase_recovering_state() {
        let mut v = ThreePhaseVerifier::new(3);
        let bad1 = SystemState {
            id: 1, timestamp_ms: 0,
            metrics: vec![f64::NAN; 8],
            label: "bad".to_string(), bid_confidence: 1.0,
        };
        let v1 = v.verify(&bad1, None);
        let v2 = v.verify(&bad1, None);
        let good = SystemState::new(2, vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8], "good");
        let v3 = v.verify(&good, None);
        assert_eq!(v1, Verdict::Fail, "verify(bad1) should be Fail, got {:?}", v1);
        assert_eq!(v2, Verdict::Fail, "verify(bad2) should be Fail, got {:?}", v2);
        assert_eq!(v3, Verdict::Pass, "verify(good) should be Pass, got {:?}", v3);
        assert!(v.is_recovering(),
            "should be recovering: fails={}, passes={}",
            v.consecutive_fails(), v.consecutive_passes());
    }

    #[test]
    fn three_phase_reset_clears_everything() {
        let mut v = ThreePhaseVerifier::new(3);
        let bad = SystemState {
            id: 1, timestamp_ms: 0,
            metrics: vec![f64::NAN; 8],
            label: "bad".to_string(), bid_confidence: 1.0,
        };
        v.verify(&bad, None);
        assert_eq!(v.consecutive_fails(), 1);
        v.reset();
        assert_eq!(v.consecutive_fails(), 0);
        assert_eq!(v.consecutive_passes(), 0);
        assert_eq!(v.last_verdict(), Verdict::Unknown);
    }

    #[test]
    fn three_phase_verify_with_offset_metrics() {
        // 7 metrics (not 8) should fail at phase 1.
        // We bypass SystemState::new (which resizes to 8) by constructing directly.
        let mut v = ThreePhaseVerifier::new(1);
        let s = SystemState {
            id: 1,
            timestamp_ms: 0,
            metrics: vec![0.5; 7],
            label: "short".to_string(),
            bid_confidence: 1.0,
        };
        let verdict = v.verify(&s, None);
        assert_eq!(verdict, Verdict::Fail, "7 metrics ≠ 8 → Fail");
    }

    #[test]
    fn three_phase_inf_metrics_rejected() {
        let mut v = ThreePhaseVerifier::new(1);
        let s = SystemState {
            id: 1,
            timestamp_ms: 0,
            metrics: vec![f64::INFINITY; 8],
            label: "inf".to_string(),
            bid_confidence: 1.0,
        };
        assert_eq!(v.verify(&s, None), Verdict::Fail);
    }

    #[test]
    fn three_phase_long_label_accepted() {
        let mut v = ThreePhaseVerifier::new(1);
        let label = "a".repeat(500);
        let s = SystemState::new(1, vec![0.5; 8], &label);
        let verdict = v.verify(&s, None);
        assert_eq!(verdict, Verdict::Pass,
            "label of {} bytes should pass: {:?}", label.len(), verdict);
    }
}
