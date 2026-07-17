//! incidence.rs — the discrete gradient / divergence (oriented-edge incidence)
//! primitive, and the CANONICAL reference Laplacian `L = Bᵀ W B`.
//!
//! WHY THIS EXISTS (BLUEPRINT-E1 §2a, Correspondence P2 "one concept, one
//! primitive").  The graph Laplacian `L = D − A` is implemented ≥3 times in the
//! tree — dense (`spectral::laplacian`), sparse-CSR (`csr::laplacian_spmv`), and
//! as an implicit grid stencil (`engine::field_frame::laplacian`) — and nothing
//! pins them to one another.  Worse, the grid stencil uses the OPPOSITE sign
//! convention: `field_frame` computes `∇²U = −(D−A)U`, while `csr`/`spectral`
//! compute `+(D−A)`.  Each side is internally correct and internally tested; the
//! *relationship between them was asserted by nothing* — a latent sign bug that
//! produces zero failures until a caller crosses the seam and gets anti-diffusion
//! (divergence).
//!
//! This module is the small, hand-oracle-tested REFERENCE operator every other
//! Laplacian is parity-checked against.  It is NOT a variant of CSR SpMV: an
//! oriented-edge factorization `L = BᵀWB` is a genuinely different shape (one
//! oriented edge per undirected pair, a rectangular `n_edges × n` incidence `B`),
//! so it earns its own primitive rather than being bolted onto `Csr`'s symmetric
//! two-edge adjacency.  It shares the `&[(usize,usize,f64)]` edge-tuple input
//! contract with [`crate::csr::Csr::from_edges`].
//!
//! CONVENTION.  `laplacian` emits the **POSITIVE** graph Laplacian `+(D−A)`,
//! matching `csr::laplacian_spmv(_, Unnormalized)` and `spectral::laplacian`.
//! The engine's grid stencil is its negation; that identity is pinned by a
//! sign-pin test at the kernel↔engine seam (engine `field_energy` module).
//!
//! DETERMINISTIC, zero deps, pure `std`.

/// Oriented-edge incidence structure.  One oriented edge per undirected pair,
/// canonically oriented so the head index exceeds the tail index (`head > tail`);
/// the choice of orientation does not affect `L = BᵀWB`, but a canonical one
/// makes `from_edges` order-independent and the hand oracles reproducible.
#[derive(Debug, Clone, PartialEq)]
pub struct Incidence {
    /// Number of nodes.
    n: usize,
    /// Oriented edges as `(tail, head, weight)` with `tail <= head` (canonical).
    /// A self-loop (`tail == head`) contributes zero flow and is harmless.
    edges: Vec<(usize, usize, f64)>,
}

impl Incidence {
    /// Build from an UNDIRECTED edge list — ONE tuple per undirected pair
    /// (unlike [`crate::csr::Csr::from_edges`], which wants both directions).
    /// Each `(s, d, w)` is oriented canonically to `(min, max, w)` so the head
    /// index is the larger.  Edges with an endpoint `>= n` are ignored (matching
    /// CSR's out-of-range handling).
    pub fn from_edges(n: usize, edges: &[(usize, usize, f64)]) -> Self {
        let mut oriented = Vec::with_capacity(edges.len());
        for &(s, d, w) in edges {
            if s < n && d < n {
                let (tail, head) = if s <= d { (s, d) } else { (d, s) };
                oriented.push((tail, head, w));
            }
        }
        Self { n, edges: oriented }
    }

    /// Number of nodes `n`.
    #[inline]
    pub fn nodes(&self) -> usize {
        self.n
    }

    /// Number of oriented edges (rows of `B`).
    #[inline]
    pub fn n_edges(&self) -> usize {
        self.edges.len()
    }

    /// Discrete gradient `B x`: node-field → edge-flow.  For each oriented edge
    /// `e = (tail, head)`, `(B x)_e = x_head − x_tail`.  Unweighted (`W` is
    /// applied separately in [`Incidence::laplacian`]).  Output length =
    /// `n_edges`.
    pub fn grad(&self, x: &[f64]) -> Vec<f64> {
        debug_assert_eq!(x.len(), self.n, "incidence::grad: x length must be n");
        self.edges
            .iter()
            .map(|&(tail, head, _)| x[head] - x[tail])
            .collect()
    }

    /// Discrete divergence `Bᵀ flow`: edge-flow → node-field.  For each oriented
    /// edge `e = (tail, head)`, `flow_e` adds to the head node and subtracts from
    /// the tail node (`B` has `+1` at head, `−1` at tail, so `Bᵀ` scatters the
    /// flow with those signs).  Output length = `n`.
    pub fn div(&self, flow: &[f64]) -> Vec<f64> {
        debug_assert_eq!(
            flow.len(),
            self.edges.len(),
            "incidence::div: flow length must be n_edges"
        );
        let mut out = vec![0.0f64; self.n];
        for (&(tail, head, _), &f) in self.edges.iter().zip(flow) {
            out[head] += f;
            out[tail] -= f;
        }
        out
    }

    /// The canonical reference Laplacian `L x = div(W · grad(x)) = (BᵀWB) x`.
    /// This is the **POSITIVE** graph Laplacian `+(D−A)x`: for a single edge
    /// `(i,j,w)` it contributes `w(x_i − x_j)` to node `i` and `w(x_j − x_i)` to
    /// node `j`, so row `i` sums to `deg_i·x_i − Σ_j A_ij x_j`.  Matches
    /// `csr::laplacian_spmv(_, Unnormalized)` and `spectral::laplacian`.
    pub fn laplacian(&self, x: &[f64]) -> Vec<f64> {
        debug_assert_eq!(x.len(), self.n, "incidence::laplacian: x length must be n");
        let mut out = vec![0.0f64; self.n];
        // Fused grad → weight → div: one pass over edges, no intermediate alloc.
        for &(tail, head, w) in &self.edges {
            let flow = w * (x[head] - x[tail]); // W · B x
            out[head] += flow; // Bᵀ (…)
            out[tail] -= flow;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::csr::{Csr, LaplacianKind};

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }
    fn close_vec(a: &[f64], b: &[f64], tol: f64) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(&x, &y)| close(x, y, tol))
    }

    // ── HAND ORACLE — K₃ (triangle) grad + laplacian ───────────────────────
    // Edges (0,1),(1,2),(0,2) unit weight → canonical (0,1),(1,2),(0,2).
    // x = [0.1, 0.4, 0.9]:
    //   grad = [x1−x0, x2−x1, x2−x0] = [0.3, 0.5, 0.8]
    //   L = (D−A)x = [2·.1−.4−.9, 2·.4−.1−.9, 2·.9−.1−.4] = [−1.1, −0.2, 1.3]
    #[test]
    fn incidence_k3_grad_and_laplacian_hand_oracle() {
        let inc = Incidence::from_edges(3, &[(0, 1, 1.0), (1, 2, 1.0), (0, 2, 1.0)]);
        assert_eq!(inc.n_edges(), 3);
        let x = [0.1, 0.4, 0.9];
        assert!(close_vec(&inc.grad(&x), &[0.3, 0.5, 0.8], 1e-12));
        assert!(close_vec(&inc.laplacian(&x), &[-1.1, -0.2, 1.3], 1e-12));
    }

    // ── HAND ORACLE — P₃ (path 0—1—2) laplacian ────────────────────────────
    // Edges (0,1),(1,2). Dense L = [[1,-1,0],[-1,2,-1],[0,-1,1]].
    // x = [0.3,0.7,0.2] → L x = [0.3−0.7, −0.3+1.4−0.2, −0.7+0.2] = [−0.4, 0.9, −0.5]
    #[test]
    fn incidence_p3_laplacian_hand_oracle() {
        let inc = Incidence::from_edges(3, &[(0, 1, 1.0), (1, 2, 1.0)]);
        let x = [0.3, 0.7, 0.2];
        assert!(close_vec(&inc.laplacian(&x), &[-0.4, 0.9, -0.5], 1e-12));
    }

    // ── HAND ORACLE — div is the ADJOINT of grad (div == Bᵀ == gradᵀ) ───────
    // ⟨B x, f⟩_edges == ⟨x, Bᵀ f⟩_nodes  for all x, f  (the defining identity).
    #[test]
    fn incidence_div_is_transpose_of_grad() {
        let inc = Incidence::from_edges(4, &[(0, 1, 1.0), (1, 2, 1.0), (2, 3, 1.0), (0, 3, 1.0)]);
        let x = [1.5, -2.0, 0.5, 3.0];
        let f = [0.7, -1.1, 2.3, 0.4]; // arbitrary edge flow
        let lhs: f64 = inc.grad(&x).iter().zip(&f).map(|(&g, &fe)| g * fe).sum();
        let rhs: f64 = x.iter().zip(inc.div(&f)).map(|(&xi, di)| xi * di).sum();
        assert!(close(lhs, rhs, 1e-12), "⟨Bx,f⟩={lhs} != ⟨x,Bᵀf⟩={rhs}");
    }

    // ── HAND ORACLE — weighted edges scale the Laplacian rows ───────────────
    // Path 0—1—2 with weights w01=2, w12=3. L = [[2,-2,0],[-2,5,-3],[0,-3,3]].
    #[test]
    fn incidence_weighted_laplacian_hand_oracle() {
        let inc = Incidence::from_edges(3, &[(0, 1, 2.0), (1, 2, 3.0)]);
        let x = [1.0, 0.0, 0.0];
        // L·e0 = column 0 of L = [2, -2, 0].
        assert!(close_vec(&inc.laplacian(&x), &[2.0, -2.0, 0.0], 1e-12));
    }

    // ── L·1 = 0 for ANY graph (rows of D−A sum to 0 ⇒ constant field conserved)
    #[test]
    fn incidence_constant_field_is_null() {
        let inc = Incidence::from_edges(4, &[(0, 1, 1.0), (1, 2, 1.0), (2, 3, 1.0), (0, 2, 1.5)]);
        let ones = [1.0, 1.0, 1.0, 1.0];
        for &v in &inc.laplacian(&ones) {
            assert!(close(v, 0.0, 1e-12), "L·1 must be 0, got {v}");
        }
    }

    // ── PARITY #1 — incidence.laplacian == csr.laplacian_spmv(Unnormalized) ──
    // The SAME undirected graph: incidence gets ONE tuple per pair, CSR gets
    // BOTH directions (its contract). Tested on K₃ and P₃ with weights.
    #[test]
    fn parity_incidence_equals_csr_unnormalized() {
        // K₃ (unit weights).
        let inc_k3 = Incidence::from_edges(3, &[(0, 1, 1.0), (1, 2, 1.0), (0, 2, 1.0)]);
        let csr_k3 = Csr::from_edges(
            3,
            &[
                (0, 1, 1.0),
                (1, 0, 1.0),
                (1, 2, 1.0),
                (2, 1, 1.0),
                (0, 2, 1.0),
                (2, 0, 1.0),
            ],
        );
        for x in [[0.1, 0.4, 0.9], [-2.0, 3.5, 1.0], [1.0, 1.0, 1.0]] {
            let mut y_csr = [0.0; 3];
            csr_k3.laplacian_spmv(&x, &mut y_csr, LaplacianKind::Unnormalized);
            assert!(
                close_vec(&inc_k3.laplacian(&x), &y_csr, 1e-12),
                "K3 parity failed for x={x:?}"
            );
        }

        // P₃ path with non-unit weights (w01=2, w12=3): non-regular + weighted.
        let inc_p = Incidence::from_edges(3, &[(0, 1, 2.0), (1, 2, 3.0)]);
        let csr_p = Csr::from_edges(3, &[(0, 1, 2.0), (1, 0, 2.0), (1, 2, 3.0), (2, 1, 3.0)]);
        for x in [[0.3, 0.7, 0.2], [1.5, -1.0, 4.0]] {
            let mut y_csr = [0.0; 3];
            csr_p.laplacian_spmv(&x, &mut y_csr, LaplacianKind::Unnormalized);
            assert!(
                close_vec(&inc_p.laplacian(&x), &y_csr, 1e-12),
                "P3 weighted parity failed for x={x:?}"
            );
        }
    }

    // ── PARITY #2 — incidence.laplacian == spectral::laplacian(adj) · x ──────
    // Bind the reference to the DENSE +(D−A) operator too.
    #[test]
    fn parity_incidence_equals_spectral_dense() {
        let inc = Incidence::from_edges(3, &[(0, 1, 1.0), (1, 2, 1.0), (0, 2, 1.0)]);
        // Dense symmetric adjacency for the same K₃.
        let adj = vec![
            vec![0.0, 1.0, 1.0],
            vec![1.0, 0.0, 1.0],
            vec![1.0, 1.0, 0.0],
        ];
        let l = crate::spectral::laplacian(&adj); // dense +(D−A)
        let x = [0.3, 0.7, 0.2];
        // Dense L · x.
        let mut dense = [0.0f64; 3];
        for i in 0..3 {
            dense[i] = (0..3).map(|j| l[i][j] * x[j]).sum();
        }
        assert!(
            close_vec(&inc.laplacian(&x), &dense, 1e-12),
            "incidence vs dense spectral::laplacian mismatch"
        );
    }

    // ── PARITY #3 (Normalized-branch coverage, BLUEPRINT §7 item 3 / §9b) ────
    // The live trigger caller (`engine::bridge.rs:125`) uses
    // `laplacian_spmv(Normalized)`, an operator the Unnormalized parity web does
    // NOT cover. Bind it to the incidence reference via the exact factorization
    //   L_sym = D^{-1/2} · L_un · D^{-1/2}     (valid on connected graphs, deg>0)
    // so the branch that fires on the trigger caller is itself pinned.
    #[test]
    fn parity_incidence_reference_matches_csr_normalized() {
        // Helper: symmetric-normalized reference built FROM the incidence primitive.
        fn normalized_ref(inc: &Incidence, deg: &[f64], x: &[f64]) -> Vec<f64> {
            let inv_sqrt: Vec<f64> = deg
                .iter()
                .map(|&d| if d > 0.0 { d.sqrt().recip() } else { 0.0 })
                .collect();
            let scaled: Vec<f64> = x.iter().zip(&inv_sqrt).map(|(&xi, &s)| xi * s).collect();
            let lun = inc.laplacian(&scaled); // L_un · (D^{-1/2} x)
            lun.iter().zip(&inv_sqrt).map(|(&v, &s)| v * s).collect() // D^{-1/2} · (…)
        }

        // K₃ (regular, deg 2) and P₃ (non-regular, deg 1,2,1) — both connected.
        let inc_k3 = Incidence::from_edges(3, &[(0, 1, 1.0), (1, 2, 1.0), (0, 2, 1.0)]);
        let csr_k3 = Csr::from_edges(
            3,
            &[
                (0, 1, 1.0),
                (1, 0, 1.0),
                (1, 2, 1.0),
                (2, 1, 1.0),
                (0, 2, 1.0),
                (2, 0, 1.0),
            ],
        );
        for x in [[0.1, 0.4, 0.9], [1.0, -2.0, 0.5]] {
            let mut y_csr = [0.0; 3];
            csr_k3.laplacian_spmv(&x, &mut y_csr, LaplacianKind::Normalized);
            let y_ref = normalized_ref(&inc_k3, &[2.0, 2.0, 2.0], &x);
            assert!(
                close_vec(&y_ref, &y_csr, 1e-12),
                "K3 Normalized parity failed for x={x:?}: ref={y_ref:?} csr={y_csr:?}"
            );
        }

        let inc_p = Incidence::from_edges(3, &[(0, 1, 1.0), (1, 2, 1.0)]);
        let csr_p = Csr::from_edges(3, &[(0, 1, 1.0), (1, 0, 1.0), (1, 2, 1.0), (2, 1, 1.0)]);
        for x in [[0.3, 0.7, 0.2], [2.0, -1.0, 4.0]] {
            let mut y_csr = [0.0; 3];
            csr_p.laplacian_spmv(&x, &mut y_csr, LaplacianKind::Normalized);
            let y_ref = normalized_ref(&inc_p, &[1.0, 2.0, 1.0], &x);
            assert!(
                close_vec(&y_ref, &y_csr, 1e-12),
                "P3 Normalized parity failed for x={x:?}: ref={y_ref:?} csr={y_csr:?}"
            );
        }
    }
}
