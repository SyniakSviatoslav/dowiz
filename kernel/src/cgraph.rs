//! cgraph.rs — semi-Markovian causal **graph** primitives for the ID / IDC
//! identification algorithms (Shpitser & Pearl, 2006/2008; 2012).
//!
//! This module is the *structural* half of the do-calculus identifiability
//! decider. `causal.rs` owns the recursive `id()` / `idc()` procedures; this
//! module owns the graph algebra they recurse over:
//!
//! * a [`CGraph`] carrying both **directed** arcs (`parents[i]` — `i`'s direct
//!   causes) and **bidirected** arcs (`bidirected[i]` — latent confounding /
//!   "spurious correlation" between `i` and the listed nodes);
//! * [`ancestors`] / [`descendants`] closures over directed edges;
//! * [`c_components`] — the maximal confounded-component partition, the spine of
//!   the ID recursion (Def. 9, Shpitser–Pearl 2008);
//! * [`d_separated_bi`] — d-separation **extended to bidirected arcs** (Def. 4 in
//!   the JMLR 2008 paper): a path is blocked by `z` if it contains a chain
//!   `i→m→j`, a fork `i←m→j`, or a **bidirected** `i↔m→j` / `i←m↔j` with `m∈z`,
//!   or a collider `i→m←j` whose descendants avoid `z`. The ordinary
//!   [`causal::d_separated`] is the bidirected-free special case.
//! * subgraph constructors `g_x_removed` (`G\X`), `subgraph_on` (`G[V]`), and a
//!   `[nodes]` listing.
//!
//! Trust boundary: every public constructor validates node indices and rejects
//! self-loops / reflexive bidirected arcs (those would be degenerate or
//! malformed); invalid input yields `Err`, never a panic.

use std::collections::{BTreeSet, HashSet, VecDeque};

/// A semi-Markovian causal diagram: observable nodes `0..n`, directed parent
/// lists, and a bidirected-adjacency list (symmetric — `i↔j` appears in both
/// `bidirected[i]` and `bidirected[j]`).
///
/// `bidirected` models *latent* (unobserved) confounding: `i↔j` means some
/// hidden common cause drives both. It is exactly the structure no prior
/// function in this crate could even *detect* as non-identifiable (the bow-arc
/// case `X→Z←Y` with `Z` unobserved ⇒ `X↔Y` ⇔ a hedge).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CGraph {
    pub n: usize,
    /// `parents[i]` = direct causes of node `i` (directed edges `p → i`).
    pub parents: Vec<Vec<usize>>,
    /// `bidirected[i]` = nodes `j` with a latent-confounding arc `i↔j`.
    pub bidirected: Vec<Vec<usize>>,
    /// `present[i]` = whether node `i` is part of the *current* (sub)graph.
    /// Subgraph constructors (`subgraph_on`, `g_x_removed`) drop nodes by
    /// clearing this flag rather than reindexing, so node indices stay stable
    /// across the ID recursion (which threads the vertex set `v` explicitly,
    /// mirroring `igraph::induced.subgraph` in the reference `causaleffect`
    /// implementation). `ancestors`, `c_components`, and `nodes` all ignore
    /// absent nodes.
    present: Vec<bool>,
}

impl CGraph {
    /// Build a `CGraph`, validating indices and bidirected symmetry.
    ///
    /// `parents[i]` lists `i`'s direct causes; `bidirected[i]` lists `i`'s latent
    /// confounders (the pair `i↔j` must be listed in *both*). We accept it either
    /// way and *enforce* symmetry internally so callers cannot create an
    /// asymmetric (malformed) graph by accident.
    pub fn new(parents: Vec<Vec<usize>>, bidirected: Vec<Vec<usize>>) -> Result<Self, String> {
        let n = parents.len();
        if bidirected.len() != n {
            return Err(format!(
                "parents.len()={n} != bidirected.len()={}",
                bidirected.len()
            ));
        }
        for i in 0..n {
            for &p in &parents[i] {
                if p >= n {
                    return Err(format!("parent {p} of node {i} out of range (n={n})"));
                }
                if p == i {
                    return Err(format!("self-loop parent on node {i} (degenerate DAG)"));
                }
            }
            for &b in &bidirected[i] {
                if b >= n {
                    return Err(format!(
                        "bidirected neighbour {b} of node {i} out of range (n={n})"
                    ));
                }
                if b == i {
                    return Err(format!("reflexive bidirected arc on node {i} (malformed)"));
                }
            }
        }
        // Enforce bidirected symmetry.
        let mut bi = bidirected;
        for i in 0..n {
            let neighbors: Vec<usize> = bi[i].clone();
            for j in neighbors {
                if !bi[j].contains(&i) {
                    bi[j].push(i);
                }
            }
        }
        // Sort + dedup for deterministic c-component / d-sep behaviour.
        for v in bi.iter_mut() {
            v.sort_unstable();
            v.dedup();
        }
        Ok(CGraph {
            n,
            parents,
            bidirected: bi,
            present: vec![true; n],
        })
    }

    /// Nodes currently present in this (sub)graph, in ascending index order.
    /// This is the vertex set `v` the ID recursion threads through every call.
    pub fn nodes(&self) -> Vec<usize> {
        (0..self.n).filter(|&i| self.present[i]).collect()
    }
    pub fn ancestors(&self, set: &[usize]) -> Vec<bool> {
        let mut anc = vec![false; self.n];
        let mut stack: Vec<usize> = set.to_vec();
        for &s in set {
            anc[s] = true;
        }
        while let Some(v) = stack.pop() {
            if !self.present[v] {
                continue;
            }
            for &p in &self.parents[v] {
                if !anc[p] && self.present[p] {
                    anc[p] = true;
                    stack.push(p);
                }
            }
        }
        anc
    }

    /// Directed descendants of `set` (inclusive), BFS over child edges.
    pub fn descendants(&self, set: &[usize]) -> Vec<bool> {
        let mut children: Vec<Vec<usize>> = vec![Vec::new(); self.n];
        for (i, ps) in self.parents.iter().enumerate() {
            for &p in ps {
                children[p].push(i);
            }
        }
        let mut desc = vec![false; self.n];
        let mut stack: Vec<usize> = set.to_vec();
        for &s in set {
            desc[s] = true;
        }
        while let Some(v) = stack.pop() {
            for &c in &children[v] {
                if !desc[c] {
                    desc[c] = true;
                    stack.push(c);
                }
            }
        }
        desc
    }

    /// Maximal **c-components** (confounded components, Def. 9, Shpitser–Pearl
    /// 2008). Two observable nodes belong to the same c-component iff they are
    /// connected by a path of **bidirected** arcs (latent confounding). This is
    /// the connected-components of the *bidirected subgraph only* — directed
    /// edges are deliberately **NOT** part of the c-component relation. (A common
    /// error is to moralize the directed edges too; that collapses every DAG into
    /// a single c-component and makes the ID recursion hedge every identifiable
    /// chain. The ID algorithm's correctness relies on c-components being
    /// bidirected-only.)
    ///
    /// Returns the partition as a list of node-sets (each sorted, ascending).
    pub fn c_components(&self) -> Vec<Vec<usize>> {
        // bidirected-connected-components adjacency (no directed edges)
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); self.n];
        for i in 0..self.n {
            for &j in &self.bidirected[i] {
                if j > i {
                    adj[i].push(j);
                    adj[j].push(i);
                }
            }
        }
        // connected components over `adj`, restricted to present nodes
        let mut seen = vec![false; self.n];
        let mut comps = Vec::new();
        for start in 0..self.n {
            if seen[start] || !self.present[start] {
                continue;
            }
            let mut comp = Vec::new();
            let mut stack = vec![start];
            seen[start] = true;
            while let Some(v) = stack.pop() {
                comp.push(v);
                for &w in &adj[v] {
                    if !seen[w] && self.present[w] {
                        seen[w] = true;
                        stack.push(w);
                    }
                }
            }
            comp.sort_unstable();
            comps.push(comp);
        }
        comps
    }

    /// `G_{\overline{X}}` — keep every node but **delete only the incoming edges**
    /// of `X` (intervened nodes stay in the graph, their causes are severed).
    /// This is the exact graph `An(Y)_{G_{\overline{X}}}` is computed on (ID line 2).
    /// Distinct from [`g_x_removed`] (`G_{\underline{X}}`), which deletes `X` and
    /// all incident edges — that one is line 1's marginalization, NOT line 2.
    pub fn g_x_incoming_removed(&self, x: &[usize]) -> CGraph {
        let mut removed = vec![false; self.n];
        for &v in x {
            removed[v] = true;
        }
        let mut parents = vec![Vec::new(); self.n];
        let bidirected = self.bidirected.clone();
        for i in 0..self.n {
            if removed[i] {
                // delete incoming edges of i (i is intervened); keep the node
                parents[i] = Vec::new();
            } else {
                parents[i] = self.parents[i].clone();
            }
        }
        CGraph { n: self.n, parents, bidirected, present: self.present.clone() }
    }

    /// `G\\\\X` — delete `X` and all edges (directed or bidirected) incident to any
    /// node in `X`. Used by the ID recursion to express `P_x(·)` over the
    /// post-intervention graph.
    pub fn g_x_removed(&self, x: &[usize]) -> CGraph {
        let mut removed = vec![false; self.n];
        for &v in x {
            removed[v] = true;
        }
        let mut parents = vec![Vec::new(); self.n];
        let mut bidirected = vec![Vec::new(); self.n];
        let mut present = vec![false; self.n];
        for i in 0..self.n {
            if removed[i] {
                continue;
            }
            present[i] = true;
            parents[i] = self.parents[i].iter().copied().filter(|&p| !removed[p]).collect();
            bidirected[i] = self.bidirected[i]
                .iter()
                .copied()
                .filter(|&b| !removed[b])
                .collect();
        }
        CGraph { n: self.n, parents, bidirected, present }
    }

    /// `G[V]` — restrict to node set `V` (node indices preserved). All edges
    /// (directed + bidirected) between surviving nodes are kept. Used by the
    /// `An(Y)G` restriction in `id`.
    pub fn subgraph_on(&self, v: &[usize]) -> CGraph {
        let mut keep = vec![false; self.n];
        for &node in v {
            keep[node] = true;
        }
        let mut parents = vec![Vec::new(); self.n];
        let mut bidirected = vec![Vec::new(); self.n];
        let mut present = vec![false; self.n];
        for &i in v {
            present[i] = true;
            parents[i] = self.parents[i].iter().copied().filter(|&p| keep[p]).collect();
            bidirected[i] = self.bidirected[i].iter().copied().filter(|&b| keep[b]).collect();
        }
        CGraph { n: self.n, parents, bidirected, present }
    }

    /// In `G\X`, is `a` d-separated from `b` given `given`? Bidirected-aware
    /// (Def. 4, JMLR 2008). This is the single structural primitive the ID
    /// recursion needs to recognise colliders and confounded forks.
    pub fn d_separated_bi(
        &self,
        x: &[usize],
        a: usize,
        b: usize,
        given: &[usize],
    ) -> Result<bool, String> {
        let g = self.g_x_removed(x);
        g.d_separated_raw(a, b, given)
    }

    /// Raw d-separation over this graph (already in `G\X` form). Walks active
    /// trails treating bidirected arcs as the *open* (confounded) third case.
    fn d_separated_raw(&self, a: usize, b: usize, given: &[usize]) -> Result<bool, String> {
        let n = self.n;
        if a >= n || b >= n {
            return Err(format!("d-sep node out of range: a={a}, b={b}, n={n}"));
        }
        let mut given_set = vec![false; n];
        for &z in given {
            if z >= n {
                return Err(format!("conditioning node out of range: z={z}, n={n}"));
            }
            given_set[z] = true;
        }
        // children[i] = nodes that have i as a parent
        let mut children: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (i, ps) in self.parents.iter().enumerate() {
            for &p in ps {
                children[p].push(i);
            }
        }
        // descendant closure of `given` opens colliders
        let mut desc = vec![false; n];
        let mut stack: Vec<usize> = given.to_vec();
        while let Some(v) = stack.pop() {
            if desc[v] {
                continue;
            }
            desc[v] = true;
            for &c in &children[v] {
                if !desc[c] {
                    stack.push(c);
                }
            }
        }

        // BFS over active trails. State = (node, dir, via):
        //   dir = 0 start | 1 arrived downstream (from a parent) | 2 from a child
        //   via = node we came from (n = sentinel at start)
        // A trail is open iff every triple along it is open.
        let mut visited: HashSet<(usize, u8, usize)> = HashSet::new();
        let mut queue: VecDeque<(usize, u8, usize)> = VecDeque::new();
        for &c in &children[a] {
            queue.push_back((c, 1, a));
        }
        for &p in &self.parents[a] {
            queue.push_back((p, 2, a));
        }
        // Bidirected arcs off the start node are active (latent common cause):
        // open iff `a` is NOT in the conditioning set (Def. 4 case (c)).
        if !given_set[a] {
            for &w in &self.bidirected[a] {
                queue.push_back((w, 0, a));
            }
        }
        // start node is "traversed" — mark to avoid re-expansion
        visited.insert((a, 0, n));
        while let Some((u, dir, via)) = queue.pop_back() {
            if u == b {
                return Ok(false); // active trail reached target ⇒ d-CONNECTED
            }
            if !visited.insert((u, dir, via)) {
                continue;
            }
            // Walk to children u → c: chain/fork/bidirected-child at u.
            if !given_set[u] {
                for &c in &children[u] {
                    if c != via {
                        queue.push_back((c, 1, u));
                    }
                }
            }
            // Walk to parents p → u.
            for &p in &self.parents[u] {
                if p == via {
                    continue;
                }
                if dir == 1 {
                    // arrived downstream (parent→u): prev→u←p is a COLLIDER —
                    // open iff u's descendant is in `given`.
                    if desc[u] {
                        queue.push_back((p, 2, u));
                    }
                } else {
                    // arrived upstream (child→u): p→u→prev is a CHAIN —
                    // open iff u ∉ given.
                    if !given_set[u] {
                        queue.push_back((p, 2, u));
                    }
                }
            }
            // Walk bidirected u ↔ w: open iff u (hence the confounded pair) is in
            // `given`. The latent common cause is "conditioned" precisely when u
            // is in the conditioning set — exactly Def. 4 case (c).
            if !given_set[u] {
                for &w in &self.bidirected[u] {
                    if w != via {
                        queue.push_back((w, 0, u));
                    }
                }
            }
        }
        Ok(true) // no active trail ⇒ d-SEPARATED
    }

    /// Kahn topological order of the **directed** edges (bidirected arcs are
    /// ignored — they never constrain a topological order). Returns `None` if a
    /// directed cycle exists (an invariant a valid semi-Markovian diagram must
    /// satisfy). Used to linearise the factorization `P(v) = ∏ P(v_i | pa(v_i))`.
    pub fn topological_order(&self) -> Option<Vec<usize>> {
        let mut indeg = vec![0usize; self.n];
        let mut children: Vec<Vec<usize>> = vec![Vec::new(); self.n];
        for (i, ps) in self.parents.iter().enumerate() {
            indeg[i] = ps.len();
            for &p in ps {
                children[p].push(i);
            }
        }
        let mut queue: Vec<usize> = (0..self.n).filter(|&i| indeg[i] == 0).collect();
        let mut order = Vec::with_capacity(self.n);
        while let Some(u) = queue.pop() {
            order.push(u);
            for &c in &children[u] {
                indeg[c] -= 1;
                if indeg[c] == 0 {
                    queue.push(c);
                }
            }
        }
        if order.len() == self.n {
            Some(order)
        } else {
            None
        }
    }

    /// d-separation in `G_{X, Z̲}` — the *underlined* do-calculus graph for
    /// rule 2 / rule 3: nodes in `removed` are deleted entirely (as in `G\\X`),
    /// while nodes in `remove_incoming` keep their vertex but lose their
    /// **incoming directed edges** (they are intervened, so their causes are
    /// severed but they remain queryable). Bidirected arcs incident to a removed
    /// node are dropped; all others survive. Used by `idc` to test the swap
    /// `(Y ⊥ Z' | X, Z\\Z')_{G_{X, Z̲}}`.
    pub fn d_separated_underlined(
        &self,
        removed: &[usize],
        remove_incoming: &[usize],
        a: usize,
        b: usize,
        given: &[usize],
    ) -> Result<bool, String> {
        let mut rem = vec![false; self.n];
        for &v in removed {
            if v >= self.n {
                return Err(format!("underlined removed node {v} out of range"));
            }
            rem[v] = true;
        }
        let mut inc = vec![false; self.n];
        for &v in remove_incoming {
            if v >= self.n {
                return Err(format!("underlined remove_incoming node {v} out of range"));
            }
            inc[v] = true;
        }
        let mut parents = vec![Vec::new(); self.n];
        let mut bidirected = vec![Vec::new(); self.n];
        let mut present = vec![false; self.n];
        for i in 0..self.n {
            if rem[i] {
                continue;
            }
            present[i] = true;
            for &p in &self.parents[i] {
                if rem[p] {
                    continue;
                }
                if inc[i] {
                    continue; // drop incoming edges of i (i is intervened)
                }
                parents[i].push(p);
            }
            for &bd in &self.bidirected[i] {
                if rem[bd] {
                    continue;
                }
                bidirected[i].push(bd);
            }
        }
        let g = CGraph { n: self.n, parents, bidirected, present };
        g.d_separated_raw(a, b, given)
    }

    /// Root set of a subgraph: nodes with **no directed parent** within it. A
    /// maximal c-component rooted at these is an R-rooted C-forest (the `R` in
    /// the hedge definition). Used to populate [`HedgeWitness::root`].
    pub fn roots(&self) -> Vec<usize> {
        let mut r = Vec::new();
        for i in 0..self.n {
            if self.present[i] && self.parents[i].is_empty() {
                r.push(i);
            }
        }
        r
    }

    /// Does `vec` (as a set) contain every element of `subset`?
    pub fn contains_all(vec: &[usize], subset: &[usize]) -> bool {
        let set: BTreeSet<usize> = vec.iter().copied().collect();
        subset.iter().all(|&s| set.contains(&s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Simple DAG: chain 0→1→2 (no bidirected). c-components = singletons.
    fn chain_graph() -> CGraph {
        CGraph::new(
            vec![vec![], vec![0], vec![1]], // 0 root, 1 pa 0, 2 pa 1
            vec![vec![], vec![], vec![]],
        )
        .unwrap()
    }

    // Bow-arc / M-graph: 0→2←1  with 2 unobserved ⇒ bidirected 0↔1.
    // c-components: {0,1} (confounded) and {2}.
    fn m_graph() -> CGraph {
        CGraph::new(
            vec![vec![], vec![], vec![0, 1]], // 2 has parents 0 and 1
            vec![vec![1], vec![0], vec![]],   // 0↔1 latent confound
        )
        .unwrap()
    }

    // ── GREEN: c-components of a pure DAG are singletons ──
    #[test]
    fn green_dag_c_components_are_singletons() {
        let g = chain_graph();
        let comps = g.c_components();
        assert_eq!(comps.len(), 3, "three singleton c-components");
        for c in &comps {
            assert_eq!(c.len(), 1, "each c-component is a single node");
        }
    }

    // ── GREEN: bow-arc (M-graph) yields a confounded {0,1} c-component ──
    #[test]
    fn green_bow_arc_groups_confounded_pair() {
        let g = m_graph();
        let comps = g.c_components();
        // Exactly two c-components: {0,1} and {2}.
        assert_eq!(comps.len(), 2, "two c-components");
        let pair = comps.iter().find(|c| c.len() == 2).expect("a 2-node c-comp");
        assert_eq!(pair, &vec![0, 1], "the confounded pair {{0,1}}");
    }

    // ── GREEN: d-separation is preserved for plain chains (no bidirected) ──
    #[test]
    fn green_chain_dsep_matches_plain_oracle() {
        let g = chain_graph();
        // 0 and 2 d-connected via 0→1→2; blocked by conditioning on 1.
        assert!(!g.d_separated_raw(0, 2, &[]).unwrap(), "chain open");
        assert!(g.d_separated_raw(0, 2, &[1]).unwrap(), "chain blocked by 1");
    }

    // ── GREEN: bidirected confounded pair is fundamentally non-separable ──
    #[test]
    fn green_bidirected_fork_opens_then_closes() {
        let g = m_graph();
        // 0↔1 (latent confound) plus 0→2←1 (collider). The bidirected arc makes
        // 0,1 d-connected under empty given (Def. 4 case (c)). Conditioning on
        // the shared child 2 OPENS the collider, so they stay connected. And
        // conditioning on 0 closes the bidirected arc but, because 2 is a
        // descendant of 0, opens the collider 0→2←1 again — still connected.
        // Symmetrically for conditioning on 1. The pair is therefore d-connected
        // under EVERY non-trivial conditioning set — this non-separability is
        // precisely why {0,1} is ONE c-component.
        assert!(!g.d_separated_raw(0, 1, &[]).unwrap(), "bidirected 0↔1 d-connected (empty)");
        assert!(!g.d_separated_raw(0, 1, &[2]).unwrap(), "conditioning on collider child keeps 0,1 connected");
        assert!(!g.d_separated_raw(0, 1, &[0]).unwrap(), "conditioning on 0 opens collider via descendant 2 — still connected");
        assert!(!g.d_separated_raw(0, 1, &[1]).unwrap(), "conditioning on 1 opens collider via descendant 2 — still connected");
    }

    // ── RED (trust boundary): malformed graph rejected, never panic ──
    #[test]
    fn red_malformed_graph_rejected() {
        assert!(CGraph::new(
            vec![vec![], vec![9], vec![]], // parent 9 out of range
            vec![vec![], vec![], vec![]],
        )
        .is_err());
        assert!(CGraph::new(
            vec![vec![], vec![], vec![]],
            vec![vec![1], vec![], vec![]], // asymmetric bidirected (0↔1, 1 missing 0)
        )
        .is_ok()); // symmetry is enforced, not required at input
        // reflexive bidirected rejected
        assert!(CGraph::new(
            vec![vec![], vec![], vec![]],
            vec![vec![0], vec![], vec![]],
        )
        .is_err());
    }
}
