//! dsu.rs — Disjoint-Set Union (union-find) + Kruskal MST.
//!
//! P04 product-math library (BLUEPRINT-P04 §3). The single canonical, zero-
//! dependency DSU/MST primitive for the kernel. Consumed by:
//!   * `cgraph::c_components` (parity-swap: the flood-fill delegates here),
//!   * Phase 9 mesh-heal (partition detect + spanning-tree overlay),
//!   * Phase 13 partition-tolerant delivery.
//!
//! DETERMINISM CONTRACT. `components(present)` reproduces the cgraph flood-fill
//! ordering **byte-for-byte**: members ascending WITHIN a set; sets ordered by
//! ascending minimum member. `kruskal_mst` sorts edges by
//! `(weight, min(u,v), max(u,v))` so its output is reproducible regardless of
//! input edge order. Pure `std`, zero external deps.

use std::cmp::Ordering;

/// Disjoint-set (union-find) with path compression + union by rank.
#[derive(Debug, Clone)]
pub struct Dsu {
    parent: Vec<usize>,
    rank: Vec<usize>,
}

impl Dsu {
    /// A forest of `n` singleton sets `{0}, {1}, …, {n-1}`.
    pub fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            rank: vec![0; n],
        }
    }

    /// Number of elements the structure was built over.
    #[inline]
    pub fn len(&self) -> usize {
        self.parent.len()
    }

    /// True when the structure holds no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.parent.is_empty()
    }

    /// Representative (root) of `x`'s set, with **path compression** (halving).
    pub fn find(&mut self, x: usize) -> usize {
        let mut root = x;
        while self.parent[root] != root {
            root = self.parent[root];
        }
        // Path compression: point every node on the walk directly at the root.
        let mut cur = x;
        while self.parent[cur] != root {
            let next = self.parent[cur];
            self.parent[cur] = root;
            cur = next;
        }
        root
    }

    /// Merge the sets containing `a` and `b`, **union by rank**. Returns `false`
    /// (no-op) if they were already in the same set, `true` if a merge occurred.
    pub fn union(&mut self, a: usize, b: usize) -> bool {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra == rb {
            return false;
        }
        match self.rank[ra].cmp(&self.rank[rb]) {
            Ordering::Less => self.parent[ra] = rb,
            Ordering::Greater => self.parent[rb] = ra,
            Ordering::Equal => {
                self.parent[rb] = ra;
                self.rank[ra] += 1;
            }
        }
        true
    }

    /// True iff `a` and `b` belong to the same set.
    pub fn connected(&mut self, a: usize, b: usize) -> bool {
        self.find(a) == self.find(b)
    }

    /// Canonical partition over the nodes flagged `present[i] == true`.
    ///
    /// BYTE-PARITY CONTRACT (with `cgraph::c_components`):
    ///   * members ascending WITHIN each set,
    ///   * sets ordered by ascending minimum member.
    ///
    /// Absent nodes never appear. Implementation: bucket present nodes by root
    /// (nodes scanned `0..n`, so each bucket is already ascending and keyed at
    /// its smallest member), then order the buckets by that smallest member.
    pub fn components(&mut self, present: &[bool]) -> Vec<Vec<usize>> {
        let n = self.parent.len().min(present.len());
        // root -> (min_member, members-ascending)
        let mut roots: Vec<usize> = Vec::new();
        let mut buckets: Vec<Vec<usize>> = Vec::new();
        // index of a root within `roots`, or usize::MAX
        let mut slot = vec![usize::MAX; self.parent.len()];
        for i in 0..n {
            if !present[i] {
                continue;
            }
            let r = self.find(i);
            let s = slot[r];
            if s == usize::MAX {
                slot[r] = roots.len();
                roots.push(r);
                buckets.push(vec![i]); // i is the smallest member (first seen)
            } else {
                buckets[s].push(i); // ascending: i increases as we scan
            }
        }
        // Order sets by ascending minimum member. Each bucket[0] is its min.
        let mut order: Vec<usize> = (0..buckets.len()).collect();
        order.sort_by_key(|&b| buckets[b][0]);
        order
            .into_iter()
            .map(|b| std::mem::take(&mut buckets[b]))
            .collect()
    }
}

/// Orderable f64 for the MST edge sort (NaN-safe via `total_cmp`).
fn weight_cmp(a: f64, b: f64) -> Ordering {
    a.total_cmp(&b)
}

/// Kruskal minimum spanning tree/forest over undirected weighted edges.
///
/// Deterministic tie-break: edges are sorted by `(weight, min(u,v), max(u,v))`
/// ascending, so the chosen edge set is identical regardless of input order.
/// Returns `(chosen_edges, total_weight)`. Self-loops and out-of-range
/// endpoints (`>= n`) are ignored. On a disconnected graph this yields an MST
/// *forest* (the minimum over each component).
///
/// Each returned edge is normalised to `(min(u,v), max(u,v), weight)`.
pub fn kruskal_mst(n: usize, edges: &[(usize, usize, f64)]) -> (Vec<(usize, usize, f64)>, f64) {
    let mut es: Vec<(usize, usize, f64)> = edges
        .iter()
        .filter(|&&(u, v, _)| u < n && v < n && u != v)
        .map(|&(u, v, w)| {
            let (lo, hi) = if u <= v { (u, v) } else { (v, u) };
            (lo, hi, w)
        })
        .collect();
    es.sort_by(|a, b| weight_cmp(a.2, b.2).then(a.0.cmp(&b.0)).then(a.1.cmp(&b.1)));
    let mut dsu = Dsu::new(n);
    let mut chosen = Vec::new();
    let mut total = 0.0;
    for (u, v, w) in es {
        if dsu.union(u, v) {
            chosen.push((u, v, w));
            total += w;
        }
    }
    (chosen, total)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── DSU basics ──────────────────────────────────────────────────────
    #[test]
    fn union_find_basic() {
        let mut d = Dsu::new(5);
        assert!(d.union(0, 1));
        assert!(d.union(1, 2));
        assert!(!d.union(0, 2), "already joined ⇒ false");
        assert!(d.connected(0, 2));
        assert!(!d.connected(0, 3));
        assert_eq!(d.len(), 5);
    }

    // ── components() canonical ordering (byte-parity contract) ───────────
    #[test]
    fn components_canonical_ordering() {
        // sets {0,3,4} and {1,2}; node 5 absent.
        let mut d = Dsu::new(6);
        d.union(0, 3);
        d.union(3, 4);
        d.union(1, 2);
        let present = [true, true, true, true, true, false];
        let comps = d.components(&present);
        // members ascending within a set; sets by ascending min member.
        assert_eq!(comps, vec![vec![0, 3, 4], vec![1, 2]]);
    }

    #[test]
    fn components_respects_absent() {
        let mut d = Dsu::new(4);
        d.union(0, 1);
        // node 1 absent ⇒ only 0 shows for its set.
        let present = [true, false, true, true];
        let comps = d.components(&present);
        assert_eq!(comps, vec![vec![0], vec![2], vec![3]]);
    }

    // ── MST hand oracle (§6 acceptance #4) ──────────────────────────────
    // Classic 4-node graph:
    //   0-1: 1, 1-2: 2, 0-2: 2, 2-3: 3, 1-3: 5
    // MST picks 0-1(1), 1-2(2) [tie with 0-2 broken by (min,max)], 2-3(3) → 6.
    #[test]
    fn kruskal_known_minimum() {
        let edges = [
            (0usize, 1, 1.0),
            (1, 2, 2.0),
            (0, 2, 2.0),
            (2, 3, 3.0),
            (1, 3, 5.0),
        ];
        let (chosen, total) = kruskal_mst(4, &edges);
        assert_eq!(chosen.len(), 3, "MST of 4 nodes has 3 edges");
        assert!(
            (total - 6.0).abs() < 1e-12,
            "MST weight = 1+2+3 = 6, got {total}"
        );
        // deterministic tie-break: (0,2,2.0) chosen before (1,2,2.0) [min 0<1]
        assert_eq!(chosen, vec![(0, 1, 1.0), (0, 2, 2.0), (2, 3, 3.0)]);
    }

    #[test]
    fn kruskal_forest_on_disconnected() {
        // two disjoint edges + an isolated node ⇒ forest of 2 edges.
        let edges = [(0usize, 1, 1.0), (2, 3, 2.0)];
        let (chosen, total) = kruskal_mst(5, &edges);
        assert_eq!(chosen.len(), 2);
        assert!((total - 3.0).abs() < 1e-12);
    }

    #[test]
    fn kruskal_ignores_self_and_oob() {
        let edges = [(0usize, 0, 9.0), (0, 9, 9.0), (0, 1, 1.0)];
        let (chosen, total) = kruskal_mst(2, &edges);
        assert_eq!(chosen, vec![(0, 1, 1.0)]);
        assert!((total - 1.0).abs() < 1e-12);
    }
}
