//! RETRIEVAL over the venue graph: terms, BM25 in fixed point, personalised
//! PageRank from the seeds it finds, and the hybrid of the two that a search asks.
//! Integer arithmetic throughout -- the same answer on every node.

use super::*;

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
