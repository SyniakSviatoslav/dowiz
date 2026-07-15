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

/// Front-door adjustment (Pearl's front-door criterion).
///
/// Used when the confounder `U` of `X` and `Y` is **unobserved** (so back-door
/// adjustment is impossible) but `X` affects `Y` *only through* a mediator `M`,
/// and `M` is itself unconfounded with `X`. Then `P(Y | do(X))` is identified as
///
/// ```text
///     P(Y | do(X=x)) = Σ_m  P(M=m | X=x) · Σ_x'  P(Y=1 | M=m, X=x') · P(X=x')
/// ```
///
/// The inner sum weights each level of `X` by its base rate, so the *direct*
/// `X → Y` edge is correctly integrated and the unobserved `U → X, U → Y`
/// back-door is bypassed entirely through `M`.
///
/// * `p_m_x[idx]` with `idx = x_idx * n_m + m_idx` is `P(M=m | X=x)`.
/// * `p_y_mx[idx]` is `P(Y=1 | M=m, X=x)`.
/// * `p_x[x_idx]` is `P(X=x)` — the treatment marginal.
///
/// Fail-closed on the same trust-boundary violations as [`backdoor_adjust`]:
/// empty dims, length/shape mismatch, probabilities outside `[0,1]`, `M|X` rows
/// or `P(X)` not summing to 1.
pub fn frontdoor_adjust(
    p_m_x: &[f64],
    p_y_mx: &[f64],
    p_x: &[f64],
    n_x: usize,
    n_m: usize,
) -> Result<CausalEffect, &'static str> {
    if n_x == 0 || n_m == 0 {
        return Err("treatment (n_x) and mediator (n_m) must be non-empty");
    }
    if p_m_x.len() != n_x * n_m {
        return Err("p_m_x length must equal n_x * n_m");
    }
    if p_y_mx.len() != n_x * n_m {
        return Err("p_y_mx length must equal n_x * n_m");
    }
    if p_x.len() != n_x {
        return Err("p_x length must equal n_x");
    }
    for &p in p_m_x.iter().chain(p_y_mx).chain(p_x) {
        if !(0.0..=1.0).contains(&p) {
            return Err("every probability must lie in [0,1]");
        }
    }
    if (p_x.iter().sum::<f64>() - 1.0).abs() > 1e-9 {
        return Err("p_x must sum to 1");
    }
    // Each P(M | X=x) row must itself be a distribution.
    for xi in 0..n_x {
        let row_sum: f64 = (0..n_m).map(|mi| p_m_x[xi * n_m + mi]).sum();
        if (row_sum - 1.0).abs() > 1e-9 {
            return Err("every P(M | X=x) row must sum to 1");
        }
    }

    let mut do_p_y = vec![0.0; n_x];
    let mut naive_p_y = vec![0.0; n_x];
    for xi in 0..n_x {
        // do(X=xi): Σ_m P(M=m|X=xi) · [ Σ_x' P(Y=1|M=m,X=x')·P(X=x') ]
        let mut do_sum = 0.0;
        // naive: Σ_m P(Y=1|M=m,X=xi)·P(M=m|X=xi)
        let mut naive = 0.0;
        for mi in 0..n_m {
            let pmx = p_m_x[xi * n_m + mi];
            // inner sum over x' for the mediation distribution of Y at (M=m)
            let mut inner = 0.0;
            for xp in 0..n_x {
                inner += p_y_mx[xp * n_m + mi] * p_x[xp];
            }
            do_sum += pmx * inner;
            naive += p_y_mx[xi * n_m + mi] * pmx;
        }
        do_p_y[xi] = do_sum;
        naive_p_y[xi] = naive;
    }
    Ok(CausalEffect { do_p_y, naive_p_y })
}

/// Instrumental-variable (Wald) estimation of the causal effect of `X` on `Y`.
///
/// Used when **no back-door set is observable** and there is no mediator, but a
/// valid instrument `Z` exists: `Z → X → Y`, with `Z`'s *only* path to `Y` going
/// through `X` (`Z ⊥ Y` given `X`), and `Z` does shift `X`. Then the Local Average
/// Treatment Effect (constant-effect / monotonicity) is the **Wald estimand**
///
/// ```text
///     β = (E[Y | Z=1] − E[Y | Z=0]) / (P(X=1 | Z=1) − P(X=1 | Z=0))
/// ```
///
/// giving `do(X=1) − do(X=0) = β`. The unadjusted (observational) `E[Y | X]` is
/// passed in separately as `naive_x*` — it may be *spuriously* large when an
/// unobserved `U` confounds `X,Y`, while the IV estimate is immune to `U`.
///
/// * `px_z1` = `P(X=1 | Z=1)`, `px_z0` = `P(X=1 | Z=0)`.
/// * `ey_z1` = `E[Y | Z=1]`, `ey_z0` = `E[Y | Z=0]`.
/// * `naive_x1` = `E[Y | X=1]`, `naive_x0` = `E[Y | X=0]` (observational, possibly confounded).
///
/// Fail-closed: probabilities in `[0,1]`, `E[Y]` in `[0,1]`, and — critically —
/// the instrument must move `X` (`px_z1 != px_z0`); an instrument that does not
/// shift `X` makes `β` undefined and is rejected (`Err`).
pub fn instrumental_adjust(
    px_z1: f64,
    px_z0: f64,
    ey_z1: f64,
    ey_z0: f64,
    naive_x1: f64,
    naive_x0: f64,
) -> Result<CausalEffect, &'static str> {
    for p in [px_z1, px_z0, ey_z1, ey_z0, naive_x1, naive_x0] {
        if !(0.0..=1.0).contains(&p) {
            return Err("every probability / expectation must lie in [0,1]");
        }
    }
    // The instrument must actually shift X, else the Wald denominator is 0 (and
    // Z was never a valid instrument).
    if (px_z1 - px_z0).abs() < 1e-12 {
        return Err("instrument Z must shift X (px_z1 != px_z0)");
    }
    let beta = (ey_z1 - ey_z0) / (px_z1 - px_z0); // Wald estimand
    let base = ey_z1 - beta * px_z1; // E[Y | do(X=0)]
    let do_p_y = vec![base, base + beta];
    let naive_p_y = vec![naive_x0, naive_x1];
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

    // ── Front-door fixtures (Pearl): X→M→Y, unobserved U confounds X,Y ──
    // Valid front-door model: Y⊥X | M (no direct X→Y edge), P(X) symmetric.
    //   p_m_x: X0:(M0,M1)=(0.5,0.5)  X1:(0.1,0.9)
    //   p_y_mx: M0=0.2, M1=0.7   (Y depends on M only)
    // Hand sum: do(X=0)=0.5·0.2+0.5·0.7=0.45 ; do(X=1)=0.1·0.2+0.9·0.7=0.65
    fn frontdoor_fixture() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let p_m_x = vec![
            0.5, 0.5, // X=0: M=0, M=1
            0.1, 0.9, // X=1: M=0, M=1
        ];
        let p_y_mx = vec![
            0.2, 0.7, // X=0: M=0, M=1
            0.2, 0.7, // X=1: M=0, M=1
        ];
        let p_x = vec![0.5, 0.5];
        (p_m_x, p_y_mx, p_x)
    }

    // ── GREEN: front-door matches the hand-derived oracle exactly ──
    #[test]
    fn green_frontdoor_matches_hand_derivation() {
        let (pmx, pymx, px) = frontdoor_fixture();
        let eff = frontdoor_adjust(&pmx, &pymx, &px, 2, 2).expect("valid tables");
        assert!(approx(eff.do_p_y[0], 0.45), "do(0)={}", eff.do_p_y[0]);
        assert!(approx(eff.do_p_y[1], 0.65), "do(1)={}", eff.do_p_y[1]);
        // With no direct X→Y edge (Y⊥X|M), the front-door do coincides with the
        // naive conditional — the identifier is internally consistent.
        assert!(approx(eff.do_p_y[0], eff.naive_p_y[0]), "do==naive at X=0 (Y⊥X|M)");
        assert!(approx(eff.do_p_y[1], eff.naive_p_y[1]), "do==naive at X=1 (Y⊥X|M)");
    }

    // ── GREEN: the mediator M is actually used (not a pass-through) ──
    #[test]
    fn green_frontdoor_routes_through_mediator() {
        let (pmx, mut pymx, px) = frontdoor_fixture();
        // Flip the outcome-on-mediator map: M1 now BAD (0.2), M0 GOOD (0.7).
        pymx = vec![
            0.7, 0.2, // X=0: M=0, M=1
            0.7, 0.2, // X=1: M=0, M=1
        ];
        let eff = frontdoor_adjust(&pmx, &pymx, &px, 2, 2).unwrap();
        // do(X=1)=0.1·0.7+0.9·0.2=0.25 ; do(X=0)=0.5·0.7+0.5·0.2=0.45
        assert!(approx(eff.do_p_y[1], 0.25), "do(1) must track M=1's new outcome");
        assert!(approx(eff.do_p_y[0], 0.45), "do(0) unchanged (M distn unchanged)");
        // A pass-through (ignoring M) would have reported 0.65/0.45 — it didn't.
        assert!(!approx(eff.do_p_y[1], 0.65), "implementation must NOT skip M");
    }

    // ── GREEN: no X→M edge ⇒ no causal effect of X on Y ──
    #[test]
    fn green_frontdoor_no_x_to_m_means_no_effect() {
        let mut pmx = vec![0.5, 0.5, 0.5, 0.5]; // P(M|X) constant
        let pymx = vec![0.2, 0.7, 0.2, 0.7];
        let px = vec![0.5, 0.5];
        let eff = frontdoor_adjust(&pmx, &pymx, &px, 2, 2).unwrap();
        // do(X=0)=do(X=1)=0.5·0.2+0.5·0.7=0.45 ⇒ causal effect is exactly 0.
        assert!(approx(eff.do_p_y[0], eff.do_p_y[1]), "no X→M ⇒ identical do(X)");
        assert!(approx(eff.do_p_y[0], 0.45));
    }

    // ── RED (trust boundary): malformed mediator tables fail-closed ──
    #[test]
    fn red_frontdoor_malformed_is_rejected() {
        let (pmx, pymx, px) = frontdoor_fixture();
        // P(M|X) row not summing to 1
        let mut bad_pmx = pmx.clone();
        bad_pmx[0] += 0.1;
        assert!(frontdoor_adjust(&bad_pmx, &pymx, &px, 2, 2).is_err());
        // treatment marginal not summing to 1
        let mut bad_px = px.clone();
        bad_px[0] = 0.4;
        assert!(frontdoor_adjust(&pmx, &pymx, &bad_px, 2, 2).is_err());
        // probability out of range
        let mut bad_y = pymx.clone();
        bad_y[0] = 1.5;
        assert!(frontdoor_adjust(&pmx, &bad_y, &px, 2, 2).is_err());
        // empty mediator
        assert!(frontdoor_adjust(&pmx, &pymx, &px, 2, 0).is_err());
    }

    // ── Instrumental-variable (Wald) fixtures ──
    // Valid instrument Z (e.g. random assignment): Z shifts X, only path Z→X→Y.
    //   P(X=1|Z=1)=0.9  P(X=1|Z=0)=0.1   (instrument moves X strongly)
    //   E[Y|Z=1]=0.55    E[Y|Z=0]=0.25
    // Wald β = (0.55-0.25)/(0.9-0.1) = 0.30/0.80 = 0.375
    //   do(X=0) = 0.55 - 0.375*0.9 = 0.2125 ; do(X=1) = 0.5875
    // Observational (confounded) E[Y|X=1]=0.7, E[Y|X=0]=0.3 => naive effect 0.40
    // (slightly inflated vs the deconfounded 0.375 — U biases the naive estimate).
    fn iv_fixture() -> (f64, f64, f64, f64, f64, f64) {
        (0.9, 0.1, 0.55, 0.25, 0.7, 0.3)
    }

    // ── GREEN: Wald estimand matches the hand-derived value ──
    #[test]
    fn green_instrumental_matches_wald_hand_derivation() {
        let (px1, px0, ey1, ey0, nx1, nx0) = iv_fixture();
        let eff = instrumental_adjust(px1, px0, ey1, ey0, nx1, nx0).expect("valid instrument");
        // do(X=0)=0.2125, do(X=1)=0.5875, causal effect = 0.375
        assert!(approx(eff.do_p_y[0], 0.2125), "do(0)={}", eff.do_p_y[0]);
        assert!(approx(eff.do_p_y[1], 0.5875), "do(1)={}", eff.do_p_y[1]);
        assert!(approx(eff.do_p_y[1] - eff.do_p_y[0], 0.375), "Wald β=0.375");
        // naive (confounded) effect is reported alongside and differs
        assert!(approx(eff.naive_p_y[1] - eff.naive_p_y[0], 0.40), "naive effect 0.40");
        assert!(eff.naive_p_y[1] - eff.naive_p_y[0] > eff.do_p_y[1] - eff.do_p_y[0],
            "unobserved U inflates the naive effect over the IV estimate");
    }

    // ── GREEN: the instrument's strength changes β (not a constant) ──
    #[test]
    fn green_instrumental_uses_instrument_strength() {
        let (_, _, ey1, ey0, nx1, nx0) = iv_fixture();
        // Weaker instrument: P(X=1|Z=1)=0.6, P(X=1|Z=0)=0.4 => denom 0.2
        let eff = instrumental_adjust(0.6, 0.4, ey1, ey0, nx1, nx0).unwrap();
        // β = 0.30/0.20 = 1.5 — but clamped by base? base = ey1 - β*px1 = 0.55 - 1.5*0.6 = -0.35
        // do(X=0) = -0.35, do(X=1) = 1.15 — these exceed [0,1] because a unit LATE
        // of 1.5 is impossible for a binary Y; this is the known Wald limit (it
        // assumes a *constant* effect, violated here). The test asserts the
        // arithmetic is faithful, NOT that it stays in [0,1].
        assert!(approx(eff.do_p_y[1] - eff.do_p_y[0], 1.5), "weak IV => larger β by formula");
        assert!(!approx(eff.do_p_y[1] - eff.do_p_y[0], 0.375), "β tracks instrument strength");
    }

    // ── RED (trust boundary): a non-instrument (Z does not shift X) is rejected ──
    #[test]
    fn red_instrument_must_shift_x() {
        let (_, _, ey1, ey0, nx1, nx0) = iv_fixture();
        // P(X|Z=1) == P(X|Z=0) => Z never moves X => not a valid instrument
        assert!(instrumental_adjust(0.5, 0.5, ey1, ey0, nx1, nx0).is_err(),
            "instrument that does not shift X must be rejected");
    }

    // ── RED (trust boundary): out-of-range inputs rejected ──
    #[test]
    fn red_instrumental_malformed_is_rejected() {
        let (px1, px0, ey1, ey0, nx1, nx0) = iv_fixture();
        assert!(instrumental_adjust(1.4, px0, ey1, ey0, nx1, nx0).is_err(), "px_z1 in [0,1]");
        assert!(instrumental_adjust(px1, px0, 1.2, ey0, nx1, nx0).is_err(), "ey_z1 in [0,1]");
        assert!(instrumental_adjust(px1, px0, ey1, ey0, -0.1, nx0).is_err(), "naive in [0,1]");
    }
}
