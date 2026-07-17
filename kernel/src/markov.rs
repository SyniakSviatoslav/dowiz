//! markov.rs — the Markov-chain attractor detector for the self-improvement loop.
//!
//! REVERSE-ENGINEERING LOOP #R1. ASCENDed from `tools/loop-signals/markov_attractor.py`
//! (287 LOC Python on the agent's own hot path). It reuses `crate::spectral` for the
//! eigen-core — **killing the dual-authority hazard**: the Faddeev-LeVerrier + Durand-Kerner
//! eigensolver previously existed byte-for-byte in both Python and `spectral.rs` with no parity
//! gate between them. There is now ONE eigensolver, in the kernel, and the Python's own frozen
//! 12-case test corpus is reproduced below as VbM parity tests (`green_parity_*`).
//!
//! MODEL (Jurafsky & Martin SLP3 App. A). The recent tool-outcome stream is a first-order Markov
//! chain: states Q, empirical transition matrix Â (row-normalised counts), damped stationary π.
//! Deterministic signals derived from Â:
//!   * entropy rate  H = −Σᵢ πᵢ Σⱼ Âᵢⱼ log₂ Âᵢⱼ  (0 = deterministic cycle),
//!   * escape mass   Σ_{s∈ESCAPE} π_s  (long-run time in a progress state),
//!   * spectrum      slem = |λ₂| + a PERIOD signal (an eigenvalue near the unit circle away
//!                   from +1 = a real oscillation, incl. μ≈−1 period-2),
//!   * NEW continuous dial: gap γ = 1−slem ⇒ mixing time τ≈1/γ and iteration budget
//!                   k ≈ ln(1/tol)/ln(1/slem) — the master dial the research identified.
//! ADVISORY (the harness gates decide). Float is fine — this is dynamics, never money.

use crate::spectral;

const MIN_EVENTS: usize = 8; // short window ⇒ stay quiet (cold start)
const H_LO: f64 = 0.5; // bits/step; rows >~75% deterministic ⇒ "cyclic"
const ESCAPE_LO: f64 = 0.05; // <5% long-run time in a progress state ⇒ no escape
const DAMPING: f64 = 0.02; // PageRank teleport ⇒ irreducible+aperiodic ⇒ unique π
const POWER_ITERS: usize = 300; // fixed; the damped chain contracts fast for small n

/// Progress potential V (Foster-Lyapunov). Only `run_ok` (a verify/progress command) escapes.
fn potential(s: &str) -> f64 {
    match s {
        "run_ok" => 1.0,
        "edit_fail" | "run_fail" => -1.0,
        _ => 0.0, // edit, probe, unknown — neutral, never an escape state
    }
}
fn is_escape(s: &str) -> bool {
    s == "run_ok"
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Quiet work or progress reachable — no intervention.
    Healthy,
    /// Trapped AND (low entropy OR a spectral oscillation) — a cycle across ≥1 signatures.
    LimitCycle,
    /// Trapped, high-entropy churn, no clean period — busy going nowhere.
    StrangeAttractor,
}

#[derive(Debug, Clone)]
pub struct Report {
    pub verdict: Verdict,
    pub events: usize,
    pub entropy_rate_bits: f64,
    pub escape_mass: f64,
    pub drift: f64,
    pub has_failure: bool,
    pub slem: f64,
    pub period: bool,
    /// NEW — spectral gap γ = 1 − |λ₂|. γ→0 ⇒ never mixes (trapped).
    pub gap: f64,
    /// NEW — mixing time τ ≈ 1/γ (∞ for a non-mixing cycle).
    pub mixing_time: f64,
}

/// Iteration budget: how many steps a power method needs to reach `tol` at this mixing rate —
/// `k ≈ ln(1/tol)/ln(1/slem)`. A spectrum-derived replacement for a fixed retry cap.
pub fn budget(slem: f64, tol: f64) -> f64 {
    if slem <= 0.0 || slem >= 1.0 || tol <= 0.0 {
        return f64::INFINITY;
    }
    (1.0 / tol).ln() / (1.0 / slem).ln()
}

fn idx_of(alpha: &[&str], s: &str) -> usize {
    alpha.iter().position(|&a| a == s).unwrap()
}

/// Full analysis output — the exact JSON contract `markov_attractor.py` emitted
/// (`json.dumps(analyze(toks))`), so shell consumers (`check.sh`) and parity
/// tests keep working once the Python is deleted. The kernel is the single
/// source of truth; the CLI bin serialises this to JSON.
pub struct DetailedReport {
    pub report: Report,
    /// Sorted unique state alphabet (matches Python `alphabet`).
    pub alphabet: Vec<String>,
    /// Eigenvalues of Â as (re, im), sorted by modulus descending (Python `eigs`).
    pub eigs: Vec<(f64, f64)>,
    /// Stationary π keyed by state name (Python `stationary`).
    pub stationary: Vec<(String, f64)>,
    /// Human-readable reason string (Python `reason`).
    pub reason: String,
}

impl DetailedReport {
    /// Verdict as the Python's upper-case string ("HEALTHY" / "LIMIT_CYCLE" / …).
    pub fn verdict_str(&self) -> &'static str {
        match self.report.verdict {
            Verdict::Healthy => "HEALTHY",
            Verdict::LimitCycle => "LIMIT_CYCLE",
            Verdict::StrangeAttractor => "STRANGE_ATTRACTOR",
        }
    }
}

/// Analyse a window of tool-outcome tokens, returning the FULL report (verdict +
/// metrics + spectrum + stationary + reason). Pure. Fail-open (short window ⇒
/// HEALTHY with empty spectral fields).
pub fn analyze_detailed(states: &[&str]) -> DetailedReport {
    let states: Vec<&str> = states.iter().copied().filter(|s| !s.is_empty()).collect();
    let l = states.len();
    let cold = DetailedReport {
        report: Report {
            verdict: Verdict::Healthy,
            events: l,
            entropy_rate_bits: 0.0,
            escape_mass: 0.0,
            drift: 0.0,
            has_failure: false,
            slem: 0.0,
            period: false,
            gap: 1.0,
            mixing_time: 1.0,
        },
        alphabet: Vec::new(),
        eigs: Vec::new(),
        stationary: Vec::new(),
        reason: "window too short".to_string(),
    };
    if l < MIN_EVENTS {
        return cold;
    }

    // alphabet = sorted unique
    let mut alpha: Vec<&str> = states.clone();
    alpha.sort_unstable();
    alpha.dedup();
    let n = alpha.len();

    // bigram counts → row-normalised transition matrix Â (unseen row ⇒ uniform)
    let mut counts = vec![vec![0.0f64; n]; n];
    for t in 0..l - 1 {
        counts[idx_of(&alpha, states[t])][idx_of(&alpha, states[t + 1])] += 1.0;
    }
    let mut a = vec![vec![0.0f64; n]; n];
    for i in 0..n {
        let s: f64 = counts[i].iter().sum();
        for j in 0..n {
            a[i][j] = if s > 0.0 {
                counts[i][j] / s
            } else {
                1.0 / n as f64
            };
        }
    }

    // damped stationary π = πÂ′, Â′ = (1−d)Â + (d/n)J, via power iteration
    let d = DAMPING;
    let mut pi = vec![1.0 / n as f64; n];
    for _ in 0..POWER_ITERS {
        let mut nxt = vec![0.0f64; n];
        for i in 0..n {
            let pii = pi[i];
            if pii == 0.0 {
                continue;
            }
            for j in 0..n {
                nxt[j] += pii * ((1.0 - d) * a[i][j] + d / n as f64);
            }
        }
        let sum: f64 = nxt.iter().sum::<f64>().max(f64::MIN_POSITIVE);
        for v in nxt.iter_mut() {
            *v /= sum;
        }
        pi = nxt;
    }

    // entropy rate, escape mass, Foster drift (drift reported, deliberately NOT gated)
    let mut h = 0.0;
    for i in 0..n {
        let mut row_h = 0.0;
        for j in 0..n {
            let p = a[i][j];
            if p > 0.0 {
                row_h -= p * p.log2();
            }
        }
        h += pi[i] * row_h;
    }
    let escape: f64 = (0..n).filter(|&i| is_escape(alpha[i])).map(|i| pi[i]).sum();
    let mut drift = 0.0;
    for i in 0..n {
        let vi = potential(alpha[i]);
        let step: f64 = (0..n).map(|j| a[i][j] * (potential(alpha[j]) - vi)).sum();
        drift += pi[i] * step;
    }

    // spectrum — the SHARED kernel eigensolver (no more Python duplicate).
    // Route `slem` through the content-addressed DecompCache so a re-analysis
    // of an UNCHANGED transition matrix reuses the prior eigen-decomposition
    // instead of re-running Faddeev-LeVerrier + Durand-Kerner. Keyed on a
    // deterministic hash of the matrix contents (markov has no store handle;
    // the cache is honest & content-addressed without one).
    // TODO(operator): lift the cache to a longer-lived `&mut` (caller-owned) so
    // it survives across `analyze_detailed` calls; here it warms/reuses within
    // a single call and exercises the primitive end-to-end.
    let mut decomp_cache = crate::spectral_cache::DecompCache::new();
    let a_tile = crate::csr::NormalizedTile::from_dense(&a);
    let slem = crate::spectral_cache::slem_cached(&mut decomp_cache, &a_tile);
    let period = spectral::dominant_period(&a).is_some();
    let gap = 1.0 - slem;
    let mixing_time = if gap > 1e-12 {
        1.0 / gap
    } else {
        f64::INFINITY
    };

    // a trap requires EVIDENCE OF STRUGGLE — at least one failure in the window.
    let has_failure = states.iter().any(|&s| s == "run_fail" || s == "edit_fail");
    let trapped = escape <= ESCAPE_LO && has_failure;

    let verdict = if trapped && (h <= H_LO || period) {
        Verdict::LimitCycle
    } else if trapped && h > H_LO {
        Verdict::StrangeAttractor
    } else {
        Verdict::Healthy // not struggling, or progress reachable
    };

    let reason = match verdict {
        Verdict::LimitCycle => format!(
            "cyclic trap: escape={escape:.3} H={h:.3} period={period} slem={slem:.3}"
        ),
        Verdict::StrangeAttractor => format!(
            "bounded high-entropy churn never reaching progress: escape={escape:.3} H={h:.3} slem={slem:.3}"
        ),
        Verdict::Healthy if !has_failure => {
            format!("quiet work, no failures in window (escape={escape:.3} H={h:.3})")
        }
        Verdict::Healthy => {
            format!("progress reachable: escape={escape:.3} drift={drift:+.3} H={h:.3}")
        }
    };

    // eigenvalues (modulus-desc), stationary keyed by alphabet
    let mut eigs: Vec<(f64, f64)> = spectral::eigenvalues(&a)
        .iter()
        .map(|c| (c.re, c.im))
        .collect();
    eigs.sort_by(|a, b| {
        (b.0 * b.0 + b.1 * b.1)
            .partial_cmp(&(a.0 * a.0 + a.1 * a.1))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let stationary = alpha
        .iter()
        .enumerate()
        .map(|(i, s)| (s.to_string(), pi[i]))
        .collect();

    DetailedReport {
        report: Report {
            verdict,
            events: l,
            entropy_rate_bits: h,
            escape_mass: escape,
            drift,
            has_failure,
            slem,
            period,
            gap,
            mixing_time,
        },
        alphabet: alpha.iter().map(|s| s.to_string()).collect(),
        eigs,
        stationary,
        reason,
    }
}

/// Analyse a window of tool-outcome tokens. Pure. Fail-open (short window ⇒ HEALTHY).
/// Thin view over [`analyze_detailed`] — the kernel's single analysis path.
pub fn analyze(states: &[&str]) -> Report {
    analyze_detailed(states).report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rep<'a>(p: &[&'a str], times: usize) -> Vec<&'a str> {
        let mut v = Vec::new();
        for _ in 0..times {
            v.extend_from_slice(p);
        }
        v
    }

    /// Port of the Python `lcg_walk` — identical arithmetic so the SAME token sequence is fed.
    fn lcg_walk(alphabet: &[&'static str], n: usize, seed: u64) -> Vec<&'static str> {
        let mut x = seed;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            x = (1103515245u64.wrapping_mul(x).wrapping_add(12345)) & 0x7FFF_FFFF;
            out.push(alphabet[((x >> 16) as usize) % alphabet.len()]);
        }
        out
    }

    // ── PARITY with the frozen 12-case Python corpus (test_markov_attractor.py) ──
    #[test]
    fn green_parity_healthy_rhythm() {
        // low entropy BUT high escape — the case naive "alternation == stuck" gets wrong.
        assert_eq!(
            analyze(&rep(&["edit", "run_ok"], 8)).verdict,
            Verdict::Healthy
        );
    }
    #[test]
    fn green_parity_healthy_varied_progress() {
        let s = [
            "edit", "run_fail", "edit", "run_ok", "edit", "run_ok", "run_fail", "edit", "run_ok",
            "edit", "run_ok",
        ];
        assert_eq!(analyze(&s).verdict, Verdict::Healthy);
    }
    #[test]
    fn green_parity_limit_cycle_thrash() {
        assert_eq!(
            analyze(&rep(&["edit", "run_fail"], 8)).verdict,
            Verdict::LimitCycle
        );
    }
    #[test]
    fn green_parity_limit_cycle_3cycle() {
        assert_eq!(
            analyze(&rep(&["edit", "run_fail", "edit_fail"], 5)).verdict,
            Verdict::LimitCycle
        );
    }
    #[test]
    fn green_parity_strange_attractor_churn() {
        let s = lcg_walk(&["edit", "edit_fail", "run_fail"], 40, 1);
        assert_eq!(analyze(&s).verdict, Verdict::StrangeAttractor);
    }
    #[test]
    fn green_parity_strange_churn_robust_across_seeds() {
        for seed in [1u64, 2, 3, 7, 11] {
            let s = lcg_walk(&["edit", "edit_fail", "run_fail", "probe"], 44, seed);
            let r = analyze(&s);
            assert_eq!(r.verdict, Verdict::StrangeAttractor, "seed {seed}");
            assert!(
                r.escape_mass == 0.0 && r.has_failure,
                "seed {seed} must be a genuine trap"
            );
        }
    }
    #[test]
    fn green_parity_cold_start_short_window() {
        assert_eq!(
            analyze(&["edit", "run_fail", "edit"]).verdict,
            Verdict::Healthy
        );
    }
    #[test]
    fn green_parity_unblinded_probe_cycle() {
        let r = analyze(&rep(&["edit", "run_fail", "probe"], 5));
        assert_eq!(r.verdict, Verdict::LimitCycle);
        assert_eq!(
            r.escape_mass, 0.0,
            "probe must NOT count as escape/progress"
        );
    }
    #[test]
    fn green_parity_spectral_only_bipartite_star() {
        // H > H_LO so the entropy path is SILENT; only the spectral λ≈−1 period fires.
        let star = rep(
            &["edit", "run_fail", "edit", "edit_fail", "edit", "probe"],
            4,
        );
        let r = analyze(&star);
        assert_eq!(r.verdict, Verdict::LimitCycle);
        assert!(
            r.entropy_rate_bits > H_LO,
            "entropy path must be silent (H>H_LO)"
        );
        assert!(
            r.period,
            "spectral period signal (λ≈−1) must fire — the eigen-core cross-check"
        );
        assert_eq!(r.escape_mass, 0.0);
    }
    #[test]
    fn green_parity_wrapup_no_failures_healthy() {
        let r = analyze(&[
            "probe", "edit", "probe", "edit", "edit", "edit", "edit", "edit",
        ]);
        assert_eq!(r.verdict, Verdict::Healthy);
        assert!(!r.has_failure, "no failure states ⇒ not a trap");
    }

    // ── spectral separation proof (same asserts as the Python bottom block) ──
    #[test]
    fn green_spectral_separation_lc_vs_sa() {
        let lc = analyze(&rep(&["edit", "run_fail"], 8));
        let sa = analyze(&lcg_walk(&["edit", "edit_fail", "run_fail"], 40, 1));
        assert!(
            lc.entropy_rate_bits <= H_LO && H_LO < sa.entropy_rate_bits,
            "entropy separates"
        );
        assert!(
            lc.escape_mass <= ESCAPE_LO && sa.escape_mass <= ESCAPE_LO,
            "both trapped"
        );
        assert!(lc.period, "a clean 2-cycle shows the period signal");
        assert!(lc.slem >= 0.95, "a 2-cycle has |λ₂|≈1 (poorly mixing)");
        assert!(
            sa.slem < lc.slem,
            "churn mixes faster (smaller |λ₂|) than a clean cycle"
        );
    }

    // ── NEW continuous dial (γ / τ / budget) — VbM ──
    #[test]
    fn green_gap_and_mixing_time() {
        let lc = analyze(&rep(&["edit", "run_fail"], 8)); // 2-cycle ⇒ slem≈1 ⇒ γ≈0 ⇒ τ→∞
        assert!(lc.gap < 0.05, "a trap has a vanishing spectral gap");
        assert!(lc.mixing_time > 20.0, "and a large mixing time");
        let sa = analyze(&lcg_walk(&["edit", "edit_fail", "run_fail"], 40, 1)); // mixes ⇒ γ>0
        assert!(
            sa.gap > lc.gap,
            "churn mixes ⇒ larger gap than a clean cycle"
        );
    }
    #[test]
    fn green_budget_monotone_in_slem() {
        // poorer mixing (larger slem) ⇒ larger iteration budget.
        assert!(budget(0.9, 1e-3) > budget(0.5, 1e-3));
        assert!(budget(0.99, 1e-3).is_finite());
        assert_eq!(budget(1.0, 1e-3), f64::INFINITY);
    }

    // ── T3/A5 markov regression pin: |slem_new − slem_old| ≤ 1e-9 ────────────
    /// Rebuild the exact row-normalised transition matrix Â that `analyze_detailed`
    /// feeds to `spectral_cache::slem_cached`, from a state sequence, so we can
    /// compare the cached SLEM against the raw `spectral::slem` baseline.
    fn transition_matrix(states: &[&str]) -> Vec<Vec<f64>> {
        let states: Vec<&str> = states.iter().copied().filter(|s| !s.is_empty()).collect();
        let l = states.len();
        let mut alpha: Vec<&str> = states.clone();
        alpha.sort_unstable();
        alpha.dedup();
        let n = alpha.len();
        let idx_of = |alpha: &[&str], s: &str| alpha.iter().position(|&x| x == s).unwrap();
        let mut counts = vec![vec![0.0f64; n]; n];
        for t in 0..l.saturating_sub(1) {
            counts[idx_of(&alpha, states[t])][idx_of(&alpha, states[t + 1])] += 1.0;
        }
        let mut a = vec![vec![0.0f64; n]; n];
        for i in 0..n {
            let s: f64 = counts[i].iter().sum();
            for j in 0..n {
                a[i][j] = if s > 0.0 {
                    counts[i][j] / s
                } else {
                    1.0 / n as f64
                };
            }
        }
        a
    }

    /// REGRESSION (T3/A5): the canonical-key change in `slem_cached` must NOT
    /// move the markov SLEM by more than ULPs (division + remultiplication by the
    /// pivot). Pin `|slem_new − slem_old| ≤ 1e-9` over the existing fixture corpus.
    #[test]
    fn a5_slem_cached_matches_raw_slem_within_1e9() {
        // The parity corpus (each yields a distinct transition matrix Â).
        let cases: Vec<Vec<&str>> = vec![
            rep(&["edit", "run_ok"], 8),
            rep(&["edit", "run_fail"], 8),
            rep(&["edit", "run_fail", "edit_fail"], 5),
            lcg_walk(&["edit", "edit_fail", "run_fail"], 40, 1),
            lcg_walk(&["edit", "edit_fail", "run_fail", "probe"], 44, 7),
            ["edit", "run_fail", "edit"].to_vec(),
            rep(&["edit", "run_ok"], 3),
        ];
        for (k, c) in cases.iter().enumerate() {
            let a = transition_matrix(c);
            if a.len() < 2 {
                continue; // too short to have a λ₂
            }
            let slem_old = crate::spectral::slem(&a);
            let mut cache = crate::spectral_cache::DecompCache::new();
            let a_tile = crate::csr::NormalizedTile::from_dense(&a);
            let slem_new = crate::spectral_cache::slem_cached(&mut cache, &a_tile);
            assert!(
                (slem_new - slem_old).abs() <= 1e-9,
                "case {k}: |slem_new({slem_new}) − slem_old({slem_old})| > 1e-9"
            );
            // And the cache must be honest: a repeat on the SAME matrix hits.
            let slem_repeat = crate::spectral_cache::slem_cached(&mut cache, &a_tile);
            assert_eq!(
                cache.recomputes(),
                0,
                "case {k}: same matrix ⇒ zero recomputes (cache honest)"
            );
            assert!(
                (slem_repeat - slem_new).abs() < 1e-15,
                "case {k}: repeat must be bit-identical to first solve"
            );
        }
    }
}
