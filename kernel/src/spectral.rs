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
    /// Complex conjugate.
    pub fn conj(self) -> Self {
        Self::new(self.re, -self.im)
    }
    pub fn add(self, o: Complex) -> Complex {
        Complex::new(self.re + o.re, self.im + o.im)
    }
    pub fn sub(self, o: Complex) -> Complex {
        Complex::new(self.re - o.re, self.im - o.im)
    }
    pub fn mul(self, o: Complex) -> Complex {
        Complex::new(
            self.re * o.re - self.im * o.im,
            self.re * o.im + self.im * o.re,
        )
    }
    pub fn div(self, o: Complex) -> Complex {
        let d = o.re * o.re + o.im * o.im;
        Complex::new(
            (self.re * o.re + self.im * o.im) / d,
            (self.im * o.re - self.re * o.im) / d,
        )
    }
    /// Complex square root (principal branch).
    pub fn sqrt(self) -> Complex {
        let r = self.abs();
        let re = ((r + self.re) / 2.0).sqrt();
        let im = ((r - self.re) / 2.0).sqrt();
        // choose sign of im to match arg (so sqrt matches the half-angle)
        if self.im < 0.0 {
            Complex::new(re, -im)
        } else {
            Complex::new(re, im)
        }
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

/// Eigenvalues of a general real matrix.
///
/// Fast path: for n ≤ 32 the matrix fits the stack-only Householder engine
/// (`householder::eigenvalues_contig`) — O(n³), no heap, numerically armored,
/// captures complex conjugate pairs (e.g. the μ≈−1 period-2 cycle). The legacy
/// O(n⁴) Faddeev-LeVerrier + Durand-Kerner path is retained as a fallback for
/// n > 32 and as the parity oracle in `householder::tests`.
pub fn eigenvalues(a: &[Vec<f64>]) -> Vec<Complex> {
    let n = a.len();
    if n <= 32 {
        let mut buf = vec![0.0f64; n * n];
        for i in 0..n {
            for j in 0..n {
                buf[i * n + j] = a[i][j];
            }
        }
        return crate::householder::eigenvalues_contig(&mut buf, n);
    }
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
///
/// NaN-safe fold: a non-finite eigenvalue (numerical divergence / poisoned
/// input) is excluded from the max so this fn never returns NaN. Callers that
/// need to *reject* ill-formed operators must check [`classify_drift`], which
/// hard-fails on any non-finite entry. (P-B `RetainedBase::admit` depended on
/// the un-masked version — NaN leaked through as `Resonant` and was admitted.
/// This fold alone is NOT sufficient: see `classify_drift`'s guard.)
pub fn spectral_radius(a: &[Vec<f64>]) -> f64 {
    eigenvalues(a)
        .iter()
        .map(|e| e.abs())
        .filter(|m| m.is_finite())
        .fold(0.0, f64::max)
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

/// Graph energy E = Σ|λᵢ| over ALL eigenvalues of the adjacency matrix
/// (Gutman–Adrić, 2001). A structural invariant independent of any embedding
/// (vectorless): E is large when the spectrum spans many alternating-sign modes.
/// Bounds: for an n-vertex graph, 2(n−1) ≤ E ≤ 2n√(n−1) (empty / complete
/// extremal graphs). This is the missing spectral quantity the FSM/mesh work
/// needed — the "how active is this graph" dial, complementary to ρ (stability)
/// and λ₂ (connectivity).
pub fn graph_energy(adj: &[Vec<f64>]) -> f64 {
    eigenvalues(adj).iter().map(|e| e.abs()).sum()
}

/// One-shot spectral profile of a graph's adjacency matrix — the full
/// vectorless signature the kernel exposes: stability (ρ), mixing (|λ₂|,
/// gap), connectivity (λ₂-Laplacian), activity (energy), and drift class.
/// Single eigenvalue pass; all downstream quantities derived from the spectrum.
pub struct GraphSpectrum {
    pub spectral_radius: f64, // ρ = max|λ|  (stability)
    pub slem: f64,            // |λ₂|          (mixing rate)
    pub spectral_gap: f64,    // 1 − |λ₂|
    pub fiedler: f64,         // λ₂(L)         (algebraic connectivity)
    pub energy: f64,          // Σ|λ|          (graph activity)
    pub drift: DriftClass,    // ρ vs unit circle
}

pub fn graph_spectrum(adj: &[Vec<f64>]) -> GraphSpectrum {
    let eigs = eigenvalues(adj);
    let mut mags: Vec<f64> = eigs.iter().map(|e| e.abs()).collect();
    mags.sort_by(|x, y| y.partial_cmp(x).unwrap_or(core::cmp::Ordering::Equal));
    let rho = mags.first().copied().unwrap_or(0.0);
    let slem_v = if mags.len() > 1 { mags[1] } else { 0.0 };
    let l = laplacian(adj);
    let mut re: Vec<f64> = eigenvalues(&l).iter().map(|e| e.re).collect();
    re.sort_by(|x, y| x.partial_cmp(y).unwrap_or(core::cmp::Ordering::Equal));
    let fiedler = if re.len() > 1 { re[1] } else { 0.0 };
    let energy = mags.iter().sum();
    GraphSpectrum {
        spectral_radius: rho,
        slem: slem_v,
        spectral_gap: 1.0 - slem_v,
        fiedler,
        energy,
        drift: classify_drift(adj),
    }
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

impl DriftClass {
    /// The single authority for the numeric wire code carried across the
    /// kernel→engine FE-07 bridge (`Damped=0, Resonant=1, Unstable=2`). The wasm
    /// flat-encoder (`wasm.rs::spectral_flat_logic`) calls this instead of
    /// re-declaring the mapping inline, and the engine's `drift_from_code`
    /// (`engine/src/bridge.rs`) decodes it back — pinned round-trip in
    /// `drift_wire_contract_matches_kernel`. Exhaustive match: a new variant
    /// fails to compile here, forcing the wire code to be assigned consciously.
    pub const fn wire_code(self) -> u8 {
        match self {
            DriftClass::Damped => 0,
            DriftClass::Resonant => 1,
            DriftClass::Unstable => 2,
        }
    }
}

/// Tolerance band around ρ=1 for drift classification AND the integrity
/// hysteresis derivation. Was function-local `BAND` in `classify_drift`.
pub const DRIFT_BAND: f64 = 1e-6;

pub fn classify_drift(a: &[Vec<f64>]) -> DriftClass {
    // FAIL-CLOSED (BLUEPRINT-P-B §4.2 / gap-audit round-2): any non-finite entry
    // (NaN poison, ±inf overflow) means the rebuilt operator is ill-formed.
    // Retaining it would snapshot divergent dynamics, so it MUST classify as
    // Unstable and be rejected by `RetainedBase::admit`. The old code let NaN
    // slip through `f64::max` into `Resonant` (a silent admit) — fixed here.
    for row in a {
        for &x in row {
            if !x.is_finite() {
                return DriftClass::Unstable;
            }
        }
    }
    // FAIL-CLOSED (root-cause: index-leak / jagged-matrix OOB). A ragged
    // operator (rows of unequal length) would let `Mat::from_vecvec` stride
    // past a short row's bound on `get(i,j)` → out-of-bounds read of a
    // neighbouring element (silent corruption) or a release-path panic.
    // Reject ragged input as Unstable before any indexing happens.
    let width = a.first().map_or(0, |r| r.len());
    for row in a {
        if row.len() != width {
            return DriftClass::Unstable;
        }
    }
    // Secondary defense: if the operator cannot even be built (ragged / non-finite),
    // `from_vecvec_checked` returns Err — treat as Unstable.
    if crate::mat::Mat::from_vecvec_checked(a).is_err() {
        return DriftClass::Unstable;
    }
    let rho = spectral_radius(a);
    if rho < 1.0 - DRIFT_BAND {
        DriftClass::Damped
    } else if rho > 1.0 + DRIFT_BAND {
        DriftClass::Unstable
    } else {
        DriftClass::Resonant
    }
}

/// One-line human-readable spectral report for a graph adjacency matrix — the
/// vectorless "at-a-glance" signature. Combines the four structural invariants
/// the kernel already computes:
///   * energy          E = Σ|λ|            (graph activity, Gutman 1978)
///   * spectral_radius ρ = max|λ|          (Perron–Frobenius stability)
///   * fiedler         λ₂(L)               (algebraic connectivity)
///   * drift ∈ {Damped|Resonant|Unstable} (ρ vs the unit circle)
/// No embedding, no I/O, single pass over the spectrum via the reused helpers.
pub fn graph_energy_report(adj: &[Vec<f64>]) -> String {
    let drift = match classify_drift(adj) {
        DriftClass::Damped => "Damped",
        DriftClass::Resonant => "Resonant",
        DriftClass::Unstable => "Unstable",
    };
    format!(
        "energy={:.6} spectral_radius={:.6} fiedler={:.6} drift={}",
        graph_energy(adj),
        spectral_radius(adj),
        algebraic_connectivity(adj),
        drift,
    )
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

    // Wire-code authority pin (row #23): `DriftClass::wire_code()` is the single
    // source of the `Damped=0/Resonant=1/Unstable=2` mapping the FE-07 bridge
    // carries. The wasm encoder and the engine decoder both key off this.
    #[test]
    fn drift_wire_code_is_canonical() {
        assert_eq!(DriftClass::Damped.wire_code(), 0);
        assert_eq!(DriftClass::Resonant.wire_code(), 1);
        assert_eq!(DriftClass::Unstable.wire_code(), 2);
    }

    // ── FAIL-CLOSED (gap-audit round-2): NaN/±inf in the operator MUST NOT be
    // admitted as `Resonant`/`Damped`. They classify as `Unstable` so
    // `RetainedBase::admit` rejects them. The pre-fix code let NaN leak through
    // `f64::max` into `Resonant` (silent admit of a poisoned snapshot). ──
    #[test]
    fn red_nan_entry_classifies_unstable() {
        let poisoned = vec![vec![0.0, f64::NAN], vec![0.0, 0.0]];
        assert_eq!(classify_drift(&poisoned), DriftClass::Unstable);
    }

    #[test]
    fn red_inf_entry_classifies_unstable() {
        let poisoned = vec![vec![f64::INFINITY, 0.0], vec![0.0, 0.0]];
        assert_eq!(classify_drift(&poisoned), DriftClass::Unstable);
    }

    #[test]
    fn red_neg_inf_entry_classifies_unstable() {
        let poisoned = vec![vec![-f64::INFINITY, 0.0], vec![0.0, 0.0]];
        assert_eq!(classify_drift(&poisoned), DriftClass::Unstable);
    }

    // spectral_radius itself must never return NaN even with a poisoned input
    // (defense for the other consumers: graph_spectrum / slem / gap).
    #[test]
    fn red_spectral_radius_never_nan() {
        let poisoned = vec![vec![0.0, f64::NAN], vec![0.0, 0.0]];
        assert!(spectral_radius(&poisoned).is_finite());
    }

    // ── FAIL-CLOSED (root-cause: index-leak / jagged-matrix OOB). A ragged
    // operator (rows of unequal length) would let `Mat::from_vecvec` stride
    // past a short row's bound on `get(i,j)` → out-of-bounds read of a
    // neighbouring element (silent corruption) or release-path panic. classify_drift
    // MUST reject it as Unstable before any indexing. (Same root-cause as the
    // NaN fold — both are "malformed input admitted as healthy".)
    #[test]
    fn red_ragged_matrix_classifies_unstable() {
        // second row shorter than the first → ragged.
        let ragged = vec![vec![1.0, 2.0], vec![3.0]];
        assert_eq!(classify_drift(&ragged), DriftClass::Unstable);
        // first row shorter → ragged the other way.
        let ragged2 = vec![vec![1.0], vec![2.0, 3.0]];
        assert_eq!(classify_drift(&ragged2), DriftClass::Unstable);
    }

    // The checked constructor refuses ragged / non-finite input instead of
    // building a Mat whose `get` would read out of bounds.
    #[test]
    fn red_from_vecvec_checked_rejects_malformed() {
        use crate::mat::{Mat, MatrixError};
        assert_eq!(
            Mat::from_vecvec_checked(&vec![vec![1.0, 2.0], vec![3.0]]),
            Err(MatrixError::Ragged)
        );
        assert_eq!(
            Mat::from_vecvec_checked(&vec![vec![0.0, f64::NAN], vec![0.0, 0.0]]),
            Err(MatrixError::NonFinite)
        );
        // well-formed rectangular finite input builds fine.
        assert!(Mat::from_vecvec_checked(&vec![vec![1.0, 0.0], vec![0.0, 1.0]]).is_ok());
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

    // ── GREEN: graph energy E = Σ|λ| for the complete graph K₃. K₃ has
    //    adjacency eigenvalues {2, −1, −1} ⇒ E = 2+1+1 = 4. Also check the
    //    extremal lower bound 2(n−1)=4 for a complete graph. ──
    #[test]
    fn green_graph_energy_complete_k3() {
        // K3: every pair connected.
        let k3 = vec![
            vec![0.0, 1.0, 1.0],
            vec![1.0, 0.0, 1.0],
            vec![1.0, 1.0, 0.0],
        ];
        assert!(approx(graph_energy(&k3), 4.0, 1e-6), "E(K3)=4");
        let s = graph_spectrum(&k3);
        assert!(approx(s.energy, 4.0, 1e-6));
        assert!(approx(s.spectral_radius, 2.0, 1e-6), "ρ(K3)=2");
        assert!(s.fiedler > 0.0, "K3 is connected ⇒ λ₂(L)>0");
    }

    // ── GREEN: empty graph E=0 (all eigenvalues 0); disconnected graph has
    //    Fiedler λ₂(L)=0 ⇒ algebraic connectivity 0. ──
    #[test]
    fn green_empty_graph_zero_energy_disconnected_fiedler_zero() {
        let empty = vec![vec![0.0; 3]; 3];
        assert!(approx(graph_energy(&empty), 0.0, 1e-9));
        assert!(
            approx(graph_spectrum(&empty).fiedler, 0.0, 1e-9),
            "disconnected ⇒ λ₂=0"
        );

        // Two isolated edges (disconnected): Fiedler must be 0.
        let disc = vec![
            vec![0.0, 1.0, 0.0, 0.0],
            vec![1.0, 0.0, 0.0, 0.0],
            vec![0.0, 0.0, 0.0, 1.0],
            vec![0.0, 0.0, 1.0, 0.0],
        ];
        assert!(
            approx(graph_spectrum(&disc).fiedler, 0.0, 1e-6),
            "disconnected ⇒ λ₂=0"
        );
    }

    // ── GREEN: graph_spectrum returns the documented fields coherently for a
    //    path graph P₃. P₃ adjacency eigenvalues are {+√2, −√2, 0} (bipartite,
    //    so ±symmetric; NOT the 2-cycle which has {1,−1}). ρ=√2, energy=2√2. ──
    #[test]
    fn green_graph_spectrum_path_p3() {
        let p3 = vec![
            vec![0.0, 1.0, 0.0],
            vec![1.0, 0.0, 1.0],
            vec![0.0, 1.0, 0.0],
        ];
        let s = graph_spectrum(&p3);
        let sqrt2 = 2.0_f64.sqrt();
        assert!(approx(s.spectral_radius, sqrt2, 1e-6), "ρ(P3)=√2");
        assert!(approx(s.energy, 2.0 * sqrt2, 1e-6), "E(P3)=2√2");
        assert!(approx(s.fiedler, 1.0, 1e-6), "λ₂(L)=1");
        // For the ADJACENCY matrix the "spectral gap" = 1 − |λ₂| is a stability
        // reading for stochastic matrices; here λ₂-modulus = √2 (the −√2 eig),
        // so gap = 1 − √2 (negative — an adjacency, not a transition matrix).
        assert!(
            approx(s.spectral_gap, 1.0 - sqrt2, 1e-6),
            "gap=1−√2 for P3 adjacency"
        );
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

    // ── GREEN Verified-by-Math: graph_energy_report on K3 (complete triangle).
    //    Eigenvalues of K3 adjacency are {2, −1, −1}: E=Σ|λ|=4, ρ=2 (Unstable
    //    for an adjacency vs unit circle), Fiedler λ₂(L)=3 (Laplacian of K3 has
    //    eigenvalues {0,3,3}). All values are exact, hand-verified. ──
    #[test]
    fn green_graph_energy_report_k3() {
        let k3 = vec![
            vec![0.0, 1.0, 1.0],
            vec![1.0, 0.0, 1.0],
            vec![1.0, 1.0, 0.0],
        ];
        let report = graph_energy_report(&k3);
        assert_eq!(
            report, "energy=4.000000 spectral_radius=2.000000 fiedler=3.000000 drift=Unstable",
            "K3: E=4, ρ=2, λ₂(L)=3, ρ>1 ⇒ Unstable"
        );
    }

    // ── G4 (ROUND-2 GAP-AUDIT V3 4.2): a ragged operator (rows of unequal
    //    length) must be rejected as Unstable, never indexed (index-leak /
    //    OOB read or release-path panic). ──
    #[test]
    fn ragged_matrix_is_unstable_not_oob() {
        // 2 rows, second row one element shorter → get(i,j) would stride past
        // the short row's bound. Pre-fix this was a silent OOB read.
        let ragged = vec![vec![1.0, 0.0], vec![0.0]];
        assert_eq!(
            classify_drift(&ragged),
            DriftClass::Unstable,
            "ragged operator must be fail-closed as Unstable"
        );
        // from_vecvec_checked rejects it too (the secondary defense).
        assert_eq!(
            crate::mat::Mat::from_vecvec_checked(&ragged),
            Err(crate::mat::MatrixError::Ragged)
        );
    }

    // ── G4 (ROUND-2 GAP-AUDIT V3 4.2 / NaN root-cause): a non-finite entry
    //    anywhere must be fail-closed as Unstable, not read as a healthy
    //    spectrum (the NaN-fail-open root cause). ──
    #[test]
    fn non_finite_entry_is_unstable() {
        let nan = vec![vec![1.0, f64::NAN], vec![0.0, 1.0]];
        assert_eq!(
            classify_drift(&nan),
            DriftClass::Unstable,
            "NaN entry must be fail-closed as Unstable"
        );
        assert_eq!(
            crate::mat::Mat::from_vecvec_checked(&nan),
            Err(crate::mat::MatrixError::NonFinite)
        );

        let inf = vec![vec![1.0, f64::INFINITY], vec![0.0, 1.0]];
        assert_eq!(classify_drift(&inf), DriftClass::Unstable);
    }

    // ── GREEN Verified-by-Math: graph_energy_report on a 2-cycle. Eigenvalues
    //    ±1: E=2, ρ=1 (Resonant), Fiedler λ₂(L)=2 (Laplacian eigs {0,2}). ──
    #[test]
    fn green_graph_energy_report_two_cycle_resonant() {
        let c = vec![vec![0.0, 1.0], vec![1.0, 0.0]];
        let report = graph_energy_report(&c);
        assert_eq!(
            report, "energy=2.000000 spectral_radius=1.000000 fiedler=2.000000 drift=Resonant",
            "2-cycle: E=2, ρ=1 ⇒ Resonant, λ₂(L)=2"
        );
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
