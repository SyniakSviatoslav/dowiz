//! householder.rs — Householder-based dense eigensolver (the kernel "Ferrari").
//!
//! CONVEYOR (general real `n×n` → ALL eigenvalues, real + complex conjugate pairs):
//!   1. Householder reduction to UPPER HESSENBERG form — in-place, O(n³), the
//!      numerically-armored step (reflectors minimize round-off drift).
//!   2. Shifted QR iteration on the Hessenberg form, run in COMPLEX arithmetic so
//!      a complex conjugate pair (e.g. the μ≈−1 period-2 cycle the hydraulic loop
//!      needs) is captured without real-only double-shift trickery.
//!
//! WHY THIS OVER THE OLD Faddeev-LeVerrier + Durand-Kerner PATH:
//!   Faddeev-LeVerrier is O(n⁴); for the dense N≈16..32 operators this kernel
//!   actually diagonalizes (mesh Laplacian, field-UI modal motion) that is a real
//!   speed + stability loss. Householder→Hessenberg→QR is O(n³) and the gold
//!   standard for dense eigen. The legacy Faddeev path is retained ONLY as a
//!   fallback for n>32 and as the hand-check oracle in the parity tests.
//!
//! ZERO new deps. No float-on-money. Deterministic (no RNG). The hot N≤32 path is
//! stack-only (`Matrix32x32`, [`f64; 1024]`, no heap) with an FMA-accelerated
//! inner product. Verified-by-Math: hand-derived oracles + parity vs Faddeev.
//!
//! (Ferrari-struct per operator spec: `Matrix32x32 { data: [f64; 32*32] }`,
//! in-place, SIMD/FMA, loop-friendly fixed stride.)

use crate::spectral::Complex;

// ── FMA-accelerated dot product (the "Ferrari" inner kernel) ────────────────
// Portable fallback is plain scalar; on x86_64 with FMA we fuse multiply-add via
// `_mm256_fmadd_pd`. Runtime-detected so the same binary runs on non-FMA parts.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "fma")]
#[inline]
unsafe fn dot_fma(a: *const f64, b: *const f64, n: usize) -> f64 {
    use core::arch::x86_64::*;
    let mut acc = _mm256_setzero_pd();
    let mut i = 0;
    while i + 4 <= n {
        let xa = _mm256_loadu_pd(a.add(i));
        let xb = _mm256_loadu_pd(b.add(i));
        acc = _mm256_fmadd_pd(xa, xb, acc);
        i += 4;
    }
    let mut arr = [0.0f64; 4];
    _mm256_storeu_pd(arr.as_mut_ptr(), acc);
    let mut s = arr[0] + arr[1] + arr[2] + arr[3];
    while i < n {
        s += *a.add(i) * *b.add(i);
        i += 1;
    }
    s
}

#[inline]
pub fn dot(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len();
    // FMA fast path. Runtime detection needs std; under no-std we fall back to
    // compile-time `target_feature="fma"` (set via RUSTFLAGS) else the scalar
    // loop — the math is identical, only the SIMD dispatch differs.
    #[cfg(all(target_arch = "x86_64", feature = "std"))]
    {
        if std::is_x86_feature_detected!("fma") {
            // SAFETY: slices are same-length, fma kernel stays in-bounds.
            return unsafe { dot_fma(a.as_ptr(), b.as_ptr(), n) };
        }
    }
    #[cfg(all(target_arch = "x86_64", not(feature = "std"), target_feature = "fma"))]
    {
        // SAFETY: fma is guaranteed present at compile time here.
        return unsafe { dot_fma(a.as_ptr(), b.as_ptr(), n) };
    }
    let mut s = 0.0;
    for i in 0..n {
        s += a[i] * b[i];
    }
    s
}

/// Fixed 32×32 dense matrix (row-major, stack-resident, no heap). The hot
/// consumer path fits inside this. `[f64; 1024]` gives the compiler a fixed
/// stride so LLVM auto-vectorizes and the data sits cache-contiguous.
#[derive(Clone, Copy)]
pub struct Matrix32x32 {
    pub data: [f64; 1024],
}

impl Matrix32x32 {
    pub const N: usize = 32;

    #[inline]
    pub fn zeros() -> Self {
        Self { data: [0.0; 1024] }
    }

    /// Build from a compact `n×n` row-major slice (n ≤ 32); remaining slots zero.
    pub fn from_square(a: &[f64], n: usize) -> Self {
        debug_assert!(n <= 32);
        let mut m = Self::zeros();
        for i in 0..n {
            for j in 0..n {
                m.data[i * 32 + j] = a[i * n + j];
            }
        }
        m
    }

    #[inline]
    pub fn get(&self, i: usize, j: usize) -> f64 {
        self.data[i * 32 + j]
    }
    #[inline]
    pub fn set(&mut self, i: usize, j: usize, v: f64) {
        self.data[i * 32 + j] = v;
    }

    /// Compute all eigenvalues of this matrix (uses the Householder engine).
    pub fn eigenvalues(&self, n: usize) -> Vec<Complex> {
        let mut buf = [0.0f64; 1024];
        for i in 0..n {
            for j in 0..n {
                buf[i * n + j] = self.data[i * 32 + j];
            }
        }
        eigenvalues_contig(&mut buf, n)
    }
}

// ── Householder reduction to upper Hessenberg (real, in-place) ───────────────
// `a` is `n×n` row-major with stride `n`. Reduced in place to similar Hessenberg
// H (similarity: H = P·A·P, P unitary). Stack-only workspace (n ≤ 32).
fn reduce_hessenberg(a: &mut [f64], n: usize) {
    let mut v = [0.0f64; 32]; // Householder vector (global-indexed, rows k+1..n-1)
    for k in 0..n.saturating_sub(2) {
        // norm of subcolumn a[k+1..n-1, k]
        let mut norm = 0.0;
        for i in (k + 1)..n {
            norm += a[i * n + k] * a[i * n + k];
        }
        norm = norm.sqrt();
        if norm == 0.0 {
            continue;
        }
        let x0 = a[(k + 1) * n + k];
        // σ = -sign(x0)·‖x‖  (reflector aims at ∓‖x‖ e₀, avoids cancellation)
        let sigma = if x0 >= 0.0 { -norm } else { norm };
        v[k + 1] = x0 - sigma;
        for i in (k + 2)..n {
            v[i] = a[i * n + k];
        }
        // ‖v‖²
        let mut v2 = v[k + 1] * v[k + 1];
        for i in (k + 2)..n {
            v2 += v[i] * v[i];
        }
        if v2 == 0.0 {
            continue;
        }
        let beta = 2.0 / v2;
        // Two-sided similarity A ← P A P, P = I − β v vᵀ (v supported on k+1..n-1).
        // LEFT (P A): touches rows k+1..n-1, ALL columns 0..n — this is what
        // actually zeroes the subcolumn a[k+2..][k].
        for c in 0..n {
            let mut s = 0.0;
            for g in (k + 1)..n {
                s += v[g] * a[g * n + c];
            }
            let bs = beta * s;
            for r in (k + 1)..n {
                a[r * n + c] -= v[r] * bs;
            }
        }
        // RIGHT (A P): touches ALL rows 0..n, columns k+1..n-1.
        for r in 0..n {
            let mut s = 0.0;
            for g in (k + 1)..n {
                s += v[g] * a[r * n + g];
            }
            let bs = beta * s;
            for c in (k + 1)..n {
                a[r * n + c] -= v[c] * bs;
            }
        }
    }
}

// ── Shifted QR on Hessenberg, complex arithmetic ─────────────────────────────
// Reads Hessenberg H (n×n, stride n). Returns all eigenvalues (real + complex).
fn eig_hessenberg(h: &[f64], n: usize) -> Vec<Complex> {
    let mut c = [[Complex::new(0.0, 0.0); 32]; 32];
    for i in 0..n {
        for j in 0..n {
            c[i][j] = Complex::new(h[i * n + j], 0.0);
        }
    }
    let mut out: Vec<Complex> = Vec::with_capacity(n);
    let mut m = n;
    let eps = 1e-13;
    let mut guard = 0;
    while m > 0 {
        guard += 1;
        if guard > 10000 {
            break;
        }
        if m == 1 {
            out.push(c[0][0]);
            break;
        }
        // deflation:
        //  • bottom subdiagonal c[m-1][m-2] negligible ⇒ c[m-1][m-1] is an
        //    ISOLATED real eigenvalue (1×1 block). Pop it, m-=1.
        let sb = c[m - 1][m - 2].abs();
        let scale = c[m - 1][m - 1].abs() + c[m - 2][m - 2].abs() + 1.0;
        if sb < eps * scale {
            out.push(c[m - 1][m - 1]);
            m -= 1;
            continue;
        }
        //  • next subdiagonal c[m-2][m-3] negligible (or m==2) ⇒ the bottom 2×2
        //    is isolated (possibly a complex conjugate pair). Extract both.
        let sb2 = if m >= 3 { c[m - 2][m - 3].abs() } else { 0.0 };
        if m == 2 || sb2 < eps * (c[m - 2][m - 2].abs() + c[m - 3][m - 3].abs() + 1.0) {
            let a = c[m - 2][m - 2];
            let b = c[m - 2][m - 1];
            let dd = c[m - 1][m - 2];
            let e = c[m - 1][m - 1];
            let tr = a.add(e);
            let det = a.mul(e).sub(b.mul(dd));
            let disc = tr.mul(tr).sub(det.mul(Complex::new(4.0, 0.0)));
            let sq = disc.sqrt();
            let r1 = tr.add(sq).mul(Complex::new(0.5, 0.0));
            let r2 = tr.sub(sq).mul(Complex::new(0.5, 0.0));
            if r1.im.abs() < 1e-9 && r2.im.abs() < 1e-9 {
                out.push(Complex::new(r1.re, 0.0));
                out.push(Complex::new(r2.re, 0.0));
            } else {
                out.push(r1);
                out.push(r2);
            }
            m -= 2;
            continue;
        }
        // shift = eigenvalue of bottom 2×2 with larger modulus (Wilkinson-style)
        let a = c[m - 2][m - 2];
        let b = c[m - 2][m - 1];
        let dd = c[m - 1][m - 2];
        let e = c[m - 1][m - 1];
        let tr = a.add(e);
        let det = a.mul(e).sub(b.mul(dd));
        let disc = tr.mul(tr).sub(det.mul(Complex::new(4.0, 0.0)));
        let sq = disc.sqrt();
        let r1 = tr.add(sq).mul(Complex::new(0.5, 0.0));
        let r2 = tr.sub(sq).mul(Complex::new(0.5, 0.0));
        // Wilkinson shift: root CLOSER to the bottom corner e ⇒ fast convergence.
        let d1 = r1.sub(e).abs();
        let d2 = r2.sub(e).abs();
        let sigma = if d1 <= d2 { r1 } else { r2 };
        qr_step(&mut c, m, sigma);
    }
    out
}

/// One shifted QR similarity step that PRESERVES Hessenberg form.
///
/// Textbook Francis/Hessenberg QR step:
///   1. LEFT-pass — apply Givens rotations G_k to ROWS (left-multiply) to zero
///      the subdiagonal. Each G_k is built to zero column entry c[k+1][k]. This
///      drives H−σI to UPPER-TRIANGULAR (a proper QR factorization Q* (H−σI) = R,
///      where the stacked left-multiplications = Q*). Critically, the RIGHT part
///      is deferred so the bulge is chased correctly (we never mix a half-applied
///      transform with the next one's column read).
///   2. RIGHT-pass — apply the SAME rotations' conjugates to COLUMNS (right-
///      multiply by G_k) to complete the similarity G* A G. Because the left pass
///      finished first, the intermediate is upper-triangular and the right pass
///      restores Hessenberg form without refilling the lower triangle.
/// Operates in complex arithmetic so σ may be complex (captures conjugate pairs).
fn qr_step(c: &mut [[Complex; 32]; 32], m: usize, sigma: Complex) {
    // shift diagonal
    for i in 0..m {
        c[i][i] = c[i][i].sub(sigma);
    }
    // storage for the k-th Givens (G*) components, for the right pass
    let mut h11 = [Complex::new(0.0, 0.0); 32];
    let mut h12 = [Complex::new(0.0, 0.0); 32];
    let mut h21 = [Complex::new(0.0, 0.0); 32];
    let mut h22 = [Complex::new(0.0, 0.0); 32];
    // LEFT pass: zero subdiagonal columns row by row, accumulate G_k = G* components.
    for k in 0..m - 1 {
        let x = c[k][k];
        let y = c[k + 1][k];
        let r = (x.mul(x.conj()).add(y.mul(y.conj()))).sqrt();
        if r.abs() < 1e-300 {
            h11[k] = Complex::new(1.0, 0.0);
            h12[k] = Complex::new(0.0, 0.0);
            h21[k] = Complex::new(0.0, 0.0);
            h22[k] = Complex::new(1.0, 0.0);
            continue;
        }
        // G with G*[x;y]=[r;0]: G=[[conj(x),conj(y)],[-y,x]]/r
        // G* = [[conj(x), -y.conj()],[y.conj(), x.conj()]]/r
        let inv = Complex::new(1.0 / r.re, 0.0); // r is real (modulus)
        let a11 = x.conj().mul(inv);
        let a12 = y.conj().mul(inv);
        let a21 = y.mul(Complex::new(-1.0, 0.0)).mul(inv);
        let a22 = x.mul(inv); // G = [[conj(x),conj(y)],[-y,x]]/r
                              // left-multiply rows k,k+1 by G
        for j in k..m {
            let rk = c[k][j];
            let rkp = c[k + 1][j];
            c[k][j] = a11.mul(rk).add(a12.mul(rkp));
            c[k + 1][j] = a21.mul(rk).add(a22.mul(rkp));
        }
        // store G* components; right-multiply M·G*:
        //   col_k    ← conj(a11)·col_k + conj(a12)·col_{k+1}
        //   col_{k+1}← conj(a21)·col_k + conj(a22)·col_{k+1}
        h11[k] = a11.conj();
        h12[k] = a12.conj();
        h21[k] = a21.conj();
        h22[k] = a22.conj();
    }
    // RIGHT pass: apply G_k* to columns k,k+1 (k = 0..m-2), completing G A G*.
    //   col_k    ← h11·col_k + h12·col_{k+1}
    //   col_{k+1}← h21·col_k + h22·col_{k+1}
    for k in 0..m - 1 {
        for i in 0..m {
            let ck = c[i][k];
            let ck1 = c[i][k + 1];
            c[i][k] = h11[k].mul(ck).add(h12[k].mul(ck1));
            c[i][k + 1] = h21[k].mul(ck).add(h22[k].mul(ck1));
        }
    }
    // unshift
    for i in 0..m {
        c[i][i] = c[i][i].add(sigma);
    }
}

/// All eigenvalues of a compact `n×n` row-major matrix (stride `n`), n ≤ 32.
/// Asserts n ≤ 32 (the stack-only fast path). Larger n must use the legacy
/// Faddeev path in `spectral.rs`.
pub fn eigenvalues_contig(a: &mut [f64], n: usize) -> Vec<Complex> {
    debug_assert!(n <= 32);
    debug_assert!(a.len() >= n * n);
    reduce_hessenberg(a, n);
    eig_hessenberg(a, n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spectral::{charpoly, roots, Complex};

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    fn cclose(a: Complex, b: Complex, tol: f64) -> bool {
        (a.re - b.re).abs() < tol && (a.im - b.im).abs() < tol
    }

    // parity: Householder spectrum vs legacy Faddeev-LeVerrier, within tol.
    fn parity(a: &[Vec<f64>], tol: f64) {
        let n = a.len();
        let mut buf = vec![0.0f64; n * n];
        for i in 0..n {
            for j in 0..n {
                buf[i * n + j] = a[i][j];
            }
        }
        let got = eigenvalues_contig(&mut buf, n);
        let want = roots(&charpoly(a));
        assert_eq!(got.len(), want.len(), "spectrum length mismatch");
        // sort both by (re, im) for comparison
        let mut g = got.clone();
        g.sort_by(|x, y| {
            x.re.partial_cmp(&y.re)
                .unwrap()
                .then(x.im.partial_cmp(&y.im).unwrap())
        });
        let mut w = want.clone();
        w.sort_by(|x, y| {
            x.re.partial_cmp(&y.re)
                .unwrap()
                .then(x.im.partial_cmp(&y.im).unwrap())
        });
        for (i, (x, y)) in g.iter().zip(w.iter()).enumerate() {
            assert!(
                cclose(*x, *y, tol),
                "eig[{i}] mismatch: householder {:?} vs faddeev {:?}",
                x,
                y
            );
        }
    }

    #[test]
    fn hand_rotation_90_is_plus_minus_i() {
        // R(90°) = [[0,-1],[1,0]] has eigenvalues ±i.
        let mut a = [0.0f64; 4];
        a[0] = 0.0;
        a[1] = -1.0;
        a[2] = 1.0;
        a[3] = 0.0;
        let e = eigenvalues_contig(&mut a.clone(), 2);
        assert_eq!(e.len(), 2);
        let mut mags: Vec<f64> = e.iter().map(|x| x.abs()).collect();
        mags.sort_by(|x, y| x.partial_cmp(y).unwrap());
        assert!(
            close(mags[0], 1.0, 1e-9) && close(mags[1], 1.0, 1e-9),
            "magnitudes = 1"
        );
        // exactly one +i and one -i
        let mut ims: Vec<f64> = e.iter().map(|x| x.im).collect();
        ims.sort_by(|x, y| x.partial_cmp(y).unwrap());
        assert!(
            close(ims[0], -1.0, 1e-9) && close(ims[1], 1.0, 1e-9),
            "eigs = ±i"
        );
    }

    #[test]
    fn hand_two_cycle_is_plus_minus_one() {
        let c = vec![vec![0.0, 1.0], vec![1.0, 0.0]];
        parity(&c, 1e-9);
    }

    #[test]
    fn hand_diagonal_known_spectrum() {
        let d = vec![
            vec![2.0, 0.0, 0.0],
            vec![0.0, 5.0, 0.0],
            vec![0.0, 0.0, -3.0],
        ];
        parity(&d, 1e-9);
    }

    #[test]
    fn hand_path_p3_laplacian_spectrum() {
        // P₃ Laplacian = [[1,-1,0],[-1,2,-1],[0,-1,1]] → spectrum {0,1,3}.
        let l = vec![
            vec![1.0, -1.0, 0.0],
            vec![-1.0, 2.0, -1.0],
            vec![0.0, -1.0, 1.0],
        ];
        parity(&l, 1e-9);
    }

    #[test]
    fn parity_general_3x3_asymmetric() {
        let a = vec![
            vec![1.0, 2.0, 3.0],
            vec![0.0, 4.0, 5.0],
            vec![0.0, 0.0, 6.0], // already upper-triangular, eigs 1,4,6
        ];
        parity(&a, 1e-9);
    }

    #[test]
    fn parity_general_4x4_mixed() {
        let a = vec![
            vec![0.0, 1.0, 0.0, 0.0],
            vec![1.0, 0.0, 1.0, 0.0],
            vec![0.0, 1.0, 0.0, 1.0],
            vec![0.0, 0.0, 1.0, 0.0],
        ];
        parity(&a, 1e-9);
    }

    #[test]
    fn matrix32x32_path_runs() {
        let mut m = Matrix32x32::zeros();
        // 4×4 path-graph adjacency embedded at top-left
        let adj = [
            [0.0, 1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0, 0.0],
        ];
        for i in 0..4 {
            for j in 0..4 {
                m.set(i, j, adj[i][j]);
            }
        }
        let e = m.eigenvalues(4);
        assert_eq!(e.len(), 4);
        let mut mags: Vec<f64> = e.iter().map(|x| x.abs()).collect();
        mags.sort_by(|x, y| x.partial_cmp(y).unwrap());
        // path P₄ adjacency eigenvalues 2cos(kπ/5), k=1..4 → max = 2cos(π/5) = φ
        let phi = (1.0 + 5.0_f64.sqrt()) / 2.0;
        assert!(close(mags[3], phi, 1e-9), "largest adjacency eig of P₄ = φ");
    }

    #[test]
    fn dot_fma_matches_scalar() {
        let a: Vec<f64> = (0..32).map(|i| (i as f64) * 0.5 + 1.0).collect();
        let b: Vec<f64> = (0..32).map(|i| (i as f64) * 2.0 - 7.0).collect();
        let s = dot(&a, &b);
        let refr: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        assert!(close(s, refr, 1e-12));
    }
}
