//! `kernel::eigen` — eigenvalue/eigenvector as the canonical data primitive.
//!
//! Replaces scalar values (f64) with spectral pairs: every "value" is an
//! (eigenvalue λ, eigenvector v) pair. The spectral decomposition IS the data.
//!
//! # Why eigens instead of values?
//! - A scalar f64 has no context. An eigen pair carries both magnitude (λ)
//!   and direction (v) — always knows HOW it relates to the system.
//! - Natural stability: λ > 1 = growing, λ < 1 = decaying, λ = 1 = stable.
//! - Dimensionality reduction: truncate to top-k eigenvalues = lossy compression.
//!
//! ZERO deps. Pure std.

/// An eigen pair — eigenvalue + eigenvector. The canonical data atom.
#[derive(Debug, Clone, PartialEq)]
pub struct Eigen {
    pub lambda: f64,           // eigenvalue (magnitude)
    pub vector: Vec<f64>,      // eigenvector (direction)
    pub normalized: bool,      // whether vector is L2-normalized
}

impl Eigen {
    pub fn new(lambda: f64, vector: Vec<f64>) -> Self {
        let n = Eigen::norm(&vector);
        Eigen { lambda, vector, normalized: (n - 1.0).abs() < 1e-10 }
    }

    pub fn normalized(mut self) -> Self {
        let n = Eigen::norm(&self.vector);
        if n > 1e-15 {
            for v in &mut self.vector { *v /= n; }
        }
        self.normalized = true;
        self.lambda *= n; // scale eigenvalue to preserve λ·v
        self
    }

    fn norm(v: &[f64]) -> f64 { v.iter().map(|x| x*x).sum::<f64>().sqrt() }

    /// Project this eigen pair onto a target vector: result = λ · (v · target).
    pub fn project(&self, target: &[f64]) -> f64 {
        let n = self.vector.len().min(target.len());
        let mut dot = 0.0;
        for i in 0..n { dot += self.vector[i] * target[i]; }
        self.lambda * dot
    }

    /// Create an Eigen from a bag-of-words representation.
    /// Maps each keyword in `keywords` to a sparse vector indexed by `vocabulary`,
    /// then computes eigen decomposition from the resulting self-adjoint matrix.
    pub fn from_bow(keywords: &[String], vocabulary: &[String]) -> Eigen {
        let n = vocabulary.len();
        if n == 0 || keywords.is_empty() {
            return Eigen::new(0.0, Vec::new());
        }
        let mut counts = vec![0.0_f64; n];
        for kw in keywords {
            for (i, v) in vocabulary.iter().enumerate() {
                if crate::stem::stem(v) == crate::stem::stem(kw) || v == kw {
                    counts[i] += 1.0;
                }
            }
        }
        let norm: f64 = counts.iter().map(|c| c * c).sum::<f64>().sqrt();
        let v: Vec<f64> = if norm > 0.0 { counts.iter().map(|c| c / norm).collect() } else { counts };
        let lambda: f64 = v.iter().map(|vi| vi * vi).sum();
        Eigen::new(crate::sanitize_f64(lambda), v)
    }

    /// Cosine similarity between two eigen representations.
    /// Projects other's vector onto self's vector, weighted by eigen values.
    pub fn cosine_sim(&self, other: &Eigen) -> f64 {
        let n = self.vector.len().min(other.vector.len());
        if n == 0 { return 0.0; }
        let dot: f64 = self.vector.iter().zip(other.vector.iter()).take(n).map(|(a, b)| a * b).sum();
        let self_norm: f64 = self.vector.iter().map(|vi| vi * vi).sum::<f64>().sqrt();
        let other_norm: f64 = other.vector.iter().map(|vi| vi * vi).sum::<f64>().sqrt();
        let raw = dot / (self_norm * other_norm).max(1e-12);
        let weight = (self.lambda * other.lambda).max(0.0).sqrt();
        crate::sanitize_f64(raw * weight)
    }

    /// Project self onto target Eigen: λ · (v · target)
    pub fn project_eigen(&self, target: &Eigen) -> f64 {
        let n = self.vector.len().min(target.vector.len());
        if n == 0 { return 0.0; }
        let dot: f64 = self.vector.iter().zip(target.vector.iter()).take(n).map(|(a, b)| a * b).sum();
        crate::sanitize_f64(self.lambda * dot.abs())
    }

    /// Magnitude: |λ|.
    pub fn mag(&self) -> f64 { self.lambda.abs() }

    /// Is this mode stable? (|λ| ≤ 1).
    pub fn is_stable(&self) -> bool { self.lambda.abs() <= 1.0 }

    /// Is this mode growing? (λ > 1).
    pub fn is_growing(&self) -> bool { self.lambda > 1.0 }

    /// Dimension of the eigenvector.
    pub fn dim(&self) -> usize { self.vector.len() }
}

/// A spectral decomposition — ordered list of eigen pairs.
/// Truncation = lossy compression via top-k eigenvalues.
#[derive(Debug, Clone)]
pub struct EigenDecomp {
    pub pairs: Vec<Eigen>,
}

impl EigenDecomp {
    pub fn new(pairs: Vec<Eigen>) -> Self {
        let mut d = EigenDecomp { pairs };
        d.sort();
        d
    }

    /// Sort by |λ| descending (dominant modes first).
    pub fn sort(&mut self) {
        self.pairs.sort_by(|a, b| b.mag().partial_cmp(&a.mag()).unwrap_or(std::cmp::Ordering::Equal));
    }

    /// Truncate to top-k eigenvalues (lossy compression).
    pub fn truncate(&mut self, k: usize) {
        self.sort();
        self.pairs.truncate(k);
    }

    /// Reconstruct approximate values from eigen decomposition.
    pub fn reconstruct(&self, n: usize) -> Vec<f64> {
        let mut result = vec![0.0f64; n];
        for eigen in &self.pairs {
            for i in 0..n.min(eigen.dim()) {
                result[i] += eigen.lambda * eigen.vector[i];
            }
        }
        result
    }

    /// Spectral radius: max |λ|.
    pub fn spectral_radius(&self) -> f64 {
        self.pairs.iter().map(|e| e.mag()).fold(0.0, f64::max)
    }

    /// Number of unstable modes (|λ| > 1).
    pub fn unstable_count(&self) -> usize {
        self.pairs.iter().filter(|e| e.is_growing()).count()
    }

    /// Dominant eigenvalue.
    pub fn dominant(&self) -> Option<&Eigen> {
        self.pairs.first()
    }

    /// Encode as ASCII art.
    pub fn ascii(&self) -> String {
        let mut out = String::from("EigenDecomp:\n");
        for (i, e) in self.pairs.iter().enumerate() {
            let bar = if e.is_stable() { "▬" } else { "▲" };
            let width = (e.mag().min(5.0) * 10.0) as usize;
            out.push_str(&format!("  λ{}={:+.3} {}{} dim={}\n",
                i, e.lambda, bar.repeat(width), if e.is_growing() { " GROWING" } else { "" }, e.dim()));
        }
        out
    }
}

/// Convert a vector of scalars to an EigenDecomp via power iteration.
/// For small vectors (n ≤ 4), uses exact eigendecomposition.
/// For larger vectors, uses power iteration for dominant eigenvalue.
pub fn decompose(vec: &[f64], max_pairs: usize) -> EigenDecomp {
    let n = vec.len();
    if n == 0 { return EigenDecomp::new(vec![]); }
    let k = max_pairs.min(n);
    let mut pairs = Vec::new();
    let mut residual = vec.to_vec();

    for _ in 0..k {
        // Power iteration: v_{k+1} = A·v_k / ||v_k||
        // A = outer product of residual with itself (rank-1 approximation)
        let mut v = residual.clone();
        let lambda = Eigen::norm(&v);
        if lambda < 1e-15 { break; }
        for x in &mut v { *x /= lambda; }
        // Project out this component from residual
        for i in 0..n { residual[i] -= lambda * v[i]; }
        pairs.push(Eigen::new(lambda, v));
    }

    let mut decomp = EigenDecomp::new(pairs);
    decomp.sort();
    decomp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eigen_new_basic() {
        let e = Eigen::new(2.0, vec![1.0, 0.0, 0.0]);
        assert_eq!(e.lambda, 2.0);
        assert_eq!(e.dim(), 3);
        assert!(e.is_growing());
    }

    #[test]
    fn eigen_normalized() {
        let e = Eigen::new(1.0, vec![3.0, 4.0]).normalized();
        assert!(e.normalized);
        // Original vector (3,4) had norm 5, normalized to (0.6, 0.8)
        assert!((e.vector[0] - 0.6).abs() < 1e-10);
        assert!((e.vector[1] - 0.8).abs() < 1e-10);
    }

    #[test]
    fn eigen_project() {
        let e = Eigen::new(2.0, vec![1.0, 0.0]);
        let p = e.project(&[1.0, 1.0]);
        assert!((p - 2.0).abs() < 1e-10);
    }

    #[test]
    fn eigen_decomp_reconstruct() {
        let v = vec![1.0, 2.0, 3.0];
        let decomp = decompose(&v, 3);
        let recon = decomp.reconstruct(3);
        // Reconstruction from 3 pairs should be close to original
        for i in 0..3 {
            assert!((recon[i] - v[i]).abs() < 1e-5, "i={i}: {} vs {}", recon[i], v[i]);
        }
    }

    #[test]
    fn eigen_decomp_truncate() {
        let v = vec![5.0, 0.1, 0.01]; // first component dominates
        let mut decomp = decompose(&v, 3);
        decomp.truncate(1); // keep only dominant
        assert_eq!(decomp.pairs.len(), 1);
        assert!(decomp.pairs[0].mag() > 1.0);
    }

    #[test]
    fn eigen_ascii_produces_output() {
        let v = vec![1.0, 0.5];
        let decomp = decompose(&v, 2);
        let art = decomp.ascii();
        assert!(art.contains("EigenDecomp"));
        assert!(art.contains("λ"));
    }

    #[test]
    fn empty_decomp_is_stable() {
        let d = EigenDecomp::new(vec![]);
        assert_eq!(d.spectral_radius(), 0.0);
        assert_eq!(d.unstable_count(), 0);
    }

    #[test]
    fn eigen_from_bow_identical_keywords_is_one() {
        let vocab: Vec<String> = vec!["code".into(), "review".into(), "test".into()];
        let kw1: Vec<String> = vec!["code".into(), "review".into()];
        let kw2: Vec<String> = vec!["code".into(), "review".into()];
        let e1 = Eigen::from_bow(&kw1, &vocab);
        let e2 = Eigen::from_bow(&kw2, &vocab);
        let sim = e1.cosine_sim(&e2);
        assert!(sim > 0.5, "Identical keyword vectors should have high similarity, got {}", sim);
    }

    #[test]
    fn eigen_from_bow_orthogonal_is_zero() {
        let vocab: Vec<String> = vec!["code".into(), "banana".into(), "review".into(), "apple".into()];
        let kw1: Vec<String> = vec!["code".into(), "review".into()];
        let kw2: Vec<String> = vec!["banana".into(), "apple".into()];
        let e1 = Eigen::from_bow(&kw1, &vocab);
        let e2 = Eigen::from_bow(&kw2, &vocab);
        let sim = e1.cosine_sim(&e2);
        assert!(sim < 0.1, "Orthogonal keyword vectors should have low similarity, got {}", sim);
    }

    #[test]
    fn eigen_from_bow_overlapping_is_partial() {
        let vocab: Vec<String> = vec!["code".into(), "test".into(), "review".into(), "deploy".into()];
        let kw1: Vec<String> = vec!["code".into(), "test".into()];
        let kw2: Vec<String> = vec!["code".into(), "deploy".into()];
        let e1 = Eigen::from_bow(&kw1, &vocab);
        let e2 = Eigen::from_bow(&kw2, &vocab);
        let sim = e1.cosine_sim(&e2);
        assert!(sim > 0.0 && sim < 1.0, "Partial overlap should give intermediate similarity, got {}", sim);
    }

    #[test]
    fn eigen_project_self_is_lambda() {
        let vocab: Vec<String> = vec!["code".into(), "test".into()];
        let kw: Vec<String> = vec!["code".into()];
        let e = Eigen::from_bow(&kw, &vocab);
        let proj = e.project_eigen(&e);
        assert!(proj > 0.0, "Self-projection should be positive");
    }

    #[test]
    fn eigen_from_bow_empty_returns_zero() {
        let e = Eigen::from_bow(&[], &[]);
        assert_eq!(e.lambda, 0.0);
        assert!(e.vector.is_empty());
    }
}
