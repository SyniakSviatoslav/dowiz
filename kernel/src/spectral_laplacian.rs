//! spectral_laplacian.rs — graph-Laplacian eigenmodes consumer of [`crate::spectral::eigh`].
//!
//! WAVE LAP: the *second* named consumer of the new symmetric eigensolver
//! (the first being `spectral_cache`/`markov`). The field-UI engine needs the
//! Laplacian eigenmodes `λ_k` for modal motion ([`crate::spectral`] module doc,
//! line 12: "the field-UI engine needs Laplacian eigenmodes (λ_k) for modal
//! motion"). This module is the canonical producer of that basis: the Fourier
//! modes of the graph (spectral embedding / field-UI basis).
//!
//! MATH (unnormalized combinatorial Laplacian)
//!   From a CSR adjacency `A` we build `L = D − A`, where `D` is the diagonal
//!   degree matrix (`D_ii = Σ_j A_ij`). This is the SAME convention used by
//!   [`crate::spectral::laplacian`], so the two are parity-bound. We pick the
//!   *unnormalized* `L = D − A` as the default (the blueprint/spectral.rs:12
//!   doc only names "Laplacian eigenmodes" with no normalization), and document
//!   the choice explicitly. `L` is symmetric-PSD for an undirected (symmetric)
//!   adjacency, so it is a legal input to the symmetric `eigh` engine.
//!
//! WHY NOT REIMPLEMENT
//!   We REUSE `crate::spectral::eigh` (the dense `n ≤ 32` Householder
//!   full-symmetric decomposition) — values ascending, orthonormal basis,
//!   sign-fixed ⇒ byte-deterministic. For `n > 32` the dense path is capped, so
//!   we fall back to the sparse tier `crate::spectral::topk_symmetric` (see
//!   [`laplacian_eigenmodes`] for the documented semantic caveat). No new
//!   eigensolver, no new deps.
//!
//! DETERMINISM
//!   Same CSR input ⇒ identical `f64` bits across calls/paths. Inherited from
//!   `eigh`/`topk_symmetric` (fixed seed, fixed summation order, sign-fixed
//!   basis). Pinned by [`tests::lap_modes_are_byte_deterministic`].

use crate::csr::Csr;
use crate::spectral_cache::Decomp;

/// Build the unnormalized combinatorial Laplacian `L = D − A` of a CSR graph as
/// a dense `n × n` (row-major) matrix.
///
/// `A` is materialized from the CSR via [`Csr::to_adjacency`]. For an undirected
/// (symmetric) adjacency `L` is symmetric-PSD; the caller is responsible for
/// passing a symmetric adjacency (i.e. both `(s,d,w)` and `(d,s,w)` for every
/// undirected edge). This matches the convention in
/// [`crate::spectral::laplacian`].
fn build_laplacian(csr: &Csr) -> Vec<Vec<f64>> {
    let a = csr.to_adjacency();
    let n = a.len();
    let mut l = vec![vec![0.0f64; n]; n];
    for i in 0..n {
        let deg: f64 = (0..n).map(|j| a[i][j]).sum();
        for j in 0..n {
            l[i][j] = if i == j { deg - a[i][j] } else { -a[i][j] };
        }
    }
    l
}

/// Laplacian eigenmodes of a graph — the `k` smallest-eigenvalue eigenpairs.
///
/// Builds the unnormalized combinatorial Laplacian `L = D − A` from the CSR
/// adjacency, then solves for its eigen-decomposition and returns the `k`
/// eigenpairs with the **smallest** eigenvalues (ascending order, so the first
/// `k`).
///
/// * `n ≤ 32` (dense path, the field-UI regime): routes through
///   [`crate::spectral::eigh`], which returns ascending eigenvalues with an
///   orthonormal, sign-fixed basis. The first `min(k, n)` pairs are the
///   smallest-magnitude modes — the Fourier modes / spectral-embedding basis.
/// * `n > 32` (sparse tier, documented fallback): `eigh`'s dense path is capped
///   at `n ≤ 32`, so we densify `L` and route through
///   [`crate::spectral::topk_symmetric`], which consumes the Laplacian as a CSR
///   and returns the **dominant** (largest-|λ|) `k` eigenpairs via Hotelling-
///   deflated power iteration. NOTE: at `n > 32` the result is therefore the
///   LARGEST-magnitude modes, NOT the smallest — a shift-invert solver (absent
///   in the kernel) would be required for the true smallest eigenvalues at
///   scale. The load-bearing tests exercise the `n ≤ 32` path.
///
/// Returns a [`Decomp`] `(basis, values)` where `basis[i]` is the unit
/// eigenvector for `values[i]`, `values` ascending, `basis` orthonormal
/// (`UᵀU = I`), byte-deterministic for equal input.
///
/// Never densifies-then-eighs in a way that exceeds the `n ≤ 32` cap: the dense
/// `eigh` is only called when `n ≤ 32`; larger graphs take the documented
/// `topk_symmetric` sparse branch.
pub fn laplacian_eigenmodes(csr: &Csr, k: usize) -> Decomp {
    let n = csr.nrows();
    let l = build_laplacian(csr);
    let kk = k.min(n);
    if n == 0 {
        return (Vec::new(), Vec::new());
    }
    if n <= 32 {
        // Dense symmetric path: eigh returns ascending eigenvalues; the first
        // kk are exactly the k smallest eigenmodes.
        let (basis, values) = crate::spectral::eigh(&l);
        (basis[..kk].to_vec(), values[..kk].to_vec())
    } else {
        // Sparse tier (n > 32): densify L and route through topk_symmetric.
        // Returns the dominant (largest-|λ|) kk eigenpairs — see module docs.
        let l_csr = Csr::from_dense(&l);
        crate::spectral::topk_symmetric(&l_csr, kk, 256)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    /// Undirected path graph P4: 0—1—2—3. Caller passes both edge directions so
    /// the adjacency is symmetric (required for a valid symmetric Laplacian).
    fn path_p4() -> Csr {
        Csr::from_edges(
            4,
            &[
                (0, 1, 1.0),
                (1, 0, 1.0),
                (1, 2, 1.0),
                (2, 1, 1.0),
                (2, 3, 1.0),
                (3, 2, 1.0),
            ],
        )
    }

    /// Undirected triangle K3 (complete graph on 3 nodes). Symmetric adjacency.
    fn triangle() -> Csr {
        Csr::from_edges(
            3,
            &[
                (0, 1, 1.0),
                (1, 0, 1.0),
                (1, 2, 1.0),
                (2, 1, 1.0),
                (2, 0, 1.0),
                (0, 2, 1.0),
            ],
        )
    }

    /// Closed-form Laplacian eigenvalues of a path graph P_n:
    /// `λ_k = 2 − 2·cos(π·k / n)`, `k = 0..n`. The known spectrum.
    fn path_spectrum(n: usize) -> Vec<f64> {
        (0..n)
            .map(|k| 2.0 - 2.0 * (PI * k as f64 / n as f64).cos())
            .collect()
    }

    /// Orthonormal basis check: UᵀU = I to `tol`. `basis[a]` is the a-th
    /// eigenvector (column of U); the Gram entry (a,b) = Σ_i basis[a][i]·basis[b][i].
    fn assert_orthonormal(basis: &[Vec<f64>], tol: f64) {
        let kk = basis.len();
        assert!(kk > 0, "orthonormality needs at least one vector");
        let n = basis[0].len();
        for a in 0..kk {
            assert_eq!(basis[a].len(), n, "all eigenvectors share length n");
            for b in 0..kk {
                let dot: f64 = (0..n).map(|i| basis[a][i] * basis[b][i]).sum();
                let expected = if a == b { 1.0 } else { 0.0 };
                assert!(
                    (dot - expected).abs() < tol,
                    "orthonormality violated at ({a},{b}): dot={dot}, expected {expected} (tol {tol})"
                );
            }
        }
    }

    /// Full known-spectrum check: every returned eigenvalue must match the
    /// closed-form target (within `tol`).
    fn assert_spectrum(values: &[f64], target: &[f64], tol: f64) {
        assert_eq!(
            values.len(),
            target.len(),
            "number of eigenpairs must equal spectrum size"
        );
        for (i, (got, want)) in values.iter().zip(target.iter()).enumerate() {
            assert!(
                (got - want).abs() < tol,
                "eigenvalue[{i}] = {got}, expected {want} (tol {tol})"
            );
        }
    }

    #[test]
    fn path_p4_known_spectrum_and_orthonormality() {
        let csr = path_p4();
        // Request all 4 modes (the full spectrum) to test orthonormality cleanly.
        let (basis, values) = laplacian_eigenmodes(&csr, 4);
        assert_eq!(values.len(), 4);
        // Known Laplacian spectrum of P4: 2 − 2cos(πk/4), k=0..3
        // = 0, 2−√2≈0.5858, 2, 2+√2≈3.4142.
        assert_spectrum(&values, &path_spectrum(4), 1e-6);
        // Basis is orthonormal (UᵀU = I to 1e-9).
        assert_orthonormal(&basis, 1e-9);
    }

    #[test]
    fn triangle_known_spectrum_and_orthonormality() {
        let csr = triangle();
        let (basis, values) = laplacian_eigenmodes(&csr, 3);
        assert_eq!(values.len(), 3);
        // K3 unnormalized Laplacian eigenvalues: 0, 3, 3.
        assert_spectrum(&values, &[0.0, 3.0, 3.0], 1e-6);
        assert_orthonormal(&basis, 1e-9);
    }

    #[test]
    fn k_smallest_subset_is_orthonormal() {
        // Requesting only k=2 smallest modes of P4 must still yield an
        // orthonormal 2×4 basis (the field-UI embedding subset).
        let csr = path_p4();
        let (basis, values) = laplacian_eigenmodes(&csr, 2);
        assert_eq!(values.len(), 2);
        // The two smallest P4 eigenvalues: 0 and 2−√2.
        assert_spectrum(&values, &path_spectrum(4)[..2], 1e-6);
        assert_orthonormal(&basis, 1e-9);
    }

    #[test]
    fn lap_modes_are_byte_deterministic() {
        // Same input ⇒ identical f64 bits across two independent calls.
        let csr = path_p4();
        let a = laplacian_eigenmodes(&csr, 4);
        let b = laplacian_eigenmodes(&csr, 4);
        assert_eq!(a.1.len(), b.1.len());
        for (x, y) in a.1.iter().zip(b.1.iter()) {
            assert_eq!(
                x.to_bits(),
                y.to_bits(),
                "eigenvalue bits differ across calls — not byte-deterministic"
            );
        }
        assert_eq!(a.0.len(), b.0.len());
        for (va, vb) in a.0.iter().zip(b.0.iter()) {
            assert_eq!(va.len(), vb.len());
            for (x, y) in va.iter().zip(vb.iter()) {
                assert_eq!(
                    x.to_bits(),
                    y.to_bits(),
                    "eigenvector bits differ across calls — not byte-deterministic"
                );
            }
        }
    }

    #[test]
    fn lap_modes_match_spectral_laplacian_parity() {
        // Our mode eigenvalues must agree with the existing `spectral::laplacian`
        // + `eigh` pipeline (parity with the kernel's own Laplacian convention).
        let csr = triangle();
        let adj = csr.to_adjacency();
        let l = crate::spectral::laplacian(&adj);
        let (_bl, vl) = crate::spectral::eigh(&l);
        let (_, vmodes) = laplacian_eigenmodes(&csr, 3);
        for (got, want) in vmodes.iter().zip(vl.iter()) {
            assert!(
                (got - want).abs() < 1e-9,
                "mode eigenvalue {got} disagrees with spectral::laplacian+eigh {want}"
            );
        }
    }
}
