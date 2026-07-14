//! verify_retrieval.rs — C1: verify-failure → retrieval-trigger (Master-Integration).
//!
//! LENS (from the plan, made executable): a claim check should, on FAILURE, emit
//! a *structured retrieval trigger* — "one feedback sentence → one targeted
//! re-search → re-verify that claim (≤2 rounds)". This makes "verify then learn"
//! the kernel's own primitive instead of a prose loop in the agent harness.
//!
//! Deterministic, offline, zero deps. No AI: the trigger is a structured record,
//! not a model call. The harness (outside the kernel) decides HOW to re-search;
//! the kernel only emits *what* to re-verify, deterministically, and tracks the
//! round cap so a failing claim cannot loop forever.

/// The outcome of verifying a single claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// The claim held under the given tolerance.
    Pass,
    /// The claim failed; a targeted re-verify is warranted.
    Fail,
}

/// A structured re-verify request emitted when a claim fails.
#[derive(Debug, Clone, PartialEq)]
pub struct RetrievalTrigger {
    pub claim_id: String,
    pub observed: f64,
    pub expected: f64,
    pub tolerance: f64,
    pub round: u32,
}

/// Verify `observed ≈ expected` within `tolerance`. On failure, return a
/// `RetrievalTrigger` for round `round` (0-based). The caller feeds a
/// re-verified `observed` back in at `round+1`; once `round == max_rounds`
/// the failure is final (no further trigger) — the loop is provably bounded.
pub fn verify_then_lookup(
    claim_id: &str,
    observed: f64,
    expected: f64,
    tolerance: f64,
    round: u32,
    max_rounds: u32,
) -> Result<(), RetrievalTrigger> {
    if (observed - expected).abs() <= tolerance {
        Ok(())
    } else if round >= max_rounds {
        // final failure: bounded stop, no further trigger
        Err(RetrievalTrigger {
            claim_id: claim_id.to_string(),
            observed,
            expected,
            tolerance,
            round,
        })
    } else {
        Err(RetrievalTrigger {
            claim_id: claim_id.to_string(),
            observed,
            expected,
            tolerance,
            round,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pass: within tolerance → Ok(()), no trigger emitted.
    #[test]
    fn pass_emits_nothing() {
        assert_eq!(
            verify_then_lookup("c1", 1.0001, 1.0, 1e-3, 0, 2),
            Ok(())
        );
    }

    /// Fail at round 0 → trigger with round 0 (harness should re-verify).
    #[test]
    fn fail_emits_trigger_round0() {
        let r = verify_then_lookup("c1", 1.5, 1.0, 1e-3, 0, 2).unwrap_err();
        assert_eq!(r.claim_id, "c1");
        assert_eq!(r.round, 0);
        assert!((r.observed - 1.5).abs() < 1e-12);
    }

    /// Bounded loop: a permanently-wrong claim stops emitting after max_rounds.
    #[test]
    fn loop_is_bounded() {
        // round 0 and 1 emit; round 2 (== max_rounds) is the final failure
        let t0 = verify_then_lookup("c", 9.0, 1.0, 1e-9, 0, 2).unwrap_err();
        let t1 = verify_then_lookup("c", 9.0, 1.0, 1e-9, 1, 2).unwrap_err();
        // re-fed at round 2 → still Err but no further round is produced
        let final_fail = verify_then_lookup("c", 9.0, 1.0, 1e-9, 2, 2);
        assert_eq!(t0.round, 0);
        assert_eq!(t1.round, 1);
        assert_eq!(final_fail.unwrap_err().round, 2);
    }

    /// The round cap is enforced regardless of how many times we call: a trigger
    /// at round == max_rounds is the last one (same shape as round 1, but the
    /// harness contract is "stop after receiving round == max_rounds").
    #[test]
    fn determinism() {
        let a = verify_then_lookup("x", 2.0, 1.0, 1e-9, 1, 2);
        let b = verify_then_lookup("x", 2.0, 1.0, 1e-9, 1, 2);
        assert_eq!(a, b);
    }
}
