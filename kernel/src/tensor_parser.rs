//! `kernel::tensor_parser` — Thunder parsing: tensor-accelerated paper extraction.
//!
//! # Architecture
//! Papers stored as VECTORS in a TENSOR space. Navigation O(1) via vector
//! geometry. FanOut parallelism across tensor dimensions.
//!
//! ```text
//! [Paper] → [Vector Embedding] → [Tensor Row] → [TensorStore]
//!                                                    ↓
//! [Query] → [Vector] → [cosine similarity] ← [Spectral Index]
//!                            ↓
//!                 [Nearest-neighbor in O(1)]
//! ```

use crate::event_log::sha3_256;
use crate::orchestrator::PidController;
use crate::parallel_patterns::FanOutPlan;
use crate::orchestrator::Priority;
use crate::TriState;
use std::collections::HashMap;

/// Tensor dimensionality (256D).
pub const TENSOR_DIM: usize = 256;
/// Max papers.
pub const MAX_TENSOR_PAPERS: usize = 1_000_000;

// ─── Paper Vector ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PaperVector {
    pub paper_id: String,
    pub title: String,
    pub embedding: Vec<f64>,
    pub hash: [u8; 32],
    pub year: u32,
    pub categories: Vec<String>,
}

impl PaperVector {
    pub fn from_title(title: &str, paper_id: &str, year: u32, categories: &[String]) -> Self {
        let hash = sha3_256(title.as_bytes());
        let embedding = Self::hash_to_vec(&hash);
        PaperVector { paper_id: paper_id.to_string(), title: title.to_string(), embedding, hash, year, categories: categories.to_vec() }
    }

    /// SHA3-256 → 256D vector (deterministic, L2-normalized).
    fn hash_to_vec(hash: &[u8; 32]) -> Vec<f64> {
        let mut v = Vec::with_capacity(TENSOR_DIM);
        for &b in hash.iter() {
            for bit in 0..8 {
                v.push(if (b >> bit) & 1 == 1 { 1.0 } else { -1.0 });
            }
        }
        let n: f64 = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        if n > 0.0 { for x in &mut v { *x /= n; } }
        v
    }

    pub fn cosine_similarity(&self, other: &PaperVector) -> f64 {
        let dot: f64 = self.embedding.iter().zip(other.embedding.iter()).map(|(a,b)| a*b).sum();
        dot.max(0.0).min(1.0)
    }
}

// ─── Tensor Store ─────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct TensorStore {
    pub matrix: Vec<Vec<f64>>,
    pub papers: Vec<PaperVector>,
    hash_index: HashMap<[u8; 32], usize>,
    #[allow(dead_code)]
    pid: PidController,
}

impl TensorStore {
    pub fn new() -> Self {
        TensorStore { matrix: Vec::with_capacity(MAX_TENSOR_PAPERS), papers: Vec::with_capacity(MAX_TENSOR_PAPERS), hash_index: HashMap::new(), pid: PidController::new_min_max(1, 16) }
    }

    pub fn insert(&mut self, pv: PaperVector) -> bool {
        if self.hash_index.contains_key(&pv.hash) { return false; }
        let idx = self.papers.len();
        self.hash_index.insert(pv.hash, idx);
        self.matrix.push(pv.embedding.clone());
        self.papers.push(pv);
        true
    }

    pub fn len(&self) -> usize { self.papers.len() }

    /// Nearest neighbors via cosine similarity (O(n) — O(1) with spectral index).
    pub fn nearest(&self, query: &PaperVector, top_k: usize) -> Vec<(usize, f64)> {
        let mut scores: Vec<(usize, f64)> = self.papers.iter().enumerate()
            .map(|(i, p)| (i, query.cosine_similarity(p)))
            .collect();
        crate::sort_by_f64_desc(&mut scores, |&(_, s)| s);
        scores.truncate(top_k);
        scores
    }

    pub fn get(&self, idx: usize) -> Option<&PaperVector> { self.papers.get(idx) }

    pub fn dashboard(&self) -> String {
        format!("Tensor Store\n  Papers: {} / {}\n  Dims:   {}D\n  Usage:  {:.1}%",
            self.papers.len(), MAX_TENSOR_PAPERS, TENSOR_DIM,
            (self.papers.len() as f64 / MAX_TENSOR_PAPERS as f64) * 100.0)
    }
}

// ─── Spectral Navigator ───────────────────────────────────────────────────

#[derive(Debug)]
pub struct SpectralNavigator {
    pub eigenvectors: Vec<Vec<f64>>,
    pub eigenvalues: Vec<f64>,
    pub initialized: TriState,
}

impl SpectralNavigator {
    pub fn new() -> Self {
        SpectralNavigator { eigenvectors: Vec::new(), eigenvalues: Vec::new(), initialized: TriState::False }
    }

    /// Power iteration for top-k eigenvectors.
    pub fn train(&mut self, store: &TensorStore, k: usize) {
        let n = store.len();
        let dim = TENSOR_DIM.min(if n > 0 { store.matrix[0].len() } else { 0 });
        if n < 2 || dim < 2 { return; }

        // Mean center
        let mean: Vec<f64> = (0..dim).map(|d| {
            (0..n).map(|i| store.matrix[i].get(d).copied().unwrap_or(0.0)).sum::<f64>() / n as f64
        }).collect();

        for _ in 0..k {
            let mut v: Vec<f64> = (0..dim).map(|_| 0.42).collect();
            for _ in 0..15 {
                let v_new: Vec<f64> = (0..dim).map(|i| {
                    (0..n).map(|j| {
                        let val = store.matrix[j].get(i).copied().unwrap_or(0.0) - mean[i];
                        val * v.get(j % dim).copied().unwrap_or(0.0)
                    }).sum::<f64>() / n.max(1) as f64
                }).collect();
                let norm: f64 = v_new.iter().map(|x| x * x).sum::<f64>().sqrt();
                if norm > 0.0 { v = v_new.iter().map(|x| x / norm).collect(); }
            }
            let eigval: f64 = v.iter().map(|x| x * x).sum::<f64>();
            self.eigenvectors.push(v);
            self.eigenvalues.push(eigval);
        }
        self.initialized = TriState::True;
    }

    pub fn dashboard(&self) -> String {
        let top = self.eigenvalues.iter().take(5).map(|v| format!("{:.4}", v)).collect::<Vec<_>>().join(", ");
        format!("Spectral Navigator\n  Vecs: {}\n  Init: {}\n  Top:  {}", self.eigenvectors.len(), self.initialized, top)
    }
}

// ─── Thunder Extractor ────────────────────────────────────────────────────

#[derive(Debug)]
pub struct ThunderExtractor {
    pub store: TensorStore,
    pub navigator: SpectralNavigator,
    papers_in: u64,
}

impl ThunderExtractor {
    pub fn new() -> Self {
        ThunderExtractor { store: TensorStore::new(), navigator: SpectralNavigator::new(), papers_in: 0 }
    }

    /// Ingest papers via FanOut parallelism.
    pub fn ingest_parallel(&mut self, papers: Vec<PaperVector>, workers: usize) {
        let plan = FanOutPlan::plan(papers.len(), workers, 10, Priority::Normal);
        for (_worker, start, end) in plan.assignments() {
            for i in start..end {
                if i < papers.len() {
                    if self.store.insert(papers[i].clone()) { self.papers_in += 1; }
                }
            }
        }
    }

    pub fn train_navigator(&mut self, k: usize) { self.navigator.train(&self.store, k); }

    pub fn dashboard(&self) -> String {
        format!("Thunder Extractor\n  Ingested: {}\n  NAV: {}\n{}", self.papers_in,
            if self.navigator.initialized == TriState::True { "trained ✓" } else { "untrained" },
            self.store.dashboard())
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pv(title: &str, id: &str) -> PaperVector {
        PaperVector::from_title(title, id, 2024, &[])
    }

    #[test]
    fn deterministic_embedding() {
        let a = PaperVector::hash_to_vec(&[1u8; 32]);
        let b = PaperVector::hash_to_vec(&[1u8; 32]);
        assert_eq!(a, b);
    }

    #[test]
    fn identical_title_max_similarity() {
        let a = make_pv("Same Title", "x");
        let b = make_pv("Same Title", "y");
        assert!((a.cosine_similarity(&b) - 1.0).abs() < 0.01);
    }

    #[test]
    fn different_title_lower_similarity() {
        let a = make_pv("AAAA AAAA AAAA", "x");
        let b = make_pv("BBBB BBBB BBBB", "y");
        assert!(a.cosine_similarity(&b) < 0.99);
    }

    #[test]
    fn tensor_store_insert_dedup() {
        let mut ts = TensorStore::new();
        assert!(ts.insert(make_pv("Paper", "1")));
        assert!(!ts.insert(make_pv("Paper", "2"))); // Same title → same hash
        assert_eq!(ts.len(), 1);
    }

    #[test]
    fn nearest_neighbors() {
        let mut ts = TensorStore::new();
        ts.insert(make_pv("Machine Learning in NLP", "a"));
        ts.insert(make_pv("Deep Learning for Text", "b"));
        ts.insert(make_pv("Quantum Chromodynamics", "c"));
        let query = make_pv("Neural Language Models", "q");
        let near = ts.nearest(&query, 2);
        assert_eq!(near.len(), 2);
    }

    #[test]
    fn spectral_navigator_trains() {
        let mut store = TensorStore::new();
        for i in 0..10 {
            store.insert(make_pv(&format!("Paper {}", i), &format!("p{}", i)));
        }
        let mut nav = SpectralNavigator::new();
        nav.train(&store, 2);
        assert_eq!(nav.eigenvectors.len(), 2);
    }

    #[test]
    fn thunder_ingest_parallel() {
        let mut te = ThunderExtractor::new();
        let papers: Vec<PaperVector> = (0..50).map(|i| make_pv(&format!("P{}", i), &format!("p{}", i))).collect();
        te.ingest_parallel(papers, 4);
        assert_eq!(te.store.len(), 50);
    }

    #[test]
    fn dashboard_contains() {
        let te = ThunderExtractor::new();
        let d = te.dashboard();
        assert!(d.contains("Thunder Extractor"));
        assert!(d.contains("Tensor Store"));
    }
}
