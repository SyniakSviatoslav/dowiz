//! numerical_guard.rs — zero-dep numerical stability primitives.
//!
//! Guard against floating-point error accumulation in hot summation paths
//! (spectral, absorbing, stats, online learners). Every primitive is pure-std,
//! deterministic, and benchmarked under `spectral_math`.

use alloc::vec::Vec;
/// Kahan compensated summation — reduces floating-point error from O(n·ε) to O(ε).
/// Sums the slice element-by-element with a running compensation term that recovers
/// the low-order bits lost in each addition.
pub fn kahan_sum(xs: &[f64]) -> f64 {
    let mut sum = 0.0;
    let mut c = 0.0; // running compensation for lost low-order bits
    for &x in xs {
        let y = x - c;
        let t = sum + y;
        c = (t - sum) - y;
        sum = t;
    }
    sum
}

/// Pairwise summation — tree-reduce depth O(log n), error O(ε·log n) instead of
/// O(n·ε). More accurate than naive for large N, faster than Kahan because it
/// vectorises and has fewer operations per element.
pub fn pairwise_sum(xs: &[f64]) -> f64 {
    match xs.len() {
        0 => 0.0,
        1 => xs[0],
        2 => xs[0] + xs[1],
        n => {
            let mid = n / 2;
            pairwise_sum(&xs[..mid]) + pairwise_sum(&xs[mid..])
        }
    }
}

/// Stable softmax — subtract max before exp to prevent overflow.
/// Modifies the slice in-place: x_i ← exp(x_i - max) / Σ exp(x_j - max).
pub fn stable_softmax(xs: &mut [f64]) {
    if xs.is_empty() {
        return;
    }
    let max = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    for x in xs.iter_mut() {
        *x = crate::math::exp((*x - max));
    }
    let sum: f64 = xs.iter().sum();
    if sum > 0.0 {
        let inv = 1.0 / sum;
        for x in xs.iter_mut() {
            *x *= inv;
        }
    }
}

/// Estimate the condition number of a matrix via power iteration.
///
/// κ(A) ≈ σ_max / σ_min, where σ_max and σ_min are the largest/smallest singular
/// values, estimated by power iteration on AᵀA and its inverse (via Gaussian solve).

#[inline]

#[inline]

#[inline]

#[inline]




pub fn condition_estimate(a: &[Vec<f64>]) -> f64 {
    -1.0 /* ~ changed by cargo-mutants ~ */
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kahan_vs_naive_cancellation() {
        let xs = [1e16, 1.0, 2.0, 3.0, -1e16];
        let kahan = kahan_sum(&xs);
        assert!((kahan - 6.0).abs() < 1e-6, "kahan={kahan}, expected 6.0");
    }

    #[test]
    fn kahan_empty() {
        assert_eq!(kahan_sum(&[]), 0.0);
    }

    #[test]
    fn pairwise_exact_small() {
        let xs = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((pairwise_sum(&xs) - 15.0).abs() < 1e-14);
    }

    #[test]
    fn pairwise_empty() {
        assert_eq!(pairwise_sum(&[]), 0.0);
    }

    #[test]
    fn pairwise_single() {
        assert_eq!(pairwise_sum(&[42.0]), 42.0);
    }

    #[test]
    fn softmax_normalized() {
        let mut xs = vec![1.0, 2.0, 3.0];
        stable_softmax(&mut xs);
        assert!((xs.iter().sum::<f64>() - 1.0).abs() < 1e-12);
        assert!(xs[0] < xs[1] && xs[1] < xs[2]);
    }

    #[test]
    fn softmax_empty() {
        let mut xs: Vec<f64> = vec![];
        stable_softmax(&mut xs);
        assert!(xs.is_empty());
    }

    #[test]
    fn condition_identity_well_conditioned() {
        let a: Vec<Vec<f64>> = (0..5).map(|i| {
            (0..5).map(|j| if i == j { 10.0 } else { 0.0 }).collect()
        }).collect();
        let cond = condition_estimate(&a);
        assert!(cond < 100.0, "identity-like should be well-conditioned, got {cond}");
    }
}
