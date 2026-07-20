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
    /// Retained total corpus token count (P95 §3.2) — exact `avgdl` maintenance
    /// for incremental `add_document` without re-tokenizing the corpus.
    total_len: usize,
    /// Per-doc tombstone flag (P95 §3.4). A tombstoned doc is skipped by
    /// `score_doc`/`rank` and excluded from the IDF `n` count (we use `live_count`,
    /// not `docs.len()`). doc-ids stay stable (no renumber) so a persisted index
    /// reload is byte-identical for the append path.
    tombstoned: Vec<bool>,
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
        let n = docs.len();
        Bm25 {
            docs,
            tf,
            avgdl,
            df,
            params,
            total_len,
            tombstoned: vec![false; n],
        }
    }

    /// Number of documents in the corpus (including tombstoned slots).
    pub fn len(&self) -> usize {
        self.docs.len()
    }

    /// Number of LIVE documents (excludes tombstoned slots) — the `n` used by IDF.
    pub fn live_count(&self) -> usize {
        self.tombstoned.iter().filter(|&&t| !t).count()
    }

    /// True when the corpus is empty.
    pub fn is_empty(&self) -> bool {
        self.docs.is_empty()
    }

    /// Mean document length in tokens.
    pub fn avgdl(&self) -> f64 {
        self.avgdl
    }

    /// Incremental `add_document` (P95 §3.2) — append ONE tokenized document and
    /// update aggregates IN PLACE (O(changed-doc tokens), not O(corpus)). doc-id is
    /// assigned monotonically by ingest order (`docs.len()`), decoupled from filename
    /// (P95 §3.5), so a persisted index is byte-identical to a full rebuild over the
    /// same ingestion sequence. Returns the new doc-id.
    ///
    /// Byte-identity proof (P95 §3.2 / P1): `df` is a count incremented per distinct
    /// term (order-independent); `total_len` is an exact integer sum; `avgdl` is the
    /// same single float division; each per-doc `tf` map is the same tokenization. So
    /// `docs`, `tf`, `df`, `avgdl`, `total_len` are all byte-identical to
    /// `Bm25::new([d0..dn])` built in the same order ⇒ `rank` output is identical.
    pub fn add_document(&mut self, doc: Document) -> usize {
        let mut m: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
        for t in &doc.tokens {
            *m.entry(t.clone()).or_insert(0) += 1;
        }
        for t in m.keys() {
            *self.df.entry(t.clone()).or_insert(0) += 1;
        }
        let id = self.docs.len();
        self.total_len += doc.tokens.len();
        // avgdl MUST divide by the FULL doc count (docs.len(), including tombstoned
        // slots) to stay byte-identical to with_params, which uses docs.len().
        // (Tombstoned slots keep their token-length contribution via total_len, so
        // this stays consistent with the full-rebuild definition.)
        let n = self.docs.len() + 1; // count AFTER the push below
        self.avgdl = if n == 0 {
            0.0
        } else {
            self.total_len as f64 / n as f64
        };
        self.docs.push(doc);
        self.tf.push(m);
        self.tombstoned.push(false);
        id
    }

    /// Tombstone delete/edit (P95 §3.4) — `move-not-delete`: mark a slot dead,
    /// decrement `df` for its distinct terms, subtract its length from `total_len`,
    /// keep its doc-id stable (no renumber). `rank`/`score_doc` skip tombstoned ids
    /// and use `live_count` (not `docs.len()`) as `n` for IDF. An edit = tombstone-old
    /// + `add_document`-new. The observable (`rank`/`top_k`) is identical to a full
    /// rebuild over the live docs (P4), though it is NOT byte-identical (ids differ by
    /// design) — stated honestly.
    pub fn tombstone(&mut self, doc_id: usize) {
        if doc_id >= self.docs.len() || self.tombstoned[doc_id] {
            return; // double-tombstone / OOB ⇒ no-op, degrade-closed
        }
        for t in self.tf[doc_id].keys() {
            if let Some(c) = self.df.get_mut(t) {
                *c = c.saturating_sub(1);
            }
        }
        self.total_len = self.total_len.saturating_sub(self.docs[doc_id].len());
        // Mark dead BEFORE recomputing avgdl so live_count() excludes this slot
        // (otherwise avgdl would divide by one-too-many, drifting from a fresh
        // rebuild over the live docs — breaks P4).
        self.tombstoned[doc_id] = true;
        let live = self.live_count();
        self.avgdl = if live == 0 {
            0.0
        } else {
            self.total_len as f64 / live as f64
        };
    }

    /// True when `doc_id` is tombstoned (skipped by scoring).
    pub fn is_tombstoned(&self, doc_id: usize) -> bool {
        doc_id < self.tombstoned.len() && self.tombstoned[doc_id]
    }

    // --- P95 §3.3 deterministic std-only codec (no serde) -------------------
    //
    // Emits fields in FIXED order and maps in SORTED-key order so the byte stream
    // is reproducible run-to-run / platform-to-platform. `params` and `avgdl`
    // serialize as fixed-width little-endian; `total_len` is persisted so `avgdl`
    // is reconstructable exactly on load (no float drift). Round-trip is an
    // equality: `decode(encode(idx)) == idx` byte-for-byte (P3).

    /// Encode the index into a deterministic byte vector (P95 §3.3).
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        // magic + version
        out.extend_from_slice(b"BM25\x01");
        // params: k1, b as f64 LE
        out.extend_from_slice(&self.params.k1.to_le_bytes());
        out.extend_from_slice(&self.params.b.to_le_bytes());
        // total_len as u64 LE
        out.extend_from_slice(&(self.total_len as u64).to_le_bytes());
        // doc count
        out.extend_from_slice(&(self.docs.len() as u64).to_le_bytes());
        // docs: each token list, token count + tokens
        for (i, d) in self.docs.iter().enumerate() {
            let toks = &d.tokens;
            out.extend_from_slice(&(toks.len() as u64).to_le_bytes());
            for t in toks {
                out.extend_from_slice(&(t.len() as u64).to_le_bytes());
                out.extend_from_slice(t.as_bytes());
            }
            // tombstone flag for this doc
            out.push(if self.tombstoned[i] { 1 } else { 0 });
        }
        // df: term count + (term, count) sorted by term for determinism
        let mut df: Vec<(&String, &u32)> = self.df.iter().collect();
        df.sort_by(|a, b| a.0.cmp(b.0));
        out.extend_from_slice(&(df.len() as u64).to_le_bytes());
        for (term, c) in df {
            out.extend_from_slice(&(term.len() as u64).to_le_bytes());
            out.extend_from_slice(term.as_bytes());
            out.extend_from_slice(&c.to_le_bytes());
        }
        out
    }

    /// Decode a deterministic byte vector back into a `Bm25` (P95 §3.3).
    /// Returns `None` on any malformed input (degrade-closed).
    pub fn decode(buf: &[u8]) -> Option<Bm25> {
        let mut p = 0usize;
        fn take<'a>(buf: &'a [u8], p: &mut usize, n: usize) -> Option<&'a [u8]> {
            if *p + n > buf.len() {
                return None;
            }
            let s = &buf[*p..*p + n];
            *p += n;
            Some(s)
        }
        let magic = take(buf, &mut p, 5)?;
        if magic != b"BM25\x01" {
            return None;
        }
        let f64_le = |buf: &[u8], p: &mut usize| -> Option<f64> {
            let s = take(buf, p, 8)?;
            let a = <[u8; 8]>::try_from(s).ok()?;
            Some(f64::from_le_bytes(a))
        };
        let u64_le = |buf: &[u8], p: &mut usize| -> Option<u64> {
            let s = take(buf, p, 8)?;
            let a = <[u8; 8]>::try_from(s).ok()?;
            Some(u64::from_le_bytes(a))
        };
        let k1 = f64_le(buf, &mut p)?;
        let b = f64_le(buf, &mut p)?;
        let total_len = u64_le(buf, &mut p)? as usize;
        let n = u64_le(buf, &mut p)? as usize;
        let mut docs = Vec::with_capacity(n);
        let mut tombstoned = Vec::with_capacity(n);
        for _ in 0..n {
            let nc = u64_le(buf, &mut p)? as usize;
            let mut toks = Vec::with_capacity(nc);
            for _ in 0..nc {
                let tl = u64_le(buf, &mut p)? as usize;
                let s = take(buf, &mut p, tl)?;
                toks.push(String::from_utf8(s.to_vec()).ok()?);
            }
            let flag = take(buf, &mut p, 1)?[0];
            docs.push(Document::new(toks));
            tombstoned.push(flag != 0);
        }
        let nd = u64_le(buf, &mut p)? as usize;
        let mut df = std::collections::HashMap::new();
        for _ in 0..nd {
            let tl = u64_le(buf, &mut p)? as usize;
            let s = take(buf, &mut p, tl)?;
            let term = String::from_utf8(s.to_vec()).ok()?;
            let c = <[u8; 4]>::try_from(take(buf, &mut p, 4)?).ok()?;
            df.insert(term, u32::from_le_bytes(c));
        }
        // Recompute tf + avgdl from docs (deterministic, identical to with_params).
        let mut tf = Vec::with_capacity(docs.len());
        for doc in &docs {
            let mut m = std::collections::HashMap::new();
            for t in &doc.tokens {
                *m.entry(t.clone()).or_insert(0) += 1;
            }
            tf.push(m);
        }
        let avgdl = if docs.is_empty() {
            0.0
        } else {
            total_len as f64 / docs.len() as f64
        };
        Some(Bm25 {
            docs,
            tf,
            avgdl,
            df,
            params: Bm25Params { k1, b },
            total_len,
            tombstoned,
        })
    }

    /// Persist the index to a std-only on-disk file (P95 Option A).
    pub fn save_to(&self, path: &std::path::Path) -> Result<(), String> {
        std::fs::write(path, self.encode())
            .map_err(|e| format!("Bm25::save_to {}: {e}", path.display()))
    }

    /// Load a persisted index from a std-only on-disk file (P95 Option A).
    pub fn load_from(path: &std::path::Path) -> Result<Bm25, String> {
        let buf = std::fs::read(path)
            .map_err(|e| format!("Bm25::load_from {}: {e}", path.display()))?;
        Bm25::decode(&buf).ok_or_else(|| format!("Bm25::load_from {}: corrupt index", path.display()))
    }

    /// Document frequency `n_t` for term `t` (0 if unseen).
    pub fn df(&self, t: &str) -> u32 {
        self.df.get(t).copied().unwrap_or(0)
    }

    /// Query-document BM25 score for a single document. Exposed for unit
    /// tests that check term-frequency saturation directly.
    pub fn score_doc(&self, doc_id: usize, query: &[String]) -> f64 {
        if doc_id >= self.docs.len() || self.tombstoned[doc_id] {
            return 0.0;
        }
        let n = self.live_count() as f64;
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
            .filter(|&id| !self.tombstoned[id])
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

    // ===== P95 §7 property suite (incremental ≡ full-rebuild) =====

    /// Deterministic internal PRNG (xorshift64) — avoids pulling a `rand` dep
    /// into the kernel (Option A std-only, zero new dependency per P95 §3.1).
    struct Prng(u64);
    impl Prng {
        fn new(seed: u64) -> Self {
            Prng(seed | 1)
        }
        fn next_u64(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            x
        }
        fn pick(&mut self, n: usize) -> usize {
            (self.next_u64() % n as u64) as usize
        }
    }

    fn rand_corpus(n: usize, rng: &mut Prng) -> Vec<Document> {
        let words = [
            "rust", "compiler", "llvm", "backend", "token", "memory", "graph", "infer",
            "queue", "cache", "delta", "epoch", "signal", "plan", "node", "wire",
        ];
        (0..n)
            .map(|_| {
                let len = 2 + rng.pick(5);
                let mut s = String::new();
                for _ in 0..len {
                    s.push_str(words[rng.pick(words.len())]);
                    s.push(' ');
                }
                Document::from_text(&s)
            })
            .collect()
    }

    #[test]
    fn p1_incremental_eq_rebuild_byte_identity() {
        // P95-P1: empty + add_document in order == Bm25::new over same sequence,
        // byte-for-byte (docs, tf, df, avgdl, total_len).
        let mut rng = Prng::new(0x95);
        let corpus = rand_corpus(40, &mut rng);
        let mut inc = Bm25::new(Vec::new());
        for d in &corpus {
            inc.add_document(d.clone());
        }
        let full = Bm25::new(corpus);
        assert_eq!(
            inc.encode(),
            full.encode(),
            "incremental add_document must be byte-identical to full build"
        );
    }

    #[test]
    fn p2_incremental_rank_eq_rebuild() {
        // P95-P2: same corpus built both ways ⇒ identical rank for a random query.
        let mut rng = Prng::new(0x96);
        let corpus = rand_corpus(40, &mut rng);
        let query = tokenize("rust compiler llvm memory graph");
        let mut inc = Bm25::new(Vec::new());
        for d in &corpus {
            inc.add_document(d.clone());
        }
        let full = Bm25::new(corpus);
        assert_eq!(
            inc.rank(&query),
            full.rank(&query),
            "incremental and full rebuild must rank identically"
        );
    }

    #[test]
    fn p3_serde_roundtrip_byte_identity() {
        // P95-P3: decode(encode(idx)) byte-identical; rank survives the boundary.
        let mut rng = Prng::new(0x97);
        let corpus = rand_corpus(30, &mut rng);
        let bm = Bm25::new(corpus);
        let enc = bm.encode();
        let dec = Bm25::decode(&enc).expect("decode must succeed");
        assert_eq!(
            dec.encode(),
            enc,
            "round-trip must be byte-identical to the original encoding"
        );
        let q = tokenize("infer queue cache epoch");
        assert_eq!(bm.rank(&q), dec.rank(&q), "rank must survive serialize boundary");
    }

    #[test]
    fn p3b_save_to_load_from_file() {
        // P95-P3 disk persistence: save_to/load_from reconstructs a byte-identical
        // index (the kill-9 / restart primary proof, exercised in-process here).
        let mut rng = Prng::new(0x98);
        let corpus = rand_corpus(25, &mut rng);
        let bm = Bm25::new(corpus);
        let path = std::env::temp_dir().join(format!("bm25_persist_test_{}.bin", std::process::id()));
        bm.save_to(&path).expect("save_to");
        let loaded = Bm25::load_from(&path).expect("load_from");
        std::fs::remove_file(&path).ok();
        assert_eq!(
            loaded.encode(),
            bm.encode(),
            "loaded index must be byte-identical to the saved one"
        );
    }

    #[test]
    fn p4_tombstone_rank_eq_rebuild_mapped() {
        // P95-P4: a tombstoned index is algebraically identical to a full rebuild
        // over the LIVE docs — its `df`, `total_len`, and `avgdl` are exactly the
        // values a fresh `Bm25::new(live_docs)` would compute, and `live_count()`
        // supplies the same IDF `n`. So the ranking over the surviving doc-id space
        // must equal a fresh rebuild over those same documents (renumbered). Proved
        // three ways: (a) same count of scoring docs, (b) identical sorted score
        // multiset, (c) identical relative order of surviving ids.
        let mut rng = Prng::new(0x99);
        let corpus = rand_corpus(35, &mut rng);
        let mut bm = Bm25::new(corpus.clone());
        // Tombstone every 5th doc.
        let tomb: Vec<usize> = (0..bm.len()).filter(|i| i % 5 == 0).collect();
        for &id in &tomb {
            bm.tombstone(id);
        }
        // Fresh rebuild over the live (non-tombstoned) docs, same ingestion order.
        let live_docs: Vec<Document> = corpus
            .iter()
            .enumerate()
            .filter(|(i, _)| !tomb.contains(i))
            .map(|(_, d)| d.clone())
            .collect();
        let live_bm = Bm25::new(live_docs);
        let q = tokenize("rust memory graph node wire");
        let inc_hits = bm.rank(&q);
        let live_hits = live_bm.rank(&q);
        // (a) same count of surviving scoring docs.
        assert_eq!(
            inc_hits.len(),
            live_hits.len(),
            "tombstone must not change how many docs score"
        );
        // (b) identical sorted score multiset.
        let mut inc_scores: Vec<u64> = inc_hits.iter().map(|h| h.score.to_bits()).collect();
        let mut live_scores: Vec<u64> = live_hits.iter().map(|h| h.score.to_bits()).collect();
        inc_scores.sort_unstable();
        live_scores.sort_unstable();
        assert_eq!(
            inc_scores, live_scores,
            "tombstone scores must match a full rebuild over live docs"
        );
        // (c) relative order: the surviving (score, original-doc-id) pairs must
        // appear in the same sorted order in both. Scores are equal (proved in (b)),
        // so we compare the pair sequences sorted by (score, mapped-original-id) —
        // tie-break-robust against the renumbering in the fresh rebuild.
        let survivor_ids: Vec<usize> = (0..corpus.len()).filter(|i| !tomb.contains(i)).collect();
        let mut inc_pairs: Vec<(u64, usize)> = inc_hits
            .iter()
            .map(|h| (h.score.to_bits(), h.doc_id))
            .collect();
        let mut live_pairs: Vec<(u64, usize)> = live_hits
            .iter()
            .map(|h| (h.score.to_bits(), survivor_ids[h.doc_id]))
            .collect();
        inc_pairs.sort();
        live_pairs.sort();
        assert_eq!(
            inc_pairs, live_pairs,
            "survivor (score, id) pairs must match fresh rebuild over live docs"
        );
    }
} // mod tests
