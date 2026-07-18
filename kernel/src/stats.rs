//! stats.rs — the kernel's single uncertainty primitive (E2).
//!
//! The Central Limit Theorem was already implemented in this codebase, correctly
//! and from first principles — but imprisoned inside one test
//! (`causal.rs::empirical_converges_to_analytic_as_n_grows`). Everywhere else a
//! number is reported it is reported **naked**: `evals.rs`'s `brier`/`ece`/`aurc`
//! are point estimates with no error bar; the "recall@5 = 1.0" headline is an
//! exact equality over a 12-query oracle with no confidence interval. A point
//! estimate shipped as a headline is *structurally the same object* as an
//! unverified "done" — a claim that replaces a check (RC-1 self-certification):
//! nobody can falsify it because it does not state its own confidence.
//!
//! This module promotes the √N law the kernel already wrote into a small, seeded,
//! **reusable** primitive and lets every scalar carry the check that would refute
//! it. It is a zero-dependency leaf (a sibling of `rng.rs`/`money.rs`/`noether.rs`)
//! so every layer — `causal.rs` (foundational Pearl stack), `evals.rs` (harness),
//! the retrieval/recall tests — depends on it **downward**. Putting it in
//! `evals.rs` would force `causal.rs` to import *upward* from the eval-harness
//! layer, a layering inversion P2 forbids.
//!
//! ## Which bound for which sample shape (READ THIS BEFORE CALLING)
//!
//! The single sharpest misuse of this module is applying a **large-n iid**
//! interval to a sample that is genuinely **small** or **non-iid**. Doing so
//! yields an interval that *looks* rigorous but is confidently wrong (the
//! self-certification failure wearing a lab coat). This is currently guarded by
//! **documentation convention, not by the type system** — nothing stops a caller
//! handing a 12-sample or an EMA-smoothed slice to `mean_se`/`normal_interval`.
//! Choose deliberately:
//!
//! | sample shape                         | use                                   |
//! |--------------------------------------|---------------------------------------|
//! | large-n, iid, finite-variance mean   | [`mean_se`] → [`normal_interval`]     |
//! | small-n binomial k/n (esp. p̂ at 0/1) | [`wilson_interval`] (Wald degenerates)|
//! | large-n error·√N convergence gate    | [`within_clt_envelope`]               |
//! | no closed-form SE (rank-cumulative)  | [`bootstrap_interval`] (seeded)       |
//! | EMA-smoothed / autocorrelated stream | move the stat UPSTREAM of the         |
//! |                                      | smoothing, or a circular moving-block |
//! |                                      | bootstrap — NOT `mean_se` (§2 E2)     |
//!
//! ## Reproducibility (P6 Cause-and-Effect)
//!
//! The analytic primitives use only `+ − × ÷ √` — IEEE-754 mandates `sqrt` be
//! correctly-rounded, so these are bit-identical cross-target (unlike the
//! transcendental `ln`/`sin`/… paths the `rng.rs` doctrine flags). The seeded
//! [`bootstrap_interval`] draws **only** through `crate::rng::Rng::next_index`;
//! there is no `std::time`, no thread RNG, no ambient entropy anywhere here.

/// Bessel-corrected (n−1) sample standard deviation. Private helper shared by
/// [`mean_se`] and [`bootstrap_interval`]. Returns 0.0 for n < 2 (variance
/// undefined for a single sample). Pure `+ − × ÷ √`.
fn bessel_std(samples: &[f64]) -> f64 {
    let n = samples.len();
    if n < 2 {
        return 0.0;
    }
    let nf = n as f64;
    let mean = samples.iter().sum::<f64>() / nf;
    let ss = samples.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>();
    (ss / (nf - 1.0)).sqrt()
}

/// Standard error of the mean: Bessel-corrected sample std ÷ √n.
///
/// **Assumes large-n, iid, finite-variance samples** — this is the CLT
/// precondition. For small-n binomial k/n use [`wilson_interval`]; for an
/// EMA-smoothed / autocorrelated stream do NOT use this (effective sample size
/// ≪ n → it badly under-estimates the error). Constant samples ⇒ 0.0; n < 2 ⇒ 0.0.
pub fn mean_se(samples: &[f64]) -> f64 {
    let n = samples.len();
    if n < 2 {
        return 0.0;
    }
    // se = std / √n  =  √( Σ(x−m)² / ((n−1)·n) ). One extra √; pure IEEE-754.
    bessel_std(samples) / (n as f64).sqrt()
}

/// Symmetric normal (Wald) interval `point ± z·se`.
///
/// **Assumes the estimator is approximately normal** (a sample mean at large n
/// via the CLT, or an already-computed SE). Do NOT feed it a small-n binomial
/// rate at p̂ near 0 or 1 — there [`wilson_interval`] is correct and the Wald
/// interval degenerates. `z` is the standard-normal quantile (1.96 ≈ 95%).
pub fn normal_interval(point: f64, se: f64, z: f64) -> (f64, f64) {
    (point - z * se, point + z * se)
}

/// Wilson score interval for a binomial proportion `k`/`n`.
///
/// The correct small-n binomial interval, and the one to reach for when `p̂` sits
/// at the boundary: at `p̂ = 1.0` the Wald interval collapses to `[1, 1]` (claiming
/// certainty from a handful of lucky trials — the self-certification failure in
/// miniature), whereas Wilson does not degenerate — for `k = n` it reduces to
/// `n/(n + z²)`, a lower bound strictly inside `(0, 1)`.
///
/// One `sqrt` over rationals ⇒ P6-clean cross-platform. Clopper-Pearson is the
/// exact alternative but needs an inverse-beta quantile — a transcendental the
/// `rng.rs:20-29` reproducibility doctrine flags as per-target-only — so it is
/// deliberately NOT shipped here. `n == 0` ⇒ the maximally-uncertain `(0.0, 1.0)`.
pub fn wilson_interval(k: u64, n: u64, z: f64) -> (f64, f64) {
    if n == 0 {
        return (0.0, 1.0);
    }
    let nf = n as f64;
    let p = k as f64 / nf;
    let z2 = z * z;
    let denom = 1.0 + z2 / nf;
    let center = (p + z2 / (2.0 * nf)) / denom;
    let margin = (z / denom) * (p * (1.0 - p) / nf + z2 / (4.0 * nf * nf)).sqrt();
    let lo = center - margin;
    let hi = center + margin;
    // Wilson is analytically within [0,1]; clamp only against float slop.
    (lo.max(0.0), hi.min(1.0))
}

/// The CLT/√N convergence envelope, relocated **byte-identically** from
/// `causal.rs`'s `empirical_converges_to_analytic_as_n_grows`:
///
/// ```text
/// error · √n  <  asymptotic_se · z
/// ```
///
/// `asymptotic_se` is the estimator's *true* asymptotic standard deviation of
/// `error·√n` (a domain-specific derivation the caller supplies — e.g. the
/// back-door ratio estimator's `se_factor`); `z` is the σ-multiple of the normal
/// envelope (the causal gate uses 6.0). **Assumes large n and finite variance**
/// — the normal envelope is exact-in-the-limit. For small-n binomial rates use
/// [`wilson_interval`] instead; for dependent streams see the module header.
///
/// Because this is the identical expression to the former inline predicate, a
/// test that swaps the inline compare for this call **cannot** drift numerically
/// — that byte-identity is what makes the substitution regression-safe.
pub fn within_clt_envelope(error: f64, n: usize, asymptotic_se: f64, z: f64) -> bool {
    error * (n as f64).sqrt() < asymptotic_se * z
}

/// Seeded percentile-free (normal-approximation) bootstrap interval for a
/// statistic with **no closed-form SE** — e.g. `aurc` (rank-cumulative) or any
/// estimator that is not a simple mean.
///
/// Draws `resamples` with-replacement resamples of `samples` **only** through
/// `rng.next_index` (P6-deterministic: same seed ⇒ same bytes, no `std::time`,
/// no thread RNG), computes `stat` on each, and returns
/// `normal_interval(stat(samples), SE_boot, z)` where `SE_boot` is the
/// Bessel-corrected std of the resample statistics. The point estimate is the
/// statistic on the *original* sample; the resamples estimate only its spread.
///
/// Cost is O(resamples × n). All realistic consumers here are small (aurc over an
/// eval-set of tens–hundreds, a `RegressionGate` window of a handful); the honest
/// ceiling is thousands-of-resamples × thousands-of-samples (≈10⁷–10⁸ ops) — not a
/// current consumer, but keep `resamples` sane (a few thousand) so the cost is
/// visible at the call site. Empty `samples` ⇒ `(0.0, 0.0)`.
pub fn bootstrap_interval(
    samples: &[f64],
    stat: impl Fn(&[f64]) -> f64,
    resamples: usize,
    z: f64,
    rng: &mut crate::rng::Rng,
) -> (f64, f64) {
    let n = samples.len();
    if n == 0 {
        return (0.0, 0.0);
    }
    let point = stat(samples);
    if resamples == 0 {
        return (point, point);
    }
    let mut replicates = Vec::with_capacity(resamples);
    let mut buf = vec![0.0f64; n];
    for _ in 0..resamples {
        for slot in buf.iter_mut() {
            *slot = samples[rng.next_index(n)];
        }
        replicates.push(stat(&buf));
    }
    let se_boot = bessel_std(&replicates);
    normal_interval(point, se_boot, z)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Rng;

    fn approx(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() < eps
    }

    // ── mean_se / normal_interval ────────────────────────────────────────────

    /// §3-step-2 oracle: SE of constant samples is 0 (zero variance). Exact for
    /// dyadic constants (mean reconstructs bit-exactly); for a non-dyadic constant
    /// (0.7) the reconstructed mean carries ~1e-17 of legitimate float rounding, so
    /// the SE is float-zero, not bit-zero — we assert that honestly rather than
    /// clamp it inside the primitive (which would hide real variance elsewhere).
    #[test]
    fn mean_se_of_constant_is_zero() {
        assert_eq!(mean_se(&[0.5; 8]), 0.0);
        assert_eq!(mean_se(&[2.0; 3]), 0.0);
        assert_eq!(mean_se(&[0.0; 16]), 0.0);
        // Non-dyadic constant: mathematically 0, float-zero to machine epsilon.
        assert!(
            mean_se(&[0.7; 8]) < 1e-15,
            "non-dyadic constant SE is float-zero"
        );
        // n < 2 is undefined → 0.0 by convention.
        assert_eq!(mean_se(&[42.0]), 0.0);
        assert_eq!(mean_se(&[]), 0.0);
    }

    /// Hand-derived SE oracle: samples [1,2,3,4,5] have Bessel std √2.5 and
    /// mean_se = √2.5/√5 = √0.5 ≈ 0.7071067811865476.
    #[test]
    fn mean_se_matches_hand_derivation() {
        let se = mean_se(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert!(approx(se, 0.5f64.sqrt(), 1e-15), "mean_se={se}");
    }

    #[test]
    fn normal_interval_is_symmetric_about_point() {
        let (lo, hi) = normal_interval(0.5, 0.1, 1.96);
        assert!(approx(lo, 0.5 - 0.196, 1e-15));
        assert!(approx(hi, 0.5 + 0.196, 1e-15));
        assert!(approx((lo + hi) / 2.0, 0.5, 1e-15));
    }

    // ── wilson_interval (§4 criteria 3 & 4, D4) ──────────────────────────────

    /// D4: the Wilson lower bounds are RECOMPUTED here, not copied from the
    /// blueprint. For k=n the closed form is n/(n+z²).
    #[test]
    fn wilson_lower_bound_for_full_success_matches_closed_form() {
        let z = 1.96;
        let z2 = z * z;

        let (lo12, hi12) = wilson_interval(12, 12, z);
        let closed12 = 12.0 / (12.0 + z2);
        assert!(approx(lo12, closed12, 1e-12), "12/12 lower={lo12}");
        assert!(
            approx(lo12, 0.7575, 1e-4),
            "12/12 lower ≈ 0.7575, got {lo12}"
        );
        assert!(approx(hi12, 1.0, 1e-12), "12/12 upper clamps to 1.0");

        let (lo29, _) = wilson_interval(29, 29, z);
        let closed29 = 29.0 / (29.0 + z2);
        assert!(approx(lo29, closed29, 1e-12), "29/29 lower={lo29}");
        assert!(
            approx(lo29, 0.8830, 1e-4),
            "29/29 lower ≈ 0.8830, got {lo29}"
        );
    }

    /// §4 criterion 4: Wilson does NOT degenerate to the Wald `[1,1]` at p̂=1.0.
    #[test]
    fn wilson_does_not_degenerate_at_boundary() {
        let (lo, hi) = wilson_interval(12, 12, 1.96);
        assert!(
            lo > 0.0 && lo < 1.0,
            "Wilson lower strictly inside (0,1): {lo}"
        );
        assert!(hi <= 1.0);
        // The Wald interval at p̂=1.0 is the degenerate [1,1]; Wilson must differ.
        let wald_lo = 1.0; // p̂ ± z·√(p̂(1-p̂)/n) with p̂=1 ⇒ ±0 ⇒ [1,1]
        assert!(
            lo < wald_lo,
            "Wilson {lo} must beat the degenerate Wald {wald_lo}"
        );
    }

    /// D4: a failing query (11/12) must MOVE the lower bound, not leave it at an
    /// assertable constant — the interval reacts to evidence.
    #[test]
    fn wilson_lower_bound_moves_when_a_query_fails() {
        let full = wilson_interval(12, 12, 1.96).0;
        let one_miss = wilson_interval(11, 12, 1.96).0;
        assert!(
            one_miss < full,
            "11/12 lower {one_miss} must drop below 12/12 {full}"
        );
    }

    #[test]
    fn wilson_zero_n_is_maximally_uncertain() {
        assert_eq!(wilson_interval(0, 0, 1.96), (0.0, 1.0));
    }

    // ── within_clt_envelope (§4 criterion 1, D2 falsifier) ───────────────────

    /// §4 criterion 1 / D2: the primitive is the BYTE-IDENTICAL expression to the
    /// former inline `error·√n < se·z`, over the exact causal sizes; and flipping
    /// the inequality flips the verdict (the falsifier that gives D2 teeth).
    #[test]
    fn within_clt_envelope_is_byte_identical_and_inversion_flips() {
        let se = 0.37;
        let z = 6.0;
        for &n in &[200usize, 2_000, 20_000, 200_000] {
            for &err in &[0.0, 1e-4, 1e-2, 5.0] {
                let inline = err * (n as f64).sqrt() < se * z;
                assert_eq!(
                    within_clt_envelope(err, n, se, z),
                    inline,
                    "n={n} err={err}"
                );
                // A deliberately inverted primitive must disagree wherever the
                // strict inequality is decisive (i.e. the two sides are unequal).
                if err * (n as f64).sqrt() != se * z {
                    let inverted = err * (n as f64).sqrt() >= se * z;
                    assert_ne!(
                        within_clt_envelope(err, n, se, z),
                        inverted,
                        "inverted predicate must flip the verdict (n={n} err={err})"
                    );
                }
            }
        }
    }

    // ── bootstrap_interval (§4 criterion 6, D5 determinism) ──────────────────

    /// A seeded bootstrap over a spread sample brackets the sample mean and has a
    /// positive width (the resamples see real variance).
    #[test]
    fn bootstrap_interval_brackets_the_mean() {
        let samples: Vec<f64> = (0..50).map(|i| i as f64).collect();
        let mut rng = Rng::new(0xB007, 1);
        let mean = |s: &[f64]| s.iter().sum::<f64>() / s.len() as f64;
        let point = mean(&samples);
        let (lo, hi) = bootstrap_interval(&samples, mean, 800, 1.96, &mut rng);
        assert!(
            lo < point && point < hi,
            "[{lo},{hi}] must bracket mean {point}"
        );
        assert!(
            hi - lo > 0.0,
            "bootstrap width must be positive on a spread sample"
        );
    }

    #[test]
    fn bootstrap_interval_edge_cases() {
        let mut rng = Rng::new(1, 1);
        let mean = |s: &[f64]| {
            if s.is_empty() {
                0.0
            } else {
                s.iter().sum::<f64>() / s.len() as f64
            }
        };
        assert_eq!(
            bootstrap_interval(&[], mean, 100, 1.96, &mut rng),
            (0.0, 0.0)
        );
        // Zero resamples ⇒ point ± 0.
        let (lo, hi) = bootstrap_interval(&[2.0, 4.0], mean, 0, 1.96, &mut rng);
        assert_eq!((lo, hi), (3.0, 3.0));
    }

    /// §4 criterion 6 / D5 (P6): a fixed-seed bootstrap reproduces bit-identically
    /// across a serialize→re-read→independently-fresh-recompute boundary — the
    /// exact audit-#19 shape `rng.rs:203` uses. No `std::time`/ambient entropy.
    #[test]
    fn bootstrap_interval_survives_serialize_reread_boundary() {
        let samples: Vec<f64> = (0..40).map(|i| (i % 7) as f64).collect();
        let stat = |s: &[f64]| s.iter().sum::<f64>() / s.len() as f64;
        let seed = 0x5EED_1234u64;

        let mut r = Rng::new(seed, 3);
        let (lo, hi) = bootstrap_interval(&samples, stat, 500, 1.96, &mut r);
        let serialized = format!("{lo}|{hi}");

        let path =
            std::env::temp_dir().join(format!("stats_boot_reread_{}.txt", std::process::id()));
        std::fs::write(&path, &serialized).expect("write serialized bootstrap interval");
        let reread = std::fs::read_to_string(&path).expect("re-read serialized interval");
        std::fs::remove_file(&path).ok();
        assert_eq!(
            reread, serialized,
            "byte content did not survive a disk round-trip"
        );

        // Independently fresh instance, same seed ⇒ identical interval.
        let mut fresh = Rng::new(seed, 3);
        let (lo2, hi2) = bootstrap_interval(&samples, stat, 500, 1.96, &mut fresh);
        assert_eq!(
            format!("{lo2}|{hi2}"),
            reread,
            "fresh recompute must match re-read bytes"
        );
    }
}
