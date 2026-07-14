//! absorbing.rs — absorbing Markov chain closed forms. Reverse-engineering loop #R3.
//!
//! THE ONE EQUATION. The order lifecycle is an absorbing Markov chain: transient states
//! T = {Pending, Confirmed, Preparing, Ready, InDelivery} flow to absorbing terminals
//! {Delivered, PickedUp, Rejected, Cancelled}. Split the transition matrix P = [[Q, R],[0, I]];
//! then the **fundamental matrix** answers every funnel question in closed form —
//! `N = (I − Q)⁻¹`, `t = N·1` (expected steps-to-terminal), `B = N·R` (absorption probabilities).
//!
//! SELF-SIMPLIFICATION. Because the transient subgraph is a DAG, Q is **nilpotent** (Q^|T| = 0,
//! longest lifecycle path = 4 edges ⇒ Q⁵ = 0), so N = I + Q + Q² + … + Q^{|T|−1} is an EXACT finite
//! sum — no matrix inversion, no convergence tolerance. This one identity replaces *simulating* the
//! funnel: it is derivation, not description. If a `Reopen` edge ever makes the transient subgraph
//! cyclic, Q stops being nilpotent and `fundamental_matrix` **refuses** (returns `None`) rather than
//! return a wrong answer — the guard wired to the same acyclicity the FSM golden-signature pins.
//!
//! Pure, float (dynamics, never money), Verified-by-Math below.

use crate::mat::{matmul_contig, Mat};

/// Identity matrix of dimension `n` (backed by a contiguous `Mat`).
fn identity(n: usize) -> Vec<Vec<f64>> {
    Mat::identity(n).into_vecvec()
}

/// General dense matmul (handles square N·Q and rectangular N·R). Thin `&[Vec<f64>]`
/// wrapper over the single [`matmul_contig`] implementation.
fn matmul(a: &[Vec<f64>], b: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let ra = a.len();
    if ra == 0 {
        return vec![];
    }
    let am = Mat::from_vecvec(a);
    let bm = Mat::from_vecvec(b);
    matmul_contig(&am, &bm).into_vecvec()
}

fn is_zero_matrix(m: &[Vec<f64>]) -> bool {
    m.iter().all(|row| row.iter().all(|&x| x.abs() < 1e-12))
}

/// The fundamental matrix `N = (I − Q)⁻¹ = Σ_{k≥0} Q^k` of an absorbing chain's transient sub-block
/// `Q`. Computed as the finite Neumann series — EXACT and terminating iff `Q` is nilpotent (the
/// transient subgraph is a DAG). Returns `None` if `Q` is not nilpotent within `|T|` steps
/// (a transient cycle — the funnel is no longer a DAG; refuse rather than mislead).
pub fn fundamental_matrix(q: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = q.len();
    if n == 0 {
        return Some(vec![]);
    }
    let mut nmat = identity(n); // Q⁰ = I
    let mut term = identity(n);
    for _ in 0..n {
        term = matmul(&term, q); // term = Q^k
        if is_zero_matrix(&term) {
            return Some(nmat); // nilpotent ⇒ the finite sum is exact
        }
        for i in 0..n {
            for j in 0..n {
                nmat[i][j] += term[i][j];
            }
        }
    }
    None // Q^|T| ≠ 0 ⇒ transient cycle ⇒ refuse
}

/// Expected number of steps to absorption from each transient state: `t = N·1` (row sums of N).
pub fn expected_steps(nmat: &[Vec<f64>]) -> Vec<f64> {
    nmat.iter().map(|row| row.iter().sum()).collect()
}

/// Absorption probabilities `B = N·R`: `B[i][j]` = P(absorbed in terminal j | started in transient i).
/// Each row sums to 1 (absorption is certain for a DAG transient set).
pub fn absorption_probs(nmat: &[Vec<f64>], r: &[Vec<f64>]) -> Vec<Vec<f64>> {
    matmul(nmat, r)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    // ── GREEN (textbook): linear chain T0→T1→(absorb). N=(I−Q)⁻¹=[[1,1],[0,1]]. ──
    #[test]
    fn green_textbook_linear_chain() {
        let q = vec![vec![0.0, 1.0], vec![0.0, 0.0]];
        let r = vec![vec![0.0], vec![1.0]];
        let n = fundamental_matrix(&q).unwrap();
        assert_eq!(n, vec![vec![1.0, 1.0], vec![0.0, 1.0]]);
        assert_eq!(expected_steps(&n), vec![2.0, 1.0]); // 2 steps from T0, 1 from T1
        assert_eq!(absorption_probs(&n, &r), vec![vec![1.0], vec![1.0]]);
    }

    // The live order lifecycle with uniform branch probabilities.
    // T = [Pending, Confirmed, Preparing, Ready, InDelivery];
    // A = [Delivered, PickedUp, Rejected, Cancelled].
    fn lifecycle_qr() -> (Vec<Vec<f64>>, Vec<Vec<f64>>) {
        let t3 = 1.0 / 3.0;
        // Q: transient → transient
        let q = vec![
            vec![0.0, t3, 0.0, 0.0, 0.0], // Pending → Confirmed (1/3); Rejected/Cancelled absorb
            vec![0.0, 0.0, 0.5, 0.0, 0.5], // Confirmed → Preparing, InDelivery
            vec![0.0, 0.0, 0.0, 1.0, 0.0], // Preparing → Ready
            vec![0.0, 0.0, 0.0, 0.0, 0.5], // Ready → InDelivery (1/2); PickedUp absorbs
            vec![0.0, 0.0, 0.0, 0.0, 0.0], // InDelivery → Delivered absorbs
        ];
        // R: transient → absorbing
        let r = vec![
            vec![0.0, 0.0, t3, t3], // Pending → Rejected, Cancelled
            vec![0.0, 0.0, 0.0, 0.0],
            vec![0.0, 0.0, 0.0, 0.0],
            vec![0.0, 0.5, 0.0, 0.0], // Ready → PickedUp
            vec![1.0, 0.0, 0.0, 0.0], // InDelivery → Delivered
        ];
        (q, r)
    }

    // ── GREEN (the verified fixture): expected steps-to-terminal match the hand-derivation. ──
    #[test]
    fn green_order_lifecycle_expected_steps() {
        let (q, r) = lifecycle_qr();
        let n = fundamental_matrix(&q).expect("lifecycle transient set is a DAG");
        let t = expected_steps(&n);
        let want = [1.0 + 2.75 / 3.0, 2.75, 2.5, 1.5, 1.0]; // Pending = 1 + (1/3)·2.75 = 1.9166…
        for (i, w) in want.iter().enumerate() {
            assert!(approx(t[i], *w), "t[{i}] = {} want {}", t[i], w);
        }
        let _ = r;
    }

    // ── GREEN (property): (I − Q)·N = I, and every absorption row is a probability (sums to 1). ──
    #[test]
    fn green_fundamental_matrix_inverts_and_absorbs() {
        let (q, r) = lifecycle_qr();
        let n = fundamental_matrix(&q).unwrap();
        // (I − Q) · N = I
        let mut i_minus_q = identity(5);
        for i in 0..5 {
            for j in 0..5 {
                i_minus_q[i][j] -= q[i][j];
            }
        }
        let prod = matmul(&i_minus_q, &n);
        let ident = identity(5);
        for i in 0..5 {
            for j in 0..5 {
                assert!(approx(prod[i][j], ident[i][j]), "(I−Q)N ≠ I at [{i}][{j}]");
            }
        }
        // B rows sum to 1 (absorption is certain)
        let b = absorption_probs(&n, &r);
        for (i, row) in b.iter().enumerate() {
            assert!(
                approx(row.iter().sum::<f64>(), 1.0),
                "absorption row {i} must sum to 1"
            );
        }
    }

    // ── RED (falsifiability): a transient CYCLE is not nilpotent ⇒ refuse (no wrong N). ──
    #[test]
    fn red_transient_cycle_is_refused() {
        let cyclic_q = vec![vec![0.0, 1.0], vec![1.0, 0.0]]; // T0 ⇄ T1, never absorbed
        assert!(
            fundamental_matrix(&cyclic_q).is_none(),
            "a transient cycle breaks nilpotency ⇒ must refuse"
        );
    }
}
