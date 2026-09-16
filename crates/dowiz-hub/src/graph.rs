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
    by_id: HashMap<String, usize>,
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
    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.by_id.get(id).copied()
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
        let by_id = nodes.iter().enumerate().map(|(i, n)| (n.id.clone(), i)).collect();
        let mut out = vec![Vec::new(); nodes.len()];
        let mut inc = vec![Vec::new(); nodes.len()];
        for (e, &(a, _, b)) in edges.iter().enumerate() {
            out[a].push(e);
            inc[b].push(e);
        }
        Graph { nodes, edges, by_id, out, inc }
    }

    /// THE FOLD. Reads the order log and the catalogue and relates them.
    ///
    /// Orders come from `hub.orders()`, which is already the newest state of
    /// each — so a dish that was removed from an order before it was cooked is
    /// not in the graph, because it is not in the order.
    pub fn of(hub: &Hub, catalog: &Catalog) -> Self {
        Self::of_with(hub, catalog, &HashMap::new())
    }

    /// The same fold, with LABELS THIS CRATE CANNOT KNOW.
    ///
    /// A courier's name is not in the hub. On the Worker the roster lives in a
    /// table this crate has never heard of, so a courier node could only be
    /// labelled with its own id — and a search for "Eni" found nothing, which
    /// is exactly the failure worth fixing: the graph knew the courier existed
    /// and could not say who they were.
    ///
    /// PASSED IN RATHER THAN FETCHED, because this crate has two dependencies
    /// and no network. The caller knows where its people live; this knows how
    /// they relate. Keys are node ids — `courier:<id>`, `ingredient:<id>` — so
    /// the same seam labels a shelf with its level or a customer with nothing,
    /// which is what a customer must stay labelled with.
    pub fn of_with(
        hub: &Hub,
        catalog: &Catalog,
        labels: &HashMap<String, String>,
    ) -> Self {
        let mut nodes: Vec<Node> = Vec::new();
        let mut edges: Vec<(usize, Rel, usize)> = Vec::new();
        let mut idx: HashMap<String, usize> = HashMap::new();

        let mut ensure = |nodes: &mut Vec<Node>,
                          idx: &mut HashMap<String, usize>,
                          id: String,
                          kind: Kind,
                          label: String,
                          text: String,
                          at_ms: i64| -> usize {
            if let Some(&i) = idx.get(&id) {
                // A node met twice keeps the richer description: an order names
                // a dish by id only, the catalogue names it properly, and which
                // arrives first is an accident of the fold order.
                if nodes[i].label.is_empty() && !label.is_empty() {
                    nodes[i].label = label;
                }
                if nodes[i].text.len() < text.len() {
                    nodes[i].text = text;
                }
                return i;
            }
            let i = nodes.len();
            idx.insert(id.clone(), i);
            nodes.push(Node { id, kind, label, text, at_ms });
            i
        };

        // ── the venue ──
        let venue_i = if let Some(loc) = catalog.location() {
            let name = str_field(&loc, "name").unwrap_or_default();
            let addr = str_field(&loc, "address").unwrap_or_default();
            let id = str_field(&loc, "id").unwrap_or_else(|| "venue".to_string());
            Some(ensure(
                &mut nodes,
                &mut idx,
                format!("venue:{id}"),
                Kind::Venue,
                name.clone(),
                format!("{name} {addr}"),
                0,
            ))
        } else {
            None
        };

        // ── dishes and their categories ──
        for (pid, pjson) in catalog.products() {
            let name = str_field(&pjson, "name").unwrap_or_default();
            let desc = str_field(&pjson, "description").unwrap_or_default();
            let price = int_field(&pjson, "price").unwrap_or(0);
            let dish = ensure(
                &mut nodes,
                &mut idx,
                format!("dish:{pid}"),
                Kind::Dish,
                name.clone(),
                format!("{name} {desc}"),
                0,
            );
            if let Some(v) = venue_i {
                edges.push((dish, Rel::ServedBy, v));
            }
            if let Some(cat) = str_field(&pjson, "categoryId").or_else(|| str_field(&pjson, "category")) {
                if !cat.is_empty() {
                    let c = ensure(
                        &mut nodes,
                        &mut idx,
                        format!("category:{cat}"),
                        Kind::Category,
                        cat.clone(),
                        cat.clone(),
                        0,
                    );
                    edges.push((dish, Rel::InCategory, c));
                }
            }
            // An ingredient the dish consumes, where the recipe says so. Both
            // spellings are accepted because the importer and the stock pane
            // disagree, and a graph that silently drops one would report a
            // shelf as unused.
            for item in objects_in(&pjson, "recipe")
                .into_iter()
                .chain(objects_in(&pjson, "ingredients"))
            {
                let Some(sid) = str_field(&item, "supplyId").or_else(|| str_field(&item, "id")) else {
                    continue;
                };
                if sid.is_empty() {
                    continue;
                }
                let s = ensure(
                    &mut nodes,
                    &mut idx,
                    format!("ingredient:{sid}"),
                    Kind::Ingredient,
                    sid.clone(),
                    sid.clone(),
                    0,
                );
                edges.push((dish, Rel::Uses, s));
            }
            let _ = price;
        }

        // ── ingredients the shelf knows about, dish or no dish ──
        for (sid, sjson) in catalog.supplies() {
            let name = str_field(&sjson, "name").unwrap_or_else(|| sid.clone());
            let unit = str_field(&sjson, "unit").unwrap_or_default();
            ensure(
                &mut nodes,
                &mut idx,
                format!("ingredient:{sid}"),
                Kind::Ingredient,
                name.clone(),
                format!("{name} {unit}"),
                0,
            );
        }

        // ── orders, and everyone they touch ──
        for ev in hub.orders() {
            let j = &ev.order_json;
            let status = str_field(j, "status").unwrap_or_default();
            let total = int_field(j, "total").unwrap_or(0);
            let at = int_field(j, "created_at_ms").unwrap_or(ev.seq as i64);
            let order = ensure(
                &mut nodes,
                &mut idx,
                format!("order:{}", ev.order_id),
                Kind::Order,
                ev.order_id.clone(),
                format!("{} {status}", ev.order_id),
                at,
            );
            let _ = total;

            for item in objects_in(j, "items") {
                let Some(pid) = str_field(&item, "product_id")
                    .or_else(|| str_field(&item, "productId"))
                    .or_else(|| str_field(&item, "id"))
                else {
                    continue;
                };
                if pid.is_empty() {
                    continue;
                }
                let dish = ensure(
                    &mut nodes,
                    &mut idx,
                    format!("dish:{pid}"),
                    Kind::Dish,
                    str_field(&item, "name").unwrap_or_default(),
                    str_field(&item, "name").unwrap_or_default(),
                    0,
                );
                edges.push((order, Rel::Contains, dish));
            }

            if let Some(cid) = str_field(j, "courier_id").filter(|s| !s.is_empty()) {
                let c = ensure(
                    &mut nodes,
                    &mut idx,
                    format!("courier:{cid}"),
                    Kind::Courier,
                    cid.clone(),
                    cid.clone(),
                    0,
                );
                edges.push((order, Rel::DeliveredBy, c));
            }
            // THE CUSTOMER NODE CARRIES NO PII — see `Node::text`. The key is
            // already the pseudonym the customer list is built on.
            if let Some(key) = str_field(j, "customer_key")
                .or_else(|| str_field(j, "customer_id"))
                .filter(|s| !s.is_empty())
            {
                let k = ensure(
                    &mut nodes,
                    &mut idx,
                    format!("customer:{key}"),
                    Kind::Customer,
                    String::new(),
                    String::new(),
                    0,
                );
                edges.push((order, Rel::PlacedBy, k));
            }
            if let Some(v) = venue_i {
                edges.push((order, Rel::ServedBy, v));
            }
        }

        // The caller's labels, last, so they win over an id used as a
        // placeholder. A label for a node that does not exist is ignored
        // rather than inventing one: the graph describes what happened, and
        // a courier who has carried nothing is not in it.
        for (id, label) in labels {
            if let Some(&i) = idx.get(id) {
                if !label.is_empty() {
                    nodes[i].label = label.clone();
                    nodes[i].text = if nodes[i].text.is_empty() {
                        label.clone()
                    } else {
                        format!("{} {label}", nodes[i].text)
                    };
                }
            }
        }

        Graph::build(nodes, edges)
    }
}

// ── retrieval ────────────────────────────────────────────────────────────────

/// Fold a string to comparable terms: lowercase, split on anything that is not
/// a letter or a digit.
///
/// UNICODE-AWARE BY CHARACTER, because the menu is Albanian and the console is
/// Ukrainian. Splitting on ASCII alone would make one term of "Розе" and
/// "Gjellë" both, and a search that cannot see a word cannot rank it.
pub fn terms(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in s.chars() {
        if ch.is_alphanumeric() {
            for l in ch.to_lowercase() {
                cur.push(l);
            }
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

impl Graph {
    /// BM25 over node text, in fixed point.
    ///
    /// BM25 AND NOT PLAIN TERM COUNTING because a menu is full of the same few
    /// words — "oriz sushi, nori" opens half the dishes — and a ranking that
    /// does not discount a common term returns the longest description rather
    /// than the best match. k1 and b are the usual 1.2 and 0.75, written as
    /// fractions so no float appears.
    pub fn bm25(&self, query: &str) -> Vec<(usize, i64)> {
        let q = terms(query);
        if q.is_empty() || self.nodes.is_empty() {
            return Vec::new();
        }
        let docs: Vec<Vec<String>> = self.nodes.iter().map(|n| terms(&n.text)).collect();
        let total_len: i64 = docs.iter().map(|d| d.len() as i64).sum();
        let n_docs = docs.len() as i64;
        let avg_len = (total_len.max(1)) / n_docs.max(1);

        let mut scores = vec![0i64; self.nodes.len()];
        for term in &q {
            let df = docs.iter().filter(|d| d.iter().any(|w| w == term)).count() as i64;
            if df == 0 {
                continue;
            }
            // idf = ln((N - df + 0.5)/(df + 0.5) + 1), approximated on integers
            // by a scaled log2 -- the RANKING is what matters here, and any
            // monotone transform of idf preserves it.
            let ratio = ((n_docs - df) * SCALE) / df.max(1) + SCALE;
            let idf = ilog2_fixed(ratio);
            for (i, d) in docs.iter().enumerate() {
                let tf = d.iter().filter(|w| *w == term).count() as i64;
                if tf == 0 {
                    continue;
                }
                let dl = d.len() as i64;
                // tf * (k1 + 1) / (tf + k1 * (1 - b + b * dl/avg))
                let k1_num = 12i64; // 1.2
                let k1_den = 10i64;
                let norm_num = 25 * k1_den + 75 * dl.max(1) * k1_den / avg_len.max(1); // (1-b) + b*dl/avg, x100
                let denom = tf * 100 * k1_den + k1_num * norm_num;
                let part = (tf * (k1_num + k1_den) * 100 * SCALE) / denom.max(1);
                // MULTIPLY BEFORE DIVIDING. `part / SCALE` first truncates to
                // zero for every term -- part is around 1.7e5 against a scale
                // of 1e6 -- and the whole ranking comes back empty while every
                // structural test still passes, because the walk does not use
                // it. Both stay well inside an i64: 1.7e5 x 6e7 is 1e13.
                scores[i] += part * idf / SCALE;
            }
        }
        let mut out: Vec<(usize, i64)> =
            scores.into_iter().enumerate().filter(|(_, s)| *s > 0).collect();
        out.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        out
    }

    /// Personalised PageRank from a set of seeds, in fixed point.
    ///
    /// THIS IS WHAT A KEYWORD SEARCH CANNOT DO. Asked about an ingredient, BM25
    /// finds the ingredient; PPR from it finds the dishes that use it and the
    /// orders that contained those dishes — the answer to "what does running out
    /// of this cost me", which is nowhere in any single node's text.
    ///
    /// Edges are walked in BOTH directions. A one-way walk from an order
    /// reaches its dishes and stops, because nothing points back, and the graph
    /// would be a list.
    pub fn ppr(&self, seeds: &[usize], iterations: usize) -> Vec<(usize, i64)> {
        let n = self.nodes.len();
        if n == 0 || seeds.is_empty() {
            return Vec::new();
        }
        // 0.85, as everywhere.
        const DAMP_NUM: i64 = 85;
        const DAMP_DEN: i64 = 100;

        let mut restart = vec![0i64; n];
        let share = SCALE / seeds.len() as i64;
        for &s in seeds {
            if s < n {
                restart[s] += share;
            }
        }
        // Degrees once, not once per node per iteration. `neighbours` builds a
        // Vec to answer this and `ppr` asked it n * iterations times.
        let deg: Vec<usize> = (0..n).map(|i| self.degree(i)).collect();

        let mut rank = restart.clone();
        let mut each = vec![0i64; n];
        for _ in 0..iterations {
            let mut next = vec![0i64; n];
            // What each neighbour of `i` receives. Computed for every node
            // first, so the push below can walk the edge list once instead of
            // rebuilding an adjacency list per node. Each edge appears in
            // exactly one node's `out` and one node's `inc`, so pushing both
            // ways here is the same arithmetic in the same order as asking
            // every node for its neighbours -- including duplicate edges.
            for i in 0..n {
                each[i] = if deg[i] == 0 { 0 } else { rank[i] * DAMP_NUM / DAMP_DEN / deg[i] as i64 };
            }
            for &(a, _, b) in &self.edges {
                next[b] += each[a];
                next[a] += each[b];
            }
            for i in 0..n {
                if deg[i] == 0 {
                    // A dangling node returns its mass to the seeds rather than
                    // losing it, or the totals shrink every iteration and the
                    // ranking drifts toward whatever is best connected.
                    for (j, r) in restart.iter().enumerate() {
                        next[j] += rank[i] * DAMP_NUM / DAMP_DEN * r / SCALE;
                    }
                }
            }
            for (j, r) in restart.iter().enumerate() {
                next[j] += (SCALE - SCALE * DAMP_NUM / DAMP_DEN) * r / SCALE;
            }
            rank = next;
        }
        let mut out: Vec<(usize, i64)> =
            rank.into_iter().enumerate().filter(|(_, s)| *s > 0).collect();
        out.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        out
    }

    /// PPR mass DIVIDED BY DEGREE — what the walk found that the node's own
    /// popularity does not already explain.
    ///
    /// THE RAW MASS RANKS THE GRAPH'S SHAPE, NOT THE ANSWER. A random walk on an
    /// undirected graph converges toward the degree distribution, so the
    /// best-connected node wins whatever you seeded it with. Every dish here
    /// carries a `ServedBy` edge to the one venue, which makes the venue a
    /// node of degree `n_dishes` and puts it two steps from everything. Measured
    /// on a 60-dish venue, asking for `shrimp` returned: the shrimp ingredient,
    /// then THE VENUE, then two categories, then the SALMON and TUNA
    /// ingredients — five hub nodes — and only then the dishes that actually
    /// contain shrimp. The one question the graph exists to answer was ranked
    /// sixth by the half of the ranker that exists to answer it.
    ///
    /// Dividing by degree asks the useful question instead: not "how much mass
    /// landed here" but "how much more than this node collects from anywhere".
    /// A venue every dish points at collects mass from every seed and earns no
    /// lift from any of them; a dish on three edges that the walk keeps
    /// reaching has found something. It needs no tuning constant and no list of
    /// node kinds to suppress -- the correction is the same arithmetic for the
    /// venue, a category and a popular ingredient alike.
    ///
    /// Applied to the STRUCTURAL half only. BM25 already divides by length,
    /// which is its own version of this, and the fusion downstream reads ranks
    /// rather than scores -- so this changes the ORDER the walk reports, which
    /// is the only thing RRF consumes.
    pub fn lift(&self, mass: &[(usize, i64)]) -> Vec<(usize, i64)> {
        let mut out: Vec<(usize, i64)> = mass
            .iter()
            .map(|&(i, m)| {
                // Degree 0 cannot be a divisor, and an isolated node was not
                // reached by the walk anyway -- it holds only its own restart.
                let d = self.degree(i).max(1) as i64;
                (i, m / d)
            })
            .collect();
        out.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        out
    }

    /// HYBRID: what the words say, fused with what the structure says.
    ///
    /// FUSED BY RANK, NOT BY SCORE. A BM25 score and a PageRank mass are not in
    /// the same units and never will be, so any weighted sum of them needs a
    /// constant somebody has to tune and nobody ever re-tunes. Reciprocal rank
    /// fusion ignores the magnitudes and asks only where each ranker put the
    /// item, which is the property that makes it work without a knob — and it
    /// is integer arithmetic, so it holds MANIFESTO C2's no-float rule.
    ///
    /// `k = 60` is the constant from the original RRF paper, kept because a
    /// number chosen here would be a number nobody could check.
    pub fn hybrid(&self, query: &str, k: usize) -> Vec<(usize, i64)> {
        const RRF_K: i64 = 60;
        let lexical = self.bm25(query);
        // The structural half starts where the words landed. With no lexical
        // hit there is nothing to walk from, and a PPR over every node would
        // return the graph's centre rather than an answer.
        let seeds: Vec<usize> = lexical.iter().take(5).map(|(i, _)| *i).collect();
        let structural = if seeds.is_empty() { Vec::new() } else { self.lift(&self.ppr(&seeds, 12)) };

        let mut fused: HashMap<usize, i64> = HashMap::new();
        for (rank, (i, _)) in lexical.iter().enumerate() {
            *fused.entry(*i).or_insert(0) += SCALE / (RRF_K + rank as i64 + 1);
        }
        for (rank, (i, _)) in structural.iter().enumerate() {
            *fused.entry(*i).or_insert(0) += SCALE / (RRF_K + rank as i64 + 1);
        }
        let mut out: Vec<(usize, i64)> = fused.into_iter().collect();
        out.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        out.truncate(k);
        out
    }
}

/// log2 of a fixed-point value, in fixed point. Monotone, which is all a
/// ranking needs, and integer, which is what the manifesto needs.
fn ilog2_fixed(x: i64) -> i64 {
    if x <= SCALE {
        return 0;
    }
    let whole = 63 - (x / SCALE).leading_zeros() as i64;
    // One fractional bit by comparing against the midpoint of the octave.
    let rest = x / (1i64 << whole);
    let frac = if rest > SCALE * 3 / 2 { SCALE / 2 } else { 0 };
    whole * SCALE + frac
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use crate::{EventKind, Hub};

    /// A venue the size of the real one, whose dish NAMES never say the
    /// ingredient — the only shape in which the graph earns its keep, because
    /// BM25 can answer the other one on its own.
    fn sixty_dishes() -> (Hub, Catalog) {
        let mut c = Catalog::create().expect("catalog");
        c.set_location(r#"{"id":"dubin-durres","name":"Dubin & Sushi","address":"Rruga Taulantia"}"#);
        let cats = ["maki", "nigiri", "bowls", "snacks", "chef", "drinks"];
        for i in 0..60 {
            let cat = cats[i % cats.len()];
            let (name, desc, supply) = match i % 3 {
                0 => (format!("Philadelphia {i}"), "oriz sushi, nori, krem", "salmon"),
                1 => (format!("Panko Special {i}"), "karkalec panko, chili", "shrimp"),
                _ => (format!("Spicy Roll {i}"), "oriz sushi, nori, majoneze", "tuna"),
            };
            c.set_product(
                &format!("item-{i:02}"),
                &format!(
                    r#"{{"name":"{name}","description":"{desc}","price":900,"categoryId":"{cat}","recipe":[{{"supplyId":"{supply}"}}]}}"#
                ),
            );
        }
        for sup in ["salmon", "shrimp", "tuna"] {
            c.set_supply(sup, &format!(r#"{{"name":"{sup}","unit":"kg"}}"#));
        }
        (Hub::create_sized(512 * 1024).expect("hub"), c)
    }

    /// THE HUB NODES MUST NOT OUTRANK THE ANSWER. Every dish points at the one
    /// venue, so the venue is a node of degree 60 and a raw PPR ranks it — and
    /// the other ingredients, and the categories — above every dish that
    /// actually contains what was asked for. Measured before `lift`: asking
    /// `shrimp` put the venue second, two categories third and fourth, and the
    /// SALMON and TUNA ingredients fifth and sixth, pushing the shrimp dishes
    /// to rank six and below. This asserts the walk answers the question.
    #[test]
    fn a_super_node_does_not_outrank_the_dishes_that_use_the_ingredient() {
        let (hub, cat) = sixty_dishes();
        let g = Graph::of(&hub, &cat);
        assert!(g.degree(g.index_of("venue:dubin-durres").expect("venue")) >= 60);

        for (q, other) in [("shrimp", "salmon"), ("salmon", "shrimp")] {
            let hits = g.hybrid(q, 12);
            let kinds: Vec<&str> = hits
                .iter()
                .filter_map(|(i, _)| g.node(*i))
                .map(|n| n.kind.tag())
                .collect();
            let names: Vec<String> = hits
                .iter()
                .filter_map(|(i, _)| g.node(*i))
                .map(|n| format!("{} {}", n.kind.tag(), n.label))
                .collect();

            assert!(
                !kinds.contains(&"venue"),
                "the venue is in the top 12 for '{q}': {names:?}"
            );
            assert!(
                !names.iter().any(|n| n == &format!("ingredient {other}")),
                "the '{other}' ingredient is in the top 12 for '{q}': {names:?}"
            );
            // And the dishes that DO use it are the bulk of the answer.
            let dishes = kinds.iter().filter(|k| **k == "dish").count();
            assert!(dishes >= 8, "only {dishes} of 12 hits are dishes for '{q}': {names:?}");
        }
    }

    fn venue() -> Catalog {
        let mut c = Catalog::create().expect("catalog");
        c.set_location(r#"{"id":"dubin-durres","name":"Dubin & Sushi","address":"Rruga Taulantia"}"#);
        c.set_product(
            "item-01",
            r#"{"name":"Sake Futomaki","description":"oriz sushi, nori, salmon","price":900,"categoryId":"chef","recipe":[{"supplyId":"salmon"}]}"#,
        );
        c.set_product(
            "item-31",
            r#"{"name":"Maki Salmon","description":"oriz sushi, nori, salmon","price":600,"categoryId":"maki","recipe":[{"supplyId":"salmon"}]}"#,
        );
        c.set_product(
            "item-41",
            r#"{"name":"Panko Shrimps","description":"karkalec panko, chili","price":800,"categoryId":"snacks","recipe":[{"supplyId":"shrimp"}]}"#,
        );
        c.set_supply("salmon", r#"{"name":"Salmon","unit":"kg"}"#);
        c.set_supply("shrimp", r#"{"name":"Shrimp","unit":"kg"}"#);
        c
    }

    fn with_orders() -> (Hub, Catalog) {
        let mut hub = Hub::create_sized(256 * 1024).expect("hub");
        hub.append(
            EventKind::Placed,
            "ord-1",
            r#"{"status":"delivered","total":1500,"created_at_ms":1000,"courier_id":"eni","customer_key":"cust-a","items":[{"product_id":"item-01","name":"Sake Futomaki"},{"product_id":"item-31","name":"Maki Salmon"}]}"#,
            1000,
            [0u8; 32],
        )
        .expect("append");
        hub.append(
            EventKind::Placed,
            "ord-2",
            r#"{"status":"delivered","total":800,"created_at_ms":2000,"courier_id":"blerim","customer_key":"cust-b","items":[{"product_id":"item-41","name":"Panko Shrimps"}]}"#,
            2000,
            [0u8; 32],
        )
        .expect("append");
        (hub, venue())
    }

    #[test]
    fn the_fold_relates_what_the_json_only_referenced() {
        let (hub, cat) = with_orders();
        let g = Graph::of(&hub, &cat);

        let order = g.index_of("order:ord-1").expect("the order is a node");
        let kinds: Vec<&'static str> =
            g.neighbours(order).iter().map(|(r, _, _)| r.tag()).collect();
        assert!(kinds.contains(&"contains"), "{kinds:?}");
        assert!(kinds.contains(&"delivered_by"), "{kinds:?}");
        assert!(kinds.contains(&"placed_by"), "{kinds:?}");

        // The relation the JSON never stated: this order reached an ingredient,
        // through a dish, without anything saying so.
        let dish = g.index_of("dish:item-01").expect("dish");
        let via: Vec<String> = g
            .neighbours(dish)
            .iter()
            .filter(|(r, _, fwd)| *r == Rel::Uses && *fwd)
            .map(|(_, j, _)| g.node(*j).unwrap().id.clone())
            .collect();
        assert_eq!(via, vec!["ingredient:salmon"], "the recipe must reach the shelf");
    }

    /// The customer node is the one place a retrieval index could quietly grow
    /// a copy of someone's phone number.
    #[test]
    fn no_customer_pii_reaches_the_index() {
        let mut hub = Hub::create_sized(128 * 1024).expect("hub");
        hub.append(
            EventKind::Placed,
            "ord-9",
            r#"{"status":"new","customer_key":"cust-z","contact":{"name":"Ana","phone":"+355691112233"},"items":[]}"#,
            1,
            [0u8; 32],
        )
        .expect("append");
        let g = Graph::of(&hub, &venue());
        for n in g.nodes() {
            assert!(!n.text.contains("355691112233"), "a phone reached node {}", n.id);
            assert!(!n.text.contains("Ana"), "a name reached node {}", n.id);
        }
    }

    #[test]
    fn a_word_search_finds_the_dish_that_uses_it() {
        let (hub, cat) = with_orders();
        let g = Graph::of(&hub, &cat);
        let hits = g.bm25("karkalec");
        assert!(!hits.is_empty(), "the menu says karkalec");
        assert_eq!(g.node(hits[0].0).unwrap().id, "dish:item-41");
    }

    /// The whole reason for the structural half: an ingredient's name is not in
    /// any order's text, so a word search alone cannot answer "what does the
    /// shrimp touch" — and the walk can.
    #[test]
    fn the_walk_reaches_what_the_words_cannot() {
        let (hub, cat) = with_orders();
        let g = Graph::of(&hub, &cat);

        let shrimp = g.index_of("ingredient:shrimp").expect("shrimp");
        assert!(
            g.bm25("shrimp").iter().all(|(i, _)| g.node(*i).unwrap().id != "order:ord-2"),
            "the order's own text must not mention the ingredient, or this proves nothing"
        );

        let reached: Vec<String> = g
            .ppr(&[shrimp], 12)
            .into_iter()
            .map(|(i, _)| g.node(i).unwrap().id.clone())
            .collect();
        assert!(reached.contains(&"dish:item-41".to_string()), "{reached:?}");
        assert!(reached.contains(&"order:ord-2".to_string()), "the walk must reach the order: {reached:?}");
    }

    #[test]
    fn hybrid_ranks_the_named_thing_first_and_still_brings_its_neighbours() {
        let (hub, cat) = with_orders();
        let g = Graph::of(&hub, &cat);
        let out = g.hybrid("karkalec panko", 8);
        assert!(!out.is_empty());
        let ids: Vec<String> = out.iter().map(|(i, _)| g.node(*i).unwrap().id.clone()).collect();
        assert_eq!(ids[0], "dish:item-41", "the named dish leads: {ids:?}");
        assert!(
            ids.iter().any(|id| id == "order:ord-2"),
            "and the structure brings the order that contained it: {ids:?}"
        );
    }

    /// A ranking is part of an answer, so it has to replay identically.
    #[test]
    fn the_same_graph_ranks_the_same_way_twice() {
        let (hub, cat) = with_orders();
        let a = Graph::of(&hub, &cat);
        let b = Graph::of(&hub, &cat);
        assert_eq!(a.hybrid("salmon", 10), b.hybrid("salmon", 10));
        assert_eq!(a.len(), b.len());
        assert_eq!(a.edge_count(), b.edge_count());
    }

    #[test]
    fn a_courier_can_be_named_by_the_caller_and_a_customer_cannot_be() {
        let (hub, cat) = with_orders();
        let mut labels = HashMap::new();
        labels.insert("courier:eni".to_string(), "Eni".to_string());
        // A customer label is offered and must still leave no PII: the caller
        // decides what it passes, and this asserts the seam does not smuggle.
        labels.insert("customer:cust-a".to_string(), String::new());
        let g = Graph::of_with(&hub, &cat, &labels);

        let hits = g.bm25("Eni");
        assert!(!hits.is_empty(), "a named courier must be findable by name");
        assert_eq!(g.node(hits[0].0).unwrap().id, "courier:eni");

        let cust = g.index_of("customer:cust-a").expect("customer node");
        assert!(g.node(cust).unwrap().text.is_empty(), "an empty label adds nothing");
    }

    #[test]
    fn an_empty_hub_is_an_empty_graph_not_a_panic() {
        let hub = Hub::create_sized(64 * 1024).expect("hub");
        let cat = Catalog::create().expect("catalog");
        let g = Graph::of(&hub, &cat);
        assert!(g.hybrid("anything", 5).is_empty());
        assert!(g.ppr(&[], 5).is_empty());
        assert!(g.bm25("").is_empty());
    }
}
