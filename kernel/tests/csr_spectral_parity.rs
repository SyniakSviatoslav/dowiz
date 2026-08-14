//! Cross-module parity pins for the CSR graph after its extraction to no_std
//! `dowiz-core`. The `csr` module was previously self-contained in the kernel
//! and pinned against `spectral` directly inside its own `#[cfg(test)]` module.
//! Once `csr` moved into `dowiz-core` (which cannot depend on `spectral`), those
//! cross-module pins had to move here, where BOTH representations are in scope:
//!
//!   (1) ENERGY PARITY — `spectral::csr_energy(g)` (the re-homed `Csr::energy`)
//!       must equal `spectral::graph_energy(&g.to_adjacency())`.
//!   (2) LAPLACIAN PARITY — `Csr::laplacian_spmv(Unnormalized)` (matrix-free,
//!       never forms L) must equal the DENSE `spectral::laplacian(adj)` applied
//!       as an ordinary mat-vec, for every labeled undirected graph on N ≤ 5
//!       nodes plus curated weighted/asymmetric/self-loop cases.
//!
//! This is the same ITEM 18 (§14 / §26(d)) parity pin as before — relocated,
//! not weakened.

use dowiz_kernel::csr::{Csr, LaplacianKind};
use dowiz_kernel::spectral::{csr_energy, graph_energy, laplacian};

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

/// Dense L = D − A with row-sum degree d_i = Σ_j A_ij (self-loops cancel on the
/// diagonal), the exact convention `spectral::laplacian` implements.
fn dense_laplacian_matvec(adj: &[Vec<f64>], x: &[f64]) -> Vec<f64> {
    let l = laplacian(adj);
    let n = l.len();
    (0..n).map(|i| (0..n).map(|j| l[i][j] * x[j]).sum()).collect()
}

/// Assert dense `laplacian()`·x == `laplacian_spmv(Unnormalized)` for one
/// (adjacency, field) pair, to `tol`. `adj` must be square n×n.
fn assert_unnormalized_parity(adj: &[Vec<f64>], x: &[f64], tol: f64) {
    let n = adj.len();
    assert_eq!(x.len(), n, "field length must equal node count");
    let dense = dense_laplacian_matvec(adj, x);
    let csr = Csr::from_dense(adj);
    assert_eq!(csr.nrows(), n, "from_dense must preserve node count");
    let mut got = vec![0.0; n];
    csr.laplacian_spmv(x, &mut got, LaplacianKind::Unnormalized);
    for i in 0..n {
        assert!(
            (got[i] - dense[i]).abs() <= tol,
            "Unnormalized parity FAILED at node {i}: matrix-free {} != dense L·x {} (Δ={:.3e})\n  adj = {adj:?}\n  x   = {x:?}",
            got[i],
            dense[i],
            (got[i] - dense[i]).abs(),
        );
    }
}

/// Deterministic NON-TRIVIAL field (never all-zeros / all-ones / constant).
fn nontrivial_field(n: usize) -> Vec<f64> {
    let pat = [0.3f64, -0.7, 1.1, -0.25, 0.85, -0.4, 0.6];
    let x: Vec<f64> = (0..n)
        .map(|i| pat[i % pat.len()] + 0.13 * i as f64)
        .collect();
    if n >= 2 {
        assert!(
            x.iter().any(|&v| v != x[0]),
            "test field must be non-constant"
        );
    }
    x
}

// The only divergence is float summation reordering, bounded well under 1e-12
// for the small n and O(1)-magnitude weights/fields used here.
const PARITY_TOL: f64 = 1e-12;

// ── ENERGY PARITY ──────────────────────────────────────────────────────────
#[test]
fn csr_energy_matches_graph_energy_k3_and_empty() {
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
    let e = csr_energy(&g);
    assert!(close(e, 4.0, 1e-6), "csr_energy(K3)=4, got {e}");
    // Direct pin against graph_energy (the value must be identical, not just ≈4).
    assert!(close(e, graph_energy(&g.to_adjacency()), 1e-12));

    // Empty (edgeless) graph has energy 0.
    let empty = Csr::from_edges(4, &[]);
    assert!(close(csr_energy(&empty), 0.0, 1e-9));
    assert!(close(
        csr_energy(&empty),
        graph_energy(&empty.to_adjacency()),
        1e-12
    ));
}

// ── LAPLACIAN PARITY: exhaustive labeled undirected graphs N=1..=5 ─────────
#[test]
fn laplacian_dense_vs_spmv_parity_exhaustive_small() {
    let mut graphs_tested = 0usize;
    for n in 1..=5usize {
        let pairs: Vec<(usize, usize)> = (0..n)
            .flat_map(|i| ((i + 1)..n).map(move |j| (i, j)))
            .collect();
        let m = pairs.len(); // number of possible undirected edges
        let x = nontrivial_field(n);
        for mask in 0..(1u32 << m) {
            let mut adj = vec![vec![0.0f64; n]; n];
            for (b, &(i, j)) in pairs.iter().enumerate() {
                if mask & (1 << b) != 0 {
                    adj[i][j] = 1.0;
                    adj[j][i] = 1.0;
                }
            }
            assert_unnormalized_parity(&adj, &x, PARITY_TOL);
            graphs_tested += 1;
        }
    }
    assert_eq!(
        graphs_tested,
        1 + 2 + 8 + 64 + 1024,
        "exhaustive graph count"
    );
}

// ── LAPLACIAN PARITY: curated weighted / asymmetric / self-loop cases ──────
#[test]
fn laplacian_dense_vs_spmv_parity_curated() {
    // Path P4 (non-regular, degrees 1,2,2,1).
    let path = vec![
        vec![0.0, 1.0, 0.0, 0.0],
        vec![1.0, 0.0, 1.0, 0.0],
        vec![0.0, 1.0, 0.0, 1.0],
        vec![0.0, 0.0, 1.0, 0.0],
    ];
    // Star S4 (center 0 connected to 1,2,3).
    let star = vec![
        vec![0.0, 1.0, 1.0, 1.0],
        vec![1.0, 0.0, 0.0, 0.0],
        vec![1.0, 0.0, 0.0, 0.0],
        vec![1.0, 0.0, 0.0, 0.0],
    ];
    // Complete K4.
    let k4 = vec![
        vec![0.0, 1.0, 1.0, 1.0],
        vec![1.0, 0.0, 1.0, 1.0],
        vec![1.0, 1.0, 0.0, 1.0],
        vec![1.0, 1.0, 1.0, 0.0],
    ];
    // Two disconnected components: edge {0,1} + triangle {2,3,4}.
    let disconnected = vec![
        vec![0.0, 1.0, 0.0, 0.0, 0.0],
        vec![1.0, 0.0, 0.0, 0.0, 0.0],
        vec![0.0, 0.0, 0.0, 1.0, 1.0],
        vec![0.0, 0.0, 1.0, 0.0, 1.0],
        vec![0.0, 0.0, 1.0, 1.0, 0.0],
    ];
    // Empty graph — degree 0 everywhere ⇒ L·x = 0.
    let empty = vec![vec![0.0f64; 4]; 4];
    // Weighted, ASYMMETRIC graph WITH a self-loop on node 1 (A_11 = 0.5).
    // Proves (i) weighted parity, (ii) row-sum degree on non-symmetric A,
    // (iii) self-loop cancellation on the diagonal.
    let weighted_selfloop = vec![
        vec![0.0, 2.5, 0.0],
        vec![1.0, 0.5, 3.0],
        vec![0.0, 0.0, 0.0],
    ];
    // Single node carrying only a self-loop — L_00 = d_0 − A_00 = 0.
    let single_selfloop = vec![vec![4.0]];

    for adj in [
        &path,
        &star,
        &k4,
        &disconnected,
        &empty,
        &weighted_selfloop,
        &single_selfloop,
    ] {
        let x = nontrivial_field(adj.len());
        assert_unnormalized_parity(adj, &x, PARITY_TOL);
    }
}
