//! csr.rs — deterministic CSR (compressed-sparse-row) graph + synchronous
//! Jacobi personalized-PageRank (PPR).
//!
//! WHY THIS EXISTS (retrieval-blueprint v2, BINDING — newer wins).
//!   Retrieval recall needs a *diffusion* primitive over a node/edge graph. v2
//!   SUPERSEDES v1's order-dependent local-push (Andersen-Chung-Lang async
//!   push). The binding decision: recall diffusion MUST be a SYNCHRONOUS,
//!   fixed-point (Jacobi) power iteration with a FIXED iteration count K and a
//!   FIXED summation order — so it is bit-reproducible on any hardware:
//!
//!   ```text
//!   π_{k+1} = α · e + (1 − α) · (π_k · Â)
//!   ```
//!
//!   where
//!     * Â is the ROW-STOCHASTIC adjacency (each row sums to 1),
//!     * e is the personalization / restart distribution (the seed),
//!     * α is the teleport (restart) probability, default 0.15,
//!     * π is a ROW vector, so the matrix step is the LEFT product π_k · Â.
//!
//!   Async / relaxed local-push is advisory-only and explicitly OUT OF SCOPE:
//!   it is order-dependent and therefore non-deterministic. We DO NOT implement
//!   it. Fixed K ⇒ no convergence epsilon, no early-out, same input ⇒ same
//!   bytes.
//!
//! STORAGE: a single contiguous `val: Vec<f64>` + parallel `col_idx`, plus a
//! `row_ptr` offset array — the CSR analogue of `mat.rs`'s one-contiguous-buffer
//! invariant (no `Vec<Vec<f64>>` pointer-chasing). Within each row, `col_idx` is
//! kept sorted by column so iteration order is canonical/deterministic.
//!
//! ORIENTATION: `spmv` computes the LEFT-vector product `y = xᵀ Â`
//! (out_j = Σ_i x_i · Â[i][j]) — exactly the step PPR needs. This is the
//! transpose of the usual "A·x" column product; we name it `spmv` and document
//! the orientation rather than ship two near-identical kernels.
//!
//! ZERO new dependencies. Pure `std`. HashMap is avoided entirely (the builder
//! sorts per-row vectors instead, so no iteration-order hazard).

/// Compressed-sparse-row graph.
///
/// Layout: `row_ptr` has length `n + 1`; row `i` spans
/// `col_idx[row_ptr[i]..row_ptr[i+1]]` with matching `val[..]` entries.
/// Single contiguous backing store — cache-friendly, SIMD-ready, and the CSR
/// counterpart to `mat.rs`'s one-buffer `Mat`.
#[derive(Debug, Clone, PartialEq)]
pub struct Csr {
    /// Row offsets, length `n + 1`. `row_ptr[n] == nnz`.
    pub row_ptr: Vec<usize>,
    /// Column indices for the non-zeros, grouped by row, sorted ascending
    /// within each row. Length `nnz`.
    pub col_idx: Vec<usize>,
    /// Non-zero edge weights (or normalized transition probs), length `nnz`.
    pub val: Vec<f64>,
}

impl Csr {
    /// Number of nodes (rows) `n`.
    #[inline]
    pub fn nrows(&self) -> usize {
        self.row_ptr.len() - 1
    }

    /// Number of non-zeros.
    #[inline]
    pub fn nnz(&self) -> usize {
        self.col_idx.len()
    }

    /// Build a CSR graph from weighted edges. `edges` are `(src, dst, weight)`.
    ///
    /// DIRECTED by design: an edge `(s, d, w)` contributes only to row `s`.
    /// For an UNDIRECTED graph the caller passes BOTH `(s, d, w)` and
    /// `(d, s, w)` (see the tests for the canonical pattern). Edges with
    /// `src >= n` or `dst >= n` are ignored.
    ///
    /// `col_idx` is sorted ascending within each row (deterministic order);
    /// duplicate `(src, dst)` pairs are merged by SUMMING their weights, so the
    /// builder is idempotent w.r.t. parallel/duplicate edge submissions.
    pub fn from_edges(n: usize, edges: &[(usize, usize, f64)]) -> Self {
        // Per-row buckets, then sort + merge. No HashMap ⇒ no iteration hazard.
        let mut rows: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
        for &(s, d, w) in edges {
            if s < n && d < n {
                rows[s].push((d, w));
            }
        }
        let mut row_ptr = Vec::with_capacity(n + 1);
        let mut col_idx = Vec::new();
        let mut val = Vec::new();
        row_ptr.push(0);
        for i in 0..n {
            rows[i].sort_by_key(|&(c, _)| c);
            // Merge adjacent duplicate columns (same `d`) by summing weights.
            let mut merged: Vec<(usize, f64)> = Vec::new();
            for (c, w) in rows[i].drain(..) {
                if let Some(last) = merged.last_mut() {
                    if last.0 == c {
                        last.1 += w;
                        continue;
                    }
                }
                merged.push((c, w));
            }
            for (c, w) in merged {
                col_idx.push(c);
                val.push(w);
            }
            row_ptr.push(col_idx.len());
        }
        Self {
            row_ptr,
            col_idx,
            val,
        }
    }

    /// Produce the ROW-STOCHASTIC transition matrix Â: each row is divided by
    /// its own out-weight so it sums to 1.
    ///
    /// DANGLING ROW (all weights zero ⇒ out-degree 0): to keep the chain
    /// stochastic and deterministic we install a SELF-LOOP (Â[i][i] = 1). This
    /// is the standard "dangling-node" handling and avoids a separate uniform
    /// teleport term inside the matrix. For connected graphs there are no
    /// dangling rows, so the choice is moot except at the boundary.
    pub fn row_normalize(&self) -> Csr {
        let n = self.nrows();
        let mut col_idx = Vec::new();
        let mut val = Vec::new();
        let mut row_ptr = Vec::with_capacity(n + 1);
        row_ptr.push(0);
        for i in 0..n {
            let start = self.row_ptr[i];
            let end = self.row_ptr[i + 1];
            let s: f64 = self.val[start..end].iter().sum();
            if s == 0.0 {
                // Deterministic self-loop for the dangling node.
                col_idx.push(i);
                val.push(1.0);
            } else {
                for k in start..end {
                    col_idx.push(self.col_idx[k]);
                    val.push(self.val[k] / s);
                }
            }
            row_ptr.push(col_idx.len());
        }
        Csr {
            row_ptr,
            col_idx,
            val,
        }
    }

    /// Sparse matrix-vector product, LEFT-vector orientation:
    ///
    /// ```text
    ///   out = xᵀ Â      i.e.   out_j = Σ_i x_i · Â[i][j]
    /// ```
    ///
    /// FIXED summation order: rows are visited `0..n`; within a row, entries
    /// are visited in ascending-column order. Because `out_j` only ever
    /// accumulates `x_i · Â[i][j]` for increasing `i`, the addition order for
    /// every `out_j` is canonical ⇒ deterministic, reproducible across runs.
    ///
    /// `x` and `out` must both have length `n`. `out` is overwritten.
    pub fn spmv(&self, x: &[f64], out: &mut [f64]) {
        let n = self.nrows();
        debug_assert_eq!(x.len(), n, "csr::spmv: x length must be n");
        debug_assert_eq!(out.len(), n, "csr::spmv: out length must be n");
        for o in out.iter_mut() {
            *o = 0.0;
        }
        for i in 0..n {
            let xi = x[i];
            if xi == 0.0 {
                continue; // skip zero left-entries (fixed order preserved)
            }
            let start = self.row_ptr[i];
            let end = self.row_ptr[i + 1];
            for k in start..end {
                let j = self.col_idx[k];
                out[j] += xi * self.val[k];
            }
        }
    }

    /// SYNCHRONOUS Jacobi personalized-PageRank.
    ///
    /// FIXED-POINT (Jacobi) power iteration, exactly `iters` steps:
    ///
    /// ```text
    ///   π_0      = normalize(e)
    ///   π_{k+1}  = α · e + (1 − α) · (π_k · Â)      // π_k · Â via `spmv`
    /// ```
    ///
    /// `self` MUST already be the row-stochastic transition matrix Â — call
    /// [`Csr::row_normalize`] first on a raw graph. `seed` is the
    /// personalization/restart distribution `e`; it is normalized internally
    /// (a zero seed falls back to uniform). The returned `π` is normalized to
    /// sum to 1 (guards sub-ULP drift; the iteration itself preserves the sum).
    ///
    /// DETERMINISM: fixed step count `iters`, fixed `spmv` summation order ⇒
    /// identical bytes for identical inputs on a given build (no `-ffast-math`).
    pub fn personalized_pagerank(&self, seed: &[f64], alpha: f64, iters: usize) -> Vec<f64> {
        let n = self.nrows();
        // Normalize the fixed restart distribution e.
        let ssum: f64 = seed.iter().sum();
        let mut e = vec![0.0; n];
        if ssum != 0.0 {
            for i in 0..n {
                e[i] = seed[i] / ssum;
            }
        } else {
            let u = 1.0 / n as f64;
            for v in e.iter_mut() {
                *v = u;
            }
        }

        let mut pi = e.clone();
        let mut next = vec![0.0; n];
        // Jacobi: the WHOLE vector is updated from the previous iterate (no
        // in-place Gauss-Seidel mixing), so every step is a clean matrix apply.
        for _ in 0..iters {
            self.spmv(&pi, &mut next);
            for i in 0..n {
                next[i] = alpha * e[i] + (1.0 - alpha) * next[i];
            }
            std::mem::swap(&mut pi, &mut next);
        }

        // Final normalize (deterministic; guards tiny f64 drift).
        let t: f64 = pi.iter().sum();
        if t != 0.0 {
            for v in pi.iter_mut() {
                *v /= t;
            }
        }
        pi
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Approximate float equality for hand-derived oracles (no stats crate).
    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    // ---------------------------------------------------------------------
    // 1. CSR round-trips a known directed 3-node graph.
    //    edges: 0→1, 0→2, 1→2, 2→0  (unique, weights 1.0)
    //    row_ptr = [0,2,3,4], col_idx = [1,2, 2, 0], val = [1,1,1,1]
    // ---------------------------------------------------------------------
    #[test]
    fn csr_roundtrip_known_graph() {
        let edges = [(0usize, 1, 1.0), (0, 2, 1.0), (1, 2, 1.0), (2, 0, 1.0)];
        let csr = Csr::from_edges(3, &edges);
        assert_eq!(csr.row_ptr, vec![0, 2, 3, 4]);
        assert_eq!(csr.col_idx, vec![1, 2, 2, 0]);
        assert_eq!(csr.val, vec![1.0, 1.0, 1.0, 1.0]);
        assert_eq!(csr.nrows(), 3);
        assert_eq!(csr.nnz(), 4);
    }

    // ---------------------------------------------------------------------
    // 2. row_normalize: every non-dangling row sums to 1.0 (±1e-12).
    //    row0 has out-weights 2.0, 1.0 ⇒ normalized 2/3, 1/3 (sum 1).
    // ---------------------------------------------------------------------
    #[test]
    fn row_normalize_sums_to_one() {
        let edges = [(0usize, 1, 2.0), (0, 2, 1.0), (1, 0, 1.0), (2, 0, 1.0)];
        let csr = Csr::from_edges(3, &edges);
        let a = csr.row_normalize();
        for i in 0..a.nrows() {
            let s: f64 = a.val[a.row_ptr[i]..a.row_ptr[i + 1]].iter().sum();
            assert!(close(s, 1.0, 1e-12), "row {i} sums to {s}");
        }
        // Hand-check row 0: cols [1,2], normalized weights [2/3, 1/3].
        assert_eq!(a.col_idx[a.row_ptr[0]..a.row_ptr[1]], [1, 2]);
        assert!(close(a.val[a.row_ptr[0]], 2.0 / 3.0, 1e-12));
        assert!(close(a.val[a.row_ptr[0] + 1], 1.0 / 3.0, 1e-12));
    }

    // ---------------------------------------------------------------------
    // 3. spmv matches a hand-computed y = xᵀÂ for a directed 3-cycle.
    //    Â = [[0,1,0],[0,0,1],[1,0,0]] (each row single entry ⇒ unchanged).
    //    x = [0.2, 0.3, 0.5]  ⇒  y = [0.5, 0.2, 0.3]
    //      y_0 = x_2·A[2][0] = 0.5;  y_1 = x_0·A[0][1] = 0.2;  y_2 = x_1·A[1][2] = 0.3
    // ---------------------------------------------------------------------
    #[test]
    fn spmv_matches_hand_computed() {
        let edges = [(0usize, 1, 1.0), (1, 2, 1.0), (2, 0, 1.0)];
        let a = Csr::from_edges(3, &edges).row_normalize();
        let x = [0.2, 0.3, 0.5];
        let mut y = [0.0; 3];
        a.spmv(&x, &mut y);
        assert!(close(y[0], 0.5, 1e-12));
        assert!(close(y[1], 0.2, 1e-12));
        assert!(close(y[2], 0.3, 1e-12));
    }

    // ---------------------------------------------------------------------
    // 4. PPR determinism: two IDENTICAL calls ⇒ byte-identical Vec (assert_eq).
    // ---------------------------------------------------------------------
    #[test]
    fn ppr_byte_identical() {
        let edges = [
            (0usize, 1, 1.0),
            (1, 2, 1.0),
            (2, 0, 1.0),
            (0, 2, 1.0),
            (2, 1, 1.0),
        ];
        let a = Csr::from_edges(3, &edges).row_normalize();
        let seed = [0.2, 0.5, 0.3];
        let p1 = a.personalized_pagerank(&seed, 0.15, 60);
        let p2 = a.personalized_pagerank(&seed, 0.15, 60);
        assert_eq!(p1, p2, "PPR must be bit-reproducible for identical inputs");
    }

    // ---------------------------------------------------------------------
    // 5. PPR stationary (SYMMETRIC undirected, uniform seed, converged).
    //    Undirected triangle: each node degree 2 ⇒ regular graph.
    //    For a uniform seed e, the fixed point of  π = α e + (1−α) π·Â  is the
    //    uniform vector: if π is uniform then π·Â is uniform (Â row-stochastic,
    //    columns also sum to 1 by symmetry) ⇒ π = α·uniform + (1−α)·uniform =
    //    uniform. Contraction (spectral radius 1−α < 1) ⇒ converges to [1/3,1/3,1/3],
    //    which IS the degree-proportional stationary (deg/2|E| = 2/6 = 1/3).
    //    Triangle is connected + non-bipartite (odd cycle) as required.
    // ---------------------------------------------------------------------
    #[test]
    fn ppr_stationary_uniform_triangle() {
        // Undirected triangle: both directions for each edge.
        let edges = [
            (0usize, 1, 1.0),
            (1, 0, 1.0),
            (1, 2, 1.0),
            (2, 1, 1.0),
            (2, 0, 1.0),
            (0, 2, 1.0),
        ];
        let a = Csr::from_edges(3, &edges).row_normalize();
        let seed = [1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0];
        let pi = a.personalized_pagerank(&seed, 0.15, 200);
        for i in 0..3 {
            assert!(
                close(pi[i], 1.0 / 3.0, 1e-6),
                "node {i} stationary = {} (want 1/3)",
                pi[i]
            );
        }
        // Fixed-point invariant: one more iteration barely moves π (< 1e-9).
        let mut extra = vec![0.0; 3];
        a.spmv(&pi, &mut extra);
        for i in 0..3 {
            let pi_next = 0.15 * seed[i] + (1.0 - 0.15) * extra[i];
            assert!(close(pi_next, pi[i], 1e-9), "node {i} not a fixed point");
        }
    }

    // ---------------------------------------------------------------------
    // 6. PPR seed-locality: one-hot seed on a path node ⇒ mass stays local.
    //    Path 0—1—2—3 (undirected). Seed one-hot on node 1, α=0.15, 2 iters.
    //    Hand trace (Â row-stochastic, degrees 1,2,2,1):
    //      π0 = [0,1,0,0]
    //      π0·Â = row1 = [0.5,0,0.5,0]
    //      π1 = .15·e + .85·[.5,0,.5,0] = [.425,.15,.425,0]
    //      π1·Â = [.075,.6375,.075,.2125]
    //      π2 = .15·e + .85·[.075,.6375,.075,.2125]
    //         = [.06375, .691875, .06375, .180625]
    //    ⇒ π[1] is the maximum (seed-locality holds).
    // ---------------------------------------------------------------------
    #[test]
    fn ppr_seed_locality_path() {
        let edges = [
            (0usize, 1, 1.0),
            (1, 0, 1.0),
            (1, 2, 1.0),
            (2, 1, 1.0),
            (2, 3, 1.0),
            (3, 2, 1.0),
        ];
        let a = Csr::from_edges(4, &edges).row_normalize();
        let seed = [0.0, 1.0, 0.0, 0.0]; // one-hot on node 1
        let pi = a.personalized_pagerank(&seed, 0.15, 2);
        // Seed node must carry the largest mass (locality).
        assert!(pi[1] > pi[0] + 1e-9, "π[1]={} not > π[0]={}", pi[1], pi[0]);
        assert!(pi[1] > pi[2] + 1e-9, "π[1]={} not > π[2]={}", pi[1], pi[2]);
        assert!(pi[1] > pi[3] + 1e-9, "π[1]={} not > π[3]={}", pi[1], pi[3]);
    }
}
