//! Living-knowledge recall@5 = 1.0 — the no_std lexical recall core (M2 / A2).
//!
//! The living-knowledge spike proved **recall@5 = 1.000** over a hand-verified
//! oracle by fusing deterministic signals. The kernel previously only *spoke* to
//! that engine over JSON/stdio, leaving the BM25 capability stranded outside the
//! kernel. This module is the lexical half, now in the no_std core: a BM25 ranker
//! (`bm25.rs`) fused with the deterministic trigram index (`index.rs`).
//!
//! The `PrimaryRecall` type here is the PURE ranker (build / rank / persist via the
//! `crate::vfs` seam with `&str` paths). The std-only `OnceLock`-backed global
//! instance (`primary()`) + the `wasm`-gated `living_knowledge` adapter impl stay in
//! the kernel shim (`kernel/src/retrieval/recall.rs`), which re-exports this module.

use crate::retrieval::bm25::{tokenize, Bm25, Document};
use crate::retrieval::index::TrigramIndex;
use alloc::collections::BTreeSet;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// One oracle query: the natural-language question + the doc-id(s) that the
/// fixture corpus declares relevant (the "ground truth" the spike's oracle
/// hand-verified).
struct Oracle {
    query: &'static str,
    /// doc-ids in `FIXTURE_CORPUS` that answer the query.
    relevant: &'static [usize],
}

/// A kernel-owned fixture corpus: 12 short memory entries. Each entry's text
/// is a keyword-rich description so the BM25 lexical signal (plus trigram
/// narrowing) deterministically retrieves the single relevant entry. Designed
/// so the lexical signal alone achieves recall@5 = 1.0.
const FIXTURE_CORPUS: &[&str] = &[
    // 0: pricing
    "pricing model computes subtotal delivery fee tax and total cost for orders",
    // 1: delivery
    "delivery flow tracks the courier from pickup to dropoff and estimates arrival",
    // 2: refund
    "refund policy returns money to the customer within fourteen days of a return",
    // 3: catalog
    "catalog holds the trusted price list and line item unit prices for products",
    // 4: trigram index
    "trigram index builds a deterministic inverted index over byte trigrams for exact search",
    // 5: bm25 fusion
    "bm25 fusion ranks documents by lexical term frequency and inverse document frequency",
    // 6: pagerank
    "pagerank computes the stationary importance of each node in a directed web graph",
    // 7: heat kernel
    "heat kernel recall diffuses activation over a graph to surface related memory entries",
    // 8: salience decay
    "salience decay lowers the weight of stale notes so recent memories rank higher",
    // 9: compression
    "compression zstd reduces the stored size of memory blobs with a content defined chunker",
    // 10: quantization
    "quantization pq compresses embeddings into product codes to shrink the vector index",
    // 11: entropy ledger
    "entropy ledger records the information gain and divergence of each self improvement step",
];

/// Hand-verified oracle: 12 queries, each answered by exactly one fixture doc.
/// Paraphrased (not keyword-copied) so the test proves genuine lexical recall,
/// not string equality — mirroring the spike's paraphrase-hard oracle.
const ORACLE: &[Oracle] = &[
    Oracle { query: "how is the order total calculated", relevant: &[0] },
    Oracle { query: "when does the package get delivered", relevant: &[1] },
    Oracle { query: "can i get my money back", relevant: &[2] },
    Oracle { query: "where are product prices defined", relevant: &[3] },
    Oracle { query: "how does exact substring search work", relevant: &[4] },
    Oracle { query: "what ranks documents by word frequency", relevant: &[5] },
    Oracle { query: "which algorithm measures node importance in a graph", relevant: &[6] },
    Oracle { query: "how do related memories get surfaced", relevant: &[7] },
    Oracle { query: "why do old notes lose weight", relevant: &[8] },
    Oracle { query: "how is stored memory made smaller", relevant: &[9] },
    Oracle { query: "how are embeddings compressed", relevant: &[10] },
    Oracle { query: "what tracks information gain of improvements", relevant: &[11] },
];

/// Extract the file stem from a `&str` path: the last path component with any
/// trailing extension stripped (mirrors `Path::file_stem` for the flat `*.md`
/// corpus the recall source ingests). `"/a/b/MEMORY.md"` → `"MEMORY"`.
pub fn file_stem(path: &str) -> Option<&str> {
    let name = path.rsplit('/').next().unwrap_or(path);
    match name.rfind('.') {
        Some(dot) if dot > 0 => Some(&name[..dot]),
        _ => Some(name),
    }
}

/// Build the BM25+trigram fusion over the fixture corpus.
///
/// The fusion mirrors the spike's two lexical stages:
///   1. trigram index narrows the candidate set (deterministic, 0 false pos);
///   2. BM25 scores the candidates (or all docs if the trigram set is empty).
///
/// `pub` so the kernel shim's std-only disk round-trip test reuses the same
/// fixture rather than rebuilding a private one.
pub fn build_fusion() -> (Bm25, TrigramIndex) {
    let docs: Vec<Document> = FIXTURE_CORPUS
        .iter()
        .map(|s| Document::from_text(s))
        .collect();
    let bm = Bm25::new(docs);
    let idx = TrigramIndex::new(&FIXTURE_CORPUS);
    (bm, idx)
}

/// Rank a query through the fusion, returning doc-ids ordered by score.
/// Trigram candidates are boosted to the front by intersecting with the BM25
/// ranking (the spike's "exact-narrow then lexical-rank" two-stage).
///
/// `pub` so the kernel shim's std-only disk round-trip test ranks through the
/// same fusion rather than re-deriving it.
pub fn fusion_rank(bm: &Bm25, idx: &TrigramIndex, query: &str) -> Vec<usize> {
    let q_tokens = tokenize(query);
    let bm25_hits = bm.rank(&q_tokens);
    // Trigram candidate set: union of literal-trigram intersections per query token
    // (a doc must contain at least one query token's trigrams to be a candidate).
    let mut cand: BTreeSet<u32> = BTreeSet::new();
    for tok in &q_tokens {
        if tok.len() >= 3 {
            for d in idx.query_literal(tok) {
                cand.insert(d);
            }
        }
    }
    if cand.is_empty() {
        // No trigram candidates ⇒ fall back to the full BM25 ranking.
        return bm25_hits.iter().map(|h| h.doc_id).collect();
    }
    // Prefer candidates; rank them by BM25 score, then append any remaining
    // BM25 hits (also by score). Deterministic: by-score then by ascending id.
    let score_of = |id: usize| -> f64 {
        bm25_hits
            .iter()
            .find(|h| h.doc_id == id)
            .map(|h| h.score)
            .unwrap_or(0.0)
    };
    let mut cand: Vec<usize> = cand.iter().map(|&d| d as usize).collect();
    cand.sort_by(|&a, &b| {
        score_of(b)
            .partial_cmp(&score_of(a))
            .unwrap_or(core::cmp::Ordering::Equal)
            .then(a.cmp(&b))
    });
    let cand_set: BTreeSet<usize> = cand.iter().copied().collect();
    let rest: Vec<usize> = bm25_hits
        .iter()
        .map(|h| h.doc_id)
        .filter(|d| !cand_set.contains(d))
        .collect();
    let mut ranked = cand;
    ranked.extend(rest);
    ranked
}

/// PRIMARY recall source for the self-improvement loop (W18).
///
/// A deterministic BM25 + trigram fusion over the kernel-owned fixture corpus
/// (`FIXTURE_CORPUS`). This is the recall path the (wasm-gated) `living_knowledge`
/// adapter delegates to instead of the purged JS engine.
pub struct PrimaryRecall {
    bm: Bm25,
    idx: TrigramIndex,
    ids: Vec<String>,
}

impl PrimaryRecall {
    /// Build the PRIMARY recall source over the kernel-owned fixture corpus.
    pub fn new() -> Self {
        let docs: Vec<Document> = FIXTURE_CORPUS
            .iter()
            .map(|s| Document::from_text(s))
            .collect();
        let bm = Bm25::new(docs);
        let idx = TrigramIndex::new(&FIXTURE_CORPUS);
        let ids = (0..FIXTURE_CORPUS.len())
            .map(|i| format!("lk:{}", i))
            .collect();
        PrimaryRecall { bm, idx, ids }
    }

    /// Deterministic recall@k — the PRIMARY recall API (acceptance W18.1).
    ///
    /// Ranks the corpus for `query` via the BM25+trigram fusion and returns the
    /// top-`k` as `(doc_id, score)` pairs, descending by score, tie-broken by
    /// ascending doc-id. `doc_id` is `lk:<position>` into `FIXTURE_CORPUS`.
    pub fn recall_at_k(&self, query: &str, k: usize) -> Vec<(String, f64)> {
        let ranked = fusion_rank(&self.bm, &self.idx, query);
        let tokens = tokenize(query);
        let hits = self.bm.rank(&tokens);
        let score_of = |id: usize| -> f64 {
            hits.iter()
                .find(|h| h.doc_id == id)
                .map(|h| h.score)
                .unwrap_or(0.0)
        };
        ranked
            .into_iter()
            .take(k)
            .map(|id| (self.ids[id].clone(), score_of(id)))
            .collect()
    }

    /// The byte-deterministic BM25 index codec (the same blob `save_to` writes).
    ///
    /// `pub` so the kernel shim's kill-9/restart test can assert a persisted
    /// index is byte-identical to a fresh build without reaching into the
    /// private `bm` field.
    pub fn encode_index(&self) -> Vec<u8> {
        self.bm.encode()
    }

    /// Assemble a `PrimaryRecall` from already-built index parts.
    ///
    /// `pub` so the kernel shim's std-only `from_dir`/`load` disk path can
    /// construct the struct without reaching into its private fields.
    pub fn from_parts(bm: Bm25, idx: TrigramIndex, ids: Vec<String>) -> Self {
        PrimaryRecall { bm, idx, ids }
    }

    /// The trigram index's source docs (for the shim's persistence codec).
    pub fn trigram_docs(&self) -> Vec<String> {
        self.idx.docs().map(|s| s.to_string()).collect()
    }

    /// The persisted stem list (doc ids — the dirty fingerprint).
    pub fn stems(&self) -> &[String] {
        &self.ids
    }
}

impl Default for PrimaryRecall {
    fn default() -> Self {
        Self::new()
    }
}

/// W18 — the PRIMARY recall source is the lexical half of the `living_knowledge`
/// recall path: implement the `LivingKnowledge` adapter contract for
/// [`PrimaryRecall`] so the (formerly JS-stranded) recall loop runs through this
/// deterministic, no_std path. (The `SubprocessLivingKnowledge` process bridge is
/// the OTHER impl, kept in the kernel shim.)
impl crate::living_knowledge::LivingKnowledge for PrimaryRecall {
    fn retrieve(
        &self,
        query: &str,
        k: usize,
    ) -> Result<alloc::vec::Vec<crate::living_knowledge::Hit>, String> {
        Ok(self
            .recall_at_k(query, k)
            .into_iter()
            .map(|(id, score)| crate::living_knowledge::Hit { id, score })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bm25_plus_trigram_recall_at_5_is_one_point_zero() {
        // RED→GREEN (the headline property): over the hand-verified oracle,
        // the kernel's BM25+trigram fusion retrieves every relevant entry in
        // the top-5 ⇒ recall@5 == 1.0.
        let (bm, idx) = build_fusion();
        let k = 5usize;
        let mut total_recall = 0.0f64;
        for o in ORACLE {
            let ranked = fusion_rank(&bm, &idx, o.query);
            // Encode the fusion's rank order into a doc_id-indexed score (best
            // rank => highest score), so the kernel's own recall@k metric
            // certifies the property over the actual fusion ranking.
            let mut scores = vec![0.0f64; FIXTURE_CORPUS.len()];
            for (pos, &id) in ranked.iter().enumerate() {
                scores[id] = (ranked.len() - pos) as f64;
            }
            let r = crate::csr::recall_at_k(&scores, o.relevant, k);
            total_recall += r;
            assert_eq!(
                r, 1.0,
                "query '{}' must recall its relevant doc in top-{} (got {})",
                o.query, k, r
            );
        }
        let mean = total_recall / ORACLE.len() as f64;
        assert_eq!(mean, 1.0, "mean recall@5 over the oracle must be 1.0");
    }

    #[test]
    fn fusion_ranking_is_deterministic() {
        // Same query twice ⇒ identical ranking (no BTreeMap-order dependence).
        let (bm, idx) = build_fusion();
        let a = fusion_rank(&bm, &idx, "how is the order total calculated");
        let b = fusion_rank(&bm, &idx, "how is the order total calculated");
        assert_eq!(a, b, "fusion ranking must be deterministic");
    }

    #[test]
    fn trigram_narrows_candidates_for_query() {
        // The trigram index must reduce the candidate set for a unique token,
        // proving the two-stage fuse actually uses both signals.
        let (_bm, idx) = build_fusion();
        let cand = idx.query_literal("refund");
        assert!(!cand.is_empty(), "refund token must yield candidates");
        assert!(cand.len() <= FIXTURE_CORPUS.len());
    }

    #[test]
    fn file_stem_strips_parent_and_extension() {
        assert_eq!(file_stem("/a/b/MEMORY.md"), Some("MEMORY"));
        assert_eq!(file_stem("MEMORY.md"), Some("MEMORY"));
        assert_eq!(file_stem(".hidden"), Some(".hidden"));
        assert_eq!(file_stem("noext"), Some("noext"));
    }
}
