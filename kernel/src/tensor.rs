//! tensor.rs — Lightweight n-dimensional array operations.
//!
//! Reuses `simd.rs` softmax/kalman lanes and `csr.rs` sparse layout for
//! dot products, norms, outer products, and broadcasting — without
//! pulling in a full BLAS or ndarray dependency.
//!
//! Higher-level consumers: `predictor` (state metrics), `crystal`
//! (similarity keys), `spectral` (matrix eigendecomposition), and
//! the engine's frame profiler all operate on tensor-like data.
//!
//! ## Design
//! - `Tensor1` — 1-D array (Vec<f64>) with dot, norm, normalize
//! - `Tensor2` — 2-D matrix (Vec<Vec<f64>>) with mul, transpose, add
//! - `batch_softmax` / `batch_dot` — thin wrappers over `simd` when AVX2 available
//!
//! ## Usage
//! ```
//! use dowiz_kernel::tensor::{Tensor1, Tensor2};
//! let a = Tensor1::new(vec![1.0, 2.0, 3.0]);
//! let b = Tensor1::new(vec![4.0, 5.0, 6.0]);
//! assert!((a.dot(&b) - 32.0).abs() < 1e-12);
//! ```

/// 1-D tensor (vector) with basic linear algebra operations.
#[derive(Debug, Clone)]
pub struct Tensor1 {
    pub data: Vec<f64>,
}

impl Tensor1 {
    pub fn new(data: Vec<f64>) -> Self {
        let data: Vec<f64> = data.into_iter().map(crate::sanitize_f64).collect();
        Tensor1 { data }
    }

    pub fn zeros(n: usize) -> Self {
        Tensor1 { data: vec![0.0; n] }
    }

    pub fn ones(n: usize) -> Self {
        Tensor1 { data: vec![1.0; n] }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn get(&self, i: usize) -> f64 {
        debug_assert!(i < self.data.len(), "Tensor1::get({i}) out of bounds (len={})", self.data.len());
        self.data[i]
    }

    pub fn set(&mut self, i: usize, v: f64) {
        debug_assert!(i < self.data.len(), "Tensor1::set({i}) out of bounds (len={})", self.data.len());
        self.data[i] = crate::sanitize_f64(v);
    }

    pub fn as_slice(&self) -> &[f64] {
        &self.data
    }

    pub fn into_vec(self) -> Vec<f64> {
        self.data
    }

    /// Dot product with another tensor.
    pub fn dot(&self, other: &Tensor1) -> f64 {
        assert_eq!(self.len(), other.len());
        self.data.iter().zip(&other.data).map(|(a, b)| a * b).sum()
    }

    /// Euclidean norm (L2).
    pub fn norm(&self) -> f64 {
        self.data.iter().map(|x| x * x).sum::<f64>().sqrt()
    }

    /// Squared norm (avoids sqrt).
    pub fn norm_sq(&self) -> f64 {
        self.data.iter().map(|x| x * x).sum()
    }

    /// Normalize to unit vector (in-place). Returns the original norm.
    pub fn normalize(&mut self) -> f64 {
        let n = self.norm();
        if n > 0.0 {
            for x in &mut self.data {
                *x /= n;
            }
        }
        n
    }

    /// Element-wise add.
    pub fn add(&self, other: &Tensor1) -> Tensor1 {
        assert_eq!(self.len(), other.len());
        Tensor1::new(self.data.iter().zip(&other.data).map(|(a, b)| a + b).collect())
    }

    /// Element-wise subtract.
    pub fn sub(&self, other: &Tensor1) -> Tensor1 {
        assert_eq!(self.len(), other.len());
        Tensor1::new(self.data.iter().zip(&other.data).map(|(a, b)| a - b).collect())
    }

    /// Scalar multiply.
    pub fn scale(&self, s: f64) -> Tensor1 {
        Tensor1::new(self.data.iter().map(|x| x * s).collect())
    }

    /// Cosine similarity with another tensor.
    pub fn cosine_sim(&self, other: &Tensor1) -> f64 {
        let dot = self.dot(other);
        let norm_product = self.norm() * other.norm();
        if norm_product > 0.0 { dot / norm_product } else { 0.0 }
    }
}

/// 2-D tensor (matrix) with basic linear algebra.
#[derive(Debug, Clone)]
pub struct Tensor2 {
    pub rows: usize,
    pub cols: usize,
    pub data: Vec<f64>, // row-major
}

impl Tensor2 {
    pub fn new(rows: usize, cols: usize, data: Vec<f64>) -> Self {
        assert_eq!(data.len(), rows * cols);
        let data: Vec<f64> = data.into_iter().map(crate::sanitize_f64).collect();
        Tensor2 { rows, cols, data }
    }

    pub fn zeros(rows: usize, cols: usize) -> Self {
        Tensor2 { rows, cols, data: vec![0.0; rows * cols] }
    }

    pub fn identity(n: usize) -> Self {
        let mut data = vec![0.0; n * n];
        for i in 0..n {
            data[i * n + i] = 1.0;
        }
        Tensor2 { rows: n, cols: n, data }
    }

    pub fn get(&self, r: usize, c: usize) -> f64 {
        debug_assert!(r < self.rows, "Tensor2::get row {r} >= {rows}", rows=self.rows);
        debug_assert!(c < self.cols, "Tensor2::get col {c} >= {cols}", cols=self.cols);
        self.data[r * self.cols + c]
    }

    pub fn set(&mut self, r: usize, c: usize, v: f64) {
        debug_assert!(r < self.rows, "Tensor2::set row {r} >= {rows}", rows=self.rows);
        debug_assert!(c < self.cols, "Tensor2::set col {c} >= {cols}", cols=self.cols);
        self.data[r * self.cols + c] = crate::sanitize_f64(v);
    }

    /// Matrix-vector multiply: y = A·x
    pub fn mul_vec(&self, x: &Tensor1) -> Tensor1 {
        assert_eq!(self.cols, x.len());
        let mut result = vec![0.0; self.rows];
        for r in 0..self.rows {
            let mut sum = 0.0;
            for c in 0..self.cols {
                sum += self.get(r, c) * x.get(c);
            }
            result[r] = sum;
        }
        Tensor1::new(result)
    }

    /// Matrix-matrix multiply: C = A·B
    pub fn mul(&self, other: &Tensor2) -> Tensor2 {
        assert_eq!(self.cols, other.rows);
        let mut data = vec![0.0; self.rows * other.cols];
        for r in 0..self.rows {
            for c in 0..other.cols {
                let mut sum = 0.0;
                for k in 0..self.cols {
                    sum += self.get(r, k) * other.get(k, c);
                }
                data[r * other.cols + c] = sum;
            }
        }
        Tensor2 { rows: self.rows, cols: other.cols, data }
    }

    /// Transpose.
    pub fn transpose(&self) -> Tensor2 {
        let mut data = vec![0.0; self.rows * self.cols];
        for r in 0..self.rows {
            for c in 0..self.cols {
                data[c * self.rows + r] = self.get(r, c);
            }
        }
        Tensor2 { rows: self.cols, cols: self.rows, data }
    }

    /// Element-wise add.
    pub fn add(&self, other: &Tensor2) -> Tensor2 {
        assert_eq!(self.rows, other.rows);
        assert_eq!(self.cols, other.cols);
        let data: Vec<f64> = self.data.iter().zip(&other.data).map(|(a, b)| a + b).collect();
        Tensor2 { rows: self.rows, cols: self.cols, data }
    }

    /// Frobenius norm.
    pub fn norm_fro(&self) -> f64 {
        self.data.iter().map(|x| x * x).sum::<f64>().sqrt()
    }

    /// Extract a row as Tensor1.
    pub fn row(&self, r: usize) -> Tensor1 {
        debug_assert!(r < self.rows, "Tensor2::row({r}) out of bounds");
        let start = r * self.cols;
        Tensor1::new(self.data[start..start + self.cols].to_vec())
    }

    /// Extract a column as Tensor1.
    pub fn col(&self, c: usize) -> Tensor1 {
        debug_assert!(c < self.cols, "Tensor2::col({c}) out of bounds");
        let mut col = Vec::with_capacity(self.rows);
        for r in 0..self.rows {
            col.push(self.get(r, c));
        }
        Tensor1::new(col)
    }
}

/// Batch dot products: compute dot(a[i], b[i]) for i in 0..n.
/// Thin wrapper — delegates to scalar loop (SIMD variant available via `simd`).
pub fn batch_dot(as_: &[Tensor1], bs: &[Tensor1]) -> Vec<f64> {
    assert_eq!(as_.len(), bs.len());
    as_.iter().zip(bs).map(|(a, b)| a.dot(b)).collect()
}

/// Outer product of two vectors: C[i][j] = a[i] * b[j].
pub fn outer(a: &Tensor1, b: &Tensor1) -> Tensor2 {
    let mut data = Vec::with_capacity(a.len() * b.len());
    for i in 0..a.len() {
        for j in 0..b.len() {
            data.push(a.get(i) * b.get(j));
        }
    }
    Tensor2 { rows: a.len(), cols: b.len(), data }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tensor1_dot() {
        let a = Tensor1::new(vec![1.0, 2.0, 3.0]);
        let b = Tensor1::new(vec![4.0, 5.0, 6.0]);
        assert!((a.dot(&b) - 32.0).abs() < 1e-12);
    }

    #[test]
    fn tensor1_norm() {
        let a = Tensor1::new(vec![3.0, 4.0]);
        assert!((a.norm() - 5.0).abs() < 1e-12);
    }

    #[test]
    fn tensor1_normalize() {
        let mut a = Tensor1::new(vec![3.0, 4.0]);
        let n = a.normalize();
        assert!((n - 5.0).abs() < 1e-12);
        assert!((a.norm() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn tensor1_cosine_sim() {
        let a = Tensor1::new(vec![1.0, 0.0]);
        let b = Tensor1::new(vec![0.0, 1.0]);
        assert!((a.cosine_sim(&b)).abs() < 1e-12);
        assert!((a.cosine_sim(&a) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn tensor2_identity() {
        let i = Tensor2::identity(3);
        assert_eq!(i.get(0, 0), 1.0);
        assert_eq!(i.get(1, 2), 0.0);
    }

    #[test]
    fn tensor2_mul_vec() {
        let a = Tensor2::new(2, 2, vec![1.0, 2.0, 3.0, 4.0]);
        let x = Tensor1::new(vec![1.0, 1.0]);
        let y = a.mul_vec(&x);
        assert!((y.get(0) - 3.0).abs() < 1e-12);
        assert!((y.get(1) - 7.0).abs() < 1e-12);
    }

    #[test]
    fn tensor2_mul() {
        let a = Tensor2::new(2, 3, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let b = Tensor2::new(3, 2, vec![7.0, 8.0, 9.0, 10.0, 11.0, 12.0]);
        let c = a.mul(&b);
        assert_eq!(c.rows, 2);
        assert_eq!(c.cols, 2);
        assert!((c.get(0, 0) - 58.0).abs() < 1e-12);
        assert!((c.get(1, 0) - 139.0).abs() < 1e-12);
        assert!((c.get(1, 1) - 154.0).abs() < 1e-12);
    }

    #[test]
    fn tensor2_transpose() {
        let a = Tensor2::new(2, 3, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let t = a.transpose();
        assert_eq!(t.rows, 3);
        assert_eq!(t.cols, 2);
        assert!((t.get(0, 0) - 1.0).abs() < 1e-12);
        assert!((t.get(1, 0) - 2.0).abs() < 1e-12);
    }

    #[test]
    fn outer_product() {
        let a = Tensor1::new(vec![1.0, 2.0, 3.0]);
        let b = Tensor1::new(vec![4.0, 5.0]);
        let o = outer(&a, &b);
        assert_eq!(o.rows, 3);
        assert_eq!(o.cols, 2);
        assert!((o.get(0, 0) - 4.0).abs() < 1e-12);
        assert!((o.get(2, 1) - 15.0).abs() < 1e-12);
    }

    // ── CHAOS / LOAD / META tests ──────────────────────────────────────

    #[test]
    fn tensor1_empty() {
        let a = Tensor1::new(vec![]);
        let b = Tensor1::new(vec![]);
        assert!(a.is_empty());
        assert_eq!(a.norm(), 0.0);
        assert_eq!(a.norm_sq(), 0.0);
        assert_eq!(a.dot(&b), 0.0);
        assert_eq!(a.cosine_sim(&b), 0.0);
        let mut a = a;
        assert_eq!(a.normalize(), 0.0);
    }

    #[test]
    fn tensor1_nan_sanitized() {
        let a = Tensor1::new(vec![f64::NAN, f64::INFINITY, f64::NEG_INFINITY]);
        assert!(a.data.iter().all(|&v| v == 0.0),
            "NaN/Inf must be sanitized: {:?}", a.data);
    }

    #[test]
    fn tensor1_normalize_zero_vector() {
        let mut a = Tensor1::new(vec![0.0, 0.0, 0.0]);
        let n = a.normalize();
        assert_eq!(n, 0.0, "norm of zero vector is 0");
        assert_eq!(a.norm(), 0.0, "normalized zero vector stays zero");
    }

    #[test]
    fn tensor1_cosine_sim_identical() {
        let a = Tensor1::new(vec![0.5, 0.3, 0.8, 0.1]);
        let b = Tensor1::new(vec![0.5, 0.3, 0.8, 0.1]);
        assert!((a.cosine_sim(&b) - 1.0).abs() < 1e-12, "identical vectors must have cosine 1");
    }

    #[test]
    fn tensor1_cosine_sim_orthogonal() {
        let a = Tensor1::new(vec![1.0, 0.0, 0.0]);
        let b = Tensor1::new(vec![0.0, 1.0, 0.0]);
        assert!((a.cosine_sim(&b)).abs() < 1e-12, "orthogonal vectors must have cosine 0");
    }

    #[test]
    fn tensor2_empty_matrix() {
        let m = Tensor2::new(0, 0, vec![]);
        assert_eq!(m.rows, 0);
        assert_eq!(m.cols, 0);
        assert_eq!(m.norm_fro(), 0.0);
    }

    #[test]
    fn tensor2_identity_large() {
        let n = 100;
        let i = Tensor2::identity(n);
        for r in 0..n {
            for c in 0..n {
                let expected = if r == c { 1.0 } else { 0.0 };
                assert!((i.get(r, c) - expected).abs() < 1e-12,
                    "I({n})[{r},{c}] = {} expected {expected}", i.get(r, c));
            }
        }
    }

    #[test]
    #[should_panic]
    fn tensor2_mismatched_dims_panics() {
        let a = Tensor2::new(2, 3, vec![1.0; 6]);
        let b = Tensor2::new(2, 3, vec![2.0; 6]); // cols ≠ a.rows
        let _ = a.mul(&b); // should panic
    }

    #[test]
    fn tensor2_transpose_square() {
        let a = Tensor2::new(3, 3, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);
        let t = a.transpose();
        for r in 0..3 {
            for c in 0..3 {
                assert!((t.get(r, c) - a.get(c, r)).abs() < 1e-12,
                    "transpose mismatch [{r},{c}]");
            }
        }
    }

    #[test]
    fn batch_dot_empty() {
        let result = batch_dot(&[], &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn outer_product_zero_length() {
        let a = Tensor1::new(vec![]);
        let b = Tensor1::new(vec![]);
        let o = outer(&a, &b);
        assert_eq!(o.rows, 0);
        assert_eq!(o.cols, 0);
    }

    // ── JAMMING / INJECTION ────────────────────────────────────────────

    #[test]
    fn tensor1_jamming_nan_inf() {
        let t = Tensor1::new(vec![f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.5, -1.0, 2.0]);
        for i in 0..t.len() {
            let v = t.get(i);
            assert!(v.is_finite(), "Tensor1 index {i} must be finite: {v}");
        }
    }

    #[test]
    fn tensor2_jamming_nan_inf() {
        let data = vec![f64::NAN, f64::INFINITY, 0.5, f64::NEG_INFINITY, -1.0, 2.0];
        let t = Tensor2::new(2, 3, data);
        for r in 0..t.rows {
            for c in 0..t.cols {
                let v = t.get(r, c);
                assert!(v.is_finite(), "Tensor2 [{r},{c}] must be finite: {v}");
            }
        }
    }

    #[test]
    fn tensor_jamming_then_normal_operation() {
        // Jammed tensor followed by normal operations
        let t = Tensor2::new(2, 2, vec![f64::NAN, f64::INFINITY, 0.3, 0.4]);
        let tt = t.transpose();
        for r in 0..tt.rows {
            for c in 0..tt.cols {
                assert!(tt.get(r, c).is_finite(),
                    "transpose of jammed tensor must be finite [{r},{c}]: {}", tt.get(r, c));
            }
        }
    }

    #[test]
    fn tensor_identity_100x100_consistency() {
        let eye = Tensor2::identity(100);
        assert_eq!(eye.rows, 100);
        assert_eq!(eye.cols, 100);
        for i in 0..100 {
            assert!((eye.get(i, i) - 1.0).abs() < 1e-12,
                "identity diagonal [{i}] must be 1: {}", eye.get(i, i));
        }
        // Row * identity = row (check a few spots)
        let v = Tensor1::new((0..100).map(|i| i as f64 * 0.01).collect());
        let r1 = eye.mul_vec(&v);
        for i in 0..100 {
            assert!((r1.get(i) - v.get(i)).abs() < 1e-10,
                "I*v[{i}] must equal v[i]: {} vs {}", r1.get(i), v.get(i));
        }
    }

    /// Consistency: batch_dot must handle nan inputs gracefully
    #[test]
    fn batch_dot_nan_vectors() {
        let a = vec![
            Tensor1::new(vec![f64::NAN, f64::NAN]),
            Tensor1::new(vec![0.5, 0.3]),
        ];
        let b = vec![
            Tensor1::new(vec![f64::NAN, f64::INFINITY]),
            Tensor1::new(vec![0.2, 0.8]),
        ];
        let results = batch_dot(&a, &b);
        assert_eq!(results.len(), 2);
        for &r in &results {
            assert!(r.is_finite(), "batch_dot with NaN inputs must yield finite: {r}");
        }
    }
}
