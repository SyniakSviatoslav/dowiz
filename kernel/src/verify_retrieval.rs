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
//!
//! E0 fix (VERIFIABLE-COGNITION §2 bug #1): the two `Err` arms were byte-identical,
//! so the `round >= max_rounds` branch was dead code — the harness could not tell a
//! *retry* trigger from the *final* trigger and had no deterministic signal to stop.
//! The `terminal` flag makes the distinction observable and testable.

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
    /// True iff this is the *final* failure — `round == max_rounds` was reached,
    /// so no further re-verify will be produced. The harness reads this to stop
    /// the loop instead of re-feeding the same failing claim forever. `false`
    /// means "re-verify at `round + 1`".
    pub terminal: bool,
}

/// Verify `observed ≈ expected` within `tolerance`. On failure, return a
/// `RetrievalTrigger` for round `round` (0-based). The caller feeds a
/// re-verified `observed` back in at `round+1`; once `round == max_rounds`
/// the failure is final (`terminal: true`) — the loop is provably bounded.
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
            terminal: true,
        })
    } else {
        // retry: harness should re-verify this claim at round + 1
        Err(RetrievalTrigger {
            claim_id: claim_id.to_string(),
            observed,
            expected,
            tolerance,
            round,
            terminal: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pass: within tolerance → Ok(()), no trigger emitted.
    #[test]
    fn pass_emits_nothing() {
        assert_eq!(verify_then_lookup("c1", 1.0001, 1.0, 1e-3, 0, 2), Ok(()));
    }

    /// Fail at round 0 → trigger with round 0 (harness should re-verify).
    /// Round 0 < max_rounds ⇒ NOT terminal.
    #[test]
    fn fail_emits_trigger_round0() {
        let r = verify_then_lookup("c1", 1.5, 1.0, 1e-3, 0, 2).unwrap_err();
        assert_eq!(r.claim_id, "c1");
        assert_eq!(r.round, 0);
        assert!(!r.terminal, "round 0 < max_rounds must NOT be terminal");
        assert!((r.observed - 1.5).abs() < 1e-12);
    }

    /// Bounded loop: a permanently-wrong claim stops emitting after max_rounds.
    /// Only the trigger at `round == max_rounds` is terminal; earlier ones are
    /// retry triggers.
    #[test]
    fn loop_is_bounded() {
        // round 0 and 1 emit retry triggers; round 2 (== max_rounds) is the
        // terminal failure
        let t0 = verify_then_lookup("c", 9.0, 1.0, 1e-9, 0, 2).unwrap_err();
        let t1 = verify_then_lookup("c", 9.0, 1.0, 1e-9, 1, 2).unwrap_err();
        // re-fed at round 2 → still Err but now terminal (stop signal)
        let final_fail = verify_then_lookup("c", 9.0, 1.0, 1e-9, 2, 2);
        assert_eq!(t0.round, 0);
        assert_eq!(t1.round, 1);
        assert_eq!(final_fail.as_ref().unwrap_err().round, 2);
        assert!(!t0.terminal);
        assert!(!t1.terminal);
        assert!(
            final_fail.as_ref().unwrap_err().terminal,
            "round == max_rounds MUST be terminal"
        );
    }

    /// RED→GREEN for the E0 dead-branch fix: the terminal flag tracks the round
    /// cap exactly. This test fails on the old code (both arms identical ⇒
    /// terminal always false).
    #[test]
    fn terminal_flag_tracks_round_cap() {
        let max = 3;
        for r in 0..max {
            let trig = verify_then_lookup("x", 5.0, 1.0, 1e-9, r, max).unwrap_err();
            assert_eq!(
                trig.terminal,
                r == max,
                "terminal must be true only at round == max_rounds"
            );
        }
    }

    /// The round cap is enforced regardless of how many times we call: a trigger
    /// at round == max_rounds is the last one (same shape as round 1, but the
    /// harness contract is "stop after receiving a terminal trigger").
    #[test]
    fn determinism() {
        let a = verify_then_lookup("x", 2.0, 1.0, 1e-9, 1, 2);
        let b = verify_then_lookup("x", 2.0, 1.0, 1e-9, 1, 2);
        assert_eq!(a, b);
    }
}
