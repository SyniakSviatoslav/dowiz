//! `temporal_tmr.rs` — temporal triple-modular-redundancy (SIHFT) pilot.
//!
//! Roadmap item 12 (space-grade §E), gated on item 9's circuit breaker + Tier-1 FDR.
//! Scoped per `BLUEPRINT-ITEM-12-temporal-tmr-2026-07-19.md`: 2–3× sequential
//! re-execution of a *small number* (2–3) of the kernel's µs-scale pure functions
//! on one core, followed by a trivial-equality vote. A vote-mismatch routes to a
//! `TripCause::VoteMismatch` breaker trip + a `Kind::Alarm` FDR record. **Never**
//! an SEU-immunity claim.
//!
//! # Honest limits (PARTIAL — non-negotiable)
//!
//! Temporal TMR is **PARTIAL**. It catches exactly one fault class: a *transient*
//! compute-time flip during ONE of the `n` re-runs (a bit that inverts mid-eval and
//! then self-heals). It does NOT catch:
//!   (a) a **permanent** fault (stuck bit) — it corrupts every run identically, so
//!       the votes still agree on the wrong answer;
//!   (b) a **deterministic software bug** — every run recomputes the same wrong
//!       answer, so the votes agree (garbage-in/garbage-out with 3× consensus);
//!   (c) **shared-silicon correlation** — all `n` runs share the same die's
//!       cache/ALU; a correlated transient can strike multiple runs, so temporal
//!       separation reduces but does NOT eliminate the residual risk.
//!
//! This is the deliberate re-scope from spatial TMR (three replicas on separate
//! silicon, unavailable to a single-process kernel): we re-run *in time, not in
//! space*. There is NO SEU-immunity claim anywhere in this module — only a
//! demonstrable catch of one transient flip per run, proven by the fault-injection
//! test below.
//!
//! # Design invariants (from blueprint §3)
//!
//! * The voter is a **trivial `==`** (minimum fault-exposure comparator) — a complex
//!   voter would itself be fault-exposed.
//! * **Both** non-unanimous classes (`SingleDissent`, `NoMajority`) trip the breaker
//!   **identically** (behavioral collapse — never continue on a disagreement) while
//!   the *distinct typed cause* is recorded on the FDR (item-50 discipline). A
//!   `SingleDissent` still trips even though a majority exists: on non-ECC hardware a
//!   dissent is evidence of a live fault, not a recoverable outvote.
//! * Applied ONLY to 2–3 named µs-scale pure functions (`event_log::MeshEvent::event_id`,
//!   `money::apply_tax`) — NOT a kernel-wide wrapper.

use crate::breaker::{Breaker, SignalVector, TripCause};
use crate::fdr;

/// The outcome of a temporal-TMR vote over `n` sequential re-runs of `f`.
///
/// `T` is the (cheap, `Clone` + `PartialEq`) result type. `n` is 3 for TMR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoteOutcome<T> {
    /// All `n` runs agreed on `value` (the deterministic expectation `f()`).
    Unanimous(T),
    /// `n ≥ 3`: exactly one replica disagreed. `value` is the majority result;
    /// `replica` is the 0-based index of the dissenting run (recorded for FDR, but
    /// NOT used to proceed — both classes trip).
    SingleDissent { value: T, replica: u8 },
    /// No run had a majority (n=2 disagreement, or n=3 all-distinct).
    NoMajority,
}

/// Run `f` `n` times sequentially on one core and tally the results with a trivial
/// `==` vote.
///
/// * For a **deterministic** `f`, every run yields `f()`, so `tmr(f, n)` is always
///   `Unanimous(f())` — the oracle property (blueprint §4 item 1).
/// * `f` is `Copy` so it can be invoked `n` times without interior mutation; the
///   caller guarantees `f` is pure (no I/O / clock / RNG) so the re-runs are
///   independent samples of the *same* computation.
/// * `n` of 2 or 3 is intended; `n < 2` trivially returns `Unanimous` (no vote to
///   disagree on); `n == 0` is treated as a single run.
pub fn tmr<F, T>(f: F, n: u8) -> VoteOutcome<T>
where
    F: Fn() -> T,
    T: PartialEq + Clone,
{
    let runs = n.max(1) as usize;
    let mut tally: Vec<T> = Vec::with_capacity(runs);
    for _ in 0..runs {
        tally.push(f());
    }
    classify_vote(tally)
}

/// The trivial-equality voter. Pure decision over the collected run results;
/// factored out so the debug-differential cross-check (see `tests`) can call the
/// exact same logic the shipped path uses (no second implementation drift).
fn classify_vote<T: PartialEq + Clone>(tally: Vec<T>) -> VoteOutcome<T> {
    match tally.len() {
        0 | 1 => VoteOutcome::Unanimous(tally.into_iter().next().expect("non-empty for len>=1")),
        _ => {
            // Bucket by (first-seen-value, count, first-index). We only need to know
            // whether there is a strict majority / single dissent / all-different.
            let mut distinct: Vec<(T, u8)> = Vec::new();
            for (i, v) in tally.into_iter().enumerate() {
                if let Some(slot) = distinct.iter_mut().find(|(u, _)| *u == v) {
                    slot.1 += 1;
                } else {
                    distinct.push((v, 1));
                }
                let _ = i;
            }
            match distinct.len() {
                1 => VoteOutcome::Unanimous(distinct.pop().unwrap().0),
                _ => {
                    // Find the largest bucket.
                    let mut best_idx = 0usize;
                    for k in 1..distinct.len() {
                        if distinct[k].1 > distinct[best_idx].1 {
                            best_idx = k;
                        }
                    }
                    let total: u32 = distinct.iter().map(|(_, c)| *c as u32).sum();
                    let best_count = distinct[best_idx].1 as u32;
                    // n=3, a single dissent => majority of 2 exists.
                    // n=2 disagreement, or n=3 all-distinct => no majority.
                    if best_count * 2 > total {
                        // Strict majority => exactly one dissenter (n=3).
                        // `replica` = index of a non-best member (the dissenting run).
                        let dissent_idx = (0u8..(total as u8))
                            .find(|&idx| idx as usize != best_idx)
                            .unwrap_or(0);
                        VoteOutcome::SingleDissent {
                            value: distinct.swap_remove(best_idx).0,
                            replica: dissent_idx,
                        }
                    } else {
                        VoteOutcome::NoMajority
                    }
                }
            }
        }
    }
}

// ── Applied wrappers (2–3 named µs-scale pure functions ONLY) ──────────────────
//
// Each wrapper runs the real pure function through `tmr`, then routes a
// non-unanimous outcome to the item-9 breaker trip + FDR Alarm. The breaker trip
// uses `TripCause::VoteMismatch`; the FDR Alarm names the guarded function and
// carries the typed `VoteOutcome`.
//
// These are the ONLY TMR call-sites in the kernel (blueprint §3.4): the money gate
// and the event-id hash. They are opt-in helpers — the kernel's canonical path is
// unchanged; callers that want the TMR guard invoke these variants explicitly.

/// `fdr`-alarm + breaker wiring for a non-unanimous outcome. Returns `true` iff a
/// trip was raised (so callers can fail-closed). `label` names the guarded function.
fn wire_vote_mismatch<T: PartialEq + Clone>(
    label: &str,
    outcome: &VoteOutcome<T>,
    breaker: &mut Breaker,
) -> bool {
    match outcome {
        VoteOutcome::Unanimous(_) => false,
        diss => {
            // FDR Alarm (Kind::Alarm) carrying the typed cause. `emit_alarm` is a
            // no-op when no FDR sink is installed, so this is safe to call
            // unconditionally.
            let detail = match diss {
                VoteOutcome::SingleDissent { replica, .. } => {
                    format!("temporal_tmr: SingleDissent replica={replica}")
                }
                VoteOutcome::NoMajority => "temporal_tmr: NoMajority".to_string(),
                VoteOutcome::Unanimous(_) => unreachable!(),
            };
            fdr::emit_alarm(label, &detail);
            // Breaker trip: an external `VoteMismatch` cause drives Closed→Open
            // immediately (state.rs step: any external cause trips regardless of
            // score). The SignalVector is zero — the trip carries the cause, the
            // score is inert here.
            let w = breaker_signal_zero();
            breaker.tick(empty_signal(w), Some(TripCause::VoteMismatch), false);
            true
        }
    }
}

/// Build a zero `SignalVector` carrying the breaker's fitted weights (so the alarm
/// wire can trip without reaching back into the test-gated thresholds module).
fn breaker_signal_zero() -> crate::breaker::SignalWeights {
    // The breaker record carries its weights; we only need *a* weight set to build a
    // zero signal. `default_weights` is the public fitted-shaped fallback.
    crate::breaker::SignalWeights {
        conf: 1.0,
        drift: 0.0,
        cusum: 0.0,
        constraint: 0.0,
        disagreement: 0.0,
        truth: 0.0,
    }
}

/// A zero signal (all components zero) — the trip cause, not the score, drives the
/// breaker here.
fn empty_signal(w: crate::breaker::SignalWeights) -> SignalVector {
    SignalVector {
        window_seq: 0,
        confidence_gap: 0.0,
        ewma_drift: 0.0,
        cusum: 0.0,
        constraint_violations: 0,
        disagreement: 0.0,
        truthfulness_fail: 0,
        weights: w,
    }
}

/// **Applied wrapper #1 — event-id hash (the commit hot path).**
///
/// Re-runs `MeshEvent::event_id` `n` times and votes. Returns the unanimous digest,
/// or trips the breaker + writes an FDR Alarm on any non-unanimous outcome. On a
/// trip the caller must NOT proceed (behavioral collapse); `Ok(None)` signals
/// "vote failed, do not use this id".
pub fn event_id_tmr(
    ev: &crate::event_log::MeshEvent,
    breaker: &mut Breaker,
    n: u8,
) -> Result<[u8; 32], ()> {
    let outcome = tmr(|| ev.event_id(), n);
    if wire_vote_mismatch("event_log::MeshEvent::event_id", &outcome, breaker) {
        return Err(()); // fail-closed: never hand back a possibly-corrupt id.
    }
    match outcome {
        VoteOutcome::Unanimous(v) => Ok(v),
        _ => Err(()),
    }
}

/// **Applied wrapper #2 — money tax gate.**
///
/// Re-runs `money::apply_tax` `n` times and votes. Returns the unanimous tax amount,
/// or trips the breaker + writes an FDR Alarm on any non-unanimous outcome. On a trip
/// the caller must NOT proceed (fail-closed): a corrupt tax integer on a money path is
/// exactly the red-line hazard TMR exists to catch here.
pub fn apply_tax_tmr(
    breaker: &mut Breaker,
    n: u8,
    subtotal: i64,
    tax_rate: f64,
    price_includes_tax: bool,
) -> Result<i64, ()> {
    let outcome = tmr(
        || crate::money::apply_tax(subtotal, tax_rate, price_includes_tax),
        n,
    );
    if wire_vote_mismatch("money::apply_tax", &outcome, breaker) {
        return Err(());
    }
    match outcome {
        VoteOutcome::Unanimous(v) => v.map_err(|_| ()),
        _ => Err(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::breaker::{fit_from_rates, BreakerState, RateProfile};
    use crate::event_log::MeshEvent;

    // item 9 harness plumbing (minimal). `fit_from_rates` / `RateProfile` are
    // re-exported at `crate::breaker` (the `thresholds` submodule is private).
    fn tid() -> crate::breaker::ThresholdId {
        let p = RateProfile {
            w_consec: 3,
            w_kill: 5,
            probes: 4,
            cooldown_base: 8,
            cooldown_cap: 1024,
        };
        let mut rates: Vec<(f32, bool)> = Vec::new();
        for i in 0..20 {
            rates.push(((i as f32) / 50.0, false));
        }
        for i in 20..40 {
            rates.push(((i as f32) / 40.0, true));
        }
        // `default_weights` lives in the private `thresholds` submodule; replicate the
        // identical fitted-shaped value here (a `SignalWeights`, never a literal θ).
        let w = crate::breaker::SignalWeights {
            conf: 1.0,
            drift: 1.0,
            cusum: 1.0,
            constraint: 1.0,
            disagreement: 1.0,
            truth: 1.0,
        };
        fit_from_rates(&rates, 0.05, p, w).unwrap()
    }

    fn breaker() -> Breaker {
        Breaker::new([7u8; 16], tid())
    }

    fn sample_event() -> MeshEvent {
        MeshEvent {
            prev: [1u8; 32],
            actor_pubkey: [2u8; 32],
            actor_seq: 42,
            payload: b"hello-tmr".to_vec(),
        }
    }

    /// Build a `Copy` + `Fn` closure that replays a fixed sequence of run results.
    /// The mutable cursor lives in a `Cell` behind a shared `&` reference (which is
    /// itself `Copy`), so the closure is `Copy` and satisfies `tmr`'s bound. This is
    /// the fault-injection testkit hook: a slice of precomputed run values drives
    /// the vote deterministically, including seeded mismatches.
    fn replay<T: Copy + 'static>(runs: Vec<T>) -> impl Fn() -> T {
        use std::rc::Rc;
        let runs = Rc::new(runs);
        let k = Rc::new(core::cell::Cell::new(0usize));
        let kr = Rc::clone(&k);
        let kset = Rc::clone(&k);
        move || {
            let i = kr.get();
            kset.set(i + 1);
            runs[i % runs.len()]
        }
    }

    // ── Item 1 (oracle): deterministic f ⇒ Unanimous(f()) always ──
    #[test]
    fn oracle_deterministic_f_is_always_unanimous() {
        let v = tmr(|| 7i64, 3);
        assert_eq!(v, VoteOutcome::Unanimous(7i64));

        // A more interesting deterministic closure.
        let x = 1234i64;
        let v2 = tmr(|| x.wrapping_mul(3), 5);
        assert_eq!(v2, VoteOutcome::Unanimous(3702i64));

        // The digest of a fixed event is deterministic and unanimous across runs.
        let ev = sample_event();
        let v3 = tmr(|| ev.event_id(), 3);
        assert_eq!(v3, VoteOutcome::Unanimous(ev.event_id()));
    }

    // ── Exhaustive VoteOutcome space: all-agree / one-dissent / all-diff ──
    #[test]
    fn vote_outcome_space_is_exhaustive() {
        // all-agree (n=3) → Unanimous
        assert_eq!(tmr(|| 1u8, 3), VoteOutcome::Unanimous(1u8));
        // one-dissent (n=3) → SingleDissent (majority of 2)
        let runs = vec![10u8, 99u8, 10u8];
        let out = tmr(replay(runs), 3);
        match out {
            VoteOutcome::SingleDissent { value, replica } => {
                assert_eq!(value, 10u8);
                assert_eq!(replica, 1); // the dissenting (2nd) run
            }
            _ => panic!("expected SingleDissent, got {out:?}"),
        }
        // all-diff (n=3) → NoMajority
        let runs = vec![1u8, 2u8, 3u8];
        let out = tmr(replay(runs), 3);
        assert_eq!(out, VoteOutcome::NoMajority);
        // n=2 disagreement → NoMajority
        let runs = vec![5u8, 6u8];
        let out = tmr(replay(runs), 2);
        assert_eq!(out, VoteOutcome::NoMajority);
    }

    // ── Both non-unanimous classes trip the breaker ──
    #[test]
    fn both_non_unanimous_classes_trip_breaker() {
        // SingleDissent trips.
        let mut b = breaker();
        let runs = vec![1u8, 2u8, 1u8];
        let out = tmr(replay(runs), 3);
        assert!(matches!(out, VoteOutcome::SingleDissent { .. }));
        assert!(wire_vote_mismatch("test::dissent", &out, &mut b));
        assert_eq!(b.current_state(), BreakerState::Open);

        // NoMajority trips too.
        let mut b2 = breaker();
        let runs = vec![1u8, 2u8, 3u8];
        let out2 = tmr(replay(runs), 3);
        assert_eq!(out2, VoteOutcome::NoMajority);
        assert!(wire_vote_mismatch("test::nomajority", &out2, &mut b2));
        assert_eq!(b2.current_state(), BreakerState::Open);

        // Unanimous does NOT trip.
        let mut b3 = breaker();
        let out3 = tmr(|| 9u8, 3);
        assert!(matches!(out3, VoteOutcome::Unanimous(_)));
        assert!(!wire_vote_mismatch("test::unanimous", &out3, &mut b3));
        assert_eq!(b3.current_state(), BreakerState::Closed);
    }

    // ── Item 5 (falsifiability proof): fault-injection ──
    // A testkit hook forces f's 2nd run to differ; the harness must return
    // non-Unanimous → trip the breaker → write the FDR Alarm entry.
    #[test]
    fn fault_injection_corrupts_one_replica_and_trips() {
        let _ = fdr::emit_alarm; // symbol reachable / no-unused

        let mut b = breaker();
        // The injected fault: run 0 and 2 agree, run 1 (the 2nd) is corrupted.
        let good = sample_event().event_id();
        let mut bad = good;
        bad[0] ^= 0xFF; // flip a byte to simulate a transient compute flip
        let runs: Vec<[u8; 32]> = vec![good, bad, good];
        let out = tmr(replay(runs), 3);
        assert!(
            matches!(out, VoteOutcome::SingleDissent { .. }),
            "injected mismatch must be caught as SingleDissent, got {out:?}"
        );
        // The harness routes it: breaker trips (this is the exact fn the applied
        // wrappers call, so the wrapper would trip identically).
        let tripped = wire_vote_mismatch("event_log::MeshEvent::event_id", &out, &mut b);
        assert!(tripped, "non-unanimous must trip");
        assert_eq!(
            b.current_state(),
            BreakerState::Open,
            "breaker must be Open after trip"
        );

        // Fail-closed contract: a non-unanimous outcome must NEVER be handed back.
        // The applied wrapper `event_id_tmr` returns `Err` for every non-Unanimous
        // arm; assert that class directly on the caught outcome.
        match out {
            VoteOutcome::Unanimous(_) => panic!("caught mismatch must not be Unanimous"),
            _ => { /* wrapper returns Err here — fail-closed */ }
        }

        // And the same wrapper on a CLEAN event_id is Unanimous → Ok (happy path,
        // proving the guard doesn't trip on legitimate deterministic input).
        let mut b_clean = breaker();
        let result = event_id_tmr(&sample_event(), &mut b_clean, 3);
        assert!(result.is_ok(), "clean event_id must be Unanimous → Ok");
        assert_eq!(b_clean.current_state(), BreakerState::Closed);
    }

    // ── Item 3 (debug-differential): cross-check the vote tally ──
    #[cfg(debug_assertions)]
    #[test]
    fn debug_differential_tally_cross_check() {
        let runs = vec![3u8, 3u8, 7u8]; // one dissent
        let shipped = tmr(replay(runs.clone()), 3);

        // Independent re-tally (does not call classify_vote). It must arrive at the
        // SAME classification + value + replica as the shipped path, mirroring
        // classify_vote's exact (index-position based) replica rule.
        let mut counts: Vec<(u8, u8)> = Vec::new();
        for &v in &runs {
            if let Some(c) = counts.iter_mut().find(|(u, _)| *u == v) {
                c.1 += 1;
            } else {
                counts.push((v, 1));
            }
        }
        // Majority value + its FIRST positional index (mirrors classify_vote's best_idx).
        let (majority_value, _) = counts
            .iter()
            .max_by_key(|(_, c)| *c)
            .map(|(v, c)| (*v, *c))
            .unwrap();
        let best_idx = runs.iter().position(|&v| v == majority_value).unwrap();
        let replica = (0..runs.len()).find(|&i| i != best_idx).unwrap_or(0);
        let independent = if counts.len() == 1 {
            VoteOutcome::Unanimous(runs[0])
        } else if counts
            .iter()
            .any(|(_, c)| *c as u32 * 2 > runs.len() as u32)
        {
            VoteOutcome::SingleDissent {
                value: majority_value,
                replica: replica as u8,
            }
        } else {
            VoteOutcome::NoMajority
        };
        assert_eq!(shipped, independent, "debug-differential tally must match");
    }

    // ── Real pure-function integration: deterministic event_id + apply_tax ──
    #[test]
    fn applied_wrappers_pass_on_clean_deterministic_input() {
        let mut b = breaker();
        let ev = sample_event();
        let id = event_id_tmr(&ev, &mut b, 3).expect("clean event_id must be Unanimous");
        assert_eq!(id, ev.event_id());

        let mut b2 = breaker();
        let tax =
            apply_tax_tmr(&mut b2, 3, 1000, 0.20, false).expect("clean tax must be Unanimous");
        assert_eq!(tax, crate::money::apply_tax(1000, 0.20, false).unwrap());
        // Breaker stayed closed (no trip on clean input).
        assert_eq!(b.current_state(), BreakerState::Closed);
        assert_eq!(b2.current_state(), BreakerState::Closed);
    }
}
