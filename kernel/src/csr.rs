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

    /// Materialize the dense (un-normalized) adjacency matrix `A`, where
    /// `A[i][j]` is the weight of edge `i→j` (0 if absent). Square `n×n`,
    /// row-major. Used by the spectral engine (eigenvalues / graph energy).
    pub fn to_adjacency(&self) -> Vec<Vec<f64>> {
        let n = self.nrows();
        let mut a = vec![vec![0.0f64; n]; n];
        for i in 0..n {
            let lo = self.row_ptr[i];
            let hi = self.row_ptr[i + 1];
            for k in lo..hi {
                a[i][self.col_idx[k]] = self.val[k];
            }
        }
        a
    }

    /// Graph energy E = Σ|λᵢ| over all eigenvalues of the adjacency matrix
    /// (Gutman–Adrić, 2001). A pure spectral invariant: high energy ⇔ many
    /// alternating-sign modes ⇔ structurally "active" graph. Vectorless — no
    /// embeddings, just the eigenvalue spectrum. Reuses `spectral::eigenvalues`.
    pub fn energy(&self) -> f64 {
        crate::spectral::graph_energy(&self.to_adjacency())
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

// ── Spectral core: Laplacian SpMV (W2-2) ─────────────────────────────────
// Unifies the FEM M∇²U diffusion field with the graph-energy approach: the
// per-frame physics field is `y = L·x`. The orientation matches `spmv`'s
// LEFT product; because every Laplacian variant here is built from the
// symmetric adjacency A, and for a *symmetric* matrix A[i][j] = A[j][i], the
// LEFT product `out_j = Σ_i x_i·A[i][j]` equals `(A x)_j`. We implement
// `laplacian_spmv` directly (no intermediate CSR materialization) so the
// per-edge hot loop is allocation-free; only a single O(n) degree scratch is
// allocated per call (outside the edge loop).

/// Which Laplacian to apply in [`Csr::laplacian_spmv`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaplacianKind {
    /// Standard graph Laplacian `L = D − A`. Rows sum to 0 ⇒ `L·1 = 0`
    /// (mass/momentum conserved for ANY graph).
    Unnormalized,
    /// Symmetric normalized Laplacian `L = I − D^{−1/2} A D^{−1/2}`. PSD &
    /// symmetric; null space contains `D^{1/2}·1`. Conserves mass on
    /// *regular* graphs (where it reduces to `I − (1/d)·A`).
    Normalized,
    /// Random-walk normalized Laplacian `L = I − D^{−1} A`. Every row sums
    /// to 0 ⇒ `L·1 = 0` ⇒ mass/momentum conserved for ANY graph (the
    /// divergence-free diffusion operator).
    RandomWalk,
}

impl Csr {
    /// Laplacian matrix-vector product `y = L·x` via CSR SpMV.
    ///
    /// The Laplacian is built from the row out-weight `d_i = Σ_j A_ij`
    /// (degree). For an undirected graph pass BOTH `(s,d,w)` and `(d,s,w)`
    /// so `d_i` equals the true degree.
    ///
    /// * `Unnormalized`: `y_i = d_i·x_i − Σ_j A_ij·x_j`
    /// * `Normalized`:   `y_i = x_i − Σ_j A_ij / √(d_i·d_j) · x_j`
    /// * `RandomWalk`:   `y_i = x_i − Σ_j (A_ij / d_i)·x_j`
    ///
    /// `x` and `out` must have length `n`. `out` is overwritten. No heap is
    /// touched in the per-edge accumulation loop (only an O(n) degree scratch
    /// is allocated once, before the loop).
    pub fn laplacian_spmv(&self, x: &[f64], out: &mut [f64], kind: LaplacianKind) {
        let n = self.nrows();
        debug_assert_eq!(x.len(), n, "csr::laplacian_spmv: x length must be n");
        debug_assert_eq!(out.len(), n, "csr::laplacian_spmv: out length must be n");
        // Degree (row out-weight) — O(n) scratch, computed BEFORE the hot loop.
        let deg: Vec<f64> = (0..n)
            .map(|i| self.val[self.row_ptr[i]..self.row_ptr[i + 1]].iter().sum())
            .collect();
        match kind {
            LaplacianKind::Unnormalized => {
                for i in 0..n {
                    let mut acc = deg[i] * x[i];
                    let (s, e) = (self.row_ptr[i], self.row_ptr[i + 1]);
                    for k in s..e {
                        acc -= self.val[k] * x[self.col_idx[k]];
                    }
                    out[i] = acc;
                }
            }
            LaplacianKind::Normalized => {
                for i in 0..n {
                    let inv_i = if deg[i] > 0.0 {
                        deg[i].sqrt().recip()
                    } else {
                        0.0
                    };
                    let mut acc = x[i];
                    let (s, e) = (self.row_ptr[i], self.row_ptr[i + 1]);
                    for k in s..e {
                        let j = self.col_idx[k];
                        let w = if deg[j] > 0.0 {
                            self.val[k] * inv_i * deg[j].sqrt().recip()
                        } else {
                            0.0
                        };
                        acc -= w * x[j];
                    }
                    out[i] = acc;
                }
            }
            LaplacianKind::RandomWalk => {
                for i in 0..n {
                    let inv_i = if deg[i] > 0.0 { deg[i].recip() } else { 0.0 };
                    let mut acc = x[i];
                    let (s, e) = (self.row_ptr[i], self.row_ptr[i + 1]);
                    for k in s..e {
                        acc -= self.val[k] * inv_i * x[self.col_idx[k]];
                    }
                    out[i] = acc;
                }
            }
        }
    }
}

// ── Recall / precision scorers (HippoRAG-style groundedness) ───────────────
//
// E0 fix (VERIFIABLE-COGNITION §2 bug #3): there was NO in-kernel `recall@k`
// scorer — scoring was delegated to the JS bridge. `personalized_pagerank` is
// the diffusion; these pure functions score its ranking against a relevance
// set, deterministically, with a stable tie-break. No HashMap (iteration-order
// hazard); ranking is a sorted Vec with ascending-index tie-break.

/// Rank `scores` descending with a *stable* tie-break by ascending node index
/// (canonical, deterministic). Returns node indices in rank order.
fn rank_desc(scores: &[f64]) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..scores.len()).collect();
    idx.sort_by(|&a, &b| {
        scores[b]
            .partial_cmp(&scores[a])
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.cmp(&b))
    });
    idx
}

/// `recall@k` over a ranked score vector: the fraction of `relevant` nodes that
/// appear among the top-`k` by score. This is the groundedness scorer the PPR
/// diffusion feeds. Deterministic; tie-break is ascending node index. Returns
/// `0.0` if there are no scores or no relevant nodes.
pub fn recall_at_k(scores: &[f64], relevant: &[usize], k: usize) -> f64 {
    if scores.is_empty() || relevant.is_empty() {
        return 0.0;
    }
    let kr = k.min(scores.len());
    let ranked = rank_desc(scores);
    let mut relevant_sorted = relevant.to_vec();
    relevant_sorted.sort_unstable();
    let mut hits = 0u32;
    for &node in &relevant_sorted {
        // `ranked[..kr]` is a descending ranking (not ascending-sorted), so use
        // `contains` — `binary_search` would be wrong on an unsorted slice.
        if ranked[..kr].contains(&node) {
            hits += 1;
        }
    }
    hits as f64 / relevant.len() as f64
}

/// `precision@k` over a ranked score vector: the fraction of the top-`k` nodes
/// that are in `relevant`. Complements [`recall_at_k`]. Returns `0.0` for an
/// empty score vector or `k == 0`.
pub fn precision_at_k(scores: &[f64], relevant: &[usize], k: usize) -> f64 {
    if scores.is_empty() {
        return 0.0;
    }
    let kr = k.min(scores.len());
    if kr == 0 {
        return 0.0;
    }
    let ranked = rank_desc(scores);
    let mut relevant_sorted = relevant.to_vec();
    relevant_sorted.sort_unstable();
    let mut hits = 0u32;
    for &node in &ranked[..kr] {
        if relevant_sorted.binary_search(&node).is_ok() {
            hits += 1;
        }
    }
    hits as f64 / kr as f64
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
    // 4b. Hermetic-audit Cause-and-Effect Finding B (quick-win #19): the above only
    //     compares two live values in one call stack — never crosses an actual
    //     serialization boundary. Disk round-trip + independently fresh recompute.
    // ---------------------------------------------------------------------
    #[test]
    fn csr_ppr_survives_serialize_reread_boundary() {
        let edges = [
            (0usize, 1, 1.0),
            (1, 2, 1.0),
            (2, 0, 1.0),
            (0, 2, 1.0),
            (2, 1, 1.0),
        ];
        let a = Csr::from_edges(3, &edges).row_normalize();
        let seed = [0.2, 0.5, 0.3];
        let computed = a.personalized_pagerank(&seed, 0.15, 60);
        let serialized: String = computed
            .iter()
            .map(|x| format!("{:.17e}", x))
            .collect::<Vec<_>>()
            .join(",");

        let path =
            std::env::temp_dir().join(format!("csr_ppr_reread_test_{}.txt", std::process::id()));
        std::fs::write(&path, &serialized).expect("write serialized csr ppr scores");
        let reread = std::fs::read_to_string(&path).expect("re-read serialized csr ppr scores");
        std::fs::remove_file(&path).ok();

        assert_eq!(reread, serialized, "byte content did not survive a disk round-trip");

        let reparsed: Vec<f64> = reread
            .split(',')
            .map(|s| s.parse::<f64>().expect("reparse f64"))
            .collect();
        let fresh = a.personalized_pagerank(&seed, 0.15, 60); // independently recomputed
        assert_eq!(
            reparsed, fresh,
            "value re-read from disk does not match an independently fresh computation"
        );
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

    // ---------------------------------------------------------------------
    // E0 fix (VERIFIABLE-COGNITION §2 bug #3): in-kernel recall@k / precision@k
    // scorers over a PPR ranking, deterministic with stable tie-break.
    // ---------------------------------------------------------------------
    #[test]
    fn recall_at_k_hand_oracle() {
        // scores: node 2 highest, tie between 0 and 1, 3 lowest.
        let scores = [0.5, 0.5, 1.0, 0.1];
        // relevant = {0, 2}.
        // k=1 → only node 2 in top-1 ⇒ recall 1/2 = 0.5.
        assert!(close(recall_at_k(&scores, &[0, 2], 1), 0.5, 1e-12));
        // k=2 → top-2 are {2, 0} (tie 0/1 broken by ascending index ⇒ 0 before 1)
        //       ⇒ both relevant present ⇒ recall 1.0.
        assert!(close(recall_at_k(&scores, &[0, 2], 2), 1.0, 1e-12));
        // k=2 with relevant = {0,1,3}: top-2 {2,0} ⇒ only node 0 ⇒ 1/3.
        assert!(close(recall_at_k(&scores, &[0, 1, 3], 2), 1.0 / 3.0, 1e-12));
        // empty inputs guard.
        assert_eq!(recall_at_k(&[], &[0], 2), 0.0);
        assert_eq!(recall_at_k(&scores, &[], 2), 0.0);
    }

    #[test]
    fn precision_at_k_hand_oracle() {
        let scores = [0.5, 0.5, 1.0, 0.1];
        // relevant = {0, 2}. k=1 → top {2} relevant ⇒ precision 1.0.
        assert!(close(precision_at_k(&scores, &[0, 2], 1), 1.0, 1e-12));
        // k=2 → top {2,0} both relevant ⇒ precision 1.0.
        assert!(close(precision_at_k(&scores, &[0, 2], 2), 1.0, 1e-12));
        // k=3 → top {2,0,1}: only 2 and 0 relevant ⇒ 2/3.
        assert!(close(precision_at_k(&scores, &[0, 2], 3), 2.0 / 3.0, 1e-12));
        // k=0 and empty guard.
        assert_eq!(precision_at_k(&scores, &[0], 0), 0.0);
        assert_eq!(precision_at_k(&[], &[0], 2), 0.0);
    }

    #[test]
    fn recall_scores_real_ppr_ranking() {
        // Build a graph where node 0 is the seed; the grounded sources for a
        // claim about node 0 are the seed itself + its neighbours {1, 2}.
        // The distant node 3 (two hops away) must NOT outrank them.
        let edges = [
            (0usize, 1, 1.0),
            (1, 0, 1.0),
            (1, 2, 1.0),
            (2, 1, 1.0),
            (2, 3, 1.0),
            (3, 2, 1.0),
            (0, 2, 1.0),
            (2, 0, 1.0),
        ];
        let a = Csr::from_edges(4, &edges).row_normalize();
        let seed = [1.0, 0.0, 0.0, 0.0];
        let pi = a.personalized_pagerank(&seed, 0.15, 80);
        // Seed + immediate neighbours must fully recall at k=3.
        let r = recall_at_k(&pi, &[0, 1, 2], 3);
        assert!(
            close(r, 1.0, 1e-6),
            "PPR from node 0 must recall seed+neighbours at k=3 (got {r})"
        );
        // The distant node 3 must NOT be in the top-3 ranking.
        let mut idx: Vec<usize> = (0..pi.len()).collect();
        idx.sort_by(|&a, &b| pi[b].partial_cmp(&pi[a]).unwrap().then(a.cmp(&b)));
        assert!(
            !idx[..3].contains(&3),
            "distant node 3 must not outrank seed+neighbours in top-3"
        );
    }

    // ── GREEN (vectorless graph-energy): Csr.energy() over the complete graph
    //    K₃ returns E=4, matching spectral::graph_energy. ──
    #[test]
    fn csr_energy_matches_spectral_k3() {
        // K3 as undirected edges (both directions).
        let edges = [
            (0, 1, 1.0),
            (1, 0, 1.0),
            (1, 2, 1.0),
            (2, 1, 1.0),
            (0, 2, 1.0),
            (2, 0, 1.0),
        ];
        let g = Csr::from_edges(3, &edges);
        assert!(close(g.energy(), 4.0, 1e-6), "Csr::energy(K3)=4");
    }

    // ── GREEN: empty (edgeless) graph has energy 0 via Csr. ──
    #[test]
    fn csr_energy_empty_is_zero() {
        let g = Csr::from_edges(4, &[]);
        assert!(close(g.energy(), 0.0, 1e-9), "Csr::energy(empty)=0");
    }

    // ── W2-2 RED→GREEN: laplacian_spmv on a tiny triangle graph matches a
    //    hand-computed reference AND conserves mass (Σ y == 0) on constant x
    //    for the random-walk / unnormalized Laplacians.
    //
    //    Triangle K₃, undirected (both directions), unit weights:
    //      adj A = [[0,1,1],[1,0,1],[1,1,0]], degree d = [2,2,2].
    //    For x = [0.1, 0.4, 0.9]:
    //      Unnormalized L·x:
    //        y0 = 2*0.1 - (0.4+0.9)        = 0.2 - 1.3 = -1.1
    //        y1 = 2*0.4 - (0.1+0.9)        = 0.8 - 1.0 = -0.2
    //        y2 = 2*0.9 - (0.1+0.4)        = 1.8 - 0.5 =  1.3
    //        Σ y = -1.1 - 0.2 + 1.3 = 0  ✓ (mass conserved, rows sum to 0)
    //      RandomWalk (I − D⁻¹A)·x  = x − Σ_j A_ij/d_i · x_j
    //        y0 = 0.1 - (0.4/2 + 0.9/2)   = 0.1 - 0.65 = -0.55
    //        y1 = 0.4 - (0.1/2 + 0.9/2)   = 0.4 - 0.5  = -0.1
    //        y2 = 0.9 - (0.1/2 + 0.4/2)   = 0.9 - 0.25 =  0.65
    //        Σ y = -0.55 - 0.1 + 0.65 = 0  ✓
    //      Normalized (symmetric, regular graph d=2 ⇒ reduces to I − (1/2)A):
    //        y0 = 0.1 - (0.4+0.9)/2       = 0.1 - 0.65 = -0.55
    //        (identical to RandomWalk here because the triangle is regular)
    // ──────────────────────────────────────────────────────────────────────
    #[test]
    fn laplacian_spmv_triangle_matches_hand_and_conserves() {
        let edges = [
            (0usize, 1, 1.0),
            (1, 0, 1.0),
            (1, 2, 1.0),
            (2, 1, 1.0),
            (0, 2, 1.0),
            (2, 0, 1.0),
        ];
        let g = Csr::from_edges(3, &edges);
        let x = [0.1, 0.4, 0.9];

        // ── Unnormalized L = D − A ──
        let mut y_un = [0.0; 3];
        g.laplacian_spmv(&x, &mut y_un, LaplacianKind::Unnormalized);
        assert!(close(y_un[0], -1.1, 1e-12), "unnormalized y0 = {}", y_un[0]);
        assert!(close(y_un[1], -0.2, 1e-12), "unnormalized y1 = {}", y_un[1]);
        assert!(close(y_un[2], 1.3, 1e-12), "unnormalized y2 = {}", y_un[2]);
        // Mass/momentum conservation: Σ y == 0 (rows of L sum to 0).
        let sum_un: f64 = y_un.iter().sum();
        assert!(
            close(sum_un, 0.0, 1e-12),
            "unnormalized Σy = {sum_un} (must be 0)"
        );

        // ── Random-walk normalized L = I − D⁻¹A ──
        let mut y_rw = [0.0; 3];
        g.laplacian_spmv(&x, &mut y_rw, LaplacianKind::RandomWalk);
        assert!(close(y_rw[0], -0.55, 1e-12), "randomwalk y0 = {}", y_rw[0]);
        assert!(close(y_rw[1], -0.1, 1e-12), "randomwalk y1 = {}", y_rw[1]);
        assert!(close(y_rw[2], 0.65, 1e-12), "randomwalk y2 = {}", y_rw[2]);
        let sum_rw: f64 = y_rw.iter().sum();
        assert!(
            close(sum_rw, 0.0, 1e-12),
            "randomwalk Σy = {sum_rw} (must be 0)"
        );

        // ── Symmetric normalized L = I − D⁻¹/² A D⁻¹/² (regular ⇒ == randomwalk) ──
        let mut y_n = [0.0; 3];
        g.laplacian_spmv(&x, &mut y_n, LaplacianKind::Normalized);
        assert!(close(y_n[0], -0.55, 1e-12), "normalized y0 = {}", y_n[0]);
        assert!(close(y_n[1], -0.1, 1e-12), "normalized y1 = {}", y_n[1]);
        assert!(close(y_n[2], 0.65, 1e-12), "normalized y2 = {}", y_n[2]);
        // On a regular graph the symmetric normalized Laplacian has rows summing
        // to 0 too ⇒ conserved.
        let sum_n: f64 = y_n.iter().sum();
        assert!(
            close(sum_n, 0.0, 1e-12),
            "normalized Σy = {sum_n} (regular ⇒ 0)"
        );
    }

    // ── W2-2 GREEN: constant field x = 1 ⇒ L·1 = 0 for ANY graph (strong
    //    conservation invariant). Uses a NON-regular graph (path with degrees
    //    1,2,1) to prove Unnormalized + RandomWalk conserve for arbitrary
    //    topology; Normalized only conserves on regular graphs (triangle above).
    #[test]
    fn laplacian_spmv_constant_field_conserved_nonregular() {
        // Path 0—1—2 with degrees 1,2,1 (non-regular).
        let edges = [(0usize, 1, 1.0), (1, 0, 1.0), (1, 2, 1.0), (2, 1, 1.0)];
        let g = Csr::from_edges(3, &edges);
        let x = [1.0, 1.0, 1.0];

        let mut y_un = [0.0; 3];
        g.laplacian_spmv(&x, &mut y_un, LaplacianKind::Unnormalized);
        let s_un: f64 = y_un.iter().sum();
        assert!(
            close(s_un, 0.0, 1e-12),
            "path L·1 (unnormalized) = {s_un}, want 0"
        );

        let mut y_rw = [0.0; 3];
        g.laplacian_spmv(&x, &mut y_rw, LaplacianKind::RandomWalk);
        let s_rw: f64 = y_rw.iter().sum();
        assert!(
            close(s_rw, 0.0, 1e-12),
            "path L·1 (randomwalk) = {s_rw}, want 0"
        );
    }

    // ── W2-2 GREEN: equivalence against a hand-built dense Laplacian on the
    //    triangle for Unnormalized L = D − A  (y = L x == (D − A) x, exact).
    #[test]
    fn laplacian_spmv_equals_dense_laplacian_matrix() {
        let edges = [
            (0usize, 1, 1.0),
            (1, 0, 1.0),
            (1, 2, 1.0),
            (2, 1, 1.0),
            (0, 2, 1.0),
            (2, 0, 1.0),
        ];
        let g = Csr::from_edges(3, &edges);
        let x = [0.3, 0.7, 0.2];
        // Dense L = D − A for the triangle (degrees 2):
        //   L = [[2,-1,-1],[-1,2,-1],[-1,-1,2]]
        let l = [[2.0, -1.0, -1.0], [-1.0, 2.0, -1.0], [-1.0, -1.0, 2.0]];
        let mut dense = [0.0; 3];
        for i in 0..3 {
            dense[i] = l[i][0] * x[0] + l[i][1] * x[1] + l[i][2] * x[2];
        }
        let mut y = [0.0; 3];
        g.laplacian_spmv(&x, &mut y, LaplacianKind::Unnormalized);
        for i in 0..3 {
            assert!(
                close(y[i], dense[i], 1e-12),
                "node {i}: CSR laplacian_spmv {} != dense Lx {}",
                y[i],
                dense[i]
            );
        }
    }
}
