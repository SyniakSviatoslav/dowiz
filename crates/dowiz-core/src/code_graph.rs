//! code_graph.rs — native graphify port: a queryable code knowledge graph.
//!
//! Reverse-engineered from `Graphify-Labs/graphify` (knowledge-graph-over-
//! codebase, "query instead of grep"). Stores code entities (nodes) and typed
//! relationships (edges), and answers the navigation queries the living-memory
//! graph needs at every level: neighbours, shortest path, radius subgraph,
//! PageRank importance (via [`crate::csr`]), and deterministic label-propagation
//! communities. no_std (alloc only).

use alloc::string::String;
use alloc::vec::Vec;

/// What kind of code entity a node is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Function,
    Struct,
    Enum,
    Module,
    Trait,
    Concept,
    File,
    Other,
}

/// What kind of relationship an edge is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeKind {
    Calls,
    References,
    Contains,
    Imports,
    Uses,
}

/// A code entity (or extracted concept) in the graph.
#[derive(Debug, Clone)]
pub struct CodeNode {
    pub name: String,
    pub kind: NodeKind,
    /// Community id (assigned by [`CodeGraph::label_propagation`]).
    pub community: usize,
}

/// A typed directed relationship between two nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeEdge {
    pub from: usize,
    pub to: usize,
    pub kind: EdgeKind,
}

/// A queryable directed knowledge graph over code entities.
#[derive(Debug, Clone, Default)]
pub struct CodeGraph {
    nodes: Vec<CodeNode>,
    edges: Vec<CodeEdge>,
    /// node id → indices into `edges` (out-edges).
    adj: Vec<Vec<usize>>,
}

impl CodeGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a node; returns its id.
    pub fn add_node(&mut self, name: &str, kind: NodeKind) -> usize {
        let id = self.nodes.len();
        self.nodes.push(CodeNode {
            name: String::from(name),
            kind,
            community: id,
        });
        self.adj.push(Vec::new());
        id
    }

    /// Add a directed edge; returns its index. Node ids must already exist.
    pub fn add_edge(&mut self, from: usize, to: usize, kind: EdgeKind) -> usize {
        let e = self.edges.len();
        self.edges.push(CodeEdge { from, to, kind });
        self.adj[from].push(e);
        e
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
    pub fn node(&self, id: usize) -> Option<&CodeNode> {
        self.nodes.get(id)
    }
    pub fn nodes(&self) -> &[CodeNode] {
        &self.nodes
    }
    pub fn edges(&self) -> &[CodeEdge] {
        &self.edges
    }

    /// Out-neighbour node ids (deduplicated, in edge order).
    pub fn neighbors(&self, id: usize) -> Vec<usize> {
        let mut out = Vec::new();
        for &e in &self.adj[id] {
            let to = self.edges[e].to;
            if !out.contains(&to) {
                out.push(to);
            }
        }
        out
    }

    /// Breadth-first shortest path from `from` to `to` (node ids inclusive).
    pub fn shortest_path(&self, from: usize, to: usize) -> Option<Vec<usize>> {
        let n = self.nodes.len();
        let mut prev: Vec<Option<usize>> = vec![None; n];
        let mut visited = vec![false; n];
        let mut queue = alloc::collections::VecDeque::new();
        visited[from] = true;
        queue.push_back(from);
        while let Some(u) = queue.pop_front() {
            if u == to {
                break;
            }
            for v in self.neighbors(u) {
                if !visited[v] {
                    visited[v] = true;
                    prev[v] = Some(u);
                    queue.push_back(v);
                }
            }
        }
        if !visited[to] {
            return None;
        }
        let mut path = Vec::new();
        let mut cur = to;
        while cur != from {
            path.push(cur);
            cur = prev[cur]?;
        }
        path.push(from);
        path.reverse();
        Some(path)
    }

    /// All nodes within `radius` hops of `id` (including `id`), sorted by id.
    pub fn subgraph_around(&self, id: usize, radius: usize) -> Vec<usize> {
        let n = self.nodes.len();
        let mut dist = vec![usize::MAX; n];
        let mut queue = alloc::collections::VecDeque::new();
        dist[id] = 0;
        queue.push_back(id);
        while let Some(u) = queue.pop_front() {
            if dist[u] >= radius {
                continue;
            }
            for v in self.neighbors(u) {
                if dist[v] == usize::MAX {
                    dist[v] = dist[u] + 1;
                    queue.push_back(v);
                }
            }
        }
        let mut out: Vec<usize> = (0..n).filter(|&i| dist[i] != usize::MAX).collect();
        out.sort_unstable();
        out
    }

    /// PageRank importance per node (reuses [`crate::csr::Csr::personalized_pagerank`]).
    pub fn pagerank(&self, alpha: f64, iters: usize) -> Vec<f64> {
        let n = self.nodes.len();
        if n == 0 {
            return Vec::new();
        }
        let edges: Vec<(usize, usize, f64)> =
            self.edges.iter().map(|e| (e.from, e.to, 1.0)).collect();
        let csr = crate::csr::Csr::from_edges(n, &edges);
        let seed = vec![1.0 / n as f64; n];
        csr.personalized_pagerank(&seed, alpha, iters)
    }

    /// Deterministic label propagation: assign each node to its dominant
    /// neighbour community (ties resolve to the smallest id). Updates
    /// `CodeNode::community` and returns the per-node community ids.
    pub fn label_propagation(&mut self, iters: usize) -> Vec<usize> {
        let n = self.nodes.len();
        let mut labels: Vec<usize> = (0..n).collect();
        let mut counts: Vec<usize> = vec![0; n];
        for _ in 0..iters {
            let mut changed = false;
            for u in 0..n {
                // Count neighbour labels.
                for &e in &self.adj[u] {
                    let v = self.edges[e].to;
                    counts[labels[v]] += 1;
                }
                // Pick the most common (ties → smallest label).
                let mut best = labels[u];
                let mut best_c = 0usize;
                for l in 0..n {
                    if counts[l] > best_c {
                        best = l;
                        best_c = counts[l];
                    }
                }
                // Reset counts for the next node.
                for &e in &self.adj[u] {
                    let v = self.edges[e].to;
                    counts[labels[v]] = 0;
                }
                if best != labels[u] {
                    labels[u] = best;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        for (i, node) in self.nodes.iter_mut().enumerate() {
            node.community = labels[i];
        }
        labels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    fn build_graph() -> CodeGraph {
        let mut g = CodeGraph::new();
        // 0 -> 1 -> 2, and 0 -> 3 (a branch)
        let a = g.add_node("main", NodeKind::Function);
        let b = g.add_node("parse", NodeKind::Function);
        let c = g.add_node("tokenize", NodeKind::Function);
        let d = g.add_node("log", NodeKind::Function);
        g.add_edge(a, b, EdgeKind::Calls);
        g.add_edge(b, c, EdgeKind::Calls);
        g.add_edge(a, d, EdgeKind::Calls);
        g
    }

    #[test]
    fn shortest_path_follows_edges() {
        let g = build_graph();
        let p = g.shortest_path(0, 2).unwrap();
        assert_eq!(p, vec![0, 1, 2]);
        // 0 -> 3 is direct.
        let p2 = g.shortest_path(0, 3).unwrap();
        assert_eq!(p2, vec![0, 3]);
    }

    #[test]
    fn no_path_returns_none() {
        let g = build_graph();
        assert!(g.shortest_path(3, 1).is_none());
    }

    #[test]
    fn subgraph_around_radius() {
        let g = build_graph();
        // radius 0 = just node 0; radius 1 = 0,1,3; radius 2 = all.
        assert_eq!(g.subgraph_around(0, 0), vec![0]);
        assert_eq!(g.subgraph_around(0, 1), vec![0, 1, 3]);
        assert_eq!(g.subgraph_around(0, 2), vec![0, 1, 2, 3]);
    }

    #[test]
    fn pagerank_scores_connected_nodes() {
        let g = build_graph();
        let pr = g.pagerank(0.85, 20);
        assert_eq!(pr.len(), 4);
        let sum: f64 = pr.iter().sum();
        assert!(close(sum, 1.0), "sum={sum}");
        for p in &pr {
            assert!(*p > 0.0);
        }
    }

    #[test]
    fn label_propagation_groups_connected_nodes() {
        let mut g = CodeGraph::new();
        // Component A: 0 <-> 1 <-> 2 (mutual edges).
        let a = g.add_node("a", NodeKind::Function);
        let b = g.add_node("b", NodeKind::Function);
        let c = g.add_node("c", NodeKind::Function);
        // Component B: 3 <-> 4 (disjoint from A).
        let d = g.add_node("d", NodeKind::Function);
        let e = g.add_node("e", NodeKind::Function);
        g.add_edge(a, b, EdgeKind::Calls);
        g.add_edge(b, a, EdgeKind::Calls);
        g.add_edge(b, c, EdgeKind::Calls);
        g.add_edge(c, b, EdgeKind::Calls);
        g.add_edge(d, e, EdgeKind::Calls);
        g.add_edge(e, d, EdgeKind::Calls);
        let labels = g.label_propagation(10);
        // A and B are disjoint components → same label within, different across.
        assert!(labels[a] == labels[b] && labels[b] == labels[c]);
        assert!(labels[d] == labels[e]);
        assert!(labels[a] != labels[d]);
    }

    #[test]
    fn neighbors_dedupes_parallel_edges() {
        let mut g = CodeGraph::new();
        let a = g.add_node("a", NodeKind::Function);
        let b = g.add_node("b", NodeKind::Function);
        g.add_edge(a, b, EdgeKind::Calls);
        g.add_edge(a, b, EdgeKind::References);
        assert_eq!(g.neighbors(a), vec![b]);
    }
}
