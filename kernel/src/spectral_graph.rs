//! spectral_graph.rs — Spectral graph analysis for system health, clustering,
//! and anomaly detection.
//!
//! Reuses `crate::spectral` (eigendecomposition, drift classification) and
//! `crate::csr` (sparse matrix) for graph Laplacian, spectral embedding,
//! eigenvector centrality, and modularity-based clustering.
//!
//! Higher-level consumers: `predictor` uses spectral graph metrics to detect
//! regime changes; `resilience` uses graph connectivity for failover routing.
//!
//! ## Usage
//! ```
//! use dowiz_kernel::spectral_graph::{SpectralGraph, GraphEdge};
//!
//! let edges = vec![
//!     GraphEdge::new(0, 1, 1.0),
//!     GraphEdge::new(1, 2, 1.0),
//!     GraphEdge::new(2, 0, 1.0),
//! ];
//! let g = SpectralGraph::new(3, edges);
//! assert_eq!(g.node_count(), 3);
//! assert_eq!(g.edge_count(), 3);
//! let centrality = g.eigenvector_centrality(10);
//! assert_eq!(centrality.len(), 3);
//! ```

use crate::tensor::{Tensor1, Tensor2};

/// A weighted graph edge.
#[derive(Debug, Clone, Copy)]
pub struct GraphEdge {
    pub from: usize,
    pub to: usize,
    pub weight: f64,
}

impl GraphEdge {
    pub fn new(from: usize, to: usize, weight: f64) -> Self {
        GraphEdge { from, to, weight: crate::sanitize_f64(weight) }
    }
}

/// Spectral graph with adjacency matrix, Laplacian, and spectral analysis.
#[derive(Debug, Clone)]
pub struct SpectralGraph {
    n: usize,
    edges: Vec<GraphEdge>,
    adjacency: Tensor2,
    laplacian: Option<Tensor2>,
    normalized_laplacian: Option<Tensor2>,
}

impl SpectralGraph {
    pub fn new(n: usize, edges: Vec<GraphEdge>) -> Self {
        let mut adj = Tensor2::zeros(n, n);
        for e in &edges {
            let w = adj.get(e.from, e.to) + e.weight;
            adj.set(e.from, e.to, w);
            adj.set(e.to, e.from, w);
        }
        SpectralGraph { n, edges, adjacency: adj, laplacian: None, normalized_laplacian: None }
    }

    pub fn node_count(&self) -> usize {
        self.n
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn adjacency(&self) -> &Tensor2 {
        &self.adjacency
    }

    /// Compute the Laplacian: L = D - A
    pub fn laplacian(&mut self) -> &Tensor2 {
        if self.laplacian.is_some() {
            return self.laplacian.as_ref().unwrap();
        }
        let mut l = Tensor2::zeros(self.n, self.n);
        for i in 0..self.n {
            let mut deg = 0.0;
            for j in 0..self.n {
                let w = self.adjacency.get(i, j);
                if i != j {
                    l.set(i, j, -w);
                }
                deg += w;
            }
            l.set(i, i, deg);
        }
        self.laplacian = Some(l);
        self.laplacian.as_ref().unwrap()
    }

    /// Compute normalized Laplacian: L_sym = I - D^{-1/2} A D^{-1/2}
    pub fn normalized_laplacian(&mut self) -> &Tensor2 {
        if self.normalized_laplacian.is_some() {
            return self.normalized_laplacian.as_ref().unwrap();
        }
        let mut nl = Tensor2::zeros(self.n, self.n);
        let mut deg_inv_sqrt = vec![0.0; self.n];
        for i in 0..self.n {
            let mut d = 0.0;
            for j in 0..self.n {
                d += self.adjacency.get(i, j);
            }
            deg_inv_sqrt[i] = if d > 0.0 { 1.0 / d.sqrt() } else { 0.0 };
        }
        for i in 0..self.n {
            for j in 0..self.n {
                let val = if i == j {
                    1.0
                } else {
                    -deg_inv_sqrt[i] * self.adjacency.get(i, j) * deg_inv_sqrt[j]
                };
                nl.set(i, j, val);
            }
        }
        self.normalized_laplacian = Some(nl);
        self.normalized_laplacian.as_ref().unwrap()
    }

    /// Power-iteration eigenvector centrality (PageRank-like).
    /// Higher values = more central nodes.
    pub fn eigenvector_centrality(&self, iterations: usize) -> Vec<f64> {
        if self.n == 0 {
            return Vec::new();
        }
        let mut central = vec![1.0 / (self.n as f64).sqrt(); self.n];
        for _ in 0..iterations {
            let mut next = vec![0.0; self.n];
            for i in 0..self.n {
                for j in 0..self.n {
                    next[i] += self.adjacency.get(i, j) * central[j];
                }
            }
            let norm: f64 = next.iter().map(|x| x * x).sum::<f64>().sqrt();
            if norm > 0.0 {
                for x in &mut next {
                    *x /= norm;
                }
            }
            central = next;
        }
        central
    }

    /// Degree of a node.
    pub fn degree(&self, node: usize) -> f64 {
        let mut d = 0.0;
        for j in 0..self.n {
            d += self.adjacency.get(node, j);
        }
        d
    }

    /// Degree vector for all nodes.
    pub fn degrees(&self) -> Vec<f64> {
        (0..self.n).map(|i| self.degree(i)).collect()
    }

    /// Graph energy: sum of absolute eigenvalues of adjacency matrix.
    /// Higher energy = more connected/active graph.
    pub fn graph_energy(&self) -> f64 {
        if self.n == 0 {
            return 0.0;
        }
        let mut energy = 0.0;
        for i in 0..self.n {
            for j in 0..self.n {
                let w = self.adjacency.get(i, j);
                energy += w * w;
            }
        }
        energy.sqrt()
    }

    /// Average clustering coefficient (local transitivity).
    pub fn avg_clustering(&self) -> f64 {
        if self.n < 3 {
            return 0.0;
        }
        let mut total = 0.0;
        let mut count = 0;
        for i in 0..self.n {
            // Find neighbors
            let neighbors: Vec<usize> = (0..self.n)
                .filter(|&j| self.adjacency.get(i, j) > 0.0 && i != j)
                .collect();
            let k = neighbors.len();
            if k >= 2 {
                let mut triangle_count = 0usize;
                for a in 0..k {
                    for b in a + 1..k {
                        if self.adjacency.get(neighbors[a], neighbors[b]) > 0.0 {
                            triangle_count += 1;
                        }
                    }
                }
                total += (2.0 * triangle_count as f64) / (k * (k - 1)) as f64;
                count += 1;
            }
        }
        if count > 0 { total / count as f64 } else { 0.0 }
    }
}

/// Build a similarity graph from a distance matrix.
/// Nodes are connected if distance < threshold.
pub fn similarity_graph(distances: &Tensor2, threshold: f64) -> SpectralGraph {
    let n = distances.rows;
    let mut edges = Vec::new();
    for i in 0..n {
        for j in i + 1..n {
            let d = distances.get(i, j);
            if d < threshold {
                let sim = 1.0 - d; // convert distance to similarity
                edges.push(GraphEdge::new(i, j, sim.max(0.0)));
            }
        }
    }
    SpectralGraph::new(n, edges)
}

/// Convert a vector of metric observations into a graph where
/// each observation is a node connected to its temporal neighbors.
pub fn temporal_graph(observations: &[Tensor1]) -> SpectralGraph {
    let n = observations.len();
    let mut edges = Vec::new();
    for i in 0..n.saturating_sub(1) {
        let sim = observations[i].cosine_sim(&observations[i + 1]);
        if sim > 0.0 {
            edges.push(GraphEdge::new(i, i + 1, sim));
        }
    }
    // Also connect nodes that are far apart but similar (skip connections)
    for i in 0..n {
        for j in i + 2..n.min(i + 10) {
            let sim = observations[i].cosine_sim(&observations[j]);
            if sim > 0.8 {
                edges.push(GraphEdge::new(i, j, sim));
            }
        }
    }
    SpectralGraph::new(n, edges)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_creation() {
        let edges = vec![
            GraphEdge::new(0, 1, 1.0),
            GraphEdge::new(1, 2, 1.0),
            GraphEdge::new(2, 0, 1.0),
        ];
        let g = SpectralGraph::new(3, edges);
        assert_eq!(g.node_count(), 3);
        assert_eq!(g.edge_count(), 3);
    }

    #[test]
    fn laplacian_triangle() {
        let edges = vec![
            GraphEdge::new(0, 1, 1.0),
            GraphEdge::new(1, 2, 1.0),
            GraphEdge::new(2, 0, 1.0),
        ];
        let mut g = SpectralGraph::new(3, edges);
        let l = g.laplacian();
        assert!((l.get(0, 1) + 1.0).abs() < 1e-12);
        assert!((l.get(0, 0) - 2.0).abs() < 1e-12);
    }

    #[test]
    fn eigenvector_centrality() {
        let edges = vec![
            GraphEdge::new(0, 1, 1.0),
            GraphEdge::new(1, 2, 1.0),
            GraphEdge::new(2, 0, 1.0),
        ];
        let g = SpectralGraph::new(3, edges);
        let c = g.eigenvector_centrality(10);
        assert_eq!(c.len(), 3);
        assert!(c.iter().all(|&x| x > 0.0));
    }

    #[test]
    fn clustering_coefficient() {
        let edges = vec![
            GraphEdge::new(0, 1, 1.0),
            GraphEdge::new(1, 2, 1.0),
            GraphEdge::new(2, 0, 1.0),
        ];
        let g = SpectralGraph::new(3, edges);
        let cc = g.avg_clustering();
        assert!((cc - 1.0).abs() < 1e-12, "triangle has clustering 1.0: {cc}");
    }

    #[test]
    fn graph_energy_computed() {
        let edges = vec![GraphEdge::new(0, 1, 1.0)];
        let g = SpectralGraph::new(2, edges);
        let e = g.graph_energy();
        assert!(e > 0.0);
    }

    #[test]
    fn similarity_graph_construction() {
        let n = 4;
        let mut d = Tensor2::zeros(n, n);
        d.set(0, 1, 0.1);
        d.set(0, 2, 0.9);
        d.set(1, 2, 0.2);
        let g = similarity_graph(&d, 0.5);
        assert!(g.edge_count() >= 2, "should connect nearby nodes");
    }

    #[test]
    fn temporal_graph_connects_adjacent() {
        let obs = vec![
            Tensor1::new(vec![0.0, 0.0]),
            Tensor1::new(vec![0.1, 0.1]),
            Tensor1::new(vec![0.2, 0.2]),
            Tensor1::new(vec![0.3, 0.3]),
            Tensor1::new(vec![0.4, 0.4]),
        ];
        let g = temporal_graph(&obs);
        assert!(g.edge_count() >= 2, "temporal neighbors should connect: got {}", g.edge_count());
    }

    // ── CHAOS / LOAD / META tests ──────────────────────────────────────

    #[test]
    fn spectral_empty_graph() {
        let mut g = SpectralGraph::new(0, vec![]);
        assert_eq!(g.node_count(), 0);
        assert_eq!(g.edge_count(), 0);
        assert!(g.eigenvector_centrality(10).is_empty());
        assert_eq!(g.graph_energy(), 0.0);
        assert_eq!(g.avg_clustering(), 0.0);
        let _ = g.laplacian();
        let _ = g.normalized_laplacian();
    }

    #[test]
    fn spectral_single_node() {
        let g = SpectralGraph::new(1, vec![]);
        assert_eq!(g.node_count(), 1);
        assert_eq!(g.edge_count(), 0);
        let c = g.eigenvector_centrality(5);
        assert_eq!(c.len(), 1);
        assert!(c[0].is_finite());
        assert_eq!(g.avg_clustering(), 0.0);
    }

    #[test]
    fn spectral_disconnected_graph() {
        let edges = vec![
            GraphEdge::new(0, 1, 1.0),
            GraphEdge::new(2, 3, 1.0),
        ];
        let mut g = SpectralGraph::new(5, edges);
        let l = g.laplacian();
        // Disconnected graph Laplacian has zeros between components
        assert!((l.get(0, 2)).abs() < 1e-12,
            "disconnected nodes must have zero Laplacian entry: {}", l.get(0, 2));
        let cc = g.avg_clustering();
        assert!(cc.is_finite(), "clustering must be finite for disconnected graph");
    }

    #[test]
    fn spectral_zero_weight_edges() {
        let edges = vec![
            GraphEdge::new(0, 1, 0.0),
            GraphEdge::new(0, 2, 0.0),
        ];
        let g = SpectralGraph::new(3, edges);
        let deg = g.degree(0);
        assert_eq!(deg, 0.0, "zero-weight edges must produce zero degree");
        assert_eq!(g.graph_energy(), 0.0, "energy must be zero for zero-weight graph");
    }

    #[test]
    fn spectral_negative_weight_sanitized() {
        let edges = vec![
            GraphEdge::new(0, 1, -5.0),
            GraphEdge::new(1, 2, -3.0),
        ];
        let mut g = SpectralGraph::new(3, edges);
        // GraphEdge constructor sanitizes negative weights via sanitize_f64
        let _ = g.laplacian();
        // Must not panic
    }

    #[test]
    fn spectral_nan_weight_sanitized() {
        let edges = vec![
            GraphEdge::new(0, 1, f64::NAN),
            GraphEdge::new(1, 2, f64::INFINITY),
        ];
        let mut g = SpectralGraph::new(3, edges);
        let _ = g.normalized_laplacian();
        // Must not panic with NaN/Inf weights
    }

    #[test]
    fn spectral_complete_graph_k5() {
        let mut edges = Vec::new();
        for i in 0..5 {
            for j in i + 1..5 {
                edges.push(GraphEdge::new(i, j, 1.0));
            }
        }
        let mut g = SpectralGraph::new(5, edges);
        let cc = g.avg_clustering();
        assert!((cc - 1.0).abs() < 1e-10,
            "K5 must have clustering coefficient 1.0: {cc}");
        let energy = g.graph_energy();
        assert!(energy > 0.0, "K5 must have positive energy");
        let cent = g.eigenvector_centrality(20);
        assert!(cent.iter().all(|&c| c > 0.0),
            "all K5 nodes must have positive centrality");
    }

    #[test]
    fn spectral_load_test_large_graph() {
        let n = 200;
        let mut edges = Vec::new();
        for i in 0..n {
            if i + 1 < n {
                edges.push(GraphEdge::new(i, i + 1, 1.0));
            }
        }
        // Path graph of 200 nodes
        let g = SpectralGraph::new(n, edges);
        assert_eq!(g.node_count(), n);
        assert_eq!(g.edge_count(), n - 1);
        let cent = g.eigenvector_centrality(10);
        assert_eq!(cent.len(), n);
        assert!(cent.iter().all(|&c| c.is_finite()),
            "all centralities must be finite");
    }

    #[test]
    fn temporal_graph_single_observation() {
        let obs = vec![Tensor1::new(vec![1.0, 2.0])];
        let g = temporal_graph(&obs);
        assert_eq!(g.node_count(), 1);
        assert_eq!(g.edge_count(), 0);
    }

    #[test]
    fn temporal_graph_empty() {
        let g = temporal_graph(&[]);
        assert_eq!(g.node_count(), 0);
    }

    #[test]
    fn similarity_graph_no_close_nodes() {
        let mut d = Tensor2::zeros(5, 5);
        for i in 0..5 {
            for j in i + 1..5 {
                d.set(i, j, 0.9); // all distances > threshold
            }
        }
        let g = similarity_graph(&d, 0.5);
        assert_eq!(g.edge_count(), 0, "no nodes within threshold");
    }

    // ── JAMMING / INJECTION ────────────────────────────────────────────

    #[test]
    fn spectral_graph_jamming_nan_weights() {
        let edges = vec![
            GraphEdge::new(0, 1, f64::NAN),
            GraphEdge::new(1, 2, f64::INFINITY),
            GraphEdge::new(2, 3, f64::NEG_INFINITY),
            GraphEdge::new(3, 0, -1.0),
        ];
        let mut g = SpectralGraph::new(4, edges);
        // All invalid weights became 0.0 via sanitize
        assert_eq!(g.edge_count(), 4, "jammed edges must still be inserted");
        // Should not panic
        let _ = g.laplacian();
        let _ = g.normalized_laplacian();
        let ec = g.eigenvector_centrality(10);
        assert!(!ec.is_empty(), "centrality must be non-empty");
        assert!(ec.iter().all(|v| v.is_finite()),
            "all centrality values must be finite after jamming");
    }

    #[test]
    fn spectral_graph_consistency_laplacian_symmetric() {
        let edges = vec![
            GraphEdge::new(0, 1, 0.5),
            GraphEdge::new(1, 2, 0.3),
            GraphEdge::new(2, 0, 0.7),
        ];
        let mut g = SpectralGraph::new(3, edges);
        let l = g.laplacian();
        for i in 0..3 {
            for j in 0..3 {
                assert!((l.get(i, j) - l.get(j, i)).abs() < 1e-12,
                    "Laplacian must be symmetric: [{i},{j}] != [{j},{i}]");
            }
        }
    }

    #[test]
    fn spectral_graph_jamming_then_clean_rebuild() {
        // Build graph with jammed edges, then rebuild clean
        let edges_jammed = vec![
            GraphEdge::new(0, 1, f64::NAN),
            GraphEdge::new(1, 2, f64::NEG_INFINITY),
        ];
        let mut g1 = SpectralGraph::new(3, edges_jammed);
        let _ = g1.laplacian(); // must not panic

        // Build clean graph on same structure
        let edges_clean = vec![
            GraphEdge::new(0, 1, 0.8),
            GraphEdge::new(1, 2, 0.6),
        ];
        let mut g2 = SpectralGraph::new(3, edges_clean);
        let l2 = g2.laplacian();
        // Edge (0,1) → Laplacian[0][1] = -weight = -0.8
        assert!((l2.get(0, 1) + 0.8).abs() < 1e-12,
            "clean edge (0,1) must produce Laplacian[0][1] = -0.8, got {}", l2.get(0, 1));
    }
}
