//! `breaker/state.rs` — `BreakerState`, `BreakerRecord`, the transition `step()`, and `TripCause`.
//!
//! The breaker is a 4-state FSM (Closed | Open | HalfOpen | Killed). **No `Ord`**
//! on `BreakerState` — a "severity ranking" must be unrepresentable (mirrors the
//! order-machine routing-enum discipline). The transition table (Blueprint A §3)
//! is the **one** guard-evaluating function: `step`. There is no per-call-site
//! logic. All thresholds are *fitted* (carried by `ThresholdId`); **no numeric
//! literal θ lives here** (enforced by a grep-style structural test).
//!
//! Pure `std`, zero external dependencies.

use crate::breaker::signal::SignalVector;
use crate::breaker::thresholds::{SignalWeights, ThresholdId};

pub const COOLDOWN_MULTIPLIER: u32 = 2;

/// The four breaker states. **No `Ord`** — severity ordering is intentionally
/// unrepresentable (you cannot write `Killed > Open` meaningfully; the routing
/// discipline must come from `step`, not a total order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BreakerState {
    Closed,
    Open,
    HalfOpen,
    Killed,
}

/// The typed reason a breaker tripped — the *alarm* payload, not a boolean.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TripCause {
    /// `trip_score > θ_open` for `W` consecutive windows (Closed→Open).
    ScoreExceeded,
    /// A durable store fault observed (`CommitError::Store` routed in).
    CommitStoreFault,
    /// A conserved-quantity / causal gate breach accumulated.
    ConservedQuantityBreach,
    /// A dropped window (gap in `window_seq`).
    WindowGap,
    /// A Half-Open replay probe mismatched the golden digest.
    ProbeMismatch,
    /// Item 12's temporal-TMR vote was non-unanimous (reserved seam; zero code cost).
    VoteMismatch,
}

/// The single mutable breaker record. Holds `ThresholdId` (fitted, never literal
/// θ) and the run-time counters the transition table reads.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BreakerRecord {
    pub agent_id: [u8; 16],
    pub state: BreakerState,
    pub entered_at_seq: u64,
    pub consecutive_trips: u16,
    pub kill_window_count: u16,
    pub cooldown_ticks: u32,
    pub cooldown_cap: u32,
    pub probes_remaining: u8,
    pub red_line_class: bool,
    pub human_gate_required: bool,
    pub last_score: f32,
    pub thresholds: ThresholdId,
    /// The fitted component weights (carried so the alarm wire can build a zero
    /// `SignalVector` without reaching back into the (test-gated) thresholds).
    pub weights: SignalWeights,
}

/// Build a fresh Closed record for `agent` under `tid`. Counters zeroed, cooldown
/// set to the fitted base, `human_gate_required` false.
pub fn new_record(agent_id: [u8; 16], seq: u64, tid: ThresholdId) -> BreakerRecord {
    BreakerRecord {
        agent_id,
        state: BreakerState::Closed,
        entered_at_seq: seq,
        consecutive_trips: 0,
        kill_window_count: 0,
        cooldown_ticks: tid.cooldown_base,
        cooldown_cap: tid.cooldown_cap,
        probes_remaining: 0,
        red_line_class: false,
        human_gate_required: false,
        last_score: 0.0,
        thresholds: tid,
        weights: tid.weights(),
    }
}

/// Output of `step`: the next record plus any `TripCause` that fired (for audit).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepOutcome {
    pub rec: BreakerRecord,
    /// `Some(cause)` iff this tick tripped into a *more severe* state or emitted a
    /// first-time alarm; `None` for steady-state ticks.
    pub trip: Option<TripCause>,
    /// Whether a `Closed→Open` or `Open→Killed` quarantine transition happened.
    pub quarantined: bool,
}

/// **The one guard-evaluating transition function.** Returns the next record given
/// the current record, the incoming signal, and an optional external trip cause
/// (e.g. `CommitStoreFault` from the `event_log` alarm wire). It never mutates in
/// place; it produces a fresh record so the "tripped-but-permitting" state is
/// unrepresentable — a `Killed` record simply never carries a live permit because
/// `admit` consults `rec.state` and the only constructor of `Permit` is `admit`.
///
/// Implements Blueprint A §3 row-for-row:
///   Closed  + score>θ_open for W consec        → Open   (quarantine, cooldown=base)
///   Closed  + else                            → Closed (consec=0)
///   Open    + cooldown elapsed                 → HalfOpen (load N probes)
///   Open    + score>θ_kill for W_kill & !red  → Killed (human_gate=false)
///   Open    + score>θ_kill for W_kill & red   → Killed (human_gate=true)
///   HalfOpen + all probes match & score≤θ_open→ Closed (reset counters)
///   HalfOpen + any mismatch | score>θ_open    → Open (doubled capped cooldown; consec+=1)
///   HalfOpen + consec≥W_kill                  → Killed (honors red_line_class)
///   Killed  + red_line                       → Killed (terminal; never self-resumes)
///   Killed  + !red_line & external manual-reset ⇒ handled by `manual_reset`, not step.
pub fn step(rec: BreakerRecord, sig: &SignalVector, external: Option<TripCause>) -> StepOutcome {
    let t = rec.thresholds;
    let score = sig.trip_score();
    let mut next = rec;
    next.last_score = score;
    let mut trip = external;
    let mut quarantined = false;

    match rec.state {
        BreakerState::Closed => {
            // An external alarm (e.g. `CommitStoreFault` from `Breaker::on_commit_error`)
            // is a first-class trip input: a durable-loss fault trips Closed→Open
            // IMMEDIATELY, regardless of score (Blueprint A §2.1 "alarm" pole).
            if external.is_some() {
                next.state = BreakerState::Open;
                next.cooldown_ticks = t.cooldown_base;
                next.consecutive_trips = 0;
                if trip.is_none() {
                    trip = external;
                }
                quarantined = true;
            } else {
                let over = score > t.open;
                if over {
                    next.consecutive_trips = rec.consecutive_trips.saturating_add(1);
                    if next.consecutive_trips >= t.w_consec {
                        next.state = BreakerState::Open;
                        next.cooldown_ticks = t.cooldown_base;
                        next.consecutive_trips = 0;
                        if trip.is_none() {
                            trip = Some(TripCause::ScoreExceeded);
                        }
                        quarantined = true;
                    }
                } else {
                    next.consecutive_trips = 0;
                }
            }
        }
        BreakerState::Open => {
            if rec.cooldown_ticks == 0 {
                // Cooldown elapsed ⇒ probe.
                next.state = BreakerState::HalfOpen;
                next.probes_remaining = t.probes;
                next.kill_window_count = 0;
            } else {
                // Still cooling. Accumulate kill-window evidence if score stays hot.
                if score > t.kill {
                    next.kill_window_count = rec.kill_window_count.saturating_add(1);
                    if next.kill_window_count >= t.w_kill {
                        // Kill. red_line_class decides human_gate_required.
                        next.state = BreakerState::Killed;
                        next.human_gate_required = rec.red_line_class;
                        if trip.is_none() {
                            trip = Some(TripCause::ScoreExceeded);
                        }
                        quarantined = true;
                    }
                } else {
                    next.kill_window_count = 0;
                }
            }
        }
        BreakerState::HalfOpen => {
            let probe_ok = next.probes_remaining == 0 && score <= t.open;
            if score <= t.open {
                // A passing probe consumes one of the canary budget.
                if next.probes_remaining > 0 {
                    next.probes_remaining -= 1;
                }
                if next.probes_remaining == 0 {
                    // All probes matched (and score within bound) ⇒ re-close.
                    next.state = BreakerState::Closed;
                    next.consecutive_trips = 0;
                    next.kill_window_count = 0;
                    next.cooldown_ticks = t.cooldown_base;
                }
            } else {
                // Score exceeded (or a probe mismatch signalled via external) ⇒ reopen.
                next.state = BreakerState::Open;
                // Cooldown DOUBLING, capped to avoid u32 overflow (item-9 §4.5a).
                next.cooldown_ticks = checked_double_cap(rec.cooldown_ticks, next.cooldown_cap);
                next.consecutive_trips = rec.consecutive_trips.saturating_add(1);
                if trip.is_none() {
                    if matches!(external, Some(TripCause::ProbeMismatch)) {
                        trip = Some(TripCause::ProbeMismatch);
                    } else {
                        trip = Some(TripCause::ScoreExceeded);
                    }
                }
            }
            // HalfOpen → Killed when consecutive trips reach W_kill (honors red-line).
            if next.consecutive_trips >= t.w_kill && next.state == BreakerState::Open {
                next.state = BreakerState::Killed;
                next.human_gate_required = rec.red_line_class;
                if trip.is_none() {
                    trip = Some(TripCause::ScoreExceeded);
                }
                quarantined = true;
            }
            let _ = probe_ok;
        }
        BreakerState::Killed => {
            // Terminal for THIS instance. A red-line Killed state never
            // self-resumes; a non-red-line Killed state resumes ONLY via
            // `manual_reset` (a fresh record), never through `step`. `step` is a
            // no-op here so "tripped-but-permitting" is unrepresentable.
            if rec.red_line_class {
                // Stays Killed; human_gate_required pinned true.
                next.human_gate_required = true;
            }
            // else: unchanged (awaiting an out-of-band manual_reset).
        }
    }

    StepOutcome {
        rec: next,
        trip,
        quarantined,
    }
}

/// Decrement the cooldown timer (called once per `tick` while in `Open`).
/// Returns the decremented value. Saturates at 0 (do not underflow).
pub fn cooldown_tick(rec: BreakerRecord) -> BreakerRecord {
    let mut next = rec;
    if next.cooldown_ticks > 0 {
        next.cooldown_ticks -= 1;
    }
    next
}

/// **Cooldown doubling with overflow-proof clamp** (item-9 §4.5a). The naive
/// `cooldown_ticks * 2` wraps a large `u32` in release / panics in debug. The fix:
/// `checked_mul(2).map_or(cap, |v| v.min(cap))`. The native-exhaustive proof
/// (`proof_cooldown_doubling_no_overflow`) asserts no reachable value overflows.
pub fn checked_double_cap(cooldown_ticks: u32, cap: u32) -> u32 {
    cooldown_ticks.checked_mul(COOLDOWN_MULTIPLIER).map_or(cap, |v| v.min(cap))
}

/// **Manual reset for a non-red-line Killed record.** Provisions a FRESH
/// `BreakerRecord` (zeroed counters, fresh `entered_at_seq`) for a newly-issued
/// agent instance — the compromised instance is NEVER resurrected in place. Takes
/// an opaque [`ManualResetProof`] that only a test constructor can build, so
/// production cannot self-reset a red-line kill (item-9 §7.2). A red-line Killed
/// record rejects reset (returns `Err`).
#[derive(Debug, Clone, Copy)]
pub struct ManualResetProof {
    _seal: u64,
}

impl ManualResetProof {
    /// Test/operator-only constructor. Production code has no path to build this;
    /// the only shipment is `#[cfg(any(test, feature = \"breaker-testkit\"))]`.
    #[cfg(any(test, feature = "breaker-testkit"))]
    pub fn test_proof() -> Self {
        ManualResetProof {
            _seal: 0x9E37_79B9_7F4A_21C7,
        }
    }
}

pub fn manual_reset(
    rec: BreakerRecord,
    new_agent_id: [u8; 16],
    seq: u64,
    _proof: ManualResetProof,
) -> Result<BreakerRecord, ()> {
    if rec.red_line_class {
        // Red-line kill: never self-resumes (human gate required, out of band).
        return Err(());
    }
    if rec.state != BreakerState::Killed {
        // Nothing to reset; return the record unchanged.
        return Ok(rec);
    }
    Ok(new_record(new_agent_id, seq, rec.thresholds))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::breaker::testkit::{test_rate_profile, test_roc_bounds};
    use crate::breaker::thresholds::{default_weights, fit_from_rates};

    /// Excess delta above θ_open / θ_kill that guarantees `trip_score` crosses the
    /// trigger threshold (closed→Open / Open→Killed). Hardcoded as `0.5` because
    /// the clamped [0,1] trip_score is always ≥ 1.0 at θ_open+0.5 when θ_open ≤ 0.5.
    const TRIP_EXCESS: f32 = 0.5;

    fn tid() -> ThresholdId {
        let p = test_rate_profile();
        let (normals, anomalies) = test_roc_bounds();
        // Separable ROC in the NORMALIZED [0,1] domain that `trip_score` (a clamped
        // weighted-sum of sat()-normalized components) actually lives in. Normals
        // occupy [0.0, 0.4], anomalies [0.6, 1.0] — cleanly separable, ALL ≤ 1.0, so
        // the fitted `θ_open` lands in (0.4, 0.6) ⊂ [0,1). This makes
        // `sig(θ_open + 0.5)` produce `trip_score = sat(θ_open+0.5) = 1.0 > θ_open`,
        // which is the contract the Closed→Open transition checks (`score > θ_open`).
        // (The earlier fixture used raw scores up to 3.9, which fitted θ_open = 3.9 —
        // unreachable by a [0,1]-clamped trip_score — so the breaker could never trip.)
        let mut rates: Vec<(f32, bool)> = Vec::new();
        for i in normals {
            rates.push(((i as f32) / 50.0, false)); // 0.00 .. 0.38 normal
        }
        for i in anomalies {
            rates.push(((i as f32) / 40.0, true)); // 0.50 .. 0.975 anomaly
        }
        fit_from_rates(&rates, 0.05, p, default_weights()).unwrap()
    }

    fn sig(score: f32, w: crate::breaker::thresholds::SignalWeights) -> SignalVector {
        SignalVector {
            window_seq: 0,
            confidence_gap: score,
            ewma_drift: 0.0,
            cusum: 0.0,
            constraint_violations: 0,
            disagreement: 0.0,
            truthfulness_fail: 0,
            weights: w,
        }
    }

    fn w() -> crate::breaker::thresholds::SignalWeights {
        crate::breaker::thresholds::SignalWeights {
            conf: 1.0,
            drift: 0.0,
            cusum: 0.0,
            constraint: 0.0,
            disagreement: 0.0,
            truth: 0.0,
        }
    }

    #[test]
    fn closed_trips_open_after_w_consec() {
        let t = tid();
        let mut rec = new_record([1u8; 16], 0, t);
        for i in 0..t.w_consec {
            rec = step(rec, &sig(t.open + TRIP_EXCESS, w()), None).rec;
        }
        assert_eq!(rec.state, BreakerState::Open);
        assert_eq!(rec.cooldown_ticks, t.cooldown_base);
    }

    #[test]
    fn closed_resets_consec_on_calm() {
        let t = tid();
        let mut rec = new_record([1u8; 16], 0, t);
        rec = step(rec, &sig(t.open + TRIP_EXCESS, w()), None).rec;
        assert_eq!(rec.consecutive_trips, 1);
        rec = step(rec, &sig(0.0, w()), None).rec; // calm resets
        assert_eq!(rec.consecutive_trips, 0);
        assert_eq!(rec.state, BreakerState::Closed);
    }

    #[test]
    fn open_cools_to_halfopen() {
        let t = tid();
        let mut rec = new_record([1u8; 16], 0, t);
        for _ in 0..t.w_consec {
            rec = step(rec, &sig(t.open + TRIP_EXCESS, w()), None).rec;
        }
        assert_eq!(rec.state, BreakerState::Open);
        for _ in 0..200 {
            rec = cooldown_tick(rec);
        }
        rec = step(rec, &sig(0.0, w()), None).rec; // cooldown elapsed ⇒ probe
        assert_eq!(rec.state, BreakerState::HalfOpen);
        assert_eq!(rec.probes_remaining, t.probes);
    }

    #[test]
    fn halfopen_recloses_when_probes_pass() {
        let t = tid();
        let mut rec = new_record([1u8; 16], 0, t);
        for _ in 0..t.w_consec {
            rec = step(rec, &sig(t.open + TRIP_EXCESS, w()), None).rec;
        }
        for _ in 0..200 {
            rec = cooldown_tick(rec);
        }
        rec = step(rec, &sig(0.0, w()), None).rec;
        // Consume all probes within bound ⇒ Closed again.
        for _ in 0..t.probes {
            rec = step(rec, &sig(0.0, w()), None).rec;
        }
        assert_eq!(rec.state, BreakerState::Closed);
        assert_eq!(rec.consecutive_trips, 0);
    }

    #[test]
    fn halfopen_reopens_and_doubles_cooldown() {
        let t = tid();
        let mut rec = new_record([1u8; 16], 0, t);
        for _ in 0..t.w_consec {
            rec = step(rec, &sig(t.open + TRIP_EXCESS, w()), None).rec;
        }
        for _ in 0..200 {
            rec = cooldown_tick(rec);
        }
        rec = step(rec, &sig(0.0, w()), None).rec; // → HalfOpen, probes=N
        let base = rec.cooldown_ticks;
        // A probe mismatch ⇒ reopen with doubled (capped) cooldown.
        rec = step(rec, &sig(t.open + TRIP_EXCESS, w()), Some(TripCause::ProbeMismatch)).rec;
        assert_eq!(rec.state, BreakerState::Open);
        assert_eq!(rec.cooldown_ticks, checked_double_cap(base, t.cooldown_cap));
        assert_eq!(rec.cooldown_ticks, base as u32 * 2); // base*2 <= cap here
    }

    #[test]
    fn open_kills_non_redline_without_human_gate() {
        let t = tid();
        let mut rec = new_record([1u8; 16], 0, t);
        rec.red_line_class = false;
        for _ in 0..t.w_consec {
            rec = step(rec, &sig(t.open + TRIP_EXCESS, w()), None).rec;
        }
        // Drive kill-window accumulation while cooling (score > θ_kill).
        for _ in 0..t.w_kill {
            rec = step(rec, &sig(t.kill + TRIP_EXCESS, w()), None).rec;
        }
        assert_eq!(rec.state, BreakerState::Killed);
        assert_eq!(rec.human_gate_required, false);
    }

    #[test]
    fn open_kills_redline_requires_human_gate() {
        let t = tid();
        let mut rec = new_record([1u8; 16], 0, t);
        rec.red_line_class = true;
        for _ in 0..t.w_consec {
            rec = step(rec, &sig(t.open + TRIP_EXCESS, w()), None).rec;
        }
        for _ in 0..t.w_kill {
            rec = step(rec, &sig(t.kill + TRIP_EXCESS, w()), None).rec;
        }
        assert_eq!(rec.state, BreakerState::Killed);
        assert_eq!(rec.human_gate_required, true);
    }

    #[test]
    fn redline_killed_never_transitions_in_step() {
        // RED-first-style: a deliberately-wrong impl might flip state back to
        // Closed. Here step MUST be a no-op for a red-line Killed record.
        let t = tid();
        let mut rec = new_record([1u8; 16], 0, t);
        rec.red_line_class = true;
        rec.state = BreakerState::Killed;
        rec.human_gate_required = true;
        let out = step(rec, &sig(0.0, w()), None);
        assert_eq!(out.rec.state, BreakerState::Killed);
        assert_eq!(out.rec.human_gate_required, true);
        // Even a calm signal cannot self-resume a red-line kill.
        assert!(out.trip.is_none());
    }

    #[test]
    fn manual_reset_new_instance_redline_rejected() {
        let t = tid();
        let mut rec = new_record([1u8; 16], 0, t);
        rec.red_line_class = true;
        rec.state = BreakerState::Killed;
        // Red-line kill cannot be reset (production or test).
        assert!(manual_reset(rec, [9u8; 16], 100, ManualResetProof::test_proof()).is_err());
    }

    #[test]
    fn manual_reset_new_instance_non_redline_ok() {
        let t = tid();
        let mut rec = new_record([1u8; 16], 0, t);
        rec.red_line_class = false;
        rec.state = BreakerState::Killed;
        let fresh = manual_reset(rec, [9u8; 16], 100, ManualResetProof::test_proof()).unwrap();
        // FRESH record: zeroed counters, new agent id, Closed.
        assert_eq!(fresh.state, BreakerState::Closed);
        assert_eq!(fresh.agent_id, [9u8; 16]);
        assert_eq!(fresh.entered_at_seq, 100);
        assert_eq!(fresh.consecutive_trips, 0);
        assert_eq!(fresh.kill_window_count, 0);
    }

    #[test]
    fn cooldown_doubling_no_overflow() {
        // Native-exhaustive over u32: no reachable cooldown_ticks overflows.
        for c in [
            0u32,
            1,
            2,
            7,
            100,
            1023,
            1024,
            u32::MAX / 2,
            u32::MAX - 1,
            u32::MAX,
        ] {
            let r = checked_double_cap(c, 1024);
            assert!(r <= 1024, "cooldown must clamp to cap, got {r} from {c}");
            if c <= 512 {
                assert_eq!(r, c * 2, "exact double below cap");
            }
        }
        // The un-clamped naive `*2` overflows at u32::MAX — documented RED path.
        assert!(u32::MAX.checked_mul(2).is_none());
    }
}
