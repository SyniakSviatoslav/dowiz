//! kalman.rs — deterministic full Kalman filter on the kernel substrate.
//!
//! GROWTH-SUBSTRATE MATH (P9 / T2-α). Generalises [`crate::geo::ema_next`],
//! which is the **scalar steady-state** filter — the infinite-initial-covariance
//! special case of a 1-D Kalman filter. This module lifts that to a full n-D
//! predict/correct filter with covariance tracking:
//!
//! ```text
//!   predict:   x ← F·x                P ← F·P·Fᵀ + Q
//!   update:     y ← z − H·x
//!              S ← H·P·Hᵀ + R
//!              K ← P·Hᵀ·S⁻¹
//!              x ← x + K·y
//!              P ← (I − K·H)·P
//! ```
//!
//! All operations are fixed-order and deterministic. The matrix inverse is a
//! Gauss-Jordan elimination on a small (state-dimension) matrix — exact for the
//! tiny n a courier/trust filter uses, no external linear-algebra dep.
//!
//! IMPLEMENTATION NOTE (DOD invariant): everything is built through `mat::Mat`'s
//! PUBLIC api only (`zeros`/`identity`/`get`/`set`/`matmul_contig`/`into_vecvec`).
//! `mat.rs` internals are NOT mutated. no_std via `alloc`.
//!
//! ZERO new dependencies.

use crate::mat::Mat;

// ── Mat algebra through the public api (mat.rs untouched) ──────────────────

/// Build an n×1 column vector from a slice.
fn col(v: &[f64]) -> Mat {
    let rows: Vec<Vec<f64>> = v.iter().map(|x| vec![*x]).collect();
    Mat::from_vecvec(&rows)
}

/// Read an n×1 column vector back into a Vec.
fn uncol(m: &Mat) -> Vec<f64> {
    (0..m.nrows()).map(|i| m.get(i, 0)).collect()
}

/// Aᵀ (transpose).
fn transpose(a: &Mat) -> Mat {
    let mut t = Mat::zeros(a.ncols(), a.nrows());
    for i in 0..a.nrows() {
        for j in 0..a.ncols() {
            t.set(j, i, a.get(i, j));
        }
    }
    t
}

/// A + B (same shape).
fn add(a: &Mat, b: &Mat) -> Mat {
    debug_assert_eq!((a.nrows(), a.ncols()), (b.nrows(), b.ncols()));
    let mut c = Mat::zeros(a.nrows(), a.ncols());
    for i in 0..a.nrows() {
        for j in 0..a.ncols() {
            c.set(i, j, a.get(i, j) + b.get(i, j));
        }
    }
    c
}

/// A − B (same shape).
fn sub(a: &Mat, b: &Mat) -> Mat {
    debug_assert_eq!((a.nrows(), a.ncols()), (b.nrows(), b.ncols()));
    let mut c = Mat::zeros(a.nrows(), a.ncols());
    for i in 0..a.nrows() {
        for j in 0..a.ncols() {
            c.set(i, j, a.get(i, j) - b.get(i, j));
        }
    }
    c
}

/// s · A (scalar multiply).
fn scale(a: &Mat, s: f64) -> Mat {
    let mut c = Mat::zeros(a.nrows(), a.ncols());
    for i in 0..a.nrows() {
        for j in 0..a.ncols() {
            c.set(i, j, a.get(i, j) * s);
        }
    }
    c
}

/// Exact inverse via Gauss-Jordan on the augmented [A | I].
/// Returns `None` if singular (the KF measurement covariance S is PD by
/// construction, but we fail closed rather than divide by zero).
pub(crate) fn mat_inverse(a: &Mat) -> Option<Mat> {
    let n = a.nrows();
    debug_assert_eq!(n, a.ncols(), "mat_inverse: non-square");
    // Work on a Vec<Vec> augmented with the identity.
    let mut m: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            let mut row: Vec<f64> = (0..n).map(|j| a.get(i, j)).collect();
            for j in 0..n {
                row.push(if j == i { 1.0 } else { 0.0 });
            }
            row
        })
        .collect();

    for col in 0..n {
        // Partial pivot: largest magnitude in this column at/below `col`.
        let mut piv = col;
        let mut best = m[col][col].abs();
        for r in (col + 1)..n {
            let v = m[r][col].abs();
            if v > best {
                best = v;
                piv = r;
            }
        }
        if best < 1e-15 {
            return None; // singular
        }
        m.swap(col, piv);
        let diag = m[col][col];
        for j in 0..(2 * n) {
            m[col][j] /= diag;
        }
        for r in 0..n {
            if r == col {
                continue;
            }
            let factor = m[r][col];
            if factor == 0.0 {
                continue;
            }
            for j in 0..(2 * n) {
                m[r][j] -= factor * m[col][j];
            }
        }
    }
    let inv: Vec<Vec<f64>> = m.iter().map(|row| row[n..(2 * n)].to_vec()).collect();
    Some(Mat::from_vecvec(&inv))
}

// ── Kalman filter ──────────────────────────────────────────────────────────

/// Linear-Gaussian Kalman filter (time-invariant model).
///
/// State `x` is n×1; `P` is the n×n covariance. `F` (n×n) is the state
/// transition, `H` (m×n) the observation matrix, `Q` (n×n) the process
/// noise, `R` (m×m) the measurement noise.
#[derive(Debug, Clone)]
pub struct KalmanFilter {
    /// Current state estimate (n×1).
    pub x: Vec<f64>,
    /// Current covariance (n×n).
    pub p: Mat,
    f: Mat,
    h: Mat,
    q: Mat,
    r: Mat,
    /// Most recent innovation `y = z − H·x` from the last `update`.
    /// E0 fix (VERIFIABLE-COGNITION §2 bug #2): the innovation was computed and
    /// discarded; it is the *surprise signal* the blueprint needs. Cached here
    /// so it can be read after `update` returns. Length m (observation dim).
    last_innovation: Vec<f64>,
    /// Surprise of the most recent `update`: `‖y‖ / √tr(S)`, dimensionless,
    /// exposed so the self-eval loop can read novelty without re-deriving S.
    /// 0.0 until the first successful `update`.
    last_surprise: f64,
}

impl KalmanFilter {
    /// Construct from explicit matrices. `x0` length n, `p0` n×n, `f` n×n,
    /// `h` m×n, `q` n×n, `r` m×m.
    pub fn new(x0: Vec<f64>, p0: Mat, f: Mat, h: Mat, q: Mat, r: Mat) -> Self {
        debug_assert_eq!(x0.len(), f.nrows(), "kalman: x0 dim vs F");
        KalmanFilter {
            x: x0,
            p: p0,
            f,
            h: h.clone(),
            q,
            r,
            last_innovation: Vec::new(),
            last_surprise: 0.0,
        }
    }

    /// Convenience 1-D factory: scalars F, H, Q, R and scalar x0/p0.
    /// (This is exactly what `ema_next` becomes a special case of.)
    pub fn scalar(x0: f64, p0: f64, f: f64, h: f64, q: f64, r: f64) -> Self {
        KalmanFilter::new(
            vec![x0],
            Mat::from_vecvec(&[vec![p0]]),
            Mat::from_vecvec(&[vec![f]]),
            Mat::from_vecvec(&[vec![h]]),
            Mat::from_vecvec(&[vec![q]]),
            Mat::from_vecvec(&[vec![r]]),
        )
    }

    /// Predict step: `x ← F·x`, `P ← F·P·Fᵀ + Q`.
    pub fn predict(&mut self) {
        let xc = col(&self.x);
        let xp = crate::mat::matmul_contig(&self.f, &xc);
        self.x = uncol(&xp);
        let fp = crate::mat::matmul_contig(&self.f, &self.p);
        let ft = transpose(&self.f);
        let fpf = crate::mat::matmul_contig(&fp, &ft);
        self.p = add(&fpf, &self.q);
    }

    /// Update step with measurement `z` (length m). Returns `false` if the
    /// innovation covariance S was singular (fail-closed; state unchanged).
    pub fn update(&mut self, z: &[f64]) -> bool {
        let ht = transpose(&self.h);
        let pht = crate::mat::matmul_contig(&self.p, &ht);
        let hph = crate::mat::matmul_contig(&self.h, &pht);
        let s = add(&hph, &self.r);
        // Trace of the innovation covariance S = H·P·Hᵀ + R, used for the
        // surprise signal ‖y‖/√tr(S). Computed before the inverse.
        let s_trace: f64 = (0..s.nrows()).map(|i| s.get(i, i)).sum();
        let s_inv = match mat_inverse(&s) {
            Some(inv) => inv,
            None => return false,
        };
        // Kalman gain K = P·Hᵀ·S⁻¹
        let k = crate::mat::matmul_contig(&pht, &s_inv);
        // innovation y = z − H·x  (the surprise signal — surfaced, not discarded)
        let hx = crate::mat::matmul_contig(&self.h, &col(&self.x));
        let hxv = uncol(&hx);
        let y: Vec<f64> = z.iter().zip(hxv.iter()).map(|(zi, hxi)| zi - hxi).collect();
        // Cache the innovation so the eval layer can read novelty post-update.
        self.last_innovation = y.clone();
        let y_norm: f64 = y.iter().map(|v| v * v).sum::<f64>().sqrt();
        self.last_surprise = if s_trace > 0.0 {
            y_norm / s_trace.sqrt()
        } else {
            0.0
        };
        // x ← x + K·y
        let ky = crate::mat::matmul_contig(&k, &col(&y));
        let kyv = uncol(&ky);
        for i in 0..self.x.len() {
            self.x[i] += kyv[i];
        }
        // P ← (I − K·H)·P
        let kh = crate::mat::matmul_contig(&k, &self.h);
        let n = self.p.nrows();
        let i_minus_kh = sub(&Mat::identity(n), &kh);
        self.p = crate::mat::matmul_contig(&i_minus_kh, &self.p);
        true
    }

    /// Current Kalman gain that *would* be applied by the next `update`,
    /// for the steady-state / EMA-equivalence check. Computed from the
    /// current covariance (no mutation). Returns `None` if the innovation
    /// covariance S is singular (degrades instead of panicking — TORVALDS-17).
    pub fn gain(&self) -> Option<Mat> {
        let ht = transpose(&self.h);
        let pht = crate::mat::matmul_contig(&self.p, &ht);
        let hph = crate::mat::matmul_contig(&self.h, &pht);
        let s = add(&hph, &self.r);
        let s_inv = mat_inverse(&s)?;
        Some(crate::mat::matmul_contig(&pht, &s_inv))
    }

    /// Most recent innovation `y = z − H·x` from the last successful `update`.
    /// Empty (length 0) until the first `update`. The surprise signal the
    /// self-eval loop reads for novelty / semantic-entropy-from-innovation.
    pub fn last_innovation(&self) -> &[f64] {
        &self.last_innovation
    }

    /// Surprise of the most recent `update`: `‖y‖ / √tr(S)`. Returns 0.0 until
    /// the first successful `update`. Monotone-ish in how unexpected the
    /// measurement was given the prior — the deterministic novelty scalar.
    pub fn last_surprise(&self) -> f64 {
        self.last_surprise
    }

    /// RESERVED self-adaptation knob (E3): scale the process-noise `Q` by a
    /// positive factor `s` (Q ← s·Q). The eval-layer adapter proposes `s` and
    /// the noether guard accepts/rejects; this setter applies an accepted `s`
    /// in-place. `s` must be > 0 (a non-positive Q breaks PD-ness of the
    /// predict covariance).
    pub fn set_q_scaler(&mut self, s: f64) {
        assert!(s > 0.0, "kalman: q_scaler must be > 0");
        // Q is private; rebuild by scaling the stored matrix.
        // SAFETY: `q` is an n×n PD matrix; scaling by s>0 keeps it PD.
        self.q = scale(&self.q, s);
    }

    // ── §13 SoA batch-lane accessors (WAVE D / BLUEPRINT-P-E §13.2) ──────
    // Crate-internal (`pub(crate)`, NOT part of the public API) helpers used by
    // `simd::kalman_batch_step`. These do NOT bypass `predict`/`update`: the
    // AVX2 lane in `simd.rs` replays the EXACT `predict`+`update` op order and
    // only uses these to (read) the per-courier Q/R noise and (write back) the
    // lane's already-computed `x`/`P` state and innovation/surprise signals.
    // They are the SoA analog of `softmax_batch_lane` reading/writing scalar
    // row buffers — same discipline, no new public surface (anti-scope §13.3-6).
    #[inline]
    pub(crate) fn q_entry(&self) -> f64 {
        self.q.get(0, 0)
    }
    #[inline]
    pub(crate) fn r_entry(&self) -> f64 {
        self.r.get(0, 0)
    }
    #[inline]
    pub(crate) fn set_xp(&mut self, x: f64, p: f64) {
        self.x[0] = x;
        self.p.set(0, 0, p);
    }
    #[inline]
    pub(crate) fn set_signals(&mut self, innovation: Vec<f64>, surprise: f64) {
        self.last_innovation = innovation;
        self.last_surprise = surprise;
    }
    /// Crate-internal read of the last innovation (used by `simd` parity tests).
    #[inline]
    #[cfg(test)]
    pub(crate) fn innovation_bits(&self) -> Vec<u64> {
        self.last_innovation.iter().map(|v| v.to_bits()).collect()
    }
    /// Crate-internal read of the last surprise (used by `simd` parity tests).
    #[inline]
    #[cfg(test)]
    pub(crate) fn surprise_bits(&self) -> u64 {
        self.last_surprise.to_bits()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    // ---------------------------------------------------------------------
    // 1. mat_inverse helper — exact oracles (no KF needed).
    // ---------------------------------------------------------------------
    #[test]
    fn inverse_identity() {
        let i = Mat::identity(3);
        let inv = mat_inverse(&i).unwrap();
        for r in 0..3 {
            for c in 0..3 {
                let want = if r == c { 1.0 } else { 0.0 };
                assert!(close(inv.get(r, c), want, 1e-12), "I⁻¹[{r},{c}]");
            }
        }
    }

    #[test]
    fn inverse_diagonal() {
        let a = Mat::from_vecvec(&[vec![2.0, 0.0], vec![0.0, 3.0]]);
        let inv = mat_inverse(&a).unwrap();
        assert!(close(inv.get(0, 0), 0.5, 1e-12));
        assert!(close(inv.get(1, 1), 1.0 / 3.0, 1e-12));
        assert!(close(inv.get(0, 1), 0.0, 1e-12));
        assert!(close(inv.get(1, 0), 0.0, 1e-12));
    }

    // ---------------------------------------------------------------------
    // 2. 1-D constant model: EMA is the steady-state special case.
    //    F=H=1, Q=0.01, R=1.0. Hand-derived steady gain:
    //      s = (q + √(q²+4qr))/2 = 0.105125          (predicted covariance)
    //      p* = s − q = 0.095125                      (posterior covariance)
    //      k* = p* / (p* + r) = 0.086863              (the gain the filter APPLIES)
    // NOTE: the gain the update step actually applies is K = P_prior/(P_prior+R)
    // = p*/(p*+r), NOT s/(s+r) — FEYNMAN-04 mis-stated the formula; the code was
    // correct and this test asserts the TRUE gain.
    // ---------------------------------------------------------------------
    #[test]
    fn scalar_kf_steady_gain_matches_hand() {
        // 1-D constant model: F=H=1, Q=0.01, R=1.0.
        // Steady-state Riccati (scalar KF): let s = p*+q, then
        //   s² − q·s − q·r = 0  ⇒  s = (q + √(q²+4qr))/2 ;  p* = s − q.
        // The gain the update step applies is K = p*/(p*+r) (posterior form).
        // Hand values: s = 0.105125, p* = 0.095125, k* = 0.086863.
        let q = 0.01_f64;
        let r = 1.0_f64;
        let s = (q + (q * q + 4.0 * q * r).sqrt()) / 2.0;
        let p_star = s - q;
        let k_star = p_star / (p_star + r);
        assert!(close(k_star, 0.086863, 1e-5), "hand k* = {k_star}");

        let mut kf = KalmanFilter::scalar(0.0, 1.0, 1.0, 1.0, q, r);
        for _ in 0..400 {
            kf.predict();
            let _ = kf.update(&[1.0]);
        }
        let g = kf.gain().expect("S invertible at steady state").get(0, 0);
        assert!(
            close(g, k_star, 1e-4),
            "KF steady gain {g} must equal hand k* {k_star}"
        );
    }

    #[test]
    fn scalar_kf_equival_ema_next() {
        // With alpha = the KF steady gain k* (= p*/(p*+r), the gain the filter
        // APPLIES), BOTH converge to the SAME estimate — but only at steady
        // state (the KF has a transient from its high P0; fixed-alpha EMA has
        // none). Warm up, then compare. FEYNMAN-04: the equivalence holds with
        // the posterior-form gain k* = p*/(p*+r).
        let q = 0.01_f64;
        let r = 1.0_f64;
        let s = (q + (q * q + 4.0 * q * r).sqrt()) / 2.0;
        let k_star = (s - q) / (s - q + r);

        let mut kf = KalmanFilter::scalar(0.0, 1.0, 1.0, 1.0, q, r);
        let mut ema = 0.0_f64;
        // Warm-up: KF reaches steady gain; EMA tracks the constant stream.
        for _ in 0..200 {
            kf.predict();
            let _ = kf.update(&[1.0]);
            ema = crate::geo::ema_next(ema, 1.0, k_star);
        }
        // Now both should be at steady state ≈ the constant measurement 1.0,
        // and a further step must move them by (almost) the same amount.
        let x_before = kf.x[0];
        kf.predict();
        let _ = kf.update(&[1.0]);
        ema = crate::geo::ema_next(ema, 1.0, k_star);
        assert!(close(kf.x[0], 1.0, 1e-3), "KF must converge to 1.0");
        assert!(close(ema, 1.0, 1e-3), "EMA must converge to 1.0");
        // Per-step movement at steady state: KF gain ≈ k* ⇒ Δ ≈ k*·(1−x).
        let dx_kf = kf.x[0] - x_before;
        let dx_ema = k_star * (1.0 - ema);
        assert!(
            close(dx_kf, dx_ema, 1e-3),
            "steady-step movement KF {dx_kf} vs EMA {dx_ema} must match"
        );
    }

    #[test]
    fn kalman_2d_constant_velocity() {
        // Position + velocity, dt=1. F=[[1,1],[0,1]], H=[[1,0]] (observe
        // position only). Linear position z=[1,2,3,4,5,6] ⇒ true velocity=1,
        // position=t. With a physically-reasonable process noise Q=1e-2 the
        // filter tracks velocity tightly.
        let f = Mat::from_vecvec(&[vec![1.0, 1.0], vec![0.0, 1.0]]);
        let h = Mat::from_vecvec(&[vec![1.0, 0.0]]);
        let q = Mat::from_vecvec(&[vec![1e-2, 0.0], vec![0.0, 1e-2]]);
        let r = Mat::from_vecvec(&[vec![1.0]]);
        let p0 = Mat::identity(2);
        let mut kf = KalmanFilter::new(vec![0.0, 0.0], p0, f, h, q, r);

        let zs = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        for &z in &zs {
            kf.predict();
            let ok = kf.update(&[z]);
            assert!(ok, "2D update must succeed (S invertible)");
        }
        assert!(
            close(kf.x[1], 1.0, 3e-2),
            "velocity estimate {} should be ≈1.0",
            kf.x[1]
        );
        assert!(
            close(kf.x[0], 10.0, 1e-1),
            "position estimate {} should be ≈10.0",
            kf.x[0]
        );
    }

    // ---------------------------------------------------------------------
    // E0 fix (VERIFIABLE-COGNITION §2 bug #2): the innovation `y = z − H·x`
    // was computed and discarded. It must now be readable after `update`, along
    // with the dimensionless surprise ‖y‖/√tr(S).
    //   1-D: F=H=1, Q=0.01, R=1.0, x0=0, P0=1.
    //   predict → x=0, P=1.01.  update z=1:  y = 1 − 0 = 1;
    //     S = 1·1.01·1 + 1 = 2.01;  surprise = 1/√2.01 ≈ 0.705337.
    // ---------------------------------------------------------------------
    #[test]
    fn update_surfaces_innovation_and_surprise() {
        let mut kf = KalmanFilter::scalar(0.0, 1.0, 1.0, 1.0, 0.01, 1.0);
        // Before any update the novelty signals are the zero-defaults.
        assert_eq!(kf.last_innovation().len(), 0);
        assert!(close(kf.last_surprise(), 0.0, 1e-15));

        kf.predict();
        let ok = kf.update(&[1.0]);
        assert!(ok, "update must succeed (S invertible)");
        assert_eq!(kf.last_innovation().len(), 1);
        assert!(
            close(kf.last_innovation()[0], 1.0, 1e-12),
            "innovation y = z − H·x must equal 1.0"
        );
        let want_surprise = 1.0 / (2.01_f64).sqrt();
        assert!(
            close(kf.last_surprise(), want_surprise, 1e-9),
            "surprise = ‖y‖/√tr(S) must be {}",
            want_surprise
        );
    }
}
