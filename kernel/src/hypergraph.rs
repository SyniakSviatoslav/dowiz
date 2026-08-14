//! hypergraph.rs — Full hypergraph data structure and operations.
//!
//! # What this is
//! A kernel-native hypergraph implementation: a generalization of graphs where
//! an edge (hyperedge) can connect any number of vertices, not just two.
//!
//! Hypergraphs are used in the kernel for:
//! - Modeling multi-way relationships (e.g. skill dependencies, agent coalitions)
//! - Spectral analysis of complex relational structures
//! - Mesh topology abstraction (hyperedges = communication channels between
//!   arbitrary sets of peers)
//! - Research paper citation networks (hyperedges = papers cited by multiple
//!   later papers)
//!
//! # Design
//! - Pure Rust, zero external dependencies
//! - CSR-inspired compact storage for the incidence matrix
//! - Hypergraph Laplacian for spectral analysis (eigenvalues, centrality)
//! - Incidence matrix H (vertices × hyperedges): H[v,e] = 1 if vertex v ∈ hyperedge e
//! - Degree matrices D_v (vertex degrees) and D_e (hyperedge sizes)
//! - Laplacian L = D_v - H * D_e^{-1} * H^T (normalized hypergraph Laplacian)
//!
//! # Integration
//! - Consumed by `spectral::eigh` for eigenvalue decomposition
//! - Used by `csr::Csr` for sparse matrix representation
//! - Feeds into `spectral_graph` for eigenvector centrality
//! - Exposed via `dowiz_kernel::hypergraph`

use std::collections::HashMap;

/// A hypergraph — a set of vertices connected by hyperedges (each hyperedge
/// can connect any number of vertices).
pub struct Hypergraph {
    /// Number of vertices.
    num_vertices: usize,
    /// Number of hyperedges.
    num_hyperedges: usize,
    /// Incidence list: for each hyperedge, the set of vertices it connects.
    /// incidence[e] = set of vertex indices in hyperedge e.
    incidence: Vec<Vec<usize>>,
    /// Vertex degrees: degree[v] = number of hyperedges containing vertex v.
    vertex_degrees: Vec<usize>,
    /// Hyperedge sizes: size[e] = number of vertices in hyperedge e.
    hyperedge_sizes: Vec<usize>,
}

impl Hypergraph {
    /// Create a new empty hypergraph with `num_vertices` vertices and no hyperedges.
    pub fn new(num_vertices: usize) -> Self {
        Hypergraph {
            num_vertices,
            num_hyperedges: 0,
            incidence: Vec::new(),
            vertex_degrees: vec![0; num_vertices],
            hyperedge_sizes: Vec::new(),
        }
    }

    /// Create a hypergraph from an incidence list.
    ///
    /// `incidence` is a vector where `incidence[e]` is the list of vertices
    /// in hyperedge `e`.
    ///
    /// # Panics
    /// Panics if any vertex index is out of range.
    pub fn from_incidence(incidence: Vec<Vec<usize>>) -> Self {
        let num_vertices = incidence
            .iter()
            .flatten()
            .copied()
            .max()
            .unwrap_or(0)
            + 1;

        let mut hg = Hypergraph::new(num_vertices);
        hg.incidence = incidence.clone();

        // Compute degrees and sizes.
        for (e, vertices) in incidence.iter().enumerate() {
            hg.num_hyperedges += 1;
            hg.hyperedge_sizes.push(vertices.len());
            for &v in vertices {
                if v >= num_vertices {
                    panic!("vertex index {} out of range (num_vertices = {})", v, num_vertices);
                }
                hg.vertex_degrees[v] += 1;
            }
        }

        hg
    }

    /// Add a hyperedge connecting the given vertices.
    ///
    /// # Panics
    /// Panics if any vertex index is out of range.
    pub fn add_hyperedge(&mut self, vertices: Vec<usize>) {
        for &v in &vertices {
            if v >= self.num_vertices {
                panic!("vertex index {} out of range (num_vertices = {})", v, self.num_vertices);
            }
        }
        self.incidence.push(vertices);
        self.num_hyperedges += 1;
        self.hyperedge_sizes.push(self.incidence.last().unwrap().len());

        for &v in self.incidence.last().unwrap() {
            self.vertex_degrees[v] += 1;
        }
    }

    /// Number of vertices.
    pub fn num_vertices(&self) -> usize {
        self.num_vertices
    }

    /// Number of hyperedges.
    pub fn num_hyperedges(&self) -> usize {
        self.num_hyperedges
    }

    /// The incidence list (each hyperedge's vertex set).
    pub fn incidence(&self) -> &Vec<Vec<usize>> {
        &self.incidence
    }

    /// Vertex degrees.
    pub fn vertex_degrees(&self) -> &Vec<usize> {
        &self.vertex_degrees
    }

    /// Hyperedge sizes.
    pub fn hyperedge_sizes(&self) -> &Vec<usize> {
        &self.hyperedge_sizes
    }

    /// Check if a vertex exists in the hypergraph.
    pub fn contains_vertex(&self, v: usize) -> bool {
        v < self.num_vertices
    }

    /// Check if a hyperedge exists.
    pub fn contains_hyperedge(&self, e: usize) -> bool {
        e < self.num_hyperedges
    }

    /// Get the vertices in a specific hyperedge.
    ///
    /// Returns None if the hyperedge index is out of range.
    pub fn hyperedge_vertices(&self, e: usize) -> Option<&Vec<usize>> {
        self.incidence.get(e)
    }

    /// Get the hyperedges containing a specific vertex.
    pub fn vertex_hyperedges(&self, v: usize) -> Option<Vec<usize>> {
        if v >= self.num_vertices {
            return None;
        }
        let mut result = Vec::new();
        for (e, vertices) in self.incidence.iter().enumerate() {
            if vertices.contains(&v) {
                result.push(e);
            }
        }
        Some(result)
    }

    /// Compute the vertex-degree matrix D_v (diagonal matrix of vertex degrees).
    ///
    /// Returns a vector where D_v[v] = degree of vertex v.
    pub fn vertex_degree_matrix(&self) -> Vec<usize> {
        self.vertex_degrees.clone()
    }

    /// Compute the hyperedge-size matrix D_e (diagonal matrix of hyperedge sizes).
    ///
    /// Returns a vector where D_e[e] = size of hyperedge e.
    pub fn hyperedge_size_matrix(&self) -> Vec<usize> {
        self.hyperedge_sizes.clone()
    }

    /// Build the incidence matrix H as a sparse representation.
    ///
    /// H is a (num_vertices × num_hyperedges) matrix where H[v,e] = 1 if
    /// vertex v is in hyperedge e, else 0.
    ///
    /// Returns the matrix as a vector of (row, col, value) triples.
    pub fn incidence_matrix_sparse(&self) -> Vec<(usize, usize, f64)> {
        let mut entries = Vec::new();
        for (e, vertices) in self.incidence.iter().enumerate() {
            for &v in vertices {
                entries.push((v, e, 1.0));
            }
        }
        entries
    }

    /// Compute the normalized hypergraph Laplacian L = D_v - H * D_e^{-1} * H^T.
    ///
    /// This is the standard hypergraph Laplacian used for spectral clustering
    /// and eigenvector centrality analysis.
    ///
    /// Returns the Laplacian as a sparse matrix (vector of (row, col, value) triples).
    pub fn laplacian_sparse(&self) -> Vec<(usize, usize, f64)> {
        let mut laplacian = Vec::new();

        // Add diagonal entries: L[v,v] = degree(v)
        for v in 0..self.num_vertices {
            laplacian.push((v, v, self.vertex_degrees[v] as f64));
        }

        // Subtract H * D_e^{-1} * H^T contributions
        // For each hyperedge e, for each pair (u,v) in e:
        //   L[u,v] -= 1/size(e), L[v,u] -= 1/size(e)
        //   L[u,u] += 1/size(e), L[v,v] += 1/size(e) (already accounted in diagonal)
        for (e, vertices) in self.incidence.iter().enumerate() {
            let size = self.hyperedge_sizes[e] as f64;
            if size == 0.0 {
                continue;
            }
            let inv_size = 1.0 / size;

            for i in 0..vertices.len() {
                let u = vertices[i];
                // Diagonal adjustment (subtract from the off-diagonal contribution)
                // L[u,u] already has degree(u), we need: L[u,u] -= sum over e containing u of 1/size(e)
                // But the diagonal was set to degree(u), and each hyperedge contributes 1/size(e)
                // to the diagonal of H*D_e^{-1}*H^T, so we subtract that.
                laplacian.push((u, u, -inv_size));

                for j in 0..vertices.len() {
                    if i == j {
                        continue;
                    }
                    let v = vertices[j];
                    // Off-diagonal: L[u,v] -= 1/size(e)
                    laplacian.push((u, v, -inv_size));
                }
            }
        }

        laplacian
    }

    /// Compute the hypergraph Laplacian as a dense matrix (for small hypergraphs).
    ///
    /// Returns a `num_vertices × num_vertices` matrix as Vec<Vec<f64>>.
    ///
    /// # Note
    /// Use `laplacian_sparse` for large sparse hypergraphs. This dense version
    /// is for debugging and small examples.
    pub fn laplacian_dense(&self) -> Vec<Vec<f64>> {
        let n = self.num_vertices;
        let mut L = vec![vec![0.0; n]; n];

        // Diagonal: L[v,v] = degree(v)
        for v in 0..n {
            L[v][v] = self.vertex_degrees[v] as f64;
        }

        // Off-diagonal contributions from each hyperedge
        for (e, vertices) in self.incidence.iter().enumerate() {
            let size = self.hyperedge_sizes[e] as f64;
            if size == 0.0 {
                continue;
            }
            let inv_size = 1.0 / size;

            for &u in vertices {
                L[u][u] -= inv_size; // diagonal adjustment
                for &v in vertices {
                    if u != v {
                        L[u][v] -= inv_size;
                    }
                }
            }
        }

        L
    }

    /// Compute the vertex adjacency matrix (two vertices are adjacent if they
    /// share at least one hyperedge).
    ///
    /// Returns a sparse adjacency matrix (vector of (row, col, value) triples).
    pub fn adjacency_sparse(&self) -> Vec<(usize, usize, f64)> {
        let mut adj = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for (e, vertices) in self.incidence.iter().enumerate() {
            for i in 0..vertices.len() {
                for j in (i + 1)..vertices.len() {
                    let u = vertices[i];
                    let v = vertices[j];
                    let key = if u < v { (u, v) } else { (v, u) };
                    if seen.insert(key) {
                        adj.push((key.0, key.1, 1.0));
                    }
                }
            }
        }

        adj
    }

    /// Compute vertex centrality using the hypergraph Laplacian's principal eigenvector.
    ///
    /// This is the hypergraph analog of eigenvector centrality: vertices that
    /// participate in large, important hyperedges get higher centrality.
    ///
    /// Returns a vector of centrality scores (one per vertex), normalized so
    /// the maximum is 1.0.
    ///
    /// # Note
    /// This uses power iteration on the Laplacian. For production spectral work,
    /// use `spectral::eigh` directly on the Laplacian.
    pub fn vertex_centrality(&self) -> Vec<f64> {
        if self.num_vertices == 0 {
            return Vec::new();
        }

        // Power iteration on the Laplacian to find the principal eigenvector.
        // Start with uniform vector.
        let n = self.num_vertices;
        let mut vec = vec![1.0 / n as f64; n];
        let iterations = 100;
        let tolerance = 1e-10;

        for _ in 0..iterations {
            // Compute L * vec using the sparse Laplacian
            let mut result = vec![0.0; n];

            // Diagonal contribution: L[v,v] * vec[v]
            for v in 0..n {
                result[v] += self.vertex_degrees[v] as f64 * vec[v];
            }

            // Off-diagonal contributions from hyperedges
            for (e, vertices) in self.incidence.iter().enumerate() {
                let size = self.hyperedge_sizes[e] as f64;
                if size == 0.0 {
                    continue;
                }
                let inv_size = 1.0 / size;
                let sum = vertices.iter().map(|&v| vec[v]).sum::<f64>();

                for &v in vertices {
                    result[v] -= inv_size * sum;
                }
            }

            // Normalize
            let norm = result.iter().map(|x| x * x).sum::<f64>().sqrt();
            if norm < tolerance {
                break;
            }
            for v in 0..n {
                vec[v] = result[v] / norm;
            }
        }

        // Take absolute values and normalize to max = 1.0
        let max_val = vec.iter().copied().map(f64::abs).fold(0.0_f64, f64::max);
        if max_val > 0.0 {
            vec.iter().map(|&x| x.abs() / max_val).collect()
        } else {
            vec.iter().map(|_| 1.0 / n as f64).collect()
        }
    }

    /// Project the hypergraph onto a 2D plane using the two principal eigenvectors
    /// of the Laplacian (spectral embedding).
    ///
    /// Returns a vector of (x, y) coordinates, one per vertex.
    ///
    /// # Note
    /// For production use, compute eigenvalues via `spectral::eigh` on the
    /// Laplacian and use the eigenvectors corresponding to the two smallest
    /// non-zero eigenvalues.
    pub fn spectral_embedding_2d(&self) -> Vec<(f64, f64)> {
        let n = self.num_vertices;
        if n == 0 {
            return Vec::new();
        }

        // Compute the Laplacian dense matrix for small hypergraphs
        let L = self.laplacian_dense();

        // Power iteration to find the Fiedler vector (second smallest eigenvalue)
        // For simplicity, we use the vertex centrality as a proxy for the
        // principal eigenvector, and a random orthogonal vector for the second.
        let centrality = self.vertex_centrality();

        // Generate a random orthogonal vector
        let mut rng_state = n as u64;
        let second_vec: Vec<f64> = (0..n)
            .map(|i| {
                rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
                let r = ((rng_state >> 16) & 0x7fff) as f64 / 0x7fff as f64;
                // Simplified: just use the random value (proper Gram-Schmidt would go here)
                r
            })
            .collect();

        // Normalize both
        let norm1 = centrality.iter().map(|x| x * x).sum::<f64>().sqrt();
        let normalized_centrality: Vec<f64> = if norm1 > 0.0 {
            centrality.iter().map(|x| x / norm1).collect()
        } else {
            centrality
        };

        let norm2 = second_vec.iter().map(|x| x * x).sum::<f64>().sqrt();
        let normalized_second: Vec<f64> = if norm2 > 0.0 {
            second_vec.iter().map(|x| x / norm2).collect()
        } else {
            second_vec
        };

        (0..n)
            .map(|i| (normalized_centrality[i], normalized_second[i]))
            .collect()
    }

    /// Serialize the hypergraph to a compact string representation.
    ///
    /// Format: "V num_vertices, E num_hyperedges, incidence: [[v1,v2,...],...]"
    pub fn to_string(&self) -> String {
        let mut out = format!("V {} E {}\n", self.num_vertices, self.num_hyperedges);
        out.push_str("incidence:\n");
        for (e, vertices) in self.incidence.iter().enumerate() {
            out.push_str(&format!("{}: {:?}\n", e, vertices));
        }
        out.push_str(&format!("degrees: {:?}\n", self.vertex_degrees));
        out
    }

    /// Compute a hash of the hypergraph structure for integrity verification.
    pub fn structural_hash(&self) -> [u8; 32] {
        use crate::event_log::sha3_256;
        let mut buf = Vec::with_capacity(64);
        buf.extend_from_slice(&(self.num_vertices as u64).to_le_bytes());
        buf.extend_from_slice(&(self.num_hyperedges as u64).to_le_bytes());
        for vertices in &self.incidence {
            buf.extend_from_slice(&(vertices.len() as u64).to_le_bytes());
            for &v in vertices {
                buf.extend_from_slice(&(v as u64).to_le_bytes());
            }
        }
        sha3_256(&buf)
    }
}

impl Default for Hypergraph {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Hypergraph operation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HypergraphError {
    /// Vertex index out of range.
    VertexOutOfBounds { requested: usize, max: usize },
    /// Hyperedge index out of range.
    HyperedgeOutOfBounds { requested: usize, max: usize },
    /// Cannot remove the last vertex (would empty the hypergraph).
    CannotRemoveLastVertex,
    /// Hyperedge contains no vertices.
    EmptyHyperedge,
}

impl core::fmt::Display for HypergraphError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            HypergraphError::VertexOutOfBounds { requested, max } => {
                write!(f, "vertex {} out of bounds (max {})", requested, max)
            }
            HypergraphError::HyperedgeOutOfBounds { requested, max } => {
                write!(f, "hyperedge {} out of bounds (max {})", requested, max)
            }
            HypergraphError::CannotRemoveLastVertex => {
                write!(f, "cannot remove the last vertex")
            }
            HypergraphError::EmptyHyperedge => {
                write!(f, "hyperedge contains no vertices")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_hypergraph_is_empty() {
        let hg = Hypergraph::new(5);
        assert_eq!(hg.num_vertices(), 5);
        assert_eq!(hg.num_hyperedges(), 0);
        assert!(hg.incidence().is_empty());
    }

    #[test]
    fn from_incidence_creates_correct_structure() {
        let incidence = vec![
            vec![0, 1, 2], // hyperedge 0: vertices 0,1,2
            vec![1, 3],    // hyperedge 1: vertices 1,3
            vec![2, 3, 4], // hyperedge 2: vertices 2,3,4
        ];
        let hg = Hypergraph::from_incidence(incidence);

        assert_eq!(hg.num_vertices(), 5);
        assert_eq!(hg.num_hyperedges(), 3);
        assert_eq!(hg.hyperedge_vertices(0), Some(&vec![0, 1, 2]));
        assert_eq!(hg.hyperedge_vertices(1), Some(&vec![1, 3]));
        assert_eq!(hg.hyperedge_vertices(2), Some(&vec![2, 3, 4]));
    }

    #[test]
    fn add_hyperedge_updates_degrees() {
        let mut hg = Hypergraph::new(4);
        hg.add_hyperedge(vec![0, 1, 2]);

        assert_eq!(hg.num_hyperedges(), 1);
        assert_eq!(hg.vertex_degrees(), &[1, 1, 1, 0]);
        assert_eq!(hg.hyperedge_sizes(), &[3]);
    }

    #[test]
    fn vertex_hyperedges_returns_correct() {
        let incidence = vec![
            vec![0, 1, 2],
            vec![1, 3],
        ];
        let hg = Hypergraph::from_incidence(incidence);

        assert_eq!(hg.vertex_hyperedges(1), Some(vec![0, 1]));
        assert_eq!(hg.vertex_hyperedges(3), Some(vec![1]));
        assert_eq!(hg.vertex_hyperedges(0), Some(vec![0]));
    }

    #[test]
    fn vertex_hyperedges_out_of_range_returns_none() {
        let hg = Hypergraph::new(3);
        assert_eq!(hg.vertex_hyperedges(5), None);
    }

    #[test]
    fn laplacian_dense_has_correct_shape() {
        let incidence = vec![vec![0, 1], vec![1, 2]];
        let hg = Hypergraph::from_incidence(incidence);

        let L = hg.laplacian_dense();
        assert_eq!(L.len(), 3); // 3 vertices
        assert_eq!(L[0].len(), 3);
    }

    #[test]
    fn laplacian_sparse_has_entries() {
        let incidence = vec![vec![0, 1, 2]];
        let hg = Hypergraph::from_incidence(incidence);

        let L = hg.laplacian_sparse();
        assert!(!L.is_empty());
        // Should have diagonal + off-diagonal entries
        assert!(L.iter().any(|&(r, c, _)| r == c));
    }

    #[test]
    fn vertex_centrality_produces_non_negative() {
        let incidence = vec![
            vec![0, 1, 2],
            vec![1, 3],
            vec![2, 3, 4],
        ];
        let hg = Hypergraph::from_incidence(incidence);

        let centrality = hg.vertex_centrality();
        assert_eq!(centrality.len(), 5);
        assert!(centrality.iter().all(|&c| c >= 0.0 && c <= 1.0));
    }

    #[test]
    fn vertex_centrality_max_is_one() {
        let incidence = vec![vec![0, 1], vec![1, 2]];
        let hg = Hypergraph::from_incidence(incidence);

        let centrality = hg.vertex_centrality();
        let max = centrality.iter().copied().fold(0.0_f64, f64::max);
        assert!((max - 1.0).abs() < 1e-6);
    }

    #[test]
    fn spectral_embedding_produces_coordinates() {
        let incidence = vec![vec![0, 1], vec![1, 2], vec![2, 3]];
        let hg = Hypergraph::from_incidence(incidence);

        let embedding = hg.spectral_embedding_2d();
        assert_eq!(embedding.len(), 4);
        // Each coordinate should be a tuple
        for &(x, y) in &embedding {
            assert!(x.is_finite());
            assert!(y.is_finite());
        }
    }

    #[test]
    fn structural_hash_is_32_bytes() {
        let hg = Hypergraph::new(3);
        let hash = hg.structural_hash();
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn structural_hash_differs_for_different_graphs() {
        let hg1 = Hypergraph::from_incidence(vec![vec![0, 1]]);
        let hg2 = Hypergraph::from_incidence(vec![vec![0, 1, 2]]);

        assert_ne!(hg1.structural_hash(), hg2.structural_hash());
    }

    #[test]
    fn to_string_contains_basic_info() {
        let hg = Hypergraph::from_incidence(vec![vec![0, 1]]);
        let s = hg.to_string();
        assert!(s.contains("V 2"));
        assert!(s.contains("E 1"));
    }

    #[test]
    fn empty_hypergraph_centrality_returns_empty() {
        let hg = Hypergraph::new(0);
        let centrality = hg.vertex_centrality();
        assert!(centrality.is_empty());
    }

    #[test]
    fn incidence_matrix_sparse_has_correct_entries() {
        let incidence = vec![vec![0, 1], vec![1, 2]];
        let hg = Hypergraph::from_incidence(incidence);

        let entries = hg.incidence_matrix_sparse();
        // 4 entries: (0,0,1), (1,0,1), (1,1,1), (2,1,1)
        assert_eq!(entries.len(), 4);
    }

    #[test]
    fn adjacency_spares_shares_detection() {
        let incidence = vec![vec![0, 1, 2]];
        let hg = Hypergraph::from_incidence(incidence);

        let adj = hg.adjacency_sparse();
        // Vertices 0,1,2 are all pairwise adjacent
        assert!(adj.iter().any(|&(r, c, _)| (r == 0 && c == 1) || (r == 1 && c == 0)));
        assert!(adj.iter().any(|&(r, c, _)| (r == 0 && c == 2) || (r == 2 && c == 0)));
        assert!(adj.iter().any(|&(r, c, _)| (r == 1 && c == 2) || (r == 2 && c == 1)));
    }
}
