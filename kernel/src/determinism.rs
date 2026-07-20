//! determinism.rs — Item 46 float-determinism containment goldens (ADR-046:
//! pin-under-golden, park the full fixed-point rewrite behind named triggers).
//!
//! Pure-std, zero-dep. These tests pin the EXACT IEEE-754 bit pattern
//! (`f64::to_bits()`) of every in-plane transcendental float site under the
//! pinned toolchain, so a cross-version / cross-host libm ULP drift turns the
//! always-on `cargo test` suite RED (the designed degradation of the item).
//!
//! In-plane sites (roadmap §I item 46 / blueprint scope i), each pinned below:
//!   * `attention.rs:33`  `(x - m).exp()`              → softmax (dynamics, never money)
//!   * `markov.rs:73`     `(1/tol).ln()/(1/slem).ln()` → `budget` mixing-time bound (advisory)
//!   * `markov.rs:186`    `p * p.log2()`               → entropy rate (advisory)
//!   * `spectral.rs:55`   `re.hypot(im)`               → `Complex::abs`, feeds
//!                           `spectral_radius` → `classify_drift` (the LIVE FSM
//!                           drift gate, `event_log.rs:425`) — a decision/replay surface
//!   * `spectral.rs:59`   `im.atan2(re)`               → `Complex::arg`, feeds `dominant_period`
//!
//! `token_bucket.rs:70-72` (`as_secs_f64()`, `refill_rate * elapsed`) is
//! `comparison-surface-exempt`: wall-clock-driven ⇒ inherently non-deterministic
//! by construction, never a replay/comparison surface (blueprint §6). No golden.
//!
//! See `docs/audits/determinism/FLOAT-SITES-2026-07-19.md` for the full
//! scope-(i)/(ii) inventory, the parked rewrite, and its two named reopening
//! triggers.
//!
//! NOTE (libm remains in the runtime): ADR-046 accepts that basic IEEE-754
//! arithmetic + these golden pins suffice for a fixed binary; the
//! fixed-point rewrite is parked behind the named triggers (inventory doc).

#[cfg(test)]
mod tests {
    use crate::markov;
    use crate::spectral::{classify_drift, Complex, DriftClass};

    /// `attention.rs:33` — `(x - m).exp()` inside `softmax`. The in-code claim at
    /// `attention.rs:13-15` (bit-reproducible across native/wasm32) is UNBACKED;
    /// this golden pins the exact bit pattern of the largest softmax weight for a
    /// fixed input, re-executed by the always-on suite.
    #[test]
    fn golden_attention_softmax_exp_attention_rs_33() {
        let w = crate::attention::softmax(&[1.0_f64, 2.0, 3.0]);
        // Largest weight (input 3.0) — exercises the `(x - m).exp()` site.
        assert_eq!(
            w[2].to_bits(),
            4604167177386354576, // pinned 2026-07-19 (toolchain-pinned)
            "attention.rs:33 softmax exp bit pattern drifted"
        );
    }

    /// `markov.rs:73` — mixing-time bound `budget = ln(1/tol)/ln(1/slem)`.
    /// Advisory metric; golden-pinned per ADR-046.
    #[test]
    fn golden_markov_budget_ln_markov_rs_73() {
        let b = markov::budget(0.9_f64, 1e-3);
        assert_eq!(
            b.to_bits(),
            4634314005443282009, // pinned 2026-07-19 (toolchain-pinned)
            "markov.rs:73 budget ln bit pattern drifted"
        );
    }

    /// `markov.rs:186` — Shannon entropy rate `p * p.log2()` (aggregate surface).
    /// Advisory metric; golden-pinned per ADR-046.
    #[test]
    fn golden_markov_entropy_log2_markov_rs_186() {
        let states: [&str; 8] = [
            "edit", "run_ok", "edit", "run_fail", "edit", "run_ok", "edit", "run_fail",
        ];
        let h = markov::analyze(&states).entropy_rate_bits;
        assert_eq!(
            h.to_bits(),
            4602641559526520590, // pinned 2026-07-19 (toolchain-pinned)
            "markov.rs:186 entropy log2 bit pattern drifted"
        );
    }

    /// `spectral.rs:55` — `Complex::abs()` = `re.hypot(im)`. Feeds
    /// `spectral_radius` → `classify_drift` (the live FSM drift gate). Golden.
    #[test]
    fn golden_spectral_complex_abs_hypot_spectral_rs_55() {
        let m = Complex::new(1.0_f64, 2.0).abs();
        assert_eq!(
            m.to_bits(),
            4612217596255138984, // pinned 2026-07-19 (toolchain-pinned)
            "spectral.rs:55 Complex::abs hypot bit pattern drifted"
        );
    }

    /// `spectral.rs:59` — `Complex::arg()` = `im.atan2(re)`. Feeds
    /// `dominant_period` (advisory period signal). Golden.
    #[test]
    fn golden_spectral_complex_arg_atan2_spectral_rs_59() {
        let a = Complex::new(-1.0_f64, 1.0).arg();
        assert_eq!(
            a.to_bits(),
            4612488097114038738, // pinned 2026-07-19 (toolchain-pinned)
            "spectral.rs:59 Complex::arg atan2 bit pattern drifted"
        );
    }

    /// `Complex::abs()` through the REAL decision path: `spectral_radius` of a
    /// known matrix drives `classify_drift` (the live gate). This pins the float
    /// that crosses the decision/replay boundary — the most safety-relevant
    /// in-plane site. Golden.
    #[test]
    fn golden_spectral_radius_through_drift_path() {
        // 2x2 matrix with eigenvalues ±1 (ρ = 1) ⇒ Resonant band.
        let m: Vec<Vec<f64>> = vec![vec![0.0_f64, 1.0], vec![1.0, 0.0]];
        let rho = crate::spectral::spectral_radius(&m);
        assert_eq!(
            rho.to_bits(),
            4607182418800017408, // pinned 2026-07-19 (toolchain-pinned)
            "spectral.rs:55 spectral_radius bit pattern drifted"
        );
        // The drift class that the gate consumes must be the integer-domain,
        // golden-backed decision (enum, comparison-surface safe).
        assert_eq!(classify_drift(&m), DriftClass::Resonant);
    }
}
