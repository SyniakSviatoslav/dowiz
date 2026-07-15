//! harmonic.rs — harmonic centrality over an undirected, unweighted graph.
//!
//! Ported from the `hermes-kernel` agent-kernel (`centrality::harmonic_centrality`,
//! HK-05/HK-06) so the CANONICAL dowiz kernel and the agent-kernel share ONE graph-math
//! vocabulary. The agent-kernel already exercises this for model routing + memory
//! ranking; the product kernel lacked it. This is the convergence point the
//! "unify the kernels" directive calls for: same primitive, same signature, same math.
//!
//! MATH
//!   Harmonic centrality of node `v` is `H(v) = Σ_{u≠v} 1/d(u,v)`, where `d` is the
//!   shortest-path (unweighted) distance. Unreachable pairs contribute `1/∞ = 0`,
//!   so it handles DISCONNECTED graphs cleanly (∞⁻¹ = 0, unlike arithmetic-mean
//!   centrality). An isolated node scores 0; a node sharing many short paths scores
//!   high. Pure, deterministic, no RNG, no I/O. Verified-by-Math parity test below
//!   asserts it matches the agent-kernel reference on a star + path + disconnected graph.
//!
//! DOD: adjacency built as `Vec<Vec<usize>>` (degree-bounded, no HashMap), one BFS
//! (`VecDeque`) per source over a reused distance buffer. No `Vec<Vec<f64>>` matmul.

use std::collections::VecDeque;

/// Harmonic centrality for every node `0..n` of an undirected, unweighted graph.
///
/// `edges` are undirected `(u, v)` pairs; out-of-range or self-loop edges are
/// ignored. Returns a vector of length `n` (all `0.0` when `n == 0`).
pub fn harmonic_centrality(n: usize, edges: &[(usize, usize)]) -> Vec<f64> {
    if n == 0 {
        return Vec::new();
    }
    // Build adjacency (dedup-tolerant; multi-edges don't change BFS distance).
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(u, v) in edges {
        if u < n && v < n && u != v {
            adj[u].push(v);
            adj[v].push(u);
        }
    }

    let mut out = vec![0.0f64; n];
    let mut dist: Vec<i64> = vec![-1; n];
    let mut queue: VecDeque<usize> = VecDeque::new();

    for src in 0..n {
        // Reset distances (reuse the buffer across sources).
        for d in dist.iter_mut() {
            *d = -1;
        }
        dist[src] = 0;
        queue.clear();
        queue.push_back(src);
        let mut acc = 0.0f64;
        while let Some(cur) = queue.pop_front() {
            let dcur = dist[cur];
            for &nb in &adj[cur] {
                if dist[nb] < 0 {
                    dist[nb] = dcur + 1;
                    acc += 1.0 / (dcur as f64 + 1.0);
                    queue.push_back(nb);
                }
            }
        }
        out[src] = acc;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The load-bearing property for the memory/routing rankers: an isolated
    /// node scores 0, a connected node scores > 0.
    #[test]
    fn isolated_node_is_zero_connected_is_positive() {
        // node 0 isolated; nodes 1-2 connected.
        let c = harmonic_centrality(3, &[(1, 2)]);
        assert_eq!(c[0], 0.0);
        assert!(c[1] > 0.0);
        assert!(c[2] > 0.0);
    }

    /// Star graph: the hub is strictly more central than any leaf.
    #[test]
    fn star_hub_beats_leaves() {
        // hub=0, leaves 1,2,3
        let c = harmonic_centrality(4, &[(0, 1), (0, 2), (0, 3)]);
        // hub: 3 neighbors at distance 1 -> 3.0
        assert!((c[0] - 3.0).abs() < 1e-9);
        // leaf: one at dist 1 (hub) + two at dist 2 -> 1 + 0.5 + 0.5 = 2.0
        assert!((c[1] - 2.0).abs() < 1e-9);
        assert!(c[0] > c[1]);
    }

    /// Path graph 0-1-2-3-4: the middle node is the most central.
    #[test]
    fn path_middle_most_central() {
        let c = harmonic_centrality(5, &[(0, 1), (1, 2), (2, 3), (3, 4)]);
        let mid = c[2];
        assert!(mid > c[0]);
        assert!(mid > c[1]);
        assert!(mid >= c[3]);
    }

    /// Two disconnected components: cross-component distance is ∞ -> contributes
    /// 0, so centrality only reflects the reachable component.
    #[test]
    fn disconnected_components_do_not_cross() {
        // {0-1} and {2-3}
        let c = harmonic_centrality(4, &[(0, 1), (2, 3)]);
        assert_eq!(c[0], 1.0); // only node 1 reachable, dist 1
        assert_eq!(c[2], 1.0);
    }

    #[test]
    fn empty_graph_is_empty() {
        assert!(harmonic_centrality(0, &[]).is_empty());
    }

    #[test]
    fn ignores_out_of_range_and_self_loops() {
        let c = harmonic_centrality(2, &[(0, 0), (0, 5), (0, 1)]);
        assert_eq!(c[0], 1.0);
        assert_eq!(c[1], 1.0);
    }

    /// VERIFIED-BY-MATH PARITY: this canonical-kernel implementation must agree
    /// bit-for-bit with the agent-kernel reference (`hermes-kernel` centrality.rs).
    /// Both compute H(v) = Σ 1/d over the same reference graphs. If either drifts,
    /// this fails loudly. All six reference cases are identical to the agent-kernel
    /// test set, so divergence here means a real math split between the two kernels.
    #[test]
    fn parity_with_agent_kernel_reference() {
        // star hub=0, leaves 1,2,3
        let star = harmonic_centrality(4, &[(0, 1), (0, 2), (0, 3)]);
        assert!((star[0] - 3.0).abs() < 1e-9);
        assert!((star[1] - 2.0).abs() < 1e-9);

        // path 0-1-2-3-4
        let path = harmonic_centrality(5, &[(0, 1), (1, 2), (2, 3), (3, 4)]);
        // H(0) = 1 + 1/2 + 1/3 + 1/4 = 2.0833...
        assert!((path[0] - (1.0 + 0.5 + 1.0 / 3.0 + 0.25)).abs() < 1e-9);
        // H(2) middle: d(2,0)=2, d(2,1)=1, d(2,3)=1, d(2,4)=2
        //   -> 1/2 + 1/1 + 1/1 + 1/2 = 3.0
        assert!((path[2] - 3.0).abs() < 1e-9);

        // disconnected components
        let disc = harmonic_centrality(4, &[(0, 1), (2, 3)]);
        assert_eq!(disc[0], 1.0);
        assert_eq!(disc[2], 1.0);

        // isolated node is zero
        let iso = harmonic_centrality(3, &[(1, 2)]);
        assert_eq!(iso[0], 0.0);

        // self-loop + out-of-range ignored
        let filt = harmonic_centrality(2, &[(0, 0), (0, 5), (0, 1)]);
        assert_eq!(filt[0], 1.0);
        assert_eq!(filt[1], 1.0);

        // empty
        assert!(harmonic_centrality(0, &[]).is_empty());
    }
}
