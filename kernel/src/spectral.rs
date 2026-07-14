//! spectral.rs — a general (non-symmetric) spectral engine for the kernel.
//!
//! WHY THIS EXISTS (reverse-engineering loop, iteration #1)
//!   Four subsystems each needed the same missing primitive — the *sub-dominant*
//!   eigenvalue of a matrix:
//!     * `order_machine::spectral_radius()` computes only ρ (the TOP eigenvalue)
//!       via power iteration — it cannot see the spectral gap.
//!     * `tools/loop-signals/markov_attractor.py` already computes ALL eigenvalues
//!       (Faddeev-LeVerrier → Durand-Kerner) but only in Python, on the hook path.
//!     * the hydraulic-loop design names a "general real eigensolver" as its #1
//!       missing primitive (symmetric Jacobi misreports the complex μ≈−1 2-cycle).
//!     * the field-UI engine needs Laplacian eigenmodes (λ_k) for modal motion.
//!   This module ports that proven Python core to zero-dep Rust and adds the
//!   quantities the research proved missing: the spectral gap, the Laplacian
//!   Fiedler value λ₂ (algebraic connectivity), and the DMD |μ|-vs-1 drift class.
//!
//! MATH
//!   * eigenvalues: char-poly via Faddeev-LeVerrier (1840s), roots via
//!     Durand-Kerner (Weierstrass, 1891) — finds ALL complex eigenvalues of a
//!     general real matrix, no symmetry assumption.
//!   * spectral gap γ = 1 − |λ₂| governs BOTH stability (|λ|-vs-1) and iteration
//!     speed (mixing time τ ≈ 1/γ; a power method needs ≈ log(tol)/log|λ₂/λ₁|
//!     iterations). This is the master dial the research identified.
//!
//! Float is used deliberately — this is graph/operator structure, never money
//! (the no-float rule is money-only). Pure, no I/O, deterministic (Durand-Kerner
//! seeds off a fixed complex spread, never RNG). Verified-by-Math tests below.

use crate::mat::{matmul_contig, Mat};
use core::f64::consts::PI;

/// Thin `&[Vec<f64>]` wrapper over [`matmul_contig`] — kept so the wasm surface
/// and existing tests (which pass `Vec<Vec<f64>>`) keep compiling. The `_n`
/// argument preserves the historical signature (square `n × n`).
fn matmul(a: &[Vec<f64>], b: &[Vec<f64>], _n: usize) -> Vec<Vec<f64>> {
    let am = Mat::from_vecvec(a);
    let bm = Mat::from_vecvec(b);
    matmul_contig(&am, &bm).into_vecvec()
}

/// Minimal complex number (avoids a `num-complex` dependency — kernel is zero-dep).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

impl Complex {
    pub const fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }
    /// Modulus |z|.
    pub fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }
    /// Argument arg(z) ∈ (−π, π].
    pub fn arg(self) -> f64 {
        self.im.atan2(self.re)
    }
    fn add(self, o: Complex) -> Complex {
        Complex::new(self.re + o.re, self.im + o.im)
    }
    fn sub(self, o: Complex) -> Complex {
        Complex::new(self.re - o.re, self.im - o.im)
    }
    fn mul(self, o: Complex) -> Complex {
        Complex::new(
            self.re * o.re - self.im * o.im,
            self.re * o.im + self.im * o.re,
        )
    }
    fn div(self, o: Complex) -> Complex {
        let d = o.re * o.re + o.im * o.im;
        Complex::new(
            (self.re * o.re + self.im * o.im) / d,
            (self.im * o.re - self.re * o.im) / d,
        )
    }
    fn powu(self, k: u32) -> Complex {
        let mut r = Complex::new(1.0, 0.0);
        for _ in 0..k {
            r = r.mul(self);
        }
        r
    }
    fn is_zero(self) -> bool {
        self.re == 0.0 && self.im == 0.0
    }
}

fn trace(a: &[Vec<f64>], n: usize) -> f64 {
    (0..n).map(|i| a[i][i]).sum()
}

/// Characteristic polynomial via the Faddeev-LeVerrier recurrence.
/// Returns monic coefficients, **highest degree first**: `[1, c_{n-1}, …, c_0]`.
pub fn charpoly(a: &[Vec<f64>]) -> Vec<f64> {
    let n = a.len();
    if n == 0 {
        return vec![1.0];
    }
    let mut c = vec![0.0; n + 1];
    c[n] = 1.0;
    // M_1 = I
    let mut m = vec![vec![0.0; n]; n];
    for i in 0..n {
        m[i][i] = 1.0;
    }
    c[n - 1] = -trace(&matmul(a, &m, n), n);
    for k in 2..=n {
        let am = matmul(a, &m, n); // A · M_{k-1}
        let add = c[n - k + 1];
        let mut mk = am;
        for i in 0..n {
            mk[i][i] += add; // M_k = A·M_{k-1} + c_{n-k+1}·I
        }
        m = mk;
        c[n - k] = -trace(&matmul(a, &m, n), n) / (k as f64);
    }
    (0..=n).map(|i| c[n - i]).collect() // highest-degree first
}

/// All (complex) roots of a monic polynomial (coeffs highest-degree first) via
/// Durand-Kerner simultaneous iteration. Deterministic seed — no RNG.
pub fn roots(coeffs: &[f64]) -> Vec<Complex> {
    let deg = coeffs.len().saturating_sub(1);
    if deg == 0 {
        return vec![];
    }
    if deg == 1 {
        // monic x + c₁ ⇒ root −c₁
        return vec![Complex::new(-coeffs[1], 0.0)];
    }
    let p: Vec<Complex> = coeffs.iter().map(|&x| Complex::new(x, 0.0)).collect();
    let peval = |x: Complex| -> Complex {
        let mut r = Complex::new(0.0, 0.0);
        for &co in &p {
            r = r.mul(x).add(co); // Horner, high → low
        }
        r
    };
    let seed = Complex::new(0.4, 0.9); // off the real axis so conjugate pairs separate
    let mut rts: Vec<Complex> = (0..deg).map(|k| seed.powu(k as u32)).collect();
    const DK_ITERS: usize = 200;
    for _ in 0..DK_ITERS {
        let mut maxd = 0.0f64;
        for i in 0..deg {
            let xi = rts[i];
            let mut denom = Complex::new(1.0, 0.0);
            for j in 0..deg {
                if j != i {
                    denom = denom.mul(xi.sub(rts[j]));
                }
            }
            if denom.is_zero() {
                continue; // coincident estimates — skip this step
            }
            let delta = peval(xi).div(denom);
            rts[i] = xi.sub(delta);
            let ad = delta.abs();
            if ad > maxd {
                maxd = ad;
            }
        }
        if maxd < 1e-12 {
            break;
        }
    }
    rts
}

/// Eigenvalues of a general real matrix (char-poly ∘ roots).
pub fn eigenvalues(a: &[Vec<f64>]) -> Vec<Complex> {
    let n = a.len();
    let coeffs = charpoly(a);
    // Nilpotent (char-poly = xⁿ ⇒ every eigenvalue 0). Durand-Kerner converges only
    // linearly on an n-fold zero root, so short-circuit exactly — mirrors the
    // `norm < TOL ⇒ 0` early-return in `order_machine::spectral_radius`.
    if n > 0 && coeffs[1..].iter().all(|c| c.abs() < 1e-12) {
        return vec![Complex::new(0.0, 0.0); n];
    }
    roots(&coeffs)
}

/// ρ(A) — spectral radius = largest eigenvalue modulus.
pub fn spectral_radius(a: &[Vec<f64>]) -> f64 {
    eigenvalues(a).iter().map(|e| e.abs()).fold(0.0, f64::max)
}

/// SLEM — second-largest eigenvalue modulus |λ₂| (the mixing / convergence rate).
pub fn slem(a: &[Vec<f64>]) -> f64 {
    let mut mags: Vec<f64> = eigenvalues(a).iter().map(|e| e.abs()).collect();
    mags.sort_by(|x, y| y.partial_cmp(x).unwrap_or(core::cmp::Ordering::Equal));
    if mags.len() > 1 {
        mags[1]
    } else {
        0.0
    }
}

/// Spectral gap γ = 1 − |λ₂|. For a row-stochastic matrix (λ₁ = 1) this is the
/// mixing gap: mixing time τ ≈ 1/γ, and a power method needs ≈ log(tol)/log(1−γ)
/// iterations. γ → 0 ⇒ never mixes (trapped / limit cycle).
pub fn spectral_gap(a: &[Vec<f64>]) -> f64 {
    1.0 - slem(a)
}

/// Graph Laplacian L = D − A of an (assumed symmetric) adjacency matrix.
/// Built on a contiguous `Mat` (row-major) to avoid the double `vec![vec![]]`
/// allocation, then materialized back to `Vec<Vec<f64>>` for the wasm/API contract.
pub fn laplacian(adj: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = adj.len();
    let mut l = Mat::zeros(n, n);
    for i in 0..n {
        let deg: f64 = (0..n).map(|j| adj[i][j]).sum();
        for j in 0..n {
            l.set(i, j, if i == j { deg - adj[i][j] } else { -adj[i][j] });
        }
    }
    l.into_vecvec()
}

/// Algebraic connectivity — the Fiedler value λ₂, the second-smallest eigenvalue
/// of the graph Laplacian. 0 ⇔ disconnected; larger ⇒ better-connected / faster
/// consensus. This is the quantity the FSM/mesh work was missing (only ρ existed).
pub fn algebraic_connectivity(adj: &[Vec<f64>]) -> f64 {
    let l = laplacian(adj);
    let mut re: Vec<f64> = eigenvalues(&l).iter().map(|e| e.re).collect();
    re.sort_by(|x, y| x.partial_cmp(y).unwrap_or(core::cmp::Ordering::Equal));
    if re.len() > 1 {
        re[1]
    } else {
        0.0
    }
}

/// DMD-style stability class from ρ vs the unit circle: the |μ|-vs-1 reading the
/// hydraulic-loop corpus wants (symmetric-Jacobi cannot see it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriftClass {
    /// ρ < 1 — the loop contracts (converging).
    Damped,
    /// ρ ≈ 1 — marginal / oscillatory (a limit cycle, e.g. μ≈−1 period-2).
    Resonant,
    /// ρ > 1 — divergent (getting worse / more verbose each step).
    Unstable,
}

pub fn classify_drift(a: &[Vec<f64>]) -> DriftClass {
    let rho = spectral_radius(a);
    const BAND: f64 = 1e-6;
    if rho < 1.0 - BAND {
        DriftClass::Damped
    } else if rho > 1.0 + BAND {
        DriftClass::Unstable
    } else {
        DriftClass::Resonant
    }
}

/// Dominant oscillation period from the eigenvalue nearest the unit circle that
/// points away from +1: ℓ ≈ 2π/|arg λ|. `Some(2.0)` for a period-2 (μ≈−1) cycle,
/// `None` for a non-oscillatory operator. Thresholds match the Python detector.
pub fn dominant_period(a: &[Vec<f64>]) -> Option<f64> {
    const PERIOD_MAG: f64 = 0.85;
    const PERIOD_ARG: f64 = 0.6;
    let mut best: Option<f64> = None;
    let mut best_arg = 0.0f64;
    for e in eigenvalues(a) {
        let ph = e.arg().abs();
        if e.abs() >= PERIOD_MAG && ph >= PERIOD_ARG && ph > best_arg {
            best_arg = ph;
            best = Some(2.0 * PI / ph);
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    // ── GREEN: a directed 2-cycle has eigenvalues ±1 (period-2, μ≈−1). ──
    #[test]
    fn green_two_cycle_eigs_plus_minus_one() {
        let c = vec![vec![0.0, 1.0], vec![1.0, 0.0]];
        assert!(approx(spectral_radius(&c), 1.0, 1e-6), "ρ=1 for a 2-cycle");
        assert!(
            approx(slem(&c), 1.0, 1e-6),
            "|λ₂|=1 — never mixes (trapped)"
        );
        assert_eq!(classify_drift(&c), DriftClass::Resonant);
        assert!(approx(dominant_period(&c).unwrap(), 2.0, 1e-3), "period-2");
    }

    // ── GREEN: strictly-upper-triangular ⇒ nilpotent ⇒ ρ=0 (acyclic). ──
    #[test]
    fn green_nilpotent_dag_rho_zero() {
        let n = vec![
            vec![0.0, 1.0, 1.0],
            vec![0.0, 0.0, 1.0],
            vec![0.0, 0.0, 0.0],
        ];
        assert!(spectral_radius(&n) < 1e-9);
        for e in eigenvalues(&n) {
            assert!(
                e.abs() < 1e-9,
                "every eigenvalue of a nilpotent matrix is 0"
            );
        }
    }

    // ── GREEN (VbM cross-check): the live lifecycle FSM is a DAG ⇒ ρ≈0, agreeing
    //    with the independent `order_machine::spectral_radius()` power-iteration. ──
    #[test]
    fn green_crosscheck_live_fsm_is_acyclic() {
        assert!(crate::order_machine::spectral_radius() < 1e-9);
    }

    // ── GREEN: diagonal matrix ⇒ eigenvalues are the diagonal (distinct). ──
    #[test]
    fn green_diagonal_known_spectrum() {
        let d = vec![
            vec![2.0, 0.0, 0.0],
            vec![0.0, 5.0, 0.0],
            vec![0.0, 0.0, -3.0],
        ];
        assert!(approx(spectral_radius(&d), 5.0, 1e-6));
        let mut re: Vec<f64> = eigenvalues(&d).iter().map(|e| e.re).collect();
        re.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!(approx(re[0], -3.0, 1e-6) && approx(re[1], 2.0, 1e-6) && approx(re[2], 5.0, 1e-6));
    }

    // ── GREEN: Laplacian Fiedler value. Path P₃ (0-1-2) has L-spectrum {0,1,3},
    //    so algebraic connectivity λ₂ = 1 (distinct ⇒ tight tolerance). ──
    #[test]
    fn green_path_graph_fiedler_is_one() {
        let p3 = vec![
            vec![0.0, 1.0, 0.0],
            vec![1.0, 0.0, 1.0],
            vec![0.0, 1.0, 0.0],
        ];
        assert!(approx(algebraic_connectivity(&p3), 1.0, 1e-6));
        // smallest Laplacian eigenvalue is always 0 (constant vector).
        let mut re: Vec<f64> = eigenvalues(&laplacian(&p3)).iter().map(|e| e.re).collect();
        re.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!(approx(re[0], 0.0, 1e-6));
    }

    // ── GREEN: the spectral gap separates a poorly-mixing 2-cycle from a
    //    well-mixing chain — exactly the LIMIT_CYCLE vs churn discriminator. ──
    #[test]
    fn green_spectral_gap_separates_mixing() {
        let cycle = vec![vec![0.0, 1.0], vec![1.0, 0.0]]; // eigs {1,−1} ⇒ |λ₂|=1, gap 0
        let mixing = vec![vec![0.5, 0.5], vec![0.5, 0.5]]; // eigs {1, 0} ⇒ |λ₂|=0, gap 1
        assert!(slem(&cycle) > slem(&mixing));
        assert!(spectral_gap(&mixing) > spectral_gap(&cycle));
        assert!(approx(spectral_gap(&mixing), 1.0, 1e-6));
        assert!(approx(spectral_gap(&cycle), 0.0, 1e-6));
    }

    // ── RED→GREEN: the DMD drift class discriminates contraction / margin / growth
    //    (the |μ|-vs-1 reading a symmetric solver is blind to). ──
    #[test]
    fn green_drift_class_contraction_margin_growth() {
        let damped = vec![vec![0.5, 0.0], vec![0.0, 0.3]]; // ρ=0.5 < 1
        let unstable = vec![vec![2.0, 0.0], vec![0.0, 1.5]]; // ρ=2 > 1
        let resonant = vec![vec![0.0, 1.0], vec![1.0, 0.0]]; // ρ=1 (2-cycle)
        assert_eq!(classify_drift(&damped), DriftClass::Damped);
        assert_eq!(classify_drift(&unstable), DriftClass::Unstable);
        assert_eq!(classify_drift(&resonant), DriftClass::Resonant);
    }

    // ── S0.5 FOUNDATION: parity proof that the contiguous `matmul_contig`
    //    produces bit-identical results to the old `vec![vec![]]` matmul, and a
    //    `std::time::Instant` timing of the ns delta (no criterion dep). ──
    #[test]
    fn spectral_matmul_contig_vs_vecvec() {
        // Known 3×3 matrix product used as the parity fixture.
        let a = vec![
            vec![1.0, 2.0, 3.0],
            vec![4.0, 5.0, 6.0],
            vec![7.0, 8.0, 9.0],
        ];
        let b = vec![
            vec![9.0, 8.0, 7.0],
            vec![6.0, 5.0, 4.0],
            vec![3.0, 2.0, 1.0],
        ];

        // Reference: the OLD dense `vec![vec![]]` implementation, kept inline
        // so this test is a genuine parity check against the pre-refactor math.
        fn old_matmul(a: &[Vec<f64>], b: &[Vec<f64>], n: usize) -> Vec<Vec<f64>> {
            let mut c = vec![vec![0.0; n]; n];
            for i in 0..n {
                for k in 0..n {
                    let aik = a[i][k];
                    if aik == 0.0 {
                        continue;
                    }
                    for j in 0..n {
                        c[i][j] += aik * b[k][j];
                    }
                }
            }
            c
        }

        let expected = old_matmul(&a, &b, 3);
        let am = Mat::from_vecvec(&a);
        let bm = Mat::from_vecvec(&b);
        let got = matmul_contig(&am, &bm).into_vecvec();

        // Bit-parity to 1e-12.
        for i in 0..3 {
            for j in 0..3 {
                assert!(
                    (got[i][j] - expected[i][j]).abs() < 1e-12,
                    "matmul_contig[{i}][{j}] = {} expected {}",
                    got[i][j],
                    expected[i][j]
                );
            }
        }

        // Timing delta (ns) — iterated to average out noise; printed via eprintln.
        let iters = 10_000u32;
        let start_old = std::time::Instant::now();
        for _ in 0..iters {
            let _ = old_matmul(&a, &b, 3);
        }
        let old_ns = start_old.elapsed().as_nanos() as f64 / iters as f64;
        let start_new = std::time::Instant::now();
        for _ in 0..iters {
            let _ = matmul_contig(&am, &bm);
        }
        let new_ns = start_new.elapsed().as_nanos() as f64 / iters as f64;
        eprintln!(
            "parity bench: old_vecvec={:.1} ns/call  new_contig={:.1} ns/call  delta={:+.1} ns ({:+.1}%)",
            old_ns, new_ns, new_ns - old_ns, (new_ns - old_ns) / old_ns * 100.0
        );
    }
}
