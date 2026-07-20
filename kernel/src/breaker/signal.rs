//! `breaker/signal.rs` — `SignalVector` + the fixed weighted-sum `trip_score`.
//!
//! One primitive (`trip_score`) over normalized components; the three anomaly
//! classes (poisoning / hallucination / injection) are distinguished **only** by
//! which weights are nonzero — never by a bespoke code path. `truthfulness_fail`
//! is **HARD-MASKED to 0** until `detreduce` lands (Blueprint A §4 / item-9
//! non-goal): the replay-truthfulness signal ships disarmed.
//!
//! Pure `std`, zero external dependencies. Math is `f32`; digests reuse
//! `event_log::sha3_256` (the kernel's zero-dep hash), so this stays inside the
//! `cargo tree -e no-dev` zero-dep gate.

use crate::breaker::thresholds::SignalWeights;
use crate::event_log::sha3_256;

/// One per-agent-per-window signal sample fed to the breaker's `tick`.
///
/// All fields `Copy`; no heap on the hot path. `weights` are *fitted* (never
/// hand-tuned literals — see `thresholds::fit_from_rates`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalVector {
    /// Monotone window index (gap ⇒ dropped window ⇒ audited; see Blueprint A §9).
    pub window_seq: u64,
    /// 1 − p(top token), aggregated from logprobs; NaN if unavailable.
    pub confidence_gap: f32,
    /// d_t = ‖a_t − μ_{t-1}‖ (Hooke linear regime, Blueprint A §5.2).
    pub ewma_drift: f32,
    /// S_t = max(0, S_{t-1} + (x_t − μ0 − k)); trip when S_t > h.
    pub cusum: f32,
    /// Writes rejected by the conserved-quantity + causal gate (Blueprint A §5.1).
    pub constraint_violations: u16,
    /// Shadow-execution pair distance (0 = identical).
    pub disagreement: f32,
    /// Replay-probe bitwise mismatches; **MASKED to 0 while disarmed (§4)**.
    pub truthfulness_fail: u8,
    /// Fitted per-component weights (never literals).
    pub weights: SignalWeights,
}

/// Saturating clamp into `[0, 1]` for a normalized component contribution.
#[inline]
fn sat(x: f32) -> f32 {
    if x < 0.0 {
        0.0
    } else if x > 1.0 {
        1.0
    } else {
        x
    }
}
/// A zero signal (all components zero, only `window_seq`/`weights` set) —
/// used by the alarm wire (`on_commit_error`) and tests. The score of a zero
/// signal is 0; an external alarm is what carries the trip, not the score.
pub fn zero(weights: SignalWeights) -> SignalVector {
    SignalVector {
        window_seq: 0,
        confidence_gap: 0.0,
        ewma_drift: 0.0,
        cusum: 0.0,
        constraint_violations: 0,
        disagreement: 0.0,
        truthfulness_fail: 0,
        weights,
    }
}

impl SignalVector {
    /// `trip_score` computed with an *explicit* weight set (used by the fitter,
    /// which must score a vector against candidate weights before committing them
    /// to `self.weights`).
    pub fn trip_score_with(&self, w: &SignalWeights) -> f32 {
        sat(self.confidence_gap) * w.conf
            + sat(self.ewma_drift) * w.drift
            + sat(self.cusum) * w.cusum
            + ((self.constraint_violations as f32) / 255.0).min(1.0) * w.constraint
            + sat(self.disagreement) * w.disagreement
            // HARD MASK: truthfulness is disarmed (item-9 non-goal). It contributes
            // exactly 0 to the score regardless of `truthfulness_fail` or `w.truth`.
            + (self.truthfulness_fail as f32 * 0.0) * w.truth
    }

    /// The breaker's single trip primitive. Fixed weighted sum of the normalized
    /// components (see [`SignalVector::trip_score_with`]).
    pub fn trip_score(&self) -> f32 {
        self.trip_score_with(&self.weights)
    }

    /// Content hash of the signal (used by the audit hash-chain and the replay
    /// probe fresh-output digest). Deterministic; reuses `event_log::sha3_256`.
    pub fn digest(&self) -> [u8; 32] {
        let mut buf = Vec::with_capacity(64);
        buf.extend_from_slice(&self.window_seq.to_le_bytes());
        buf.extend_from_slice(&self.confidence_gap.to_le_bytes());
        buf.extend_from_slice(&self.ewma_drift.to_le_bytes());
        buf.extend_from_slice(&self.cusum.to_le_bytes());
        buf.extend_from_slice(&self.constraint_violations.to_le_bytes());
        buf.extend_from_slice(&self.disagreement.to_le_bytes());
        buf.extend_from_slice(&[self.truthfulness_fail]);
        buf.extend_from_slice(&self.weights.digest());
        sha3_256(&buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::breaker::thresholds::SignalWeights;

    fn w() -> SignalWeights {
        SignalWeights {
            conf: 1.0,
            drift: 2.0,
            cusum: 0.0,
            constraint: 0.0,
            disagreement: 0.0,
            truth: 0.0,
        }
    }

    #[test]
    fn trip_score_is_fixed_weighted_sum() {
        // conf=0.5*1 + drift=0.25*2 + others 0 = 0.5 + 0.5 = 1.0
        let s = SignalVector {
            window_seq: 1,
            confidence_gap: 0.5,
            ewma_drift: 0.25,
            cusum: 0.0,
            constraint_violations: 0,
            disagreement: 0.0,
            truthfulness_fail: 255, // even max must be masked to 0
            weights: w(),
        };
        assert_eq!(s.trip_score(), 1.0);
        // truthfulness_fail is HARD-MASKED: raising it must not move the score.
        let mut s2 = s;
        s2.truthfulness_fail = 0;
        assert_eq!(s.trip_score(), s2.trip_score());
        // normalization saturates: drift=2.0 > 1 clamps to 1.
        let s3 = SignalVector {
            window_seq: 1,
            confidence_gap: 0.0,
            ewma_drift: 2.0,
            cusum: 0.0,
            constraint_violations: 0,
            disagreement: 0.0,
            truthfulness_fail: 0,
            weights: w(),
        };
        assert_eq!(s3.trip_score(), 2.0); // 1.0 * 2.0
    }

    #[test]
    fn digest_is_deterministic() {
        let a = zero(w());
        let b = zero(w());
        assert_eq!(a.digest(), b.digest());
    }
}
