//! neon.rs — aarch64 NEON register-file kernels for the f64 hot loops
//! (eigenvectors, tensor/attention/rgb-tensor dot products).
//!
//! A 128-bit NEON register holds 2×f64, and `vfmaq_f64` does a fused
//! multiply-accumulate in a single instruction — the register-level ceiling
//! for the dot products that `eigen` (power iteration), `tensor`, `attention`
//! (QKᵀ/V), and `kalman`/`ppr` all share. On aarch64 this runs entirely in
//! the register file; elsewhere it falls back to a scalar loop.
//!
//! Zero-dep, deterministic: same accumulation order as the scalar path is
//! approximated by fixed unrolling; a parity test pins NEON == scalar within
//! fp tolerance on this host.

use alloc::vec::Vec;
/// Dot product of two `f64` slices (truncates to the shorter length).
pub fn dot_f64(a: &[f64], b: &[f64]) -> f64 {
    #[cfg(target_arch = "aarch64")]
    {
        // SAFETY: dot_f64_neon reads a/b via aligned 16-byte loads; `neon` is
        // baseline on aarch64. Truncation to min length matches the scalar path.
        return unsafe { dot_f64_neon(a, b) };
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        let n = a.len().min(b.len());
        let mut s = 0.0f64;
        for i in 0..n {
            s += a[i] * b[i];
        }
        s
    }
}

/// NEON f64x2 FMA dot: unroll by 2 (4 f64 per iteration), one 2-lane
/// accumulator, horizontal-add once at the end. `#[inline(always)]` keeps the
/// accumulator in the register file across the whole loop.
#[cfg(target_arch = "aarch64")]
#[inline(always)]
unsafe fn dot_f64_neon(a: &[f64], b: &[f64]) -> f64 {
    use core::arch::aarch64::*;
    let n = a.len().min(b.len());
    let mut acc = vdupq_n_f64(0.0);
    let mut i = 0usize;
    // 2 f64 per 128-bit register; process 4 per iter (2 registers).
    while i + 4 <= n {
        let a0 = vld1q_f64(a.as_ptr().add(i));
        let b0 = vld1q_f64(b.as_ptr().add(i));
        acc = vfmaq_f64(acc, a0, b0);
        let a1 = vld1q_f64(a.as_ptr().add(i + 2));
        let b1 = vld1q_f64(b.as_ptr().add(i + 2));
        acc = vfmaq_f64(acc, a1, b1);
        i += 4;
    }
    // 2 f64 tail.
    if i + 2 <= n {
        let a0 = vld1q_f64(a.as_ptr().add(i));
        let b0 = vld1q_f64(b.as_ptr().add(i));
        acc = vfmaq_f64(acc, a0, b0);
        i += 2;
    }
    let mut sum = vaddvq_f64(acc); // lane0 + lane1
    // 1 f64 scalar tail.
    while i < n {
        sum += a[i] * b[i];
        i += 1;
    }
    sum
}

/// NEON f64x2 FMA matrix×vector (row-major, `n` rows × `m` cols), the
/// power-iteration / eigenvector core. `y[i] = Σ_j A[i*m+j]·x[j]`.
pub fn matvec_f64(a: &[f64], x: &[f64], m: usize) -> Vec<f64> {
    let n = if m == 0 { 0 } else { a.len() / m };
    let mut y = vec![0.0f64; n];
    for i in 0..n {
        let row = &a[i * m..(i + 1) * m];
        y[i] = dot_f64(row, x);
    }
    y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dot_basic() {
        assert!((dot_f64(&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0]) - 32.0).abs() < 1e-12);
    }

    #[test]
    fn dot_truncates_to_shorter() {
        assert!((dot_f64(&[1.0, 2.0], &[3.0, 4.0, 5.0, 6.0]) - 11.0).abs() < 1e-12);
    }

    #[test]
    fn dot_empty() {
        assert_eq!(dot_f64(&[], &[]), 0.0);
        assert_eq!(dot_f64(&[1.0], &[]), 0.0);
    }

    /// NEON vs scalar parity (pins the register path to the reference).
    #[cfg(target_arch = "aarch64")]
    #[test]
    fn neon_dot_parity_with_scalar() {
        let mut a = Vec::new();
        let mut b = Vec::new();
        for i in 0..1000usize {
            a.push(crate::math::sin((i as f64 * 0.37)));
            b.push(crate::math::cos((i as f64 * 0.61)));
        }
        let neon = dot_f64(&a, &b); // NEON path
        let scalar = a.iter().zip(&b).map(|(x, y)| x * y).sum::<f64>();
        let rel = (neon - scalar).abs() / scalar.abs().max(1e-12);
        assert!(rel < 1e-9, "NEON {} vs scalar {} (rel {})", neon, scalar, rel);
    }

    #[test]
    fn matvec_shape_and_value() {
        // [[1,2],[3,4]] · [1,1] = [3,7]
        let a = [1.0, 2.0, 3.0, 4.0];
        let x = [1.0, 1.0];
        let y = matvec_f64(&a, &x, 2);
        assert_eq!(y.len(), 2);
        assert!((y[0] - 3.0).abs() < 1e-12);
        assert!((y[1] - 7.0).abs() < 1e-12);
    }

    #[test]
    fn matvec_empty() {
        assert!(matvec_f64(&[], &[], 0).is_empty());
    }
}
