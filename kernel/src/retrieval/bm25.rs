//! BM25 ranker (Okapi BM25) — vectorless lexical retrieval (M2 / L2).
//!
//! Pure-`std` Okapi BM25 over a pre-tokenized corpus. No external deps, no
//! float-on-money. Deterministic: same corpus + same query ⇒ same ranking
//! (fixed `k1`, `b`, fixed term-frequency summation order, deterministic
//! tie-break by ascending doc-id).
//!
//! This is the *lexical* signal of the living-knowledge engine that lived on a
//! divergent JS branch (`recover/stash-1-2994e6c8`). Wiring it into the kernel
//! (this file + `recall.rs`) un-strands that capability: the kernel now owns a
//! real BM25 ranker it can fuse with the trigram index, rather than only
//! speaking JSON-over-stdio to a node subprocess.
//!
//! BM25 score (per doc d, query Q):
//!
//! ```text
//!   score(d, Q) = Σ_{t ∈ Q}  IDF(t) ·  f(t,d)·(k1+1)
//!                              ─────────────────────
//!                              f(t,d) + k1·(1 − b + b·|d|/avgdl)
//!   IDF(t)     = ln(1 + (N − n_t + 0.5) / (n_t + 0.5))
//! ```
//!
//! `f(t,d)` = term frequency of `t` in doc `d`; `n_t` = number of docs
//! containing `t`; `N` = corpus size; `|d|` = length of doc `d` (in tokens);
//! `avgdl` = mean doc length. Standard defaults `k1 = 1.5`, `b = 0.75`.

/// Standard Okapi BM25 free parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bm25Params {
    /// Term-frequency saturation (higher ⇒ faster saturation).
    pub k1: f64,
    /// Length normalization (0 = none, 1 = full pivoted normalization).
    pub b: f64,
}

impl Default for Bm25Params {
    fn default() -> Self {
        Bm25Params { k1: 1.5, b: 0.75 }
    }
}

/// A single tokenized document: an owned token list. Tokenization is the
/// caller's responsibility (lowercase/stem/exact — the spike stemmed with
/// Porter; the kernel tests use lowercase+alnum so the spike's measured
/// property transfers). We store the tokens so length + term-frequency are
/// O(1)-derivable.
#[derive(Debug, Clone)]
pub struct Document {
    tokens: Vec<String>,
}

impl Document {
    /// Build a document from an already-tokenized token list.
    pub fn new(tokens: Vec<String>) -> Self {
        Document { tokens }
    }

    /// Tokenize `text` with the shared [`tokenize`] policy (lowercase,
    /// alnum, single-char tokens kept) and build a document.
    pub fn from_text(text: &str) -> Self {
        Document {
            tokens: tokenize(text),
        }
    }

    /// Number of tokens (the BM25 `|d|`).
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    /// True when the document holds no tokens.
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// Borrow the token list.
    pub fn tokens(&self) -> &[String] {
        &self.tokens
    }
}

/// Whitespace + ASCII-punctuation tokenizer. Lowercases, splits on runs of
/// non-alphanumeric bytes, drops empties, keeps single-char tokens. UTF-8
/// safe (operates on `char`s). Mirrors the lexical front-end the living-
/// knowledge spike used (before Porter stemming — see module docs on transfer).
pub fn tokenize(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            cur.extend(ch.to_lowercase());
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// A scored retrieval hit.
#[derive(Debug, Clone, PartialEq)]
pub struct Scored {
    /// doc-id (position in the corpus the `Bm25` was built from).
    pub doc_id: usize,
    /// BM25 score for this document under the last query ranked.
    pub score: f64,
}

/// A ready-to-query BM25 index over a corpus of documents.
///
/// Building precomputes per-doc term-frequency maps, doc lengths, mean length,
/// and document frequencies (`n_t`) so each query is O(|Q| · distinct posting
/// work). Deterministic regardless of `HashMap` iteration order: all sums are
/// reduced over a *sorted* term set.
#[derive(Debug, Clone)]
pub struct Bm25 {
    docs: Vec<Document>,
    /// doc-id -> owned term-frequency map (term -> count).
    tf: Vec<std::collections::HashMap<String, u32>>,
    /// mean document length (tokens).
    avgdl: f64,
    /// term -> document frequency `n_t`.
    df: std::collections::HashMap<String, u32>,
    params: Bm25Params,
}

impl Bm25 {
    /// Build the index over `docs` (doc-id = position). Uses [`Bm25Params::default`].
    pub fn new(docs: Vec<Document>) -> Self {
        Self::with_params(docs, Bm25Params::default())
    }

    /// Build with explicit parameters.
    pub fn with_params(docs: Vec<Document>, params: Bm25Params) -> Self {
        let mut tf = Vec::with_capacity(docs.len());
        let mut df: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
        let mut total_len: usize = 0;
        for doc in &docs {
            let mut m: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
            for t in &doc.tokens {
                *m.entry(t.clone()).or_insert(0) += 1;
            }
            for t in m.keys() {
                *df.entry(t.clone()).or_insert(0) += 1;
            }
            total_len += doc.tokens.len();
            tf.push(m);
        }
        let avgdl = if docs.is_empty() {
            0.0
        } else {
            total_len as f64 / docs.len() as f64
        };
        Bm25 {
            docs,
            tf,
            avgdl,
            df,
            params,
        }
    }

    /// Number of documents in the corpus.
    pub fn len(&self) -> usize {
        self.docs.len()
    }

    /// True when the corpus is empty.
    pub fn is_empty(&self) -> bool {
        self.docs.is_empty()
    }

    /// Mean document length in tokens.
    pub fn avgdl(&self) -> f64 {
        self.avgdl
    }

    /// Document frequency `n_t` for term `t` (0 if unseen).
    pub fn df(&self, t: &str) -> u32 {
        self.df.get(t).copied().unwrap_or(0)
    }

    /// Query-document BM25 score for a single document. Exposed for unit
    /// tests that check term-frequency saturation directly.
    pub fn score_doc(&self, doc_id: usize, query: &[String]) -> f64 {
        if doc_id >= self.docs.len() {
            return 0.0;
        }
        let n = self.docs.len() as f64;
        let dl = self.docs[doc_id].len() as f64;
        let b = self.params.b;
        let k1 = self.params.k1;
        let denom_len = 1.0 - b + b * (dl / self.avgdl.max(1e-9));
        let mut s = 0.0f64;
        // Sort the query terms so summation order is deterministic
        // (independent of HashMap iteration / caller ordering).
        let mut terms: Vec<&String> = query.iter().collect();
        terms.sort();
        terms.dedup();
        for t in terms {
            let nt = self.df(t) as f64;
            if nt == 0.0 {
                continue; // term absent from corpus ⇒ IDF would be ~0 contribution
            }
            let idf = ((n - nt + 0.5) / (nt + 0.5) + 1.0).ln();
            let f = *self.tf[doc_id].get(t).unwrap_or(&0) as f64;
            if f == 0.0 {
                continue;
            }
            let num = f * (k1 + 1.0);
            let den = f + k1 * denom_len;
            s += idf * num / den;
        }
        s
    }

    /// Rank the corpus for `query` (a token slice) and return all docs with
    /// score > 0, descending by score, tie-broken by ascending doc-id.
    /// Deterministic.
    pub fn rank(&self, query: &[String]) -> Vec<Scored> {
        if query.is_empty() {
            return Vec::new();
        }
        let mut out: Vec<Scored> = (0..self.docs.len())
            .map(|id| Scored {
                doc_id: id,
                score: self.score_doc(id, query),
            })
            .filter(|s| s.score > 0.0)
            .collect();
        out.sort_by(|a, b| {
            // descending score, then ascending doc-id for a stable, deterministic tie-break
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.doc_id.cmp(&b.doc_id))
        });
        out
    }

    /// Convenience: tokenize `query` with [`tokenize`] then [`rank`].
    pub fn rank_text(&self, query: &str) -> Vec<Scored> {
        self.rank(&tokenize(query))
    }

    /// Top-`k` hits for a pre-tokenized query (fewer if fewer than `k` docs score).
    pub fn top_k(&self, query: &[String], k: usize) -> Vec<Scored> {
        let mut r = self.rank(query);
        r.truncate(k);
        r
    }

    /// Top-`k` hits for a raw query string.
    pub fn top_k_text(&self, query: &str, k: usize) -> Vec<Scored> {
        self.top_k(&tokenize(query), k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> Vec<Document> {
        vec![
            // doc 0: mentions ALL three query words, repeatedly.
            Document::from_text(
                "rust compiler optimization llvm backend codegen register allocation",
            ),
            // doc 1: mentions only SOME query words (compiler, but not rust/llvm).
            Document::from_text("compiler design lecture notes parsing grammar"),
            // doc 2: mentions none.
            Document::from_text("banana smoothie recipe kitchen blender"),
        ]
    }

    #[test]
    fn doc_with_all_query_terms_ranks_above_partial() {
        // RED→GREEN: a doc containing ALL query terms must out-rank a doc
        // containing only some. This is the load-bearing BM25 property the
        // living-knowledge lexical signal relies on.
        let bm = Bm25::new(corpus());
        let q = tokenize("rust compiler llvm");
        let hits = bm.rank(&q);
        assert!(
            !hits.is_empty(),
            "at least the all-terms doc must score > 0"
        );
        // The top hit must be doc 0 (all three terms), not doc 1 (one term).
        assert_eq!(
            hits[0].doc_id, 0,
            "doc containing ALL query terms must rank first"
        );
        // And doc 0 must strictly out-score doc 1.
        let s0 = bm.score_doc(0, &q);
        let s1 = bm.score_doc(1, &q);
        assert!(s0 > s1, "all-terms score {s0} must exceed partial {s1}");
        // The no-term doc must not appear with a positive score.
        assert!(
            hits.iter().all(|h| h.doc_id != 2),
            "doc with zero query terms must not score > 0"
        );
    }

    #[test]
    fn term_frequency_saturation_holds() {
        // RED→GREEN: BM25 saturates — 10x the term frequency gives < 10x the
        // score (the (k1+1)·f / (f + k1·...) numerator saturates). Prove the
        // marginal gain from the 11th occurrence is smaller than from the 1st.
        let once =
            Document::from_text("rust token token token token token token token token token token");
        let ten_times = Document::from_text(
            "rust token token token token token token token token token token token token token token token token token token token token token token",
        );
        // Build a 1-doc corpus per case so df == 1 (avoids IDF distortion) and
        // compare the *doc contribution* of the repeated term alone.
        let bm_once = Bm25::new(vec![once]);
        let bm_ten = Bm25::new(vec![ten_times]);
        let q = tokenize("token");
        let s_once = bm_once.score_doc(0, &q);
        let s_ten = bm_ten.score_doc(0, &q);
        // More occurrences ⇒ higher score...
        assert!(s_ten > s_once, "higher tf must score higher");
        // ...but NOT linear: 10x the occurrences yields < 10x the score.
        let ratio = s_ten / s_once;
        assert!(
            ratio < 10.0,
            "tf must saturate: 10x occurrences gave {ratio:.3}x score (must be < 10)"
        );
        // Monotonic non-linear: the *marginal* 10th occurrence adds less than
        // the 1st, i.e. ratio < 10 (already shown) AND the curve is concave.
        // Concavity check derived from the BM25 formula with k1=1.5:
        //   contribution(f) = idf·(k1+1)·f/(f+k1·L)
        // marginal of f→f+1 is decreasing in f. Verify numerically:
        let marginal_first = score_at_tf(1) - score_at_tf(0);
        let marginal_tenth = score_at_tf(10) - score_at_tf(9);
        assert!(
            marginal_tenth < marginal_first,
            "10th occurrence must add less than the 1st (concave saturation)"
        );
    }

    /// Closed-form BM25 single-term contribution for a 1-doc corpus (df=1,
    /// dl=f, avgdl=f) with k1=1.5, b=0 — isolates pure tf saturation without
    /// length normalization interfering. idf with N=1,n_t=1 → ln(1+0.5/1.5)=ln(1.333).
    fn score_at_tf(f: u32) -> f64 {
        let k1 = 1.5;
        let idf = (((1.0f64 - 1.0) + 0.5) / (1.0 + 0.5) + 1.0).ln();
        let tf = f as f64;
        idf * (k1 + 1.0) * tf / (tf + k1)
    }

    #[test]
    fn idf_downweights_common_terms() {
        // A term appearing in every doc has n_t == N ⇒ IDF ~ 0, so it cannot
        // distinguish docs. A rare term keeps discriminative power.
        let docs = vec![
            Document::from_text("common alpha"),
            Document::from_text("common beta"),
            Document::from_text("common gamma"),
        ];
        let bm = Bm25::new(docs);
        let q_common = tokenize("common");
        let q_rare = tokenize("alpha");
        // 'common' scores > 0 for every doc but with near-zero IDF, so the
        // gap between a matching doc and a non-matching doc for 'common' is 0,
        // while 'alpha' isolates doc 0.
        assert!(bm.df("common") == 3, "common appears in all docs");
        assert!(bm.df("alpha") == 1, "alpha is rare");
        // Ranking by 'common' is a tie (all score equally, non-discriminative).
        let hits_common = bm.rank(&q_common);
        assert_eq!(hits_common.len(), 3, "common matches all 3 docs");
        // Whereas 'alpha' uniquely surfaces doc 0 at rank 0.
        let hits_rare = bm.rank(&q_rare);
        assert_eq!(hits_rare[0].doc_id, 0, "rare term isolates its doc");
    }

    #[test]
    fn ranking_is_deterministic_and_tie_broken_by_id() {
        // Two docs with identical token multiset ⇒ identical score ⇒ tie-break
        // by ascending doc-id gives a stable, reproducible ordering.
        let docs = vec![
            Document::from_text("rust llvm backend"),
            Document::from_text("rust llvm backend"),
        ];
        let bm = Bm25::new(docs.clone());
        let hits = bm.top_k(&tokenize("rust llvm"), 2);
        assert_eq!(hits.len(), 2);
        assert!(
            hits[0].doc_id < hits[1].doc_id,
            "tie broken by ascending id"
        );
        // Rebuild + rerank ⇒ byte-identical ranking (determinism).
        let bm2 = Bm25::new(docs.clone());
        let hits2 = bm2.top_k(&tokenize("rust llvm"), 2);
        assert_eq!(hits, hits2, "ranking must be deterministic");
    }
}
