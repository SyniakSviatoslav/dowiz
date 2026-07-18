//! noether.rs — conserved-quantity verifier (Master-Integration P9 / C-tier
//! "invariance note", made into a tested kernel organ).
//!
//! LENS: Noether's theorem (symmetry ⇒ conservation law) as an *executable
//! check* on a deterministic update. Given a state transition `f` and an
//! invariant `I`, verify that `I` is conserved along the trajectory:
//!   |I(f(x)) − I(x)| ≤ tol   for every step.
//! This is the growth-substrate guard the P9 self-improvement work needs: a
//! learned/online update (see `online`, `micrograd`) must NOT drift a quantity
//! that is supposed to be conserved (mass, energy of a preserved mode, a
//! Lyapunov bound). If the check fails on a tight tol, the update has an
//! asymmetry bug — caught here, deterministically, before it ships.
//!
//! DETERMINISTIC, zero deps, offline. Float tol is explicit; this is a dynamics
//! guard, never money.

/// Returns true iff the invariant `I` is conserved (within `tol`) at every
/// step of the trajectory starting at `x0` under `update`, for `steps` steps.
pub fn step_preserves<F, G>(x0: &[f64], update: F, invariant: G, steps: usize, tol: f64) -> bool
where
    F: Fn(&[f64]) -> Vec<f64>,
    G: Fn(&[f64]) -> f64,
{
    let mut x = x0.to_vec();
    let mut i_prev = invariant(&x);
    for _ in 0..steps {
        let x_next = update(&x);
        if x_next.len() != x.len() {
            return false; // dimension changed ⇒ not a valid state transition
        }
        let i_next = invariant(&x_next);
        if (i_next - i_prev).abs() > tol {
            return false;
        }
        x = x_next;
        i_prev = i_next;
    }
    true
}

/// Total variation of the invariant along the trajectory (for diagnostics).
pub fn invariant_drift<F, G>(x0: &[f64], update: F, invariant: G, steps: usize) -> f64
where
    F: Fn(&[f64]) -> Vec<f64>,
    G: Fn(&[f64]) -> f64,
{
    let mut x = x0.to_vec();
    let mut i_prev = invariant(&x);
    let mut total = 0.0f64;
    for _ in 0..steps {
        let x_next = update(&x);
        if x_next.len() != x.len() {
            return f64::INFINITY;
        }
        let i_next = invariant(&x_next);
        total += (i_next - i_prev).abs();
        x = x_next;
        i_prev = i_next;
    }
    total
}

/// One-sided **Lyapunov** check (BLUEPRINT-E1 §2b): the potential `V` is monotone
/// NON-INCREASING along the trajectory, within `tol` slack — `V(f(x)) − V(x) ≤ tol`
/// at every step. Returns `false` on the first step whose increase exceeds `tol`
/// (or on a dimension change ⇒ fail-closed).
///
/// This is the correct certificate for a **dissipative** scheme (damped wave /
/// gradient flow), where energy legitimately DECAYS and must never spontaneously
/// grow. `step_preserves` is two-sided (`|ΔI| ≤ tol`) and would wrongly forbid
/// that legitimate decay; this thin sibling accepts any decrease and only rejects
/// growth. Reuse `invariant_drift` for the reported total dissipation `Σ|ΔV|`.
pub fn lyapunov_nonincreasing<F, G>(
    x0: &[f64],
    update: F,
    potential: G,
    steps: usize,
    tol: f64,
) -> bool
where
    F: Fn(&[f64]) -> Vec<f64>,
    G: Fn(&[f64]) -> f64,
{
    let mut x = x0.to_vec();
    let mut v_prev = potential(&x);
    for _ in 0..steps {
        let x_next = update(&x);
        if x_next.len() != x.len() {
            return false; // dimension changed ⇒ not a valid state transition
        }
        let v_next = potential(&x_next);
        if v_next - v_prev > tol {
            return false; // spontaneous growth beyond slack ⇒ not a Lyapunov fn
        }
        x = x_next;
        v_prev = v_next;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// HAND ORACLE 1 — mass-conserving exchange. State (a,b); update moves a
    /// fraction ε from a to b. Invariant I = a+b is preserved to float
    /// precision (the two ±flow subtractions round independently, so tol=1e-9,
    /// not 0). The contrast with the Euler test proves the checker is not
    /// vacuous.
    #[test]
    fn conserves_mass() {
        let eps = 0.1;
        let update = move |x: &[f64]| {
            let flow = eps * (x[1] - x[0]);
            vec![x[0] + flow, x[1] - flow]
        };
        let mass = |x: &[f64]| x[0] + x[1];
        assert!(step_preserves(&[1.0, 3.0], update, mass, 100, 1e-9));
    }

    /// HAND ORACLE 2 — Euler on a harmonic oscillator GAINS energy, so a tight
    /// tol must FAIL (the checker catches the asymmetry / drift). With a loose
    /// tol it passes. This proves the verifier is not vacuous.
    #[test]
    fn catches_euler_energy_drift() {
        let dt = 0.05;
        // x' = v, v' = -x   (Euler, explicit)
        let euler = |x: &[f64]| vec![x[0] + dt * x[1], x[1] - dt * x[0]];
        let energy = |x: &[f64]| 0.5 * (x[0] * x[0] + x[1] * x[1]);
        // tight tol ⇒ drift detected
        assert!(!step_preserves(&[1.0, 0.0], euler, energy, 200, 1e-9));
        // loose tol ⇒ accepted (matches physics: slow growth)
        let drift = invariant_drift(&[1.0, 0.0], euler, energy, 200);
        assert!(drift > 0.0);
    }

    /// NON-VACUOUS proof for the one-sided Lyapunov checker (BLUEPRINT-E1 §4.3),
    /// mirroring `catches_euler_energy_drift` for `step_preserves`:
    ///   (a) explicit Euler on the oscillator GAINS energy ⇒ `false` (caught);
    ///   (b) mass-conserving exchange keeps V flat (ΔV=0 ≤ tol) ⇒ `true`;
    ///   (c) a strictly DISSIPATIVE (contracting) update makes V decrease every
    ///       step ⇒ `true` — the one-sided acceptance `step_preserves` (two-sided)
    ///       would wrongly REJECT, which is the whole reason this sibling exists.
    #[test]
    fn lyapunov_catches_growth_accepts_decay() {
        let dt = 0.05;
        let energy = |x: &[f64]| 0.5 * (x[0] * x[0] + x[1] * x[1]);

        // (a) explicit Euler gains energy ⇒ NOT non-increasing.
        let euler = |x: &[f64]| vec![x[0] + dt * x[1], x[1] - dt * x[0]];
        assert!(
            !lyapunov_nonincreasing(&[1.0, 0.0], euler, energy, 200, 1e-9),
            "explicit-Euler energy growth must be caught (false)"
        );

        // (b) mass-conserving exchange: I = a+b constant ⇒ ΔV = 0 ⇒ true.
        let eps = 0.1;
        let exchange = move |x: &[f64]| {
            let flow = eps * (x[1] - x[0]);
            vec![x[0] + flow, x[1] - flow]
        };
        let mass = |x: &[f64]| x[0] + x[1];
        assert!(lyapunov_nonincreasing(
            &[1.0, 3.0],
            exchange,
            mass,
            100,
            1e-9
        ));

        // (c) strict contraction x ↦ 0.9·x drives energy DOWN every step ⇒ true
        //     for the one-sided check, but two-sided `step_preserves` REJECTS it.
        let contract = |x: &[f64]| x.iter().map(|v| 0.9 * v).collect::<Vec<_>>();
        assert!(
            lyapunov_nonincreasing(&[2.0, -1.0], contract, energy, 50, 1e-12),
            "monotone energy DECAY must be accepted (true)"
        );
        assert!(
            !step_preserves(&[2.0, -1.0], contract, energy, 50, 1e-12),
            "two-sided step_preserves must REJECT the same legitimate decay"
        );
    }

    /// HAND ORACLE 3 — identity update preserves ANY invariant.
    #[test]
    fn identity_preserves_anything() {
        let id = |x: &[f64]| x.to_vec();
        let sq = |x: &[f64]| x.iter().map(|v| v * v).sum::<f64>();
        assert!(step_preserves(&[2.0, -3.0, 5.0], id, sq, 50, 0.0));
    }

    /// Dimension change ⇒ invalid transition ⇒ false (fail-closed).
    #[test]
    fn rejects_dim_change() {
        let shrink = |x: &[f64]| vec![x[0]];
        let inj = |x: &[f64]| x[0];
        assert!(!step_preserves(&[1.0, 2.0], shrink, inj, 1, 0.0));
    }

    /// Determinism: same inputs ⇒ same verdict.
    #[test]
    fn deterministic() {
        let eps = 0.2;
        let update = move |x: &[f64]| {
            let flow = eps * (x[1] - x[0]);
            vec![x[0] + flow, x[1] - flow]
        };
        let mass = |x: &[f64]| x[0] + x[1];
        let a = step_preserves(&[4.0, 1.0], &update, &mass, 30, 1e-12);
        let b = step_preserves(&[4.0, 1.0], &update, &mass, 30, 1e-12);
        assert_eq!(a, b);
    }
}
