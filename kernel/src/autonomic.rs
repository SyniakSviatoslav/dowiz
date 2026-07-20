//! `autonomic.rs` — Item 21: deterministic bounded gain-scheduling layer.
//!
//! A **deterministic bounded-adaptive-control** module — gain-scheduling in the
//! classical (explicit control-law) sense, **explicitly not machine learning**.
//! The kernel *senses* drift (`markov::Verdict`, `spectral::DriftClass`) and this
//! module *acts* with a small, explicit, **auditable table** of
//! {classified-state → bounded adjustment} laws — one per tunable constant. Every
//! adjustment stays within a **pre-proven stability bound** `[MIN, MAX]`, is
//! **unconstructible outside that bound**, is **FDR-logged as a first-class
//! `Tuning` event**, and is a **checkable equation** (never a learned weight).
//!
//! The pilot tunable is `token_bucket`'s refill rate. The autonomic layer's most
//! severe response (extreme-end `Unstable + StrangeAttractor`) does **not** act
//! unilaterally — it routes *through* the item-9 `Breaker`'s `tick` path.
//!
//! Pure `std`, zero external dependencies.

use crate::breaker::{Breaker, BreakerState, SignalVector, SignalWeights, TripCause};
use crate::markov::Verdict;
use crate::spectral::DriftClass;

// ── Pre-proven stability bound for the token_bucket refill rate ──────────────────
//
// CFL-style stability limit (blueprint §3 / open-question 2): a refill rate the
// integrator sees must stay within a proven-stable `[MIN, MAX]` interval. The
// bound is pinned here as a named constant with a proof sketch, not guessed:
//
//   * `MIN = 0.0` — a non-negative rate is required (a negative refill would
//     *drain* the budget; the one-shot `refill_rate = 0` budget-drain pattern is
//     still representable).
//   * `MAX = 100.0` tokens/sec — the verified bound from the existing
//     `token_bucket` tests (the over-grant invariant is proven against rates in
//     this regime; `DT_STABLE`/CFL sampling cadence assumes the rate cannot
//     exceed the per-tick capacity the bucket can service without the integrator
//     going unstable). Pushing the rate above `MAX` would let a single tick
//     over-grant beyond the proven `capacity + rate*elapsed` ceiling.
//
// Every `BoundedRate` is *constructed* through one of two in-module ctors that
// both make an out-of-bound value **inexpressible**:
//   * `from_f64` — clamps silently (used by `schedule`, so the invariant can
//     never be violated by an adjustment equation).
//   * `try_from_f64` — *rejects* (returns `None`) out-of-bound values (the public
//     fallible entry; the planted-fault self-test proves it rejects unsafe laws).
const BOUND_MIN: f64 = 0.0;
const BOUND_MAX: f64 = 100.0;

/// A newtype over `f64` that **cannot** hold a value outside `[BOUND_MIN,
/// BOUND_MAX]`. The field is private and the only constructors clamp/reject, so
/// an out-of-bound `BoundedRate` is *inexpressible* — not merely avoided.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BoundedRate {
    value: f64,
}

impl BoundedRate {
    /// The proven-stable interval (named constants, not literals scattered in code).
    pub const MIN: f64 = BOUND_MIN;
    pub const MAX: f64 = BOUND_MAX;

    /// Clamping constructor — the ONLY path `schedule` uses. An out-of-bound input
    /// is brought back into `[MIN, MAX]`, so the over-grant invariant can never be
    /// broken by an adjustment equation. This is why an out-of-bound `BoundedRate`
    /// is inexpressible: any attempt to build one lands inside the bound.
    #[inline]
    pub fn from_f64(v: f64) -> Self {
        let v = if v.is_nan() {
            BOUND_MIN
        } else if v < BOUND_MIN {
            BOUND_MIN
        } else if v > BOUND_MAX {
            BOUND_MAX
        } else {
            v
        };
        debug_assert!(v >= BOUND_MIN && v <= BOUND_MAX, "BoundedRate clamp broke");
        BoundedRate { value: v }
    }

    /// Rejecting constructor — returns `None` iff `v` is outside `[MIN, MAX]`.
    /// This is the public fallible entry point and the proof surface for the
    /// planted-fault self-test: an unsafe law that would produce an out-of-bound
    /// rate is *rejected* here, never constructed.
    #[inline]
    pub fn try_from_f64(v: f64) -> Option<Self> {
        if v < BOUND_MIN || v > BOUND_MAX || v.is_nan() {
            None
        } else {
            Some(BoundedRate { value: v })
        }
    }

    /// The rate as a raw `f64`. Always within `[MIN, MAX]` (the invariant).
    #[inline]
    pub fn get(self) -> f64 {
        self.value
    }
}

/// One row of the explicit control-law table: a {classified-state → bounded
/// adjustment} law. Each adjustment is a **checkable equation** — `rate *= mult`
/// (the only numeric literal in the whole module lives in `LAW_TABLE` below) —
/// plus the FDR tag and the "route through the breaker" flag.
#[derive(Clone, Copy)]
pub struct Adjustment {
    /// Multiplier applied to the current rate: `next = current * mult`.
    pub mult: f64,
    /// FDR event classifier tag for this law (first-class `Tuning` record).
    pub tag: &'static str,
    /// If true, the extreme response does NOT apply unilaterally; it routes
    /// through the item-9 breaker's `tick` path instead.
    pub route_to_breaker: bool,
}

/// The explicit {classified-state → bounded adjustment} table — one row per
/// tunable (the pilot: `token_bucket` refill rate), enumerating all
/// `DriftClass × Verdict` = 9 combos. **The only numeric literals in the module
/// live here** (the `mult` factors); every other adjustment is a checkable
/// equation against `current`.
///
/// Law shape (back off as drift worsens; route the extreme end through the
/// breaker):
///   Damped   (stable)        → ~no-op (×1.0 .. ×0.95)
///   Resonant (ringing)      → back off (×0.95 .. ×0.85)
///   Unstable (diverging)    → strong back off (×0.8 .. ×0.5) + Unstable+StrangeAttractor routes to breaker
pub const LAW_TABLE: [(DriftClass, Verdict, Adjustment); 9] = [
    (
        DriftClass::Damped,
        Verdict::Healthy,
        Adjustment {
            mult: 1.0,
            tag: "damped_healthy",
            route_to_breaker: false,
        },
    ),
    (
        DriftClass::Damped,
        Verdict::LimitCycle,
        Adjustment {
            mult: 1.0,
            tag: "damped_limit_cycle",
            route_to_breaker: false,
        },
    ),
    (
        DriftClass::Damped,
        Verdict::StrangeAttractor,
        Adjustment {
            mult: 0.95,
            tag: "damped_strange",
            route_to_breaker: false,
        },
    ),
    (
        DriftClass::Resonant,
        Verdict::Healthy,
        Adjustment {
            mult: 0.95,
            tag: "resonant_healthy",
            route_to_breaker: false,
        },
    ),
    (
        DriftClass::Resonant,
        Verdict::LimitCycle,
        Adjustment {
            mult: 0.9,
            tag: "resonant_limit_cycle",
            route_to_breaker: false,
        },
    ),
    (
        DriftClass::Resonant,
        Verdict::StrangeAttractor,
        Adjustment {
            mult: 0.85,
            tag: "resonant_strange",
            route_to_breaker: false,
        },
    ),
    (
        DriftClass::Unstable,
        Verdict::Healthy,
        Adjustment {
            mult: 0.8,
            tag: "unstable_healthy",
            route_to_breaker: false,
        },
    ),
    (
        DriftClass::Unstable,
        Verdict::LimitCycle,
        Adjustment {
            mult: 0.6,
            tag: "unstable_limit_cycle",
            route_to_breaker: false,
        },
    ),
    (
        DriftClass::Unstable,
        Verdict::StrangeAttractor,
        Adjustment {
            mult: 0.5,
            tag: "unstable_strange",
            route_to_breaker: true,
        },
    ),
];

/// The FDR event emitted for every adjustment — a first-class `Tuning` record.
/// Pure data; `write_to_ring` / `emit` serialize it into the Tier-1 FDR.
#[derive(Clone, Copy, PartialEq)]
pub struct FdrAdjustment {
    pub class: DriftClass,
    pub verdict: Verdict,
    pub from_rate: f64,
    pub to_rate: f64,
    pub tag: &'static str,
    pub route_to_breaker: bool,
}

impl FdrAdjustment {
    /// Build the FDR `Tuning` event payload fields (deterministic, no I/O).
    fn fields(&self) -> Vec<(&'static str, String)> {
        vec![
            ("drift_class", format!("{:?}", self.class)),
            ("verdict", format!("{:?}", self.verdict)),
            ("from_rate", format!("{:.6}", self.from_rate)),
            ("to_rate", format!("{:.6}", self.to_rate)),
            ("tag", self.tag.to_string()),
            ("route_to_breaker", self.route_to_breaker.to_string()),
        ]
    }

    /// Emit this adjustment as a first-class `Tuning` FDR record through the
    /// installed FDR sink (no-op if no sink is installed, matching every other
    /// `fdr::emit_*` facade).
    pub fn emit(&self) {
        crate::fdr::emit_tuning("autonomic_gain_schedule", &self.fields());
    }

    /// Write this adjustment directly into a durable `FdrRing` as a `Tuning`
    /// record (used by the FDR-count oracle test; isolated from the process-global
    /// sink so it needs no `fdr::init`).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn write_to_ring(&self, ring: &mut crate::fdr::ring::FdrRing) {
        let ev = crate::fdr::schema::FdrEvent::stamp(
            0,
            crate::fdr::Level::Info,
            crate::fdr::schema::Kind::Tuning,
            "autonomic_gain_schedule".to_string(),
            crate::fdr::schema::StampPolicy::Cheap,
            self.fields(),
        );
        let _ = ring.append(&ev);
    }
}

/// Pure control law: table lookup → bounded adjustment → FDR event to emit.
/// Deterministic (P6): a replayed classification sequence reproduces the
/// identical `(BoundedRate, FdrAdjustment)` sequence. `debug_assert!`s the output
/// is within `[MIN, MAX]` (cross-checking the newtype's clamp against an
/// independent bound check) on every call.
pub fn schedule(
    class: DriftClass,
    verdict: Verdict,
    current: BoundedRate,
) -> (BoundedRate, FdrAdjustment) {
    // The law table is exhaustive over the 9 (DriftClass × Verdict) combos, so
    // this `unwrap` is total by construction; the exhaustive oracle test asserts it.
    let row = LAW_TABLE
        .iter()
        .find(|r| r.0 == class && r.1 == verdict)
        .expect("LAW_TABLE must cover every (DriftClass, Verdict) combo");

    // Checkable equation: `next = current * mult`, clamped into the proven-stable
    // bound by the inexpressible `BoundedRate` ctor.
    let next = BoundedRate::from_f64(current.value * row.2.mult);

    // Item 3 (debug-differential): independent bound check cross-validating the
    // newtype's clamp on every schedule call.
    debug_assert!(
        next.value >= BoundedRate::MIN && next.value <= BoundedRate::MAX,
        "schedule produced an out-of-bound rate"
    );

    let fdr = FdrAdjustment {
        class,
        verdict,
        from_rate: current.value,
        to_rate: next.value,
        tag: row.2.tag,
        route_to_breaker: row.2.route_to_breaker,
    };
    (next, fdr)
}

/// Schedule an adjustment, and — for the extreme-end response — route it *through*
/// the item-9 breaker instead of applying it unilaterally. When `route_to_breaker`
/// is set, an external `TripCause` is fed into `Breaker::tick` (forcing the trip
/// path) and the backed-off rate is *not* applied directly; the breaker owns the
/// emergency response.
pub fn schedule_into_breaker(
    class: DriftClass,
    verdict: Verdict,
    current: BoundedRate,
    breaker: &mut Breaker,
) -> (BoundedRate, FdrAdjustment) {
    let (next, fdr) = schedule(class, verdict, current);
    if fdr.route_to_breaker {
        // Do NOT act unilaterally: emit a `TripCause`-adjacent signal into the
        // breaker's tick path. A zero signal with an external `ProbeMismatch`
        // cause forces the trip transition (Closed→Open) regardless of score —
        // the breaker is the binary emergency net; the autonomic layer is the
        // graduated controller and composes with it, never replaces it.
        let w = SignalWeights {
            conf: 1.0,
            drift: 1.0,
            cusum: 1.0,
            constraint: 1.0,
            disagreement: 1.0,
            truth: 1.0,
        };
        // `SignalVector` is a public struct with public fields; construct a zero
        // signal directly (the breaker's `trip_score` of a zero vector is 0, so the
        // external `ProbeMismatch` cause is what forces the trip — the breaker owns
        // the emergency response, the autonomic layer only routes into it).
        let sig = SignalVector {
            window_seq: 0,
            confidence_gap: 0.0,
            ewma_drift: 0.0,
            cusum: 0.0,
            constraint_violations: 0,
            disagreement: 0.0,
            truthfulness_fail: 0,
            weights: w,
        };
        breaker.tick(sig, Some(TripCause::ProbeMismatch), false);
    }
    (next, fdr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::breaker::{fit_from_rates, RateProfile, SignalWeights, ThresholdId};

    fn tid() -> ThresholdId {
        let p = RateProfile {
            w_consec: 3,
            w_kill: 5,
            probes: 4,
            cooldown_base: 8,
            cooldown_cap: 1024,
        };
        let w = SignalWeights {
            conf: 1.0,
            drift: 1.0,
            cusum: 1.0,
            constraint: 1.0,
            disagreement: 1.0,
            truth: 1.0,
        };
        let mut rates: Vec<(f32, bool)> = Vec::new();
        for i in 0..20 {
            rates.push(((i as f32) / 50.0, false));
        }
        for i in 20..40 {
            rates.push(((i as f32) / 40.0, true));
        }
        fit_from_rates(&rates, 0.05, p, w).unwrap()
    }

    /// All 9 (DriftClass × Verdict) combos, in table order, for the oracles.
    fn all_combos() -> [(DriftClass, Verdict); 9] {
        [
            (DriftClass::Damped, Verdict::Healthy),
            (DriftClass::Damped, Verdict::LimitCycle),
            (DriftClass::Damped, Verdict::StrangeAttractor),
            (DriftClass::Resonant, Verdict::Healthy),
            (DriftClass::Resonant, Verdict::LimitCycle),
            (DriftClass::Resonant, Verdict::StrangeAttractor),
            (DriftClass::Unstable, Verdict::Healthy),
            (DriftClass::Unstable, Verdict::LimitCycle),
            (DriftClass::Unstable, Verdict::StrangeAttractor),
        ]
    }

    #[test]
    fn bounded_rate_rejects_out_of_bound_construction() {
        // The inexpressibility proof: outside [MIN, MAX] is rejected; inside is accepted.
        assert!(BoundedRate::try_from_f64(BoundedRate::MIN).is_some());
        assert!(BoundedRate::try_from_f64(BoundedRate::MAX).is_some());
        assert!(BoundedRate::try_from_f64(BoundedRate::MAX + 1.0).is_none());
        assert!(BoundedRate::try_from_f64(BoundedRate::MIN - 1.0).is_none());
        assert!(BoundedRate::try_from_f64(f64::NAN).is_none());
        assert!(BoundedRate::try_from_f64(f64::INFINITY).is_none());
        // from_f64 clamps inexpressibly — never constructs an out-of-bound value.
        assert_eq!(BoundedRate::from_f64(1e9).get(), BoundedRate::MAX);
        assert_eq!(BoundedRate::from_f64(-1e9).get(), BoundedRate::MIN);
    }

    #[test]
    fn exhaustive_oracle_all_nine_combos_match_law_table() {
        // Headline property: schedule produces the exact table-defined BoundedRate
        // and FDR event for every (DriftClass, Verdict) combo.
        let current = BoundedRate::from_f64(50.0);
        for (class, verdict) in all_combos() {
            let row = LAW_TABLE
                .iter()
                .find(|r| r.0 == class && r.1 == verdict)
                .unwrap();
            let expected_rate = BoundedRate::from_f64(current.get() * row.2.mult);
            let (next, fdr) = schedule(class, verdict, current);
            assert_eq!(
                next, expected_rate,
                "rate mismatch for {:?}/{:?}",
                class, verdict
            );
            assert_eq!(
                fdr.tag, row.2.tag,
                "tag mismatch for {:?}/{:?}",
                class, verdict
            );
            assert_eq!(
                fdr.route_to_breaker, row.2.route_to_breaker,
                "route flag mismatch for {:?}/{:?}",
                class, verdict
            );
            // from_rate carried through exactly.
            assert_eq!(fdr.from_rate, 50.0);
        }
    }

    #[test]
    fn planted_fault_unsafe_law_is_rejected() {
        // The item-21 analog of the planted-fault self-test: an unsafe law that
        // would produce an out-of-bound rate is demonstrably rejected. We model the
        // unsafe law as "set rate to 1e9" and show the *only* construction path
        // rejects it (the newtype makes the unsafe value inexpressible). If a
        // caller tried to apply such a law, `try_from_f64` returns `None` — the
        // law fails to construct a valid `BoundedRate`.
        const UNSAFE_TARGET: f64 = 1e9; // far outside [MIN, MAX]
        assert!(
            BoundedRate::try_from_f64(UNSAFE_TARGET).is_none(),
            "an unsafe (out-of-bound) law target must be rejected, not constructed"
        );
        // And the clamping ctor silently pins it to MAX — the invariant holds no
        // matter what an equation computes.
        assert_eq!(BoundedRate::from_f64(UNSAFE_TARGET).get(), BoundedRate::MAX);
    }

    #[test]
    fn no_classification_sequence_leaves_proven_stable_bound() {
        // Native-exhaustive over the 9-combo space: drive `schedule` from a start
        // value and assert every output stays within [MIN, MAX]. The structural
        // guarantee is the `BoundedRate` newtype; this is the belt-and-suspenders
        // confirmation (and it sweeps every combo repeatedly).
        let combos = all_combos();
        // Start inside the bound and walk a long, mixed sequence.
        let mut rate = BoundedRate::from_f64(50.0);
        for _round in 0..200 {
            for &(class, verdict) in combos.iter() {
                let (next, _fdr) = schedule(class, verdict, rate);
                assert!(
                    next.get() >= BoundedRate::MIN && next.get() <= BoundedRate::MAX,
                    "sequence left the proven-stable bound: {:?}",
                    next.get()
                );
                rate = next;
            }
        }
    }

    #[test]
    fn replay_determinism_byte_identical_sequence() {
        // P6 determinism: same classification stream in → byte-identical
        // BoundedRate/FDR sequence out. We replay the same stream twice and assert
        // the control outputs are identical.
        let stream = [
            (DriftClass::Damped, Verdict::Healthy),
            (DriftClass::Resonant, Verdict::LimitCycle),
            (DriftClass::Unstable, Verdict::Healthy),
            (DriftClass::Resonant, Verdict::StrangeAttractor),
            (DriftClass::Unstable, Verdict::StrangeAttractor),
        ];
        let run = |start: BoundedRate| -> Vec<(f64, &'static str, bool)> {
            let mut rate = start;
            let mut out = Vec::new();
            for &(c, v) in stream.iter() {
                let (next, fdr) = schedule(c, v, rate);
                out.push((next.get(), fdr.tag, fdr.route_to_breaker));
                rate = next;
            }
            out
        };
        let a = run(BoundedRate::from_f64(50.0));
        let b = run(BoundedRate::from_f64(50.0));
        assert_eq!(a, b, "replayed classification stream must be deterministic");
        // The extreme end is in the stream and must be flagged for breaker routing.
        assert!(a.last().unwrap().2, "extreme end must route to breaker");
    }

    #[test]
    fn each_adjustment_emits_one_fdr_tuning_record() {
        // Acceptance #4: N adjustments → N first-class FDR `Tuning` records.
        // Write all 9 schedule outputs into an isolated durable ring and recover it.
        #[cfg(not(target_arch = "wasm32"))]
        {
            let dir = std::env::temp_dir().join("item21_autonomic_fdr_test");
            let _ = std::fs::create_dir_all(&dir);
            let mut ring = crate::fdr::ring::FdrRing::open(dir.clone(), 1 << 20).unwrap();
            for (class, verdict) in all_combos() {
                let (_next, fdr) = schedule(class, verdict, BoundedRate::from_f64(50.0));
                fdr.write_to_ring(&mut ring);
            }
            drop(ring);
            let rec = crate::fdr::ring::recover(&dir);
            let tuning: Vec<_> = rec.records.iter().filter(|r| r.kind == "tuning").collect();
            assert_eq!(
                tuning.len(),
                9,
                "each of the 9 adjustments must produce exactly one Tuning record"
            );
            // No record should have been dropped or corrupted.
            assert_eq!(rec.crc_failures, 0);
            assert_eq!(rec.torn_tail, 0);
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    #[test]
    fn extreme_response_routes_through_breaker_not_unilaterally() {
        // Acceptance #6: Unstable + StrangeAttractor routes through the breaker
        // (item 9) rather than acting unilaterally.
        let mut b = Breaker::new([1u8; 16], tid());
        assert_eq!(b.current_state(), BreakerState::Closed);

        let (next, fdr) = schedule_into_breaker(
            DriftClass::Unstable,
            Verdict::StrangeAttractor,
            BoundedRate::from_f64(50.0),
            &mut b,
        );
        // The breaker was tripped by the routed external cause (Closed → Open).
        assert_eq!(
            b.current_state(),
            BreakerState::Open,
            "extreme response must route into the breaker (tripped Open)"
        );
        // The law is flagged for breaker routing.
        assert!(fdr.route_to_breaker);
        assert_eq!(fdr.tag, "unstable_strange");
        // The backed-off rate was computed (informational) but NOT applied to the
        // bucket unilaterally — the breaker owns the emergency response.
        assert_eq!(next.get(), 25.0);

        // A non-extreme combo does NOT trip the breaker.
        let mut b2 = Breaker::new([2u8; 16], tid());
        let (_n2, fdr2) = schedule_into_breaker(
            DriftClass::Resonant,
            Verdict::LimitCycle,
            BoundedRate::from_f64(50.0),
            &mut b2,
        );
        assert!(!fdr2.route_to_breaker);
        assert_eq!(b2.current_state(), BreakerState::Closed);
    }
}
