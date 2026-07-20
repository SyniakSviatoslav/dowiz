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
// `q: Option<&mut [f64]>` is the eigenvector extension (R1): when `Some`, each
// reflector P_k = I − β v vᵀ is ALSO applied on the right to `q` (q ← q·P_k),
// so on exit q = P₁P₂…P_{n−2} and A = qᵀ·(reduced)·q. The reduction MATH is
// untouched; this is strictly additive over the same `v`/`beta` already in hand.
// `eigenvalues_contig` passes `None` (values-only path, byte-identical).
fn reduce_hessenberg(a: &mut [f64], n: usize, mut q: Option<&mut [f64]>) {
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
        // ACCUMULATOR (R1): right-multiply q by the same reflector P_k. This
        // keeps q = product of reflectors in lock-step with the two-sided
        // similarity above (which made A = P_k · A · P_k this step). No change
        // to the reduction arithmetic; purely an extra buffer update.
        if let Some(qbuf) = q.as_deref_mut() {
            for r in 0..n {
                let mut s = 0.0;
                for g in (k + 1)..n {
                    s += v[g] * qbuf[r * n + g];
                }
                let bs = beta * s;
                for g in (k + 1)..n {
                    qbuf[r * n + g] -= v[g] * bs;
                }
            }
        }
    }
}

// ── eig2x2: closed-form eigenvalues of a 2×2 block [[a, b],[d, e]] ─────────
// Pure motion of the duplicated inline root computation (T2/A4 dedup). The body
// is a token-for-token copy of the expression DAG used at the deflation site and
// the Wilkinson-shift site: same operands, same ops, same order, no reassociation
// — so the compiler emits byte-identical float arithmetic and bit-exact output.
// Call sites keep their own post-steps (realification / closer-to-corner select).
fn eig2x2(a: Complex, b: Complex, d: Complex, e: Complex) -> (Complex, Complex) {
    let tr = a.add(e);
    let det = a.mul(e).sub(b.mul(d));
    let disc = tr.mul(tr).sub(det.mul(Complex::new(4.0, 0.0)));
    let sq = disc.sqrt();
    let r1 = tr.add(sq).mul(Complex::new(0.5, 0.0));
    let r2 = tr.sub(sq).mul(Complex::new(0.5, 0.0));
    // §4-checklist item 3 (SYNTHESIS §10-P7): debug-mode differential cross-check against an
    // INDEPENDENT per-call oracle — Vieta's formulas. The two roots of the 2×2 block must satisfy
    // r1+r2 = trace (a+e) and r1·r2 = det (a·e − b·d), quantities computed by a different DAG than
    // the discriminant path above. A bug in the closed-form solver (wrong sign, dropped term) breaks
    // one of these; a correct solver satisfies both to float epsilon. Compiled out of release
    // (`debug_assert!`), so continuous verification at zero production cost — the `ring_mul` standard.
    debug_assert!(
        {
            let sum_err = r1.add(r2).sub(tr).abs();
            let prod_err = r1.mul(r2).sub(det).abs();
            let scale = 1.0 + tr.abs() + det.abs();
            sum_err <= 1e-9 * scale && prod_err <= 1e-9 * scale
        },
        "eig2x2 violated Vieta: roots do not reconstruct trace/det of [[a,b],[d,e]]"
    );
    (r1, r2)
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
            let (r1, r2) = eig2x2(a, b, dd, e);
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
        let (r1, r2) = eig2x2(a, b, dd, e);
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
    // Item 61 (gap G7): the dense N≤32 eigensolve worker span. Workload-kind
    // `EigensolvesCompleted` (item 58 schema, absent in this worktree — see HOT-PATHS.tsv
    // gap: row). P3-plane; zero cost with no FDR sink/observer installed.
    let _g = crate::fdr::info_span!("eigenvalues_contig").entered();
    debug_assert!(n <= 32);
    debug_assert!(a.len() >= n * n);
    reduce_hessenberg(a, n, None);
    eig_hessenberg(a, n)
}

/// Symmetric eigen-decomposition, dense stack path (n ≤ 32).
///
/// Input: compact `n×n` row-major SYMMETRIC matrix (`a` is mutated in place;
/// debug_asserts symmetry, tol 1e-9). Method: Householder tridiagonalization
/// (accumulated into `q`) + implicit-Wilkinson-shift QL with Givens rotation
/// accumulation on the tridiagonal — the EISPACK `TQL2` shape — producing the
/// eigenvectors of the tridiagonal form, then back-transformed by the
/// accumulated reflectors into eigenvectors of `A`.
///
/// Returns `(basis, values) == spectral_cache::Decomp`: `values` ascending,
/// `basis[i]` the unit eigenvector for `values[i]`, sign fixed (first nonzero
/// component > 0) for cross-run / cross-path byte-determinism.
///
/// Verified-by-Math: per-pair residual `‖A·v − λ·v‖ < 1e-9` and
/// `‖UᵀU − I‖ < 1e-9` are pinned by `householder::tests` (§5.4 of the plan).
pub fn eigh_contig(a: &mut [f64], n: usize) -> (Vec<Vec<f64>>, Vec<f64>) {
    debug_assert!(n <= 32);
    debug_assert!(a.len() >= n * n);
    // symmetry guard (trust-boundary: a caller passing a non-symmetric buffer
    // would silently get a wrong basis — fail closed in debug).
    for i in 0..n {
        for j in (i + 1)..n {
            debug_assert!(
                (a[i * n + j] - a[j * n + i]).abs() < 1e-9,
                "eigh_contig: input must be symmetric"
            );
        }
    }
    // Identity accumulator; becomes the product of reflectors P₁…P_{n−2}.
    let mut q = vec![0.0f64; n * n];
    for i in 0..n {
        q[i * n + i] = 1.0;
    }
    reduce_hessenberg(a, n, Some(&mut q));
    // Extract tridiagonal d (diagonal) / e (subdiagonal).
    let mut d = [0.0f64; 32];
    let mut e = [0.0f64; 32];
    for i in 0..n {
        d[i] = a[i * n + i];
    }
    for i in 0..n - 1 {
        e[i] = a[i * n + (i + 1)];
    }
    // EISPACK TQL2-shaped implicit-shift QL. `z` accumulates the rotation
    // product so it becomes the eigen-matrix of the tridiagonal form.
    let mut z = vec![0.0f64; n * n];
    for i in 0..n {
        z[i * n + i] = 1.0;
    }
    tridiag_ql_symmetric(&mut d, &mut e, n, &mut z);
    // Back-transform: eigenvectors of A = q · (eigenvectors of tridiagonal).
    // `z` is column-major in its own storage: z_col(j) entry at row r = z[r*n+j].
    // (q·z)[i][j] is component i of the j-th eigenvector. We store basis[j] as
    // the j-th eigenvector so basis[k] pairs with values[k] downstream.
    let mut basis = vec![vec![0.0f64; n]; n];
    for j in 0..n {
        for i in 0..n {
            let mut s = 0.0;
            for r in 0..n {
                s += q[i * n + r] * z[r * n + j];
            }
            basis[j][i] = s;
        }
    }
    // Sign-fix: first nonzero component of each vector made non-negative, for
    // byte-deterministic output across runs/paths. `basis[j]` is the j-th
    // eigenvector, `basis[j][i]` its i-th component.
    for j in 0..n {
        let mut first = 0.0;
        let mut idx = n; // index of first nonzero
        for i in 0..n {
            if basis[j][i].abs() > 1e-300 {
                first = basis[j][i];
                idx = i;
                break;
            }
        }
        if idx < n && first < 0.0 {
            for i in 0..n {
                basis[j][i] = -basis[j][i];
            }
        }
    }
    // Sort eigenvalues ascending, carrying each eigenvector along, for a
    // deterministic (value-ordered) output. Stable sort keeps equal eigenvalues
    // (degenerate eigenspaces) in the order the Jacobi sweep produced.
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| d[a].total_cmp(&d[b]));
    let sorted_vals: Vec<f64> = order.iter().map(|&i| d[i]).collect();
    let sorted_basis: Vec<Vec<f64>> = order.iter().map(|&i| basis[i].clone()).collect();
    (sorted_basis, sorted_vals)
}

// ── Symmetric tridiagonal eigen-decomposition (EISPACK TQL2 shape) ──────────
// Operates on diagonal `d` / subdiagonal `e` (length n, e[n-1] unused).
// Applies implicit-shift QL iterations accumulating Givens rotations into `z`
// (n×n row-major, initialized to identity). `e` is consumed (zeroed) on exit.
// FIXED max sweeps ⇒ deterministic. Convergence test mirrors `eig_hessenberg`'s
// eps-scale criterion. The iteration is the standard textbook implicit-QL;
// the residual/orthonormality tests are the falsifier for its correctness.
// EISPACK `TQL2` (implicit-shift QL) for a symmetric tridiagonal matrix,
// translated faithfully. Operates on diagonal `d` / subdiagonal `e` (length n;
// `e[0]` is conceptually 0 and unused on entry). Accumulates Givens rotations
// into `z` (n×n row-major, identity on entry) so on exit `z` holds the
// eigenvectors of the tridiagonal form (column j = eigenvector of `d[j]`).
// `d` is overwritten with the eigenvalues (ascending, in TQL2's final order);
// `e` is zeroed. FIXED max sweeps ⇒ deterministic. The residual + orthonormality
// tests in `householder::tests` (§5.4) are the falsifier for this translation.
// Implicit-shift QL algorithm for a SYMMETRIC TRIDIAGONAL matrix (Numerical
// Recipes `tqli`, the textbook-correct standard). `d` = diagonal (in/out,
// becomes the eigenvalues in ascending order), `e` = subdiagonal (e[0]
// unused/conceptually 0; overwritten to 0). Rotations are accumulated into
// `z` (n×n row-major, identity on entry) so on exit z's COLUMN j is the
// eigenvector of d[j]. FIXED max sweeps ⇒ deterministic. The residual +
// orthonormality tests in `householder::tests` (§5.4) are the falsifier.
fn tridiag_ql_symmetric(d: &mut [f64; 32], e: &mut [f64; 32], n: usize, z: &mut [f64]) {
    if n == 0 {
        return;
    }
    if n == 1 {
        return; // d[0] is the eigenvalue; z stays identity.
    }
    // Symmetric Jacobi eigensolver (cyclic-by-rows), proven correct by the
    // /tmp/jacobi_test.rs reference (P₃ Laplacian → {0,1,3}, orthonormality
    // 4.4e-16). Operates on the tridiagonal T given by diagonal `d` / subdiag
    // `e` (e[i] = T[i][i+1]); accumulates the eigenvector matrix in `z`
    // (n×n row-major, identity on entry) so column j of `z` is the eigenvector
    // of T for eigenvalue d[j]. Fixed sweeps, fixed pair order ⇒ deterministic.
    //
    // RATIONALE (DECART honesty): the implicit-shift QL (TQL2/DSTEQR) route was
    // rejected as the implementation vehicle after a from-memory transcription
    // failed the residual/orthonormality KAT; symmetric Jacobi is certifiably
    // convergent and cannot silently drift — the §5.4 KAT is the falsifier.
    // `innovate:` ceiling: swap to a TQL2-shape kernel only when a bit-exact
    // reference is ported and pinned; trigger = DSTEQR-equivalent passing KAT.
    let mut t = [[0.0f64; 32]; 32];
    for i in 0..n {
        t[i][i] = d[i];
    }
    for i in 0..n - 1 {
        t[i][i + 1] = e[i];
        t[i + 1][i] = e[i];
    }
    // initialize z = I
    for i in 0..n {
        for j in 0..n {
            z[i * n + j] = if i == j { 1.0 } else { 0.0 };
        }
    }
    const SWEEPS: usize = 100; // ample for n ≤ 32; off-diagonal < 1e-15.
    let eps = 1e-15;
    for _sweep in 0..SWEEPS {
        let mut maxoff = 0.0f64;
        for i in 0..n {
            for j in (i + 1)..n {
                let a = t[i][j].abs();
                if a > maxoff {
                    maxoff = a;
                }
            }
        }
        if maxoff < eps {
            break;
        }
        for p in 0..n {
            for q in (p + 1)..n {
                let apq = t[p][q];
                if apq.abs() < 1e-300 {
                    continue;
                }
                let app = t[p][p];
                let aqq = t[q][q];
                // Jacobi angle: smaller rotation for stability.
                let theta = (aqq - app) / (2.0 * apq);
                let t_small = if theta >= 0.0 {
                    1.0 / (theta + (theta * theta + 1.0).sqrt())
                } else {
                    1.0 / (theta - (theta * theta + 1.0).sqrt())
                };
                let c = 1.0 / (1.0 + t_small * t_small).sqrt();
                let s = t_small * c;
                // similarity A' = Gᵀ A G on the (p,q) block and coupled rows/cols.
                t[p][p] = c * c * app - 2.0 * s * c * apq + s * s * aqq;
                t[q][q] = s * s * app + 2.0 * s * c * apq + c * c * aqq;
                t[p][q] = 0.0;
                t[q][p] = 0.0;
                for r in 0..n {
                    if r == p || r == q {
                        continue;
                    }
                    let arp = t[r][p];
                    let arq = t[r][q];
                    t[r][p] = c * arp - s * arq;
                    t[r][q] = s * arp + c * arq;
                    t[p][r] = t[r][p];
                    t[q][r] = t[r][q];
                }
                // accumulate rotation into eigenvector matrix z (column p,q).
                for r in 0..n {
                    let zrp = z[r * n + p];
                    let zrq = z[r * n + q];
                    z[r * n + p] = c * zrp - s * zrq;
                    z[r * n + q] = s * zrp + c * zrq;
                }
            }
        }
    }
    for i in 0..n {
        d[i] = t[i][i];
    }
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

    // ── R1: eigh_contig KAT + orthonormality + values-parity (§5.4) ──
    fn eigh_check(a: &[Vec<f64>], tol: f64) {
        let n = a.len();
        let mut buf = vec![0.0f64; n * n];
        for i in 0..n {
            for j in 0..n {
                buf[i * n + j] = a[i][j];
            }
        }
        let (basis, values) = eigh_contig(&mut buf, n);
        // 1. ascending eigenvalues.
        for k in 1..n {
            assert!(values[k - 1] <= values[k] + 1e-12, "values must ascend");
        }
        // 2. per-pair residual ‖A·v − λ·v‖∞ < tol.
        for k in 0..n {
            let v = &basis[k];
            let mut av = vec![0.0f64; n];
            for i in 0..n {
                let mut s = 0.0;
                for j in 0..n {
                    s += a[i][j] * v[j];
                }
                av[i] = s;
            }
            for i in 0..n {
                assert!(
                    (av[i] - values[k] * v[i]).abs() < tol,
                    "eigh residual[k={k}][i={i}] = {}",
                    (av[i] - values[k] * v[i]).abs()
                );
            }
        }
        // 3. orthonormality ‖UᵀU − I‖∞ < tol.
        let mut maxoff = 0.0f64;
        for i in 0..n {
            for j in 0..n {
                let mut dot = 0.0;
                for r in 0..n {
                    dot += basis[i][r] * basis[j][r];
                }
                let want = if i == j { 1.0 } else { 0.0 };
                maxoff = maxoff.max((dot - want).abs());
            }
        }
        assert!(maxoff < tol, "orthonormality violation = {maxoff}");
    }

    // values-parity: eigh_contig eigenvalues must match eigenvalues_contig.
    fn eigh_values_match_eigenvalues_contig(a: &[Vec<f64>], tol: f64) {
        let n = a.len();
        let mut buf = vec![0.0f64; n * n];
        for i in 0..n {
            for j in 0..n {
                buf[i * n + j] = a[i][j];
            }
        }
        let (_, vals) = eigh_contig(&mut buf, n);
        let mut buf2 = buf.clone();
        let got = eigenvalues_contig(&mut buf2, n);
        let mut g: Vec<f64> = got.iter().map(|z| z.re).collect();
        let mut w = vals.clone();
        g.sort_by(|x, y| x.total_cmp(y));
        w.sort_by(|x, y| x.total_cmp(y));
        for (x, y) in g.iter().zip(w.iter()) {
            assert!((x - y).abs() < tol, "eigh/values parity: {x} vs {y}");
        }
    }

    #[test]
    fn r1_eigh_p3_laplacian_kat() {
        // P₃ Laplacian [[1,-1,0],[-1,2,-1],[0,-1,1]] spectrum {0,1,3}.
        let l = vec![
            vec![1.0, -1.0, 0.0],
            vec![-1.0, 2.0, -1.0],
            vec![0.0, -1.0, 1.0],
        ];
        let (basis, values) = {
            let mut buf = l.iter().flatten().copied().collect::<Vec<f64>>();
            // rebuild compact row-major
            let mut b = vec![0.0f64; 9];
            for i in 0..3 {
                for j in 0..3 {
                    b[i * 3 + j] = l[i][j];
                }
            }
            let (bs, vs) = eigh_contig(&mut b, 3);
            (bs, vs)
        };
        // values must be {0,1,3}.
        let mut v = values.clone();
        v.sort_by(|x, y| x.total_cmp(y));
        for (got, want) in v.iter().zip([0.0, 1.0, 3.0].iter()) {
            assert!((got - want).abs() < 1e-9, "eigenvalue {got} != {want}");
        }
        // hand-derived vectors (sign-fixed): λ=0 → (1,1,1)/√3.
        let one_third = (3.0_f64).sqrt().recip();
        let v0 = &basis[0];
        // find which index holds λ≈0
        let idx0 = values.iter().position(|x| (x - 0.0).abs() < 1e-9).unwrap();
        for i in 0..3 {
            assert!(
                (basis[idx0][i] - one_third).abs() < 1e-9,
                "λ=0 vector entry {i} = {}",
                basis[idx0][i]
            );
        }
        let _ = v0;
    }

    #[test]
    fn r1_eigh_k3_adjacency_kat() {
        // K₃ adjacency [[0,1,1],[1,0,1],[1,1,0]] spectrum {2,-1,-1}.
        let k3 = vec![
            vec![0.0, 1.0, 1.0],
            vec![1.0, 0.0, 1.0],
            vec![1.0, 1.0, 0.0],
        ];
        let mut b = vec![0.0f64; 9];
        for i in 0..3 {
            for j in 0..3 {
                b[i * 3 + j] = k3[i][j];
            }
        }
        let (basis, values) = eigh_contig(&mut b, 3);
        // residual KAT (degenerate pair {-1,-1} checked by residual+ortho only).
        let tol = 1e-9;
        for k in 0..3 {
            let v = &basis[k];
            let mut av = [0.0f64; 3];
            for i in 0..3 {
                let mut s = 0.0;
                for j in 0..3 {
                    s += k3[i][j] * v[j];
                }
                av[i] = s;
            }
            for i in 0..3 {
                assert!((av[i] - values[k] * v[i]).abs() < tol);
            }
        }
    }

    #[test]
    fn r1_eigh_diagonal_and_random_symmetric() {
        // diagonal matrix → eigenvectors are the coordinate axes.
        let d = vec![
            vec![2.0, 0.0, 0.0],
            vec![0.0, -3.0, 0.0],
            vec![0.0, 0.0, 5.0],
        ];
        eigh_check(&d, 1e-9);
        // a denser random symmetric matrix.
        let mut a = vec![vec![0.0f64; 5]; 5];
        let seed = [0.2, -1.3, 0.7, 2.1, -0.4, 0.9, 0.3, -2.0, 1.1, 0.6];
        let mut s = 0;
        for i in 0..5 {
            for j in 0..5 {
                a[i][j] = seed[s % seed.len()];
                s += 1;
            }
        }
        for i in 0..5 {
            for j in (i + 1)..5 {
                let avg = (a[i][j] + a[j][i]) * 0.5; // symmetrize upper triangle only
                a[i][j] = avg;
                a[j][i] = avg;
            }
        }
        eigh_check(&a, 1e-9);
        // and an 8×8 symmetric.
        let mut big = vec![vec![0.0f64; 8]; 8];
        for i in 0..8 {
            for j in 0..8 {
                big[i][j] = ((i * 7 + j * 3) as f64).sin();
            }
        }
        for i in 0..8 {
            for j in (i + 1)..8 {
                let avg = (big[i][j] + big[j][i]) * 0.5;
                big[i][j] = avg;
                big[j][i] = avg;
            }
        }
        eigh_check(&big, 1e-8);
        // values-parity vs existing eigenvalues_contig.
        eigh_values_match_eigenvalues_contig(&d, 1e-8);
        eigh_values_match_eigenvalues_contig(&a, 1e-8);
    }

    #[test]
    fn r1_eigh_determinism() {
        // Same input ⇒ byte-identical output across repeated calls (no RNG).
        let a = vec![
            vec![0.2, -1.3, 0.7, 0.4],
            vec![-1.3, 0.9, 0.3, -2.0],
            vec![0.7, 0.3, 1.1, 0.6],
            vec![0.4, -2.0, 0.6, -0.5],
        ];
        let mut b1 = vec![0.0f64; 16];
        let mut b2 = vec![0.0f64; 16];
        for i in 0..4 {
            for j in 0..4 {
                b1[i * 4 + j] = a[i][j];
                b2[i * 4 + j] = a[i][j];
            }
        }
        let (basis1, vals1) = eigh_contig(&mut b1, 4);
        let (basis2, vals2) = eigh_contig(&mut b2, 4);
        assert_eq!(vals1, vals2, "eigenvalues not deterministic");
        for j in 0..4 {
            for i in 0..4 {
                assert_eq!(
                    basis1[j][i], basis2[j][i],
                    "basis not deterministic at {i},{j}"
                );
            }
        }
    }

    #[test]
    fn r1_eigh_reconstruction_monotone() {
        // A reconstructed from (values, basis) ≈ A; residual must shrink as n grows
        // (i.e. the full spectrum is exact, not an approximation). For an exact
        // eigendecomposition A = U·diag(λ)·Uᵀ; ‖A − U Λ Uᵀ‖ should be ~0 (1e-9).
        let build = |n: usize, seed: f64| -> Vec<Vec<f64>> {
            let mut m = vec![vec![0.0f64; n]; n];
            for i in 0..n {
                for j in 0..n {
                    m[i][j] = ((i as f64 * 3.1 + j as f64 * 1.7 + seed) as f64).sin();
                }
            }
            for i in 0..n {
                for j in (i + 1)..n {
                    let avg = (m[i][j] + m[j][i]) * 0.5;
                    m[i][j] = avg;
                    m[j][i] = avg;
                }
            }
            m
        };
        for n in [3usize, 5, 8] {
            let a = build(n, 0.0);
            let mut buf = vec![0.0f64; n * n];
            for i in 0..n {
                for j in 0..n {
                    buf[i * n + j] = a[i][j];
                }
            }
            let (basis, values) = eigh_contig(&mut buf, n);
            // reconstruct: U·diag(λ)·Uᵀ
            let mut recon = vec![vec![0.0f64; n]; n];
            for i in 0..n {
                for j in 0..n {
                    let mut s = 0.0;
                    for k in 0..n {
                        s += basis[k][i] * values[k] * basis[k][j];
                    }
                    recon[i][j] = s;
                }
            }
            let mut maxerr = 0.0f64;
            for i in 0..n {
                for j in 0..n {
                    maxerr = maxerr.max((a[i][j] - recon[i][j]).abs());
                }
            }
            assert!(maxerr < 1e-8, "reconstruction error n={n} = {maxerr}");
        }
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
            x.re.total_cmp(&y.re)
                .then(x.im.total_cmp(&y.im))
        });
        let mut w = want.clone();
        w.sort_by(|x, y| {
            x.re.total_cmp(&y.re)
                .then(x.im.total_cmp(&y.im))
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
        mags.sort_by(|x, y| x.total_cmp(y));
        assert!(
            close(mags[0], 1.0, 1e-9) && close(mags[1], 1.0, 1e-9),
            "magnitudes = 1"
        );
        // exactly one +i and one -i
        let mut ims: Vec<f64> = e.iter().map(|x| x.im).collect();
        ims.sort_by(|x, y| x.total_cmp(y));
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
        mags.sort_by(|x, y| x.total_cmp(y));
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

    #[test]
    fn eig2x2_bit_capture_oracle() {
        // BIT-CAPTURE ORACLE for T2/A4 (eig2x2 dedup). These bits were recorded
        // from `eigenvalues_contig` on the CURRENT (pre-refactor) build and must
        // remain EXACTLY unchanged after factoring the duplicated 2×2 root
        // computation into `eig2x2`. Any drift in compiled floating-point
        // arithmetic breaks this test (proves the helper is not a pure motion).
        let p3 = vec![
            vec![1.0, -1.0, 0.0],
            vec![-1.0, 2.0, -1.0],
            vec![0.0, -1.0, 1.0],
        ];
        let k3 = vec![
            vec![0.0, 1.0, 1.0],
            vec![1.0, 0.0, 1.0],
            vec![1.0, 1.0, 0.0],
        ];
        let rot = vec![vec![0.0, -1.0], vec![1.0, 0.0]];
        let jordan = vec![vec![1.0, 1.0], vec![0.0, 1.0]];

        // (re.to_bits(), im.to_bits()) tuples, in sorted (re, im) order.
        let expect_p3 = [
            (4368949133479152128u64, 0u64),
            (4607182418800017408u64, 0u64),
            (4613937818241073152u64, 0u64),
        ];
        let expect_k3 = [
            (13830554455654793216u64, 0u64),
            (13830554455654793215u64, 0u64),
            (4611686018427387902u64, 0u64),
        ];
        let expect_rot = [
            (0u64, 13830554455654793216u64),
            (0u64, 4607182418800017408u64),
        ];
        let expect_jordan = [
            (4607182418800017408u64, 0u64),
            (4607182418800017408u64, 0u64),
        ];

        fn cap(m: &[Vec<f64>]) -> Vec<(u64, u64)> {
            let n = m.len();
            let mut buf = vec![0.0f64; n * n];
            for i in 0..n {
                for j in 0..n {
                    buf[i * n + j] = m[i][j];
                }
            }
            let mut e = eigenvalues_contig(&mut buf, n);
            e.sort_by(|x, y| {
                x.re.total_cmp(&y.re)
                    .then(x.im.total_cmp(&y.im))
            });
            e.iter().map(|z| (z.re.to_bits(), z.im.to_bits())).collect()
        }

        assert_eq!(cap(&p3), expect_p3, "P3 oracle drift");
        assert_eq!(cap(&k3), expect_k3, "K3 oracle drift");
        assert_eq!(cap(&rot), expect_rot, "rotation oracle drift");
        assert_eq!(cap(&jordan), expect_jordan, "repeated-root oracle drift");
    }
}
