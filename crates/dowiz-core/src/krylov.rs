//! krylov.rs — Krylov subspace methods for the no_std core.
//!
//! Implements the standard Krylov machinery (AI-tools compendium, Part IV.4):
//! - [`arnoldi`] — orthonormal basis V + Hessenberg H of K_m(A, v) (general A).
//! - [`lanczos`] — symmetric tridiagonalization α/β of K_m(A, v).
//! - [`cg`] — conjugate-gradient solver for SPD systems.
//! - [`gmres`] — restarted GMRES for general (non-symmetric) systems via
//!   Arnoldi + Givens-rotation least squares.
//!
//! All operations are `no_std`; dot products reuse [`crate::householder::dot`]
//! and norms reuse the bit-exact [`crate::math::sqrt`]. Matrices are dense
//! row-major `&[Vec<f64>]` (matching the `spectral` module convention).

use crate::householder::dot;
use alloc::vec::Vec;

/// Dense row-major matrix-vector product y = A·x.
fn matvec(a: &[Vec<f64>], x: &[f64]) -> Vec<f64> {
    a.iter().map(|row| dot(row, x)).collect()
}

/// Euclidean norm ‖x‖.
fn norm(x: &[f64]) -> f64 {
    crate::math::sqrt(dot(x, x))
}

/// axpy: y += α·x (in place).
fn axpy(y: &mut [f64], alpha: f64, x: &[f64]) {
    for (yi, xi) in y.iter_mut().zip(x.iter()) {
        *yi += alpha * xi;
    }
}

/// x ← α·x (in place).
fn scale(x: &mut [f64], alpha: f64) {
    for xi in x.iter_mut() {
        *xi *= alpha;
    }
}

/// r = b − A·x (residual of a dense linear system).
fn residual(a: &[Vec<f64>], b: &[f64], x: &[f64]) -> Vec<f64> {
    let ax = matvec(a, x);
    b.iter().zip(ax.iter()).map(|(bi, axi)| bi - axi).collect()
}

/// Outcome of an iterative solver run.
#[derive(Debug, Clone, PartialEq)]
pub struct SolveOutcome {
    pub x: Vec<f64>,
    pub iterations: usize,
    pub converged: bool,
    pub residual: f64,
}

/// Conjugate gradient for SPD systems Ax = b.
///
/// `tol` is the relative residual threshold ‖r‖/‖b‖; iteration stops early on
/// a happy breakdown (exact solution reached).
pub fn cg(a: &[Vec<f64>], b: &[f64], x0: &[f64], tol: f64, max_iter: usize) -> SolveOutcome {
    let n = b.len();
    let mut x = x0.to_vec();
    let mut r = residual(a, b, &x);
    let bnorm = norm(b);
    let mut p = r.clone();
    let mut rsold = dot(&r, &r);

    for it in 0..max_iter {
        let ap = matvec(a, &p);
        let pap = dot(&p, &ap);
        if pap == 0.0 {
            break;
        }
        let alpha = rsold / pap;
        axpy(&mut x, alpha, &p);
        axpy(&mut r, -alpha, &ap);
        let rsnew = dot(&r, &r);
        if crate::math::sqrt(rsnew) <= tol * bnorm.max(1.0) {
            return SolveOutcome {
                x,
                iterations: it + 1,
                converged: true,
                residual: crate::math::sqrt(rsnew),
            };
        }
        let beta = rsnew / rsold;
        // p = r + beta·p
        for i in 0..n {
            p[i] = r[i] + beta * p[i];
        }
        rsold = rsnew;
    }

    let res = norm(&residual(a, b, &x));
    SolveOutcome {
        converged: res <= tol * bnorm.max(1.0),
        residual: res,
        x,
        iterations: max_iter,
    }
}

/// Arnoldi process: orthonormal basis V of K_{m+1}(A, v) and Hessenberg H.
///
/// Returns `(V, H)` where `V` has up to `m + 1` orthonormal columns and `H` is
/// `(m + 1) × m` upper Hessenberg (with a happy-breakdown shortcut). Uses
/// modified Gram–Schmidt for numerical stability.
pub fn arnoldi(a: &[Vec<f64>], v1: &[f64], m: usize) -> (Vec<Vec<f64>>, Vec<Vec<f64>>) {
    let n = v1.len();
    let beta = norm(v1);
    let mut v: Vec<Vec<f64>> = Vec::new();
    if beta > 0.0 {
        v.push(v1.iter().map(|xi| xi / beta).collect());
    } else {
        v.push(vec![0.0; n]);
    }
    let mut h: Vec<Vec<f64>> = Vec::with_capacity(m + 1);
    for _ in 0..=m {
        h.push(vec![0.0; m]);
    }

    for j in 0..m {
        let mut w = matvec(a, &v[j]);
        for i in 0..=j {
            h[i][j] = dot(&v[i], &w);
            axpy(&mut w, -h[i][j], &v[i]);
        }
        let hjj1 = norm(&w);
        h[j + 1][j] = hjj1;
        if hjj1 > 1e-14 {
            scale(&mut w, 1.0 / hjj1);
            v.push(w);
        } else {
            break; // happy breakdown
        }
    }
    (v, h)
}

/// Lanczos tridiagonalization for a symmetric matrix: returns `(alpha, beta)`.
///
/// `alpha` is the diagonal and `beta` the sub-diagonal of the tridiagonal
/// projection T_m = VᵀAV. Only the upper triangle of `a` is used.
pub fn lanczos(a: &[Vec<f64>], v1: &[f64], m: usize) -> (Vec<f64>, Vec<f64>) {
    let n = v1.len();
    let b0 = norm(v1);
    let mut alpha = Vec::with_capacity(m);
    let mut beta: Vec<f64> = Vec::with_capacity(m);

    let mut v_prev: Vec<f64> = vec![0.0; n];
    let mut v: Vec<f64> = if b0 > 0.0 {
        v1.iter().map(|xi| xi / b0).collect()
    } else {
        vec![0.0; n]
    };
    let mut b_cur = b0;

    for j in 0..m {
        let mut w = matvec(a, &v);
        let aj = dot(&v, &w);
        alpha.push(aj);
        // w = w − α_j·v − β_j·v_prev
        axpy(&mut w, -aj, &v);
        axpy(&mut w, -b_cur, &v_prev);
        let b_next = norm(&w);
        if b_next <= 1e-14 {
            break; // happy breakdown — tridiagonalization complete
        }
        beta.push(b_next);
        scale(&mut w, 1.0 / b_next);
        v_prev = v;
        v = w;
        b_cur = b_next;
    }
    (alpha, beta)
}

/// Restarted GMRES for general (non-symmetric) systems Ax = b.
///
/// Each restart builds a Krylov basis of size `restart` via [`arnoldi`] and
/// solves the small least-squares problem with Givens rotations.
pub fn gmres(
    a: &[Vec<f64>],
    b: &[f64],
    x0: &[f64],
    restart: usize,
    tol: f64,
    max_iter: usize,
) -> SolveOutcome {
    let n = b.len();
    let bnorm = norm(b).max(1.0);
    let mut x = x0.to_vec();

    for _ in 0..max_iter.div_ceil(restart).max(1) {
        let r = residual(a, b, &x);
        let beta = norm(&r);
        if beta <= tol * bnorm {
            return SolveOutcome {
                x,
                iterations: 0,
                converged: true,
                residual: beta,
            };
        }
        let v1: Vec<f64> = r.iter().map(|ri| ri / beta).collect();
        let (v, h) = arnoldi(a, &v1, restart);
        // Krylov dimension: `v.len()` is restart+1 without a breakdown, or
        // j+1 after a happy breakdown (where the space is A-invariant and the
        // least-squares collapses to a square solve).
        let nrows = v.len();
        let k = nrows.min(restart);
        if k == 0 {
            break;
        }

        // Solve min ‖H y − β e1‖ with Givens rotations. The Hessenberg block
        // has `nrows` rows and `k` columns (restart+1 × restart, or k × k on
        // breakdown).
        let mut g: Vec<f64> = vec![0.0; nrows];
        g[0] = beta;
        let mut hb: Vec<Vec<f64>> = h[..nrows].iter().map(|row| row[..k].to_vec()).collect();
        for i in 0..k.min(nrows - 1) {
            let (c, s) = givens(hb[i][i], hb[i + 1][i]);
            for j in i..k {
                let (a_ij, a_i1j) = rotate(c, s, hb[i][j], hb[i + 1][j]);
                hb[i][j] = a_ij;
                hb[i + 1][j] = a_i1j;
            }
            let (gi, gi1) = rotate(c, s, g[i], g[i + 1]);
            g[i] = gi;
            g[i + 1] = gi1;
        }
        // Back-substitute y from the upper-triangular R (first k rows).
        let mut y = vec![0.0; k];
        for i in (0..k).rev() {
            let mut s = g[i];
            for j in (i + 1)..k {
                s -= hb[i][j] * y[j];
            }
            if hb[i][i] != 0.0 {
                y[i] = s / hb[i][i];
            }
        }
        // x += V_k y
        for j in 0..k {
            axpy(&mut x, y[j], &v[j]);
        }

        let res = norm(&residual(a, b, &x));
        if res <= tol * bnorm {
            return SolveOutcome {
                x,
                iterations: 0,
                converged: true,
                residual: res,
            };
        }
    }

    let res = norm(&residual(a, b, &x));
    SolveOutcome {
        x,
        iterations: max_iter,
        converged: res <= tol * bnorm,
        residual: res,
    }
}

/// Givens rotation (c, s) that zeroes the second component of (a, b).
fn givens(a: f64, b: f64) -> (f64, f64) {
    if b == 0.0 {
        return (1.0, 0.0);
    }
    if a.abs() >= b.abs() {
        let t = b / a;
        let c = 1.0 / crate::math::sqrt(1.0 + t * t);
        (c, c * t)
    } else {
        let t = a / b;
        let s = 1.0 / crate::math::sqrt(1.0 + t * t);
        (s * t, s)
    }
}

/// Apply a Givens rotation [c, -s; s, c] to (x, y).
fn rotate(c: f64, s: f64, x: f64, y: f64) -> (f64, f64) {
    (c * x + s * y, -s * x + c * y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    fn close_vec(a: &[f64], b: &[f64]) -> bool {
        a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| close(*x, *y))
    }

    /// 2×2 SPD matrix [[4,1],[1,3]].
    fn spd2() -> Vec<Vec<f64>> {
        vec![vec![4.0, 1.0], vec![1.0, 3.0]]
    }

    #[test]
    fn cg_solves_spd_system() {
        let a = spd2();
        let b = vec![1.0, 2.0];
        let out = cg(&a, &b, &[0.0, 0.0], 1e-12, 50);
        assert!(out.converged);
        // exact: solve [4x+y=1, x+3y=2] -> x=1/11, y=7/11
        assert!(close_vec(&out.x, &[1.0 / 11.0, 7.0 / 11.0]));
    }

    #[test]
    fn cg_solves_1d_laplacian() {
        // tridiagonal SPD Laplacian 5×5 (diag 2, off-diag −1).
        let n = 5;
        let mut a = vec![vec![0.0; n]; n];
        for i in 0..n {
            a[i][i] = 2.0;
            if i > 0 {
                a[i][i - 1] = -1.0;
            }
            if i + 1 < n {
                a[i][i + 1] = -1.0;
            }
        }
        let b: Vec<f64> = (1..=n).map(|k| k as f64).collect();
        let out = cg(&a, &b, &vec![0.0; n], 1e-12, 100);
        assert!(out.converged);
        // check residual is small
        assert!(out.residual < 1e-10);
    }

    #[test]
    fn arnoldi_basis_is_orthonormal() {
        // non-symmetric 3×3
        let a = vec![
            vec![2.0, 1.0, 0.0],
            vec![0.0, 2.0, 1.0],
            vec![1.0, 0.0, 2.0],
        ];
        let v1 = vec![1.0, 0.0, 0.0];
        let (v, h) = arnoldi(&a, &v1, 2);
        assert!(v.len() >= 3);
        // columns orthonormal
        for i in 0..v.len() {
            for j in 0..v.len() {
                let ip = dot(&v[i], &v[j]);
                if i == j {
                    assert!(close(ip, 1.0), "v[{i}] norm != 1: {ip}");
                } else {
                    assert!(close(ip, 0.0), "v[{i}]·v[{j}] != 0: {ip}");
                }
            }
        }
        // H is upper Hessenberg: h[i][j] == 0 for i > j+1
        for j in 0..2 {
            for i in (j + 2)..h.len() {
                assert!(close(h[i][j], 0.0));
            }
        }
    }

    #[test]
    fn lanczos_produces_tridiagonal_projection() {
        let a = spd2();
        let v1 = vec![1.0, 0.0];
        let (alpha, beta) = lanczos(&a, &v1, 2);
        assert_eq!(alpha.len(), 2);
        assert_eq!(beta.len(), 1);
        // first alpha = v1^T A v1 = a[0][0] = 4.0
        assert!(close(alpha[0], 4.0));
    }

    #[test]
    fn gmres_solves_nonsymmetric_system() {
        // non-symmetric: [[3,1],[0,2]] x = [5,4] -> x = [1,2]
        let a = vec![vec![3.0, 1.0], vec![0.0, 2.0]];
        let b = vec![5.0, 4.0];
        let out = gmres(&a, &b, &[0.0, 0.0], 2, 1e-12, 20);
        assert!(out.converged, "residual {}", out.residual);
        assert!(close_vec(&out.x, &[1.0, 2.0]));
    }

    #[test]
    fn gmres_solves_spd_too() {
        let a = spd2();
        let b = vec![1.0, 2.0];
        let out = gmres(&a, &b, &[0.0, 0.0], 2, 1e-12, 20);
        assert!(out.converged);
        assert!(close_vec(&out.x, &[1.0 / 11.0, 7.0 / 11.0]));
    }

    #[test]
    fn matvec_matches_dense_definition() {
        let a = spd2();
        let x = vec![3.0, -1.0];
        let y = matvec(&a, &x);
        assert!(close(y[0], 4.0 * 3.0 + 1.0 * -1.0)); // 11
        assert!(close(y[1], 1.0 * 3.0 + 3.0 * -1.0)); // 0
    }
}
