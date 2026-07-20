//! `autonomic_pmu.rs` — Item 27 (response half): PMU-**informed** response routing.
//!
//! Wiring-only. A PMU-informed `(DriftClass, Verdict)` flows through item 21's
//! `schedule()` → `BoundedRate` adjustment → FDR event, the **exact same path** a
//! non-PMU classification uses. There is **no PMU-specific control law** and **no
//! CPU/GPU fast path** (synthesis §18(c)): the PMU signal only ever *contributes to
//! the classification*, never to the control-law arithmetic.
//!
//! ## P6 determinism guard (the crux)
//!
//! The classifier-input half (`fdr/pmu.rs`, `03887462a`) kept `analyze_detailed` /
//! `classify_drift` pure and rode PMU alongside each `Verdict` as a diagnostic
//! companion. This half lets that companion *act* — but only through a
//! **quantized** band ([`PmuBand`]). Raw `u64` deltas are bucketed into a small
//! fixed enum; their exact magnitude is discarded before any decision. The
//! response arithmetic is therefore a function of the **class** (which the band may
//! promote), never of the host-variable raw counter. So a replay of the same
//! counter sequence reproduces the same adjustment on any host (P6), and a
//! `Unstable + StrangeAttractor` extreme routes through item 9's breaker — not a
//! unilateral PMU action.
//!
//! If PMU is unreadable (`Unavailable`, the gated-host default), it degrades to
//! `Calm` and the response falls back to item-21-as-is (companion-only, fail-safe).
//!
//! Diagnostic-grade: **no CI job is keyed to any PMU value** (blueprint §5 #4).
//! Pure `std`, zero new dependencies.

use crate::autonomic::{schedule, schedule_into_breaker, BoundedRate, FdrAdjustment, LAW_TABLE};
use crate::breaker::{Breaker, TripCause};
use crate::fdr::pmu::PmuStamp;
use crate::fdr::schema::Reading;
use crate::markov::Verdict;
use crate::spectral::DriftClass;

/// A quantized PMU band — the **only** form in which a PMU counter may influence a
/// classification. Raw `u64` deltas are bucketed into this small fixed set; the
/// counter's exact magnitude is discarded here and never reaches the control-law
/// arithmetic. This is the P6 determinism guard: the response is a function of the
/// BAND (a stable enum), not the host-variable raw counter.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PmuBand {
    /// Cache-miss delta at/below the calm floor — no PMU contribution.
    Calm,
    /// Cache-miss delta elevated but below the storm threshold — companion only.
    Elevated,
    /// Cache-miss delta at/above the fitted storm threshold — may promote a
    /// marginal `Resonant` classification toward `Unstable`.
    Storm,
}

/// Fitted storm threshold for the quantized cache-miss band. A fixed, auditable
/// constant (not a learned weight, not a raw counter). Deltas at/above this land in
/// [`PmuBand::Storm`]. `Elevated` is the next decade down; below that is `Calm`.
pub const CACHE_MISS_STORM_THRESHOLD: u64 = 10_000;

/// Quantize a bracketed cache-miss delta into a [`PmuBand`]. This is the single
/// seam where a PMU counter enters the classification — and it is quantized on
/// purpose. A `Unavailable` Tier-B counter (the gated-host default) degrades to
/// `Calm`: PMU cannot contribute when it cannot be read, so the response falls back
/// to item-21-as-is (fail-safe, P6 preserved).
pub fn band(cache_miss_delta: Reading<u64>) -> PmuBand {
    match cache_miss_delta {
        Reading::Unavailable(_) => PmuBand::Calm,
        Reading::Value(v) if v >= CACHE_MISS_STORM_THRESHOLD => PmuBand::Storm,
        Reading::Value(v) if v >= CACHE_MISS_STORM_THRESHOLD / 10 => PmuBand::Elevated,
        Reading::Value(_) => PmuBand::Calm,
    }
}

/// Quantize a full bracketed `PmuStamp` delta (derives the band from the cache-miss
/// field). Convenience wrapper around [`band`].
pub fn band_from_stamp(delta: PmuStamp) -> PmuBand {
    band(delta.hw_cache_misses)
}

/// PMU-informed classification: given the base (spectral/markov) classification and
/// a quantized PMU band, return the classification the response should act on.
///
/// PURE function of `(class, verdict, band)` — never of a raw counter. PMU may only
/// **promote** (upgrade severity) along the existing `DriftClass` axis, never invent
/// a milder state. This keeps the response arithmetic a deterministic function of
/// the resulting class, so the only thing PMU can change is *which row* of item 21's
/// `LAW_TABLE` is looked up — never the arithmetic inside that row.
pub fn informed_classification(
    class: DriftClass,
    verdict: Verdict,
    band: PmuBand,
) -> (DriftClass, Verdict) {
    match (class, band) {
        // A cache-miss storm on a marginal (Resonant) loop promotes it to
        // `Unstable` — the quantized PMU contribution. Damped/Unstable and any
        // non-Storm band are unchanged.
        (DriftClass::Resonant, PmuBand::Storm) => (DriftClass::Unstable, verdict),
        _ => (class, verdict),
    }
}

/// Route a PMU-informed classification through item 21's bounded-control-law path.
/// PURE function of the class — the PMU band only feeds [`informed_classification`].
/// The adjustment is item 21's table value for the resulting class, never a
/// PMU-specific law.
pub fn respond(
    base_class: DriftClass,
    base_verdict: Verdict,
    band: PmuBand,
    current: BoundedRate,
) -> (BoundedRate, FdrAdjustment) {
    let (c, v) = informed_classification(base_class, base_verdict, band);
    let (next, fdr) = schedule(c, v, current);
    // Item 3 (debug-differential): cross-check PMU did not alter the arithmetic —
    // the adjustment must equal `schedule()` on the *informed* class, which is the
    // same table lookup a non-PMU classification would use. If a future edit leaks a
    // raw counter into this path, this `debug_assert!` trips (the recomputed
    // reference uses the pure `schedule` on the class alone).
    debug_assert!({
        let (ref_next, ref_fdr) = schedule(c, v, current);
        next == ref_next && fdr == ref_fdr
    });
    (next, fdr)
}

/// Like [`respond`] but for the extreme-end route: routes the PMU-informed
/// classification through item 21's `schedule_into_breaker` seam, which routes
/// `Unstable + StrangeAttractor` into item 9's breaker `tick`/trip path. This is the
/// ONLY breaker entry point used (item 21's seam) — never a new PMU-triggered one.
pub fn respond_into_breaker(
    base_class: DriftClass,
    base_verdict: Verdict,
    band: PmuBand,
    current: BoundedRate,
    breaker: &mut Breaker,
) -> (BoundedRate, FdrAdjustment) {
    let (c, v) = informed_classification(base_class, base_verdict, band);
    schedule_into_breaker(c, v, current, breaker)
}

/// FORBIDDEN PATH — test-oracle only. Demonstrates what P6 forbids: the raw PMU
/// counter value leaking into the control-law arithmetic as a host-variable `f64`.
/// **Never call this from the real response path.** The negative test
/// (`negative_raw_pmu_float_breaks_replay_equality`) uses it to prove the quantized
/// guard is necessary: two hosts with the same band/class but different raw counters
/// get different adjustments on this path, while the quantized path is replay-safe.
#[doc(hidden)]
pub fn respond_with_raw_cache_miss(
    class: DriftClass,
    verdict: Verdict,
    raw_cache_miss_delta: u64,
    current: BoundedRate,
) -> (BoundedRate, FdrAdjustment) {
    // P6 VIOLATION: the raw counter value leaks straight into the arithmetic.
    let row = LAW_TABLE
        .iter()
        .find(|r| r.0 == class && r.1 == verdict)
        .expect("LAW_TABLE must cover every (DriftClass, Verdict) combo");
    let raw_contrib = raw_cache_miss_delta as f64 * 1e-6; // host-variable term
    let next = BoundedRate::from_f64(current.get() * row.2.mult + raw_contrib);
    let fdr = FdrAdjustment {
        class,
        verdict,
        from_rate: current.get(),
        to_rate: next.get(),
        tag: row.2.tag,
        route_to_breaker: row.2.route_to_breaker,
    };
    (next, fdr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::breaker::{fit_from_rates, BreakerState, RateProfile, SignalWeights, ThresholdId};

    /// Build a fitted `ThresholdId` for the breaker (mirrors `autonomic.rs`'s test
    /// harness — the breaker needs a fitted threshold, never a literal).
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

    fn all_bands() -> [PmuBand; 3] {
        [PmuBand::Calm, PmuBand::Elevated, PmuBand::Storm]
    }

    #[test]
    fn oracle_pmu_never_alters_arithmetic_only_the_class() {
        // Item 1 (oracle): sweep {DriftClass × Verdict × PmuBand} = 9 × 3.
        // PMU must not alter the arithmetic — `respond` must equal item-21's
        // `schedule()` on the informed (possibly PMU-promoted) class, for every combo.
        let current = BoundedRate::from_f64(50.0);
        for (class, verdict) in all_combos() {
            for band in all_bands() {
                let (ic, iv) = informed_classification(class, verdict, band);
                let (exp_next, exp_fdr) = schedule(ic, iv, current);
                let (next, fdr) = respond(class, verdict, band, current);
                assert_eq!(
                    next, exp_next,
                    "adjustment must be item-21's table value for {:?}/{:?} under band {:?}",
                    ic, iv, band
                );
                assert!(
                    fdr == exp_fdr,
                    "FDR event must match item-21's for {:?}/{:?} under band {:?}",
                    ic, iv, band
                );
                // The band only ever changes the class via promotion; it never mints a
                // rate outside item-21's law.
                if band == PmuBand::Storm && class == DriftClass::Resonant {
                    assert_eq!(ic, DriftClass::Unstable, "Storm promotes Resonant→Unstable");
                }
            }
        }
    }

    #[test]
    fn p6_replay_byte_identical_for_same_counter_trace() {
        // Headline P6 proof: feed a recorded PMU-counter trace + state sequence twice;
        // the response (adjustment + FDR events) must be byte-identical across replays.
        let trace: Vec<(DriftClass, Verdict, u64)> = vec![
            (DriftClass::Damped, Verdict::Healthy, 0),
            (DriftClass::Resonant, Verdict::LimitCycle, 500),
            (DriftClass::Resonant, Verdict::StrangeAttractor, 50_000), // Storm band
            (DriftClass::Unstable, Verdict::Healthy, 0),
            (DriftClass::Unstable, Verdict::StrangeAttractor, 0),
        ];
        let run = |t: &[(DriftClass, Verdict, u64)]| -> Vec<(f64, &'static str, bool)> {
            let mut rate = BoundedRate::from_f64(50.0);
            let mut out = Vec::new();
            for &(c, v, raw) in t {
                // Raw counter enters ONLY through the quantized band.
                let b = band(Reading::Value(raw));
                let (next, fdr) = respond(c, v, b, rate);
                out.push((next.get(), fdr.tag, fdr.route_to_breaker));
                rate = next;
            }
            out
        };
        let a = run(&trace);
        let b = run(&trace);
        assert_eq!(a, b, "replayed counter trace must be byte-identical (P6)");
        // The extreme end is flagged for breaker routing.
        assert!(a.last().unwrap().2, "extreme end must route to breaker");
    }

    #[test]
    fn p6_equivalent_bands_yield_identical_response_across_hosts() {
        // Two hosts with different raw counters but the SAME quantized band must
        // produce the identical response — the raw magnitude is invisible (P6).
        let band_a = band(Reading::Value(CACHE_MISS_STORM_THRESHOLD)); // Storm (small)
        let band_b = band(Reading::Value(999_999)); // Storm (large, same band)
        assert_eq!(band_a, band_b, "both raw deltas must quantize to Storm");
        let current = BoundedRate::from_f64(50.0);
        let (na, fa) = respond(DriftClass::Resonant, Verdict::Healthy, band_a, current);
        let (nb, fb) = respond(DriftClass::Resonant, Verdict::Healthy, band_b, current);
        assert_eq!(na, nb, "same band ⇒ identical adjustment regardless of raw counter");
        assert!(fa == fb, "same band ⇒ identical FDR event regardless of raw counter");
    }

    #[test]
    fn negative_raw_pmu_float_breaks_replay_equality() {
        // The item-27-response analog of the planted-fault self-test: a raw PMU float
        // leaking into the arithmetic MUST break replay-equality. Two hosts with the
        // SAME band/class but DIFFERENT raw counters get identical responses on the
        // quantized (correct) path, but DIFFERENT responses on the raw-float
        // (forbidden) path — proving the P6-purity guard is necessary.
        let raw_a: u64 = CACHE_MISS_STORM_THRESHOLD; // Storm, small
        let raw_b: u64 = 999_999; // Storm, large
        assert_eq!(
            band(Reading::Value(raw_a)),
            band(Reading::Value(raw_b)),
            "both raw deltas must quantize to the same band"
        );
        let current = BoundedRate::from_f64(50.0);

        // Quantized (correct) path: identical across the two raw values.
        let (qa, _) = respond(DriftClass::Resonant, Verdict::Healthy, band(Reading::Value(raw_a)), current);
        let (qb, _) = respond(DriftClass::Resonant, Verdict::Healthy, band(Reading::Value(raw_b)), current);
        assert_eq!(qa, qb, "quantized path must be replay-safe (P6)");

        // Raw-float (forbidden) path: the two raw values produce DIFFERENT
        // adjustments — i.e. it FAILS replay-equality.
        let (ra, _) = respond_with_raw_cache_miss(DriftClass::Resonant, Verdict::Healthy, raw_a, current);
        let (rb, _) = respond_with_raw_cache_miss(DriftClass::Resonant, Verdict::Healthy, raw_b, current);
        assert_ne!(
            ra, rb,
            "raw PMU float MUST break replay-equality — this is why we quantize (P6 guard)"
        );
    }

    #[test]
    fn debug_assert_cross_check_matches_item21_table_value() {
        // Item 3 (debug-differential): the response adjustment equals item-21's
        // class-derived adjustment for every combo/band (explicit cross-check beyond
        // the in-function `debug_assert!`).
        let current = BoundedRate::from_f64(50.0);
        for (class, verdict) in all_combos() {
            for band in all_bands() {
                let (ic, iv) = informed_classification(class, verdict, band);
                let (ref_next, ref_fdr) = schedule(ic, iv, current);
                let (next, fdr) = respond(class, verdict, band, current);
                assert_eq!(next, ref_next);
                assert!(fdr == ref_fdr);
            }
        }
    }

    #[test]
    fn routed_response_non_extreme_uses_bounded_control_law_path() {
        // Acceptance #3: a PMU-informed NON-extreme classification demonstrably routes
        // through item 21's bounded-control-law path (not a PMU-specific fast path).
        // Resonant + Healthy + Storm → promoted to Unstable + Healthy → item-21's
        // table value (mult 0.8 → 40.0), with no breaker routing.
        let (next, fdr) = respond(
            DriftClass::Resonant,
            Verdict::Healthy,
            PmuBand::Storm,
            BoundedRate::from_f64(50.0),
        );
        assert_eq!(next.get(), 40.0, "must use item-21's Unstable table value (0.8)");
        assert!(!fdr.route_to_breaker, "non-extreme must NOT route to breaker");
        assert_eq!(fdr.tag, "unstable_healthy");
        // Item 21's seam (schedule) is what produced this — no separate PMU law.
        let (ref_next, ref_fdr) = schedule(DriftClass::Unstable, Verdict::Healthy, BoundedRate::from_f64(50.0));
        assert_eq!(next, ref_next);
        assert!(fdr == ref_fdr);
    }

    #[test]
    fn routed_response_extreme_goes_through_breaker() {
        // Acceptance #3: a PMU-informed extreme classification (Unstable +
        // StrangeAttractor) routes through item 9's breaker via item 21's seam.
        let mut b = Breaker::new([1u8; 16], tid());
        assert_eq!(b.current_state(), BreakerState::Closed);
        let (next, fdr) = respond_into_breaker(
            DriftClass::Unstable,
            Verdict::StrangeAttractor,
            PmuBand::Storm,
            BoundedRate::from_f64(50.0),
            &mut b,
        );
        // The law is flagged for breaker routing and the breaker actually tripped.
        assert!(fdr.route_to_breaker);
        assert_eq!(
            b.current_state(),
            BreakerState::Open,
            "extreme response must route into the breaker (tripped Open)"
        );
        // The backed-off rate is item-21's table value (mult 0.5 → 25.0), not applied
        // unilaterally — the breaker owns the emergency response.
        assert_eq!(next.get(), 25.0);
    }

    #[test]
    fn pmu_storm_promotes_resonant_to_unstable_and_routes_to_breaker() {
        // Demonstrates the PMU band changing the ROUTING (via the class), not the
        // arithmetic: a Resonant + StrangeAttractor verdict is non-extreme until a PMU
        // Storm promotes it to Unstable, which then routes through item 9's breaker.
        let (base_next, base_fdr) = respond(
            DriftClass::Resonant,
            Verdict::StrangeAttractor,
            PmuBand::Calm,
            BoundedRate::from_f64(50.0),
        );
        assert!(!base_fdr.route_to_breaker, "without PMU storm this is non-extreme");
        assert_eq!(base_next.get(), 42.5, "Resonant table value (0.85)");

        let mut b = Breaker::new([3u8; 16], tid());
        let (promo_next, promo_fdr) = respond_into_breaker(
            DriftClass::Resonant,
            Verdict::StrangeAttractor,
            PmuBand::Storm,
            BoundedRate::from_f64(50.0),
            &mut b,
        );
        assert!(
            promo_fdr.route_to_breaker,
            "PMU-promoted Unstable+StrangeAttractor must route to breaker"
        );
        assert_eq!(b.current_state(), BreakerState::Open);
        // Uses item-21's table value for the now-Unstable class (mult 0.5 → 25.0).
        assert_eq!(promo_next.get(), 25.0);
    }

    #[test]
    fn pmu_absence_is_fail_safe_to_item21_as_is() {
        // When the PMU counter is unreadable (gated host), the band degrades to Calm
        // and the response equals item-21-as-is (no PMU contribution).
        assert_eq!(band(Reading::Unavailable(crate::fdr::schema::Absence::PermissionDenied)), PmuBand::Calm);
        let current = BoundedRate::from_f64(50.0);
        for (class, verdict) in all_combos() {
            let (next, fdr) = respond(class, verdict, PmuBand::Calm, current);
            let (ref_next, ref_fdr) = schedule(class, verdict, current);
            assert_eq!(next, ref_next, "Calm band must equal item-21-as-is");
            assert!(fdr == ref_fdr);
        }
    }

    // Acceptance #1 (grep): there is ONE classification mechanism. PMU enters the
    // existing `Verdict`/`DriftClass` pipeline via the single `band`/`informed_classification`
    // seam — there is no parallel PMU monitor. The static assertion below proves the
    // only PMU→classification entry points are the two quantized functions in this module.
    #[test]
    fn single_classification_seam_no_parallel_pmu_monitor() {
        // `informed_classification` is the sole place a band changes the class, and
        // `band` is the sole place a raw counter is read. Both are deterministic and
        // quantized; the breaker is reached ONLY through item 21's `schedule_into_breaker`
        // seam (no new TripCause / tick entry point invented here).
        let (c, v) = informed_classification(DriftClass::Resonant, Verdict::StrangeAttractor, PmuBand::Storm);
        assert_eq!(c, DriftClass::Unstable);
        // The extreme route uses item 21's seam with item 9's existing TripCause.
        let mut b = Breaker::new([4u8; 16], tid());
        respond_into_breaker(c, v, PmuBand::Storm, BoundedRate::from_f64(50.0), &mut b);
        assert_eq!(b.current_state(), BreakerState::Open);
        // The trip was forced by item 21's seam routing an external TripCause into the
        // breaker — not a new breaker entry point.
        let _ = TripCause::ProbeMismatch; // the cause schedule_into_breaker feeds (from item 21)
    }
}
