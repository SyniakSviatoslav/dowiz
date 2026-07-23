//! numerical_guard.rs — zero-dep numerical stability primitives.
//!
//! Guard against floating-point error accumulation in hot summation paths
//! (spectral, absorbing, stats, online learners). Every primitive is pure-std,
//! deterministic, and benchmarked under `spectral_math`.

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
        *x = (*x - max).exp();
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
fn dot(x: &[f64], y: &[f64]) -> f64 {
    x.iter().zip(y.iter()).map(|(a, b)| a * b).sum()
}

#[inline]
fn norm(x: &[f64]) -> f64 {
    dot(x, x).sqrt()
}

#[inline]
fn mat_vec_mul(a: &[Vec<f64>], x: &[f64], out: &mut [f64]) {
    let n = a.len();
    for i in 0..n {
        out[i] = dot(&a[i], x);
    }
}

#[inline]
fn mat_vec_mul_transpose(a: &[Vec<f64>], x: &[f64], out: &mut [f64]) {
    let n = a.len();
    let m = a.first().map_or(0, |r| r.len());
    out[..m].fill(0.0);
    for i in 0..n {
        let row = &a[i];
        let xi = x[i];
        for j in 0..m {
            out[j] += row[j] * xi;
        }
    }
}

fn power_iteration_sigma_max(a: &[Vec<f64>], max_iter: usize, tol: f64) -> f64 {
    let n = a.len();
    if n == 0 {
        return 0.0;
    }
    let m = a[0].len();
    if m == 0 {
        return 0.0;
    }
    let mut v = vec![1.0 / (n as f64).sqrt(); n];
    let mut av = vec![0.0; m];
    let mut atav = vec![0.0; n];
    let mut sigma = 0.0;
    for _ in 0..max_iter {
        mat_vec_mul(a, &v, &mut av);
        mat_vec_mul_transpose(a, &av, &mut atav);
        let new_sigma = norm(&atav);
        if (new_sigma - sigma).abs() < tol * new_sigma.max(1e-12) {
            sigma = new_sigma;
            break;
        }
        sigma = new_sigma;
        if sigma > 0.0 {
            let inv = 1.0 / sigma;
            for i in 0..n {
                v[i] = atav[i] * inv;
            }
        }
    }
    sigma.sqrt()
}

/// Solve A x = b by Gaussian elimination with partial pivoting (square, fail-closed).
fn gaussian_solve(a: &[Vec<f64>], b: &[f64]) -> Vec<f64> {
    let n = a.len();
    let mut aug: Vec<Vec<f64>> = a.iter().zip(b.iter()).map(|(row, &bv)| {
        let mut r = row.clone();
        r.push(bv);
        r
    }).collect();

    for col in 0..n {
        let mut pivot_row = col;
        let mut pivot_val = aug[col][col].abs();
        for row in (col + 1)..n {
            let v = aug[row][col].abs();
            if v > pivot_val {
                pivot_val = v;
                pivot_row = row;
            }
        }
        if pivot_val < 1e-14 {
            return vec![0.0; n];
        }
        aug.swap(col, pivot_row);
        let inv_pivot = 1.0 / aug[col][col];
        for j in col..=n {
            aug[col][j] *= inv_pivot;
        }
        for row in 0..n {
            if row == col {
                continue;
            }
            let factor = aug[row][col];
            for j in col..=n {
                aug[row][j] -= factor * aug[col][j];
            }
        }
    }
    aug.iter().map(|row| row[n]).collect()
}

fn power_iteration_sigma_min(a: &[Vec<f64>], max_iter: usize, tol: f64) -> f64 {
    let n = a.len();
    if n == 0 {
        return 1.0;
    }
    let mut v = vec![1.0 / (n as f64).sqrt(); n];
    let mut av = vec![0.0; n];
    let mut lambda = 0.0;
    for _ in 0..max_iter {
        let x = gaussian_solve(a, &v);
        mat_vec_mul_transpose(a, &x, &mut av);
        let w = gaussian_solve(a, &av);
        let new_lambda = norm(&w);
        if (new_lambda - lambda).abs() < tol * new_lambda.max(1e-12) {
            lambda = new_lambda;
            break;
        }
        lambda = new_lambda;
        if lambda > 0.0 {
            let inv = 1.0 / lambda;
            for i in 0..n {
                v[i] = w[i] * inv;
            }
        }
    }
    if lambda < 1e-14 {
        1e14
    } else {
        1.0 / lambda.sqrt()
    }
}

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
