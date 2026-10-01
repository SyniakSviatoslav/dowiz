//! The hub as a GRAPH — folded, never stored.
//!
//! WHY A GRAPH. Everything a venue knows is already related to everything else:
//! an order holds dishes, a dish sits in a category and consumes ingredients, a
//! courier carried it, a customer placed it. Those relations exist today only as
//! ids buried inside JSON blobs, so the only way to ask "what does this courier
//! usually deliver" or "which dishes stall when the octopus runs out" is to
//! re-derive them by hand at every call site. A graph makes the relation the
//! thing, which is what makes retrieval, and an agent that uses it, possible.
//!
//! FOLDED, NOT STORED, and that is the load-bearing decision. This crate's rule
//! everywhere else is that a tally kept beside the events is a second number
//! that can disagree with them — the analytics say it, the courier history says
//! it, the stock ledger says it. A stored graph would be exactly that: a second
//! account of what happened, drifting from the log one write at a time, and the
//! one that drifts is the one an agent would read. So the log stays the single
//! authority and the graph is computed from it.
//!
//! WHAT IT COSTS is honest and bounded: a fold over the orders and the
//! catalogue per construction. That is the same walk `orders()` already does,
//! measured at ~20 ms against ~150 ms of storage, and it is why this returns an
//! owned graph the caller can query many times rather than a lazy view that
//! re-walks.
//!
//! NO FLOATS. MANIFESTO C2 keeps the deterministic core free of them so every
//! node replays identically, and a ranking is part of what an agent answers
//! with. Scores are integers in fixed point and ranks are fused with reciprocal
//! rank fusion, which needs no weighting constant anyone would have to tune.

use crate::catalog::Catalog;
use crate::minijson::{int_field, objects_in, str_field};
use crate::Hub;
use std::collections::HashMap;

/// Fixed-point scale for every score here. One part per million: fine enough
/// that ranks never tie by rounding, small enough that an i64 cannot overflow
/// across the whole graph.
pub const SCALE: i64 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Venue,
    Order,
    Dish,
    Category,
    Ingredient,
    Courier,
    Customer,
}

impl Kind {
    pub fn tag(self) -> &'static str {
        match self {
            Kind::Venue => "venue",
            Kind::Order => "order",
            Kind::Dish => "dish",
            Kind::Category => "category",
            Kind::Ingredient => "ingredient",
            Kind::Courier => "courier",
            Kind::Customer => "customer",
        }
    }
}

/// How two nodes are related. Directed, because the direction carries meaning —
/// an order contains a dish, a dish does not contain an order — and traversal
/// walks both ways anyway.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rel {
    Contains,
    InCategory,
    Uses,
    DeliveredBy,
    PlacedBy,
    ServedBy,
}

impl Rel {
    pub fn tag(self) -> &'static str {
        match self {
            Rel::Contains => "contains",
            Rel::InCategory => "in_category",
            Rel::Uses => "uses",
            Rel::DeliveredBy => "delivered_by",
            Rel::PlacedBy => "placed_by",
            Rel::ServedBy => "served_by",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,
    pub kind: Kind,
    /// What a person would call it.
    pub label: String,
    /// What a search reads. NO PII: a customer node carries its key and what it
    /// ordered, never a name, a phone or an address — those live behind the
    /// reveal that writes an audit entry, and a retrieval index is the last
    /// place they should be duplicated into.
    pub text: String,
    /// When this came into being, or 0 for a node that has no moment (a dish, a
    /// category). Lets a ranking prefer what is recent without a second pass.
    pub at_ms: i64,
}

pub struct Graph {
    nodes: Vec<Node>,
    /// `(from, rel, to)` as node indices.
    edges: Vec<(usize, Rel, usize)>,
    /// Adjacency both ways, built once. A graph this size rebuilt per query
    /// would spend more time on bookkeeping than on the walk.
    out: Vec<Vec<usize>>,
    inc: Vec<Vec<usize>>,
}

impl Graph {
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
    pub fn node(&self, i: usize) -> Option<&Node> {
        self.nodes.get(i)
    }
    /// Test-only: production walks from a node it already holds.
    #[cfg(test)]
    fn index_of(&self, id: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.id == id)
    }
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }
    pub fn edges(&self) -> &[(usize, Rel, usize)] {
        &self.edges
    }

    /// Everything one step away, in both directions, with the relation and
    /// which way it points.
    pub fn neighbours(&self, i: usize) -> Vec<(Rel, usize, bool)> {
        let mut out = Vec::new();
        for &e in self.out.get(i).map(|v| v.as_slice()).unwrap_or(&[]) {
            let (_, r, to) = self.edges[e];
            out.push((r, to, true));
        }
        for &e in self.inc.get(i).map(|v| v.as_slice()).unwrap_or(&[]) {
            let (from, r, _) = self.edges[e];
            out.push((r, from, false));
        }
        out
    }

    /// How many steps lead away from this node, both directions, WITHOUT
    /// building the list. `ppr` asks this of every node on every iteration, and
    /// `neighbours` allocates a `Vec` to answer it.
    pub fn degree(&self, i: usize) -> usize {
        self.out.get(i).map_or(0, |v| v.len()) + self.inc.get(i).map_or(0, |v| v.len())
    }

    fn build(nodes: Vec<Node>, edges: Vec<(usize, Rel, usize)>) -> Self {
        let mut out = vec![Vec::new(); nodes.len()];
        let mut inc = vec![Vec::new(); nodes.len()];
        for (e, &(a, _, b)) in edges.iter().enumerate() {
            out[a].push(e);
            inc[b].push(e);
        }
        Graph { nodes, edges, out, inc }
    }
}

mod build;
mod retrieval;
pub use retrieval::terms;

#[cfg(test)]
mod tests;
