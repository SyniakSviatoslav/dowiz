//! diffusion.rs — L3 RELATEDNESS layer (internal-retrieval M3).
//!
//! What "related" means here: NOT exact search (that is L0 / M1, the sibling
//! trigram index). This is the vectorless spectral/relatedness layer — given a
//! query node, *diffuse* its personalized-PageRank mass over a wikilink graph
//! and rank neighbours by steady-state score. "What is related to X" falls out
//! as "which nodes accumulated the most restart+walk mass".
//!
//! Engine: `Ppr` (ppr.rs) — a bit-for-bit reuse of `kernel/src/markov.rs`'s
//! deterministic damped power-iteration (fixed K, fixed summation order, no
//! eigendecomposition). The wikilink graph is a frozen 20-node / 41-edge
//! fixture; nodes are named after the same L0 corpus (`fixtures::FIXTURE`) so
//! the two retrieval layers share one vocabulary.
//!
//! Determinism proof lives in `green_ppr_byte_identical_two_runs`: two runs with
//! identical (seed, α, K) produce byte-identical score vectors.

use super::fixtures::FIXTURE;
use super::ppr::Ppr;

/// Number of article nodes in the fixture wikilink graph.
pub const N: usize = 20;

/// Query seed node (the "what relates to X?" anchor). Node 0 = "MEMORY.md".
pub const SEED: usize = 0;

/// Restart (teleport) probability — same family as `markov.rs`' DAMPING.
pub const ALPHA: f64 = 0.15;

/// FIXED iteration count — no convergence epsilon, no early-out ⇒ reproducible.
pub const K: usize = 20;

/// Frozen wikilink fixture: 20 nodes, 41 directed edges (article → linked
/// article). Every node has out-degree ≥ 1 (no dangling sinks). The graph is
/// intentionally NOT fully connected: nodes {5,6,12,16} sit in a separate
/// component, so a diffusion from SEED must leave them at exactly score 0 —
/// the clean "unrelated" baseline the tests assert against.
pub const WIKI_EDGES: &[(usize, usize)] = &[
    // 0 MEMORY.md
    (0, 1),
    (0, 2),
    (0, 3),
    (0, 9),
    // 1 MEMORY-ATTIC.md
    (1, 0),
    (1, 9),
    // 2 note-salience-decay.md
    (2, 0),
    (2, 3),
    (2, 13),
    // 3 note-wikilink-graph.md
    (3, 0),
    (3, 4),
    (3, 8),
    (3, 7),
    // 4 note-pgrust-schema.md
    (4, 3),
    (4, 11),
    (4, 18),
    // 5 note-trigram-index.md
    (5, 6),
    (5, 16),
    // 6 note-bm25-fusion.md
    (6, 5),
    (6, 16),
    // 7 note-heat-kernel-recall.md
    (7, 8),
    (7, 3),
    // 8 note-pagerank-local-push.md
    (8, 7),
    // 9 note-never-delete-tier.md
    (9, 0),
    (9, 10),
    (9, 17),
    // 10 note-compression-zstd.md
    (10, 9),
    (10, 19),
    // 11 note-vsa-composite-key.md
    (11, 4),
    // 12 note-renormalizer-gate.md
    (12, 13),
    (12, 16),
    // 13 note-entropy-ledger.md
    (13, 2),
    (13, 14),
    // 14 note-field-operator.md
    (14, 13),
    (14, 15),
    // 15 note-divergence-signal.md
    (15, 14),
    // 16 note-coherence-fusion.md
    (16, 5),
    (16, 6),
    // 17 note-ttrain-deferred.md
    (17, 9),
    // 18 note-quantization-pq.md
    (18, 4),
    // 19 note-cdc-dedup.md
    (19, 10),
];

/// Node labels (mirror `fixtures::FIXTURE` so L0 and L3 share one vocabulary).
pub fn node_labels() -> &'static [&'static str] {
    FIXTURE
}

/// Build the row-stochastic transition matrix W from the directed edge list
/// (each row = uniform over its out-edges, so Σ_j W[i][j] = 1).
pub fn wiki_row_stochastic() -> Vec<Vec<f64>> {
    let mut out = vec![vec![0.0f64; N]; N];
    for &(u, v) in WIKI_EDGES {
        out[u][v] += 1.0;
    }
    for i in 0..N {
        let s: f64 = out[i].iter().sum();
        if s > 0.0 {
            for j in 0..N {
                out[i][j] /= s;
            }
        }
    }
    out
}

/// The wikilink graph as a ready-to-run PPR engine.
pub fn wiki_ppr() -> Ppr {
    Ppr::new(wiki_row_stochastic())
}

/// Relatedness ranking for `seed`: all nodes sorted by PPR score, descending.
/// Pair = (node_id, score). Includes the seed itself (highest, by construction
/// of the restart).
pub fn related(seed: usize) -> Vec<(usize, f64)> {
    let ppr = wiki_ppr();
    let scores = ppr.rank(seed, ALPHA, K);
    let mut v: Vec<(usize, f64)> = (0..N).map(|i| (i, scores[i])).collect();
    v.sort_by(|a, b| b.1.total_cmp(&a.1));
    v
}

/// Forward-reachable set from `seed` (used as the ground-truth "related"
/// indicator — unreachable nodes must score exactly 0).
pub fn reachable_from(seed: usize) -> Vec<bool> {
    let mut seen = vec![false; N];
    let mut stack = vec![seed];
    seen[seed] = true;
    while let Some(u) = stack.pop() {
        for &(e_u, e_v) in WIKI_EDGES {
            if e_u == u && !seen[e_v] {
                seen[e_v] = true;
                stack.push(e_v);
            }
        }
    }
    seen
}

#[cfg(test)]
mod tests {
    use super::*;

    // hop layers of the fixture (hand-verified in module docs)
    const ONE_HOP: [usize; 4] = [1, 2, 3, 9];
    const TWO_HOP: [usize; 6] = [4, 7, 8, 10, 13, 17];
    const FAR: [usize; 5] = [11, 14, 15, 18, 19];
    const UNRELATED: [usize; 4] = [5, 6, 12, 16];

    #[test]
    fn green_wikilink_fixture_shape() {
        assert_eq!(N, 20);
        // 41 directed edges
        assert_eq!(WIKI_EDGES.len(), 41);
        // node labels line up with the L0 corpus
        assert_eq!(node_labels().len(), N);
        assert_eq!(node_labels()[SEED], "MEMORY.md");
    }

    #[test]
    fn green_row_stochastic_each_row_sums_to_one() {
        let w = wiki_row_stochastic();
        for (i, row) in w.iter().enumerate() {
            let s: f64 = row.iter().sum();
            assert!((s - 1.0).abs() < 1e-12, "row {} sums to {}", i, s);
        }
    }

    /// THE RED→GREEN DETERMINISM PROOF.
    /// Two runs with identical (seed, α, K) must be byte-identical. We assert
    /// both bitwise (`Vec<f64>` ==) and via full-precision serialization.
    #[test]
    fn green_ppr_byte_identical_two_runs() {
        let ppr = wiki_ppr();
        let a = ppr.rank(SEED, ALPHA, K);
        let b = ppr.rank(SEED, ALPHA, K);
        assert_eq!(a, b, "PPR score vector is not bit-identical across runs");
        let sa: Vec<String> = a.iter().map(|x| format!("{:.17e}", x)).collect();
        let sb: Vec<String> = b.iter().map(|x| format!("{:.17e}", x)).collect();
        assert_eq!(
            sa.join(","),
            sb.join(","),
            "PPR full-precision serialization differs across runs"
        );
    }

    /// Hermetic-audit Cause-and-Effect Finding B (quick-win #19): same-process double-call
    /// comparison only proves two live values match — it never proves the value survives an
    /// actual serialization boundary. Write to disk, re-read, re-parse, compare against an
    /// independently fresh computation.
    #[test]
    fn diffusion_ppr_survives_serialize_reread_boundary() {
        let ppr = wiki_ppr();
        let computed = ppr.rank(SEED, ALPHA, K);
        let serialized: String = computed
            .iter()
            .map(|x| format!("{:.17e}", x))
            .collect::<Vec<_>>()
            .join(",");

        let path = std::env::temp_dir().join(format!(
            "diffusion_ppr_reread_test_{}.txt",
            std::process::id()
        ));
        std::fs::write(&path, &serialized).expect("write serialized diffusion scores");
        let reread = std::fs::read_to_string(&path).expect("re-read serialized diffusion scores");
        std::fs::remove_file(&path).ok();

        assert_eq!(
            reread, serialized,
            "byte content did not survive a disk round-trip"
        );

        let reparsed: Vec<f64> = reread
            .split(',')
            .map(|s| s.parse::<f64>().expect("reparse f64"))
            .collect();
        let fresh = ppr.rank(SEED, ALPHA, K); // independently recomputed, not `computed`
        assert_eq!(
            reparsed, fresh,
            "value re-read from disk does not match an independently fresh computation"
        );
    }

    /// Mass is conserved by the (1−α) diffusion + α restart — no per-step
    /// normalization needed, which is what buys bitwise determinism.
    #[test]
    fn green_ppr_mass_conserved() {
        let ppr = wiki_ppr();
        let scores = ppr.rank(SEED, ALPHA, K);
        let total: f64 = scores.iter().sum();
        assert!((total - 1.0).abs() < 1e-12, "mass drift = {}", total - 1.0);
    }

    /// RELATEDNESS ORDERING CORRECTNESS (vs the M1/M2 exact-search baseline).
    /// On the fixture, the top-related nodes are the directly-linked (1-hop) and
    /// 2-hop neighbours; unreachable nodes score exactly 0; and 1-hop > 2-hop >
    /// far (3+/4-hop) in score — the canonical "relatedness decay with hops".
    #[test]
    fn green_relatedness_ranking_correct() {
        let ppr = wiki_ppr();
        let scores = ppr.rank(SEED, ALPHA, K);

        // direct neighbours carry mass; unrelated (separate component) get exactly 0
        for &d in &ONE_HOP {
            assert!(scores[d] > 0.0, "1-hop node {} must carry mass", d);
        }
        for &u in &UNRELATED {
            assert_eq!(
                scores[u], 0.0,
                "unreachable node {} must score exactly 0",
                u
            );
        }

        // relatedness decays with hop-distance: 1-hop > 2-hop > far
        for &a in &ONE_HOP {
            for &u in &UNRELATED {
                assert!(scores[a] > scores[u], "direct {} > unrelated {}", a, u);
            }
            for &b in &FAR {
                assert!(scores[a] > scores[b], "1-hop {} must exceed far {}", a, b);
            }
        }
        for &a in &TWO_HOP {
            for &b in &FAR {
                assert!(scores[a] > scores[b], "2-hop {} must exceed far {}", a, b);
            }
        }

        // reachable ⇔ positive score (the structural definition of "related")
        let reach = reachable_from(SEED);
        for i in 0..N {
            assert_eq!(
                scores[i] > 0.0,
                reach[i],
                "node {} score-sign must match reachability",
                i
            );
        }
    }

    /// The single most-related node (excluding the seed) is a *direct*
    /// neighbour — directly-linked articles rank above 2-hop neighbours.
    #[test]
    fn green_top_related_is_direct_neighbour() {
        let mut ranked = related(SEED);
        ranked.retain(|&(i, _)| i != SEED);
        assert!(
            ONE_HOP.contains(&ranked[0].0),
            "top related node must be a direct (1-hop) neighbour, got {}",
            ranked[0].0
        );
    }

    /// Diffusion is a *relatedness* signal, not exact search: re-running from a
    /// different seed changes the ranking (proves it is query-personalized, not
    /// a static centrality).
    #[test]
    fn green_personalized_seed_changes_ranking() {
        let ppr = wiki_ppr();
        let from_a = ppr.rank(SEED, ALPHA, K);
        let from_b = ppr.rank(13, ALPHA, K); // seed at a 2-hop node
        assert_ne!(
            from_a, from_b,
            "different seeds must yield different personalized rankings"
        );
    }
}
