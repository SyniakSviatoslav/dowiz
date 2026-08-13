//! gboost.rs — gradient boosting with regression stumps (item #15, XGBoost).
//!
//! XGBoost's core is additive gradient boosting of decision trees on the
//! residuals. This is a minimal, zero-dep, deterministic implementation:
//! squared loss ⇒ negative gradient = residual; each weak learner is a
//! depth-1 regression stump (best single feature/threshold split). Not a
//! production XGBoost — but the *algorithm* is real and the substrate (vector
//! math over `Vec<f64>`) is the kernel's own.

/// A regression stump: split on feature `j` at threshold `t`; predict
/// `left` for x[j] ≤ t, else `right`.
#[derive(Debug, Clone, PartialEq)]
pub struct Stump {
    j: usize,
    t: f64,
    left: f64,
    right: f64,
}

impl Stump {
    pub fn predict(&self, x: &[f64]) -> f64 {
        if x[self.j] <= self.t {
            self.left
        } else {
            self.right
        }
    }
}

/// Fit the best depth-1 stump to minimize squared error on (X, y).
/// `X` is n × d (row-major). Returns `None` if empty or d == 0.
pub fn fit_stump(x: &[Vec<f64>], y: &[f64]) -> Option<Stump> {
    let n = x.len();
    if n == 0 || x[0].is_empty() {
        return None;
    }
    let d = x[0].len();
    let mut best: Option<Stump> = None;
    let mut best_sse = f64::INFINITY;

    for j in 0..d {
        // Candidate thresholds: midpoints between sorted unique values.
        let mut vals: Vec<f64> = x.iter().map(|r| r[j]).collect();
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        vals.dedup_by(|a, b| (*a - *b).abs() < 1e-12);

        for w in vals.windows(2) {
            let t = (w[0] + w[1]) / 2.0;
            let (mut sl, mut sln, mut sr, mut srn) = (0.0, 0usize, 0.0, 0usize);
            for i in 0..n {
                if x[i][j] <= t {
                    sl += y[i];
                    sln += 1;
                } else {
                    sr += y[i];
                    srn += 1;
                }
            }
            if sln == 0 || srn == 0 {
                continue; // degenerate split
            }
            let left = sl / sln as f64;
            let right = sr / srn as f64;
            // SSE of the split.
            let mut sse = 0.0;
            for i in 0..n {
                let pred = if x[i][j] <= t { left } else { right };
                let e = y[i] - pred;
                sse += e * e;
            }
            if sse < best_sse {
                best_sse = sse;
                best = Some(Stump { j, t, left, right });
            }
        }
    }
    best
}

/// A gradient-boosted ensemble of stumps.
#[derive(Debug, Clone, PartialEq)]
pub struct GBoost {
    base: f64,
    trees: Vec<Stump>,
    learning_rate: f64,
}

impl GBoost {
    /// Train on (X, y) with `n_trees` weak learners and `learning_rate`.
    pub fn train(x: &[Vec<f64>], y: &[f64], n_trees: usize, learning_rate: f64) -> Option<Self> {
        if x.is_empty() || y.len() != x.len() {
            return None;
        }
        let base = y.iter().sum::<f64>() / y.len() as f64;
        let mut model = GBoost { base, trees: Vec::with_capacity(n_trees), learning_rate };

        // Working predictions (start at base).
        let mut pred = vec![base; y.len()];
        for _ in 0..n_trees {
            // Residuals (negative gradient of squared loss).
            let resid: Vec<f64> = y.iter().zip(&pred).map(|(yi, p)| yi - p).collect();
            let stump = fit_stump(x, &resid)?;
            for i in 0..y.len() {
                pred[i] += learning_rate * stump.predict(&x[i]);
            }
            model.trees.push(stump);
        }
        Some(model)
    }

    /// Predict a single row.
    pub fn predict_one(&self, x: &[f64]) -> f64 {
        let mut v = self.base;
        for t in &self.trees {
            v += self.learning_rate * t.predict(x);
        }
        v
    }

    /// Predict many rows.
    pub fn predict(&self, xs: &[Vec<f64>]) -> Vec<f64> {
        xs.iter().map(|x| self.predict_one(x)).collect()
    }

    /// Mean squared error against targets.
    pub fn mse(&self, xs: &[Vec<f64>], y: &[f64]) -> f64 {
        let p = self.predict(xs);
        let n = y.len().max(1) as f64;
        p.iter().zip(y).map(|(pi, yi)| (pi - yi).powi(2)).sum::<f64>() / n
    }

    pub fn num_trees(&self) -> usize {
        self.trees.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_stump_rejects_empty() {
        assert_eq!(fit_stump(&[], &[]), None);
        assert_eq!(fit_stump(&[vec![]], &[1.0]), None);
    }

    #[test]
    fn stump_splits_on_best_feature() {
        // Two features; only feature 0 separates y.
        let x = vec![
            vec![0.0, 0.0], vec![0.0, 1.0], vec![0.0, 2.0],
            vec![10.0, 0.0], vec![10.0, 1.0], vec![10.0, 2.0],
        ];
        let y = vec![0.0, 0.0, 0.0, 10.0, 10.0, 10.0];
        let s = fit_stump(&x, &y).unwrap();
        assert_eq!(s.j, 0);
        assert!(s.t > 0.0 && s.t < 10.0);
        assert!((s.left - 0.0).abs() < 1e-9);
        assert!((s.right - 10.0).abs() < 1e-9);
    }

    #[test]
    fn gboost_reduces_mse_vs_baseline() {
        // y = 3*x0 - 2*x1 + noise-free linear target; boosting should fit it.
        let mut x = Vec::new();
        let mut y = Vec::new();
        for i in 0..100 {
            let a = (i % 10) as f64 / 9.0;
            let b = ((i / 10) % 10) as f64 / 9.0;
            x.push(vec![a, b]);
            y.push(3.0 * a - 2.0 * b);
        }
        let model = GBoost::train(&x, &y, 20, 0.3).unwrap();
        let mse = model.mse(&x, &y);
        // Baseline: predict the mean.
        let mean = y.iter().sum::<f64>() / y.len() as f64;
        let base_mse = y.iter().map(|yi| (yi - mean).powi(2)).sum::<f64>() / y.len() as f64;
        assert!(mse < base_mse * 0.2, "mse {mse} should be well below baseline {base_mse}");
    }

    #[test]
    fn gboost_rejects_mismatched_shapes() {
        assert_eq!(GBoost::train(&[vec![1.0]], &[], 5, 0.1), None);
        assert_eq!(GBoost::train(&[], &[], 5, 0.1), None);
    }

    #[test]
    fn predict_matches_manual_accumulation() {
        let x = vec![vec![0.0], vec![1.0], vec![2.0]];
        let y = vec![1.0, 3.0, 5.0];
        let m = GBoost::train(&x, &y, 3, 0.5).unwrap();
        let one = m.predict_one(&[0.0]);
        let batch = m.predict(&x);
        assert!((one - batch[0]).abs() < 1e-12);
        assert_eq!(batch.len(), 3);
    }
}
