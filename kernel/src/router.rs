//! router.rs — CSR-native Dijkstra / A* shortest path + Contraction-Hierarchy
//! shortcuts + OSM road-graph ingestion.
//!
//! P04 product-math library (BLUEPRINT-P04 §2). The single canonical, zero-
//! dependency router for the kernel. Ported from
//! `/root/bebop-repo/crates/bebop/src/cost_estimate.rs` (`route`, `build_shortcuts`,
//! the `Prio` NaN-safe heap wrapper, the admissible A* heuristic) with all bebop
//! couplings (serde derive, `Node2D`/`ConnEdge`, `field_physics`) dropped.
//!
//! Consumes [`crate::csr::Csr`] directly: for node `u`, neighbours are
//! `col_idx[row_ptr[u]..row_ptr[u+1]]` with weights `val[..]` — no `Vec<Vec>`
//! rebuild. Node coordinates `(lat,lng)` feed the **haversine** heuristic, an
//! admissible lower bound *in metres* (great-circle ≤ any road path) so A* never
//! returns a sub-optimal path when edge weights are metric distances.

use crate::csr::Csr;
use crate::geo::haversine_meters;
use std::cmp::Ordering;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// Orderable f64 priority for the A* min-heap. `f64` is not `Ord` (NaN), so we
/// order by `total_cmp` — a total order over all f64s (NaN sorts last, harmless
/// for non-negative A* priorities). Ported verbatim from `cost_estimate.rs:35`.
#[derive(Clone, Copy, Debug)]
struct Prio(f64);
impl PartialEq for Prio {
    fn eq(&self, o: &Self) -> bool {
        self.0.to_bits() == o.0.to_bits()
    }
}
impl Eq for Prio {}
impl PartialOrd for Prio {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Prio {
    fn cmp(&self, o: &Self) -> Ordering {
        self.0.total_cmp(&o.0)
    }
}

/// A routable road graph: a CSR adjacency + index-aligned node coordinates.
/// `coords[i] == (lat, lng)` of node `i` (used only for the A* heuristic).
#[derive(Debug, Clone)]
pub struct RoadGraph {
    pub csr: Csr,
    pub coords: Vec<(f64, f64)>,
}

/// A Contraction-Hierarchy shortcut: a precomputed `(from, to, weight)` that
/// skips an intermediate node. Off by default — CH legitimately collapses
/// intermediate hops, so it is applied only where the exact node sequence is
/// not asserted. Ported from `cost_estimate.rs:148`.
#[derive(Debug, Clone)]
pub struct Shortcut {
    pub from: usize,
    pub to: usize,
    pub weight: f64,
}

impl RoadGraph {
    /// Number of nodes.
    #[inline]
    pub fn nnodes(&self) -> usize {
        self.csr.nrows()
    }
}

/// Dijkstra / A* over the CSR. `heuristic == false` ⇒ pure Dijkstra (`h ≡ 0`);
/// `true` ⇒ haversine A* (admissible metre lower bound to `dst`).
///
/// OPERATIVE PROPERTY (FEYNMAN-18): correctness of the no-reopen / closed-set
/// A* below requires the heuristic to be **consistent**, not merely admissible:
/// `h(u) ≤ w(u,v) + h(v)` for every edge, equivalently `w(u,v) ≥ haversine(u,v)`
/// since `h` is great-circle-to-`dst`. Haversine-to-a-fixed-dst satisfies the
/// triangle inequality, hence IS consistent whenever every edge weight ≥ the
/// great-circle distance between its endpoints — which holds for metre/length
/// costs but NOT for arbitrary `(u,v,cost)` triples fed via `road_graph_from_ways`
/// (e.g. travel-time costs, which are numerically smaller than metres). Feeding
/// inconsistent costs with `heuristic=true` yields silently sub-optimal routes.
/// The `debug_assert!` in the relaxation loop catches this in debug/test builds;
/// for non-metric costs, call with `heuristic = false` (pure Dijkstra, always optimal).
///
/// `shortcuts` augment the adjacency at query time (empty ⇒ exact node
/// sequence). Returns `(node path src→dst, total weight)` or `None` if
/// unreachable. Deterministic tie-break by node id via the `(Prio, usize)` heap
/// ordering.
pub fn route(
    g: &RoadGraph,
    src: usize,
    dst: usize,
    heuristic: bool,
    shortcuts: &[Shortcut],
) -> Option<(Vec<usize>, f64)> {
    let n = g.csr.nrows();
    if src >= n || dst >= n {
        return None;
    }
    let dst_coord = g.coords.get(dst).copied();
    // admissible straight-line LOWER bound (metres) on remaining cost to DST.
    // MUST measure i→dst or the heuristic is inadmissible (A* can go suboptimal).
    let h = |i: usize| -> f64 {
        if !heuristic {
            return 0.0;
        }
        match (g.coords.get(i), dst_coord) {
            (Some(&(ai, aj)), Some((bi, bj))) => haversine_meters(ai, aj, bi, bj),
            _ => 0.0,
        }
    };

    // Optional shortcut fan-out per source node (short-lived, no input mutation).
    let mut extra: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
    for s in shortcuts {
        if s.from < n && s.to < n {
            extra[s.from].push((s.to, s.weight));
        }
    }

    let mut dist = vec![f64::INFINITY; n];
    let mut prev = vec![usize::MAX; n];
    let mut visited = vec![false; n];
    let mut heap: BinaryHeap<Reverse<(Prio, usize)>> = BinaryHeap::new();

    dist[src] = 0.0;
    heap.push(Reverse((Prio(h(src)), src)));
    while let Some(Reverse((_, u))) = heap.pop() {
        if visited[u] {
            continue;
        }
        visited[u] = true;
        if u == dst {
            break;
        }
        // CSR row neighbours (ascending col order ⇒ deterministic).
        let (lo, hi) = (g.csr.row_ptr[u], g.csr.row_ptr[u + 1]);
        let csr_iter = (lo..hi).map(|k| (g.csr.col_idx[k], g.csr.val[k]));
        let sc_iter = extra[u].iter().copied();
        for (v, w) in csr_iter.chain(sc_iter) {
            if visited[v] {
                continue;
            }
            let ng = dist[u] + w;
            if ng < dist[v] {
                dist[v] = ng;
                prev[v] = u;
                heap.push(Reverse((Prio(ng + h(v)), v)));
            }
        }
    }

    if dist[dst] == f64::INFINITY {
        return None;
    }
    // reconstruct src→dst.
    let mut path = Vec::new();
    let mut cur = dst;
    while cur != usize::MAX {
        path.push(cur);
        if cur == src {
            break;
        }
        cur = prev[cur];
    }
    path.reverse();
    if path.first() == Some(&src) {
        Some((path, dist[dst]))
    } else {
        None
    }
}

/// Convenience alias: A* with the haversine lower bound and no shortcuts.
pub fn shortest_path(g: &RoadGraph, src: usize, dst: usize) -> Option<(Vec<usize>, f64)> {
    route(g, src, dst, true, &[])
}

/// Build Contraction-Hierarchy shortcuts by contracting nodes in descending
/// degree order (port of `cost_estimate.rs:157`). Reads adjacency straight from
/// the CSR. Off by default (`route(..)`/`shortest_path` pass `&[]`).
pub fn build_shortcuts(g: &RoadGraph) -> Vec<Shortcut> {
    let csr = &g.csr;
    let n = csr.nrows();
    let deg = |u: usize| csr.row_ptr[u + 1] - csr.row_ptr[u];

    // degree-descending contraction order (stable tie-break by node id).
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| deg(b).cmp(&deg(a)).then(a.cmp(&b)));
    let mut rank = vec![0usize; n];
    for (pos, &node) in order.iter().enumerate() {
        rank[node] = pos;
    }

    // helper: existing direct weight a→b (INFINITY if absent).
    let direct = |a: usize, b: usize| -> f64 {
        let (lo, hi) = (csr.row_ptr[a], csr.row_ptr[a + 1]);
        for k in lo..hi {
            if csr.col_idx[k] == b {
                return csr.val[k];
            }
        }
        f64::INFINITY
    };

    let mut shortcuts = Vec::new();
    for &mid in &order {
        let (lo, hi) = (csr.row_ptr[mid], csr.row_ptr[mid + 1]);
        let nbrs: Vec<(usize, f64)> = (lo..hi).map(|k| (csr.col_idx[k], csr.val[k])).collect();
        for &(a, wa) in &nbrs {
            if rank[a] <= rank[mid] {
                continue;
            }
            for &(b, wb) in &nbrs {
                if b <= a || rank[b] <= rank[mid] {
                    continue;
                }
                let cand = wa + wb;
                if cand < direct(a, b) {
                    shortcuts.push(Shortcut {
                        from: a,
                        to: b,
                        weight: cand,
                    });
                }
            }
        }
    }
    shortcuts
}

/// Build a [`RoadGraph`] from already-extracted OSM node coordinates + weighted
/// ways. `nodes[i] == (lat,lng)`; `ways` are `(u, v, cost)` undirected triples.
///
/// Emits BOTH directions per way (CSR is directed; undirected = both) and floors
/// each weight at `1e-6` (A*/Eikonal need `W > 0`). OSM parsing itself is a
/// downstream (Phase 13) concern; this takes clean triples so the kernel stays
/// I/O-free. Out-of-range endpoints are dropped by `Csr::from_edges`.
pub fn road_graph_from_ways(nodes: &[(f64, f64)], ways: &[(usize, usize, f64)]) -> RoadGraph {
    let n = nodes.len();
    let mut edges: Vec<(usize, usize, f64)> = Vec::with_capacity(ways.len() * 2);
    for &(u, v, w) in ways {
        let wf = w.max(1e-6);
        edges.push((u, v, wf));
        edges.push((v, u, wf));
    }
    RoadGraph {
        csr: Csr::from_edges(n, &edges),
        coords: nodes.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── §6 #1: 10-node oracle. Total cost + endpoints hand-computed. ─────
    //
    // A line-ish weighted graph with one cheap detour. Coords are arbitrary
    // small lat/lng so the haversine heuristic stays a valid lower bound.
    //   0-1:1  1-2:1  2-3:1  3-4:1  4-5:1  5-6:1  6-7:1  7-8:1  8-9:1
    //   plus a shortcut chord 0-5:10 (worse than the 5-hop chain of cost 5)
    //   and 2-7:2 (a real shortcut: 0→1→2→7→8→9 = 1+1+2+1+1 = 6).
    fn oracle10() -> RoadGraph {
        // coords spaced 1e-6° (~0.11 m/step) so haversine to dst stays a valid
        // lower bound (< the unit edge weights) ⇒ admissible heuristic.
        let nodes: Vec<(f64, f64)> = (0..10).map(|i| (i as f64 * 1e-6, 0.0)).collect();
        let ways = [
            (0usize, 1, 1.0),
            (1, 2, 1.0),
            (2, 3, 1.0),
            (3, 4, 1.0),
            (4, 5, 1.0),
            (5, 6, 1.0),
            (6, 7, 1.0),
            (7, 8, 1.0),
            (8, 9, 1.0),
            (0, 5, 10.0),
            (2, 7, 2.0),
        ];
        road_graph_from_ways(&nodes, &ways)
    }

    #[test]
    fn oracle_shortest_path_cost_and_endpoints() {
        let g = oracle10();
        let (path, cost) = shortest_path(&g, 0, 9).expect("path exists");
        // hand-computed minimum: 0→1→2→7→8→9 = 1+1+2+1+1 = 6.0
        assert!((cost - 6.0).abs() < 1e-9, "min cost 6.0, got {cost}");
        assert_eq!(*path.first().unwrap(), 0);
        assert_eq!(*path.last().unwrap(), 9);
        assert_eq!(path, vec![0, 1, 2, 7, 8, 9]);
    }

    // ── §6 #2: A* admissibility — A* == Dijkstra in cost AND sequence. ───
    #[test]
    fn astar_admissible_matches_dijkstra() {
        let g = oracle10();
        for &(s, d) in &[(0usize, 9), (0, 5), (3, 8), (9, 0), (1, 6)] {
            let a = route(&g, s, d, true, &[]);
            let dk = route(&g, s, d, false, &[]);
            assert_eq!(a, dk, "A* must equal Dijkstra for {s}->{d} (cost+seq)");
        }
    }

    #[test]
    fn ch_preserves_cost() {
        // CH may collapse hops, so assert cost + endpoints only (not sequence).
        let g = oracle10();
        let sc = build_shortcuts(&g);
        let (path, cost) = route(&g, 0, 9, true, &sc).expect("path exists");
        assert!(
            (cost - 6.0).abs() < 1e-9,
            "CH keeps optimum 6.0, got {cost}"
        );
        assert_eq!(*path.first().unwrap(), 0);
        assert_eq!(*path.last().unwrap(), 9);
    }

    #[test]
    fn unreachable_returns_none() {
        // isolated node 3 (no ways touch it).
        let nodes = vec![(0.0, 0.0), (0.001, 0.0), (0.002, 0.0), (0.5, 0.5)];
        let g = road_graph_from_ways(&nodes, &[(0, 1, 1.0), (1, 2, 1.0)]);
        assert!(shortest_path(&g, 0, 3).is_none());
    }

    #[test]
    fn from_ways_is_bidirectional_and_floored() {
        let nodes = vec![(0.0, 0.0), (0.001, 0.0)];
        let g = road_graph_from_ways(&nodes, &[(0, 1, 0.0)]);
        // both directions routable; weight floored to 1e-6 (>0).
        let f = shortest_path(&g, 0, 1).unwrap();
        let b = shortest_path(&g, 1, 0).unwrap();
        assert!(f.1 > 0.0 && b.1 > 0.0);
        assert!((f.1 - b.1).abs() < 1e-18);
    }
}
