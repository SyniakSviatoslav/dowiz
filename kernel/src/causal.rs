//! causal.rs — causal inference on the growth substrate (P9 research queue).
//!
//! Next item on the self-development spine (roadmap P9 / Wave queue §2): the
//! **do-operator / back-door adjustment** (Pearl, *Causality*). This is the
//! operator's stated PRIMARY FOCUS — reflection, metacognition, and the kernel
//! as a rigorous math substrate — so the next phase is not another product
//! feature but a *reasoning primitive* proven on the substrate.
//!
//! ## The back-door criterion
//!
//! To estimate the *causal* effect `P(Y | do(X))` from purely observational
//! data, when a confounder `Z` opens a back-door path `X ← Z → Y`, you adjust
//! for `Z`:
//!
//! ```text
//!     P(Y | do(X=x)) = Σ_z  P(Y=1 | X=x, Z=z) · P(Z=z)
//! ```
//!
//! This is provably the quantity a randomized controlled trial (randomizing
//! `X`, which severs `Z → X`) would measure. The *naive* conditional
//! `P(Y | X=x) = Σ_z P(Y|X=x,Z=z)·P(Z=z | X=x)` is **biased**: it conditions
//! on the open path through `Z` (selection on the confounder) and so
//! enumerates a spurious association. The back-door adjustment closes that
//! door.
//!
//! ## Verified-by-Math (no float fitting, no estimation)
//!
//! The caller supplies the conditional table `P(Y|X,Z)` and the confounder
//! marginal `P(Z)`; the module performs only the deterministic weighted sum.
//! Correctness is pinned by a hand-derived confounding example (see tests):
//! a beneficial treatment whose *observational* association is 6.6× overstated
//! because the health-conscious confounder both drives treatment uptake and
//! recovery. Back-door adjustment recovers the true +0.10 effect; the naive
//! collapse reports a phantom +0.66.
//!
//! Pure `f64`, deterministic, fail-closed on malformed tables (trust boundary).
//! Zero new dependencies.

/// Outcome of a back-door adjustment over a `k`-ary treatment `X`.
#[derive(Debug, Clone, PartialEq)]
pub struct CausalEffect {
    /// `do_p_y[x]` = `P(Y=1 | do(X=x))` — the *causal* quantity an RCT measures.
    pub do_p_y: Vec<f64>,
    /// `naive_p_y[x]` = `P(Y=1 | X=x)` collapsing over `Z` — the *biased*
    /// observational quantity, included so a caller can *measure* the bias the
    /// adjustment removes. `naive_p_y == do_p_y` iff no back-door confound exists.
    pub naive_p_y: Vec<f64>,
}

/// Back-door adjustment (Pearl's back-door criterion).
///
/// * `p_y_xz[idx]` with `idx = x_idx * n_z + z_idx` is `P(Y=1 | X=x_idx, Z=z_idx)`.
/// * `p_z[z_idx]` is `P(Z=z_idx)` — the confounder marginal.
/// * `p_xz[idx]` is `P(X=x_idx, Z=z_idx)` — the joint, used to compute the
///   *naive* (confounded) `P(Y | X)` by conditioning on `X`.
///
/// Returns `Err` (fail-closed) on any structural/trust-boundary violation:
/// empty treatment or confounder, length mismatch, a probability outside
/// `[0,1]`, or marginals that do not sum to 1.
pub fn backdoor_adjust(
    p_y_xz: &[f64],
    p_z: &[f64],
    p_xz: &[f64],
    n_x: usize,
    n_z: usize,
) -> Result<CausalEffect, &'static str> {
    if n_x == 0 || n_z == 0 {
        return Err("treatment (n_x) and confounder (n_z) must be non-empty");
    }
    if p_y_xz.len() != n_x * n_z {
        return Err("p_y_xz length must equal n_x * n_z");
    }
    if p_z.len() != n_z {
        return Err("p_z length must equal n_z");
    }
    if p_xz.len() != n_x * n_z {
        return Err("p_xz length must equal n_x * n_z");
    }
    for &p in p_y_xz.iter().chain(p_z).chain(p_xz) {
        if !(0.0..=1.0).contains(&p) {
            return Err("every probability must lie in [0,1]");
        }
    }
    if (p_z.iter().sum::<f64>() - 1.0).abs() > 1e-9 {
        return Err("p_z must sum to 1");
    }
    if (p_xz.iter().sum::<f64>() - 1.0).abs() > 1e-9 {
        return Err("p_xz must sum to 1");
    }

    let mut do_p_y = vec![0.0; n_x];
    let mut naive_p_y = vec![0.0; n_x];
    for xi in 0..n_x {
        // do(X=xi): Σ_z P(Y=1 | X=xi, Z=z) · P(Z=z)
        let mut do_sum = 0.0;
        // P(X=xi): needed to condition Z out of the naive estimate.
        let mut px = 0.0;
        for zi in 0..n_z {
            let idx = xi * n_z + zi;
            do_sum += p_y_xz[idx] * p_z[zi];
            px += p_xz[idx];
        }
        do_p_y[xi] = do_sum;
        if px.abs() < 1e-12 {
            return Err("treatment level has zero probability (degenerate)");
        }
        // naive: Σ_z P(Y=1 | X=xi, Z=z) · P(Z=z | X=xi)
        let mut naive = 0.0;
        for zi in 0..n_z {
            let idx = xi * n_z + zi;
            naive += p_y_xz[idx] * (p_xz[idx] / px);
        }
        naive_p_y[xi] = naive;
    }
    Ok(CausalEffect { do_p_y, naive_p_y })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    // Hand-derived confounding example (see module doc):
    //   Z (health-conscious) ~ Bernoulli(0.5)
    //   X (treatment) | Z :  P(X=1|Z=1)=0.9, P(X=1|Z=0)=0.1
    //   Y (recovery)  | X,Z:  (Z=1→)0.9/0.8, (Z=0→)0.2/0.1  for (X=1)/(X=0)
    // Implied joint P(X,Z): X0Z0=0.45 X0Z1=0.05 X1Z0=0.05 X1Z1=0.45.
    fn confounded() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        // idx = x*n_z + z  (n_x=2, n_z=2)
        let p_y_xz = vec![
            0.1, 0.8, // X=0: Z=0, Z=1
            0.2, 0.9, // X=1: Z=0, Z=1
        ];
        let p_z = vec![0.5, 0.5];
        let p_xz = vec![
            0.45, 0.05, // X=0: Z=0, Z=1
            0.05, 0.45, // X=1: Z=0, Z=1
        ];
        (p_y_xz, p_z, p_xz)
    }

    // ── GREEN: adjustment matches the hand derivation exactly ──
    #[test]
    fn green_backdoor_matches_hand_derivation() {
        let (py, pz, pxz) = confounded();
        let eff = backdoor_adjust(&py, &pz, &pxz, 2, 2).expect("valid tables");
        // do(X=0)=0.45, do(X=1)=0.55  (true causal effect +0.10)
        assert!(approx(eff.do_p_y[0], 0.45), "do(0)={}", eff.do_p_y[0]);
        assert!(approx(eff.do_p_y[1], 0.55), "do(1)={}", eff.do_p_y[1]);
        // naive(X=0)=0.17, naive(X=1)=0.83  (phantom +0.66 — confounded)
        assert!(approx(eff.naive_p_y[0], 0.17), "naive(0)={}", eff.naive_p_y[0]);
        assert!(approx(eff.naive_p_y[1], 0.83), "naive(1)={}", eff.naive_p_y[1]);
    }

    // ── GREEN: the adjustment REMOVES the confounding bias ──
    #[test]
    fn green_adjustment_removes_bias() {
        let (py, pz, pxz) = confounded();
        let eff = backdoor_adjust(&py, &pz, &pxz, 2, 2).unwrap();
        let causal_effect = eff.do_p_y[1] - eff.do_p_y[0]; // +0.10
        let phantom_effect = eff.naive_p_y[1] - eff.naive_p_y[0]; // +0.66
        // The confounder inflates the apparent effect >3× over the true causal effect.
        assert!(causal_effect > 0.0, "treatment is genuinely beneficial");
        assert!(
            phantom_effect / causal_effect > 3.0,
            "confounding overstates effect {}×",
            phantom_effect / causal_effect
        );
        // The adjustment narrows the apparent gap back to the true causal gap.
        assert!((phantom_effect - causal_effect).abs() > 0.5);
    }

    // ── GREEN: with NO confounder, do == naive (adjustment is a no-op identity) ──
    #[test]
    fn green_no_confounder_adjustment_is_identity() {
        // Z ⟂ X: build p_xz = p_x ⊗ p_z so the back-door is already closed.
        let px = [0.5, 0.5];
        let pz = [0.5, 0.5];
        let mut p_xz = vec![0.0; 4];
        let mut p_y_xz = vec![0.0; 4];
        for xi in 0..2 {
            for zi in 0..2 {
                let idx = xi * 2 + zi;
                p_xz[idx] = px[xi] * pz[zi];
                // outcome depends on X alone (no Z path)
                p_y_xz[idx] = if xi == 1 { 0.7 } else { 0.3 };
            }
        }
        let eff = backdoor_adjust(&p_y_xz, &pz, &p_xz, 2, 2).unwrap();
        for xi in 0..2 {
            assert!(
                approx(eff.do_p_y[xi], eff.naive_p_y[xi]),
                "do==naive when Z ⟂ X at x={xi}"
            );
        }
        assert!(approx(eff.do_p_y[1], 0.7));
        assert!(approx(eff.do_p_y[0], 0.3));
    }

    // ── RED (trust boundary): malformed tables must fail-closed, never panic ──
    #[test]
    fn red_empty_or_malformed_is_rejected() {
        let (py, pz, pxz) = confounded();
        assert!(backdoor_adjust(&py, &pz, &pxz, 0, 2).is_err()); // empty treatment
        assert!(backdoor_adjust(&py, &pz, &pxz, 2, 0).is_err()); // empty confounder
        // probability out of range
        let mut bad_py = py.clone();
        bad_py[0] = 1.4;
        assert!(backdoor_adjust(&bad_py, &pz, &pxz, 2, 2).is_err());
        // confounder marginal not summing to 1
        let mut bad_pz = pz.clone();
        bad_pz[0] = 0.4; // sums to 0.9
        assert!(backdoor_adjust(&py, &bad_pz, &pxz, 2, 2).is_err());
        // joint not summing to 1
        let mut bad_pxz = pxz.clone();
        bad_pxz[0] += 0.1; // sums to 1.1
        assert!(backdoor_adjust(&py, &pz, &bad_pxz, 2, 2).is_err());
    }
}
