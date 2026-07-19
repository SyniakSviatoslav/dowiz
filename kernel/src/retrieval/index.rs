//! Deterministic trigram inverted index + exact verify (M1 / L0).
//!
//! Byte-level 3-grams (not char-level) so indexing is deterministic over UTF-8
//! and cheap to hash/compare. Living-memory note names are ASCII, so byte
//! trigrams are exact.

use std::collections::HashMap;

/// A 3-byte key. `[u8;3]` is `Copy` + `Hash` + `Ord` ⇒ deterministic map keys.
pub type Trigram = [u8; 3];

/// All trigrams of `s` (contiguous byte windows of length 3).
/// Empty if `s.len() < 3`.
pub fn trigrams(s: &str) -> Vec<Trigram> {
    let b = s.as_bytes();
    if b.len() < 3 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(b.len() - 2);
    for w in 0..b.len() - 2 {
        out.push([b[w], b[w + 1], b[w + 2]]);
    }
    out
}

/// Extract candidate trigrams from a *restricted wildcard pattern* by pulling
/// literal runs of length ≥ 3. Metacharacters (`* + ? ( ) [ ] { } | ^ $ . \`)
/// end a run (a conservative superset — any meta byte terminates a run), so only
/// guaranteed-literal bytes become index keys. A doc must contain ALL returned
/// trigrams to possibly match ⇒ the candidate set is a safe superset of the
/// true matches (verify prunes the rest → 0 false positives).
pub fn literal_trigrams(pattern: &str) -> Vec<Trigram> {
    let b = pattern.as_bytes();
    let mut runs: Vec<Vec<u8>> = Vec::new();
    let mut cur: Vec<u8> = Vec::new();
    for &c in b {
        let is_meta = matches!(
            c,
            b'*' | b'+'
                | b'?'
                | b'('
                | b')'
                | b'['
                | b']'
                | b'{'
                | b'}'
                | b'|'
                | b'^'
                | b'$'
                | b'.'
                | b'\\'
        );
        if is_meta {
            if !cur.is_empty() {
                runs.push(std::mem::take(&mut cur));
            }
        } else {
            cur.push(c);
        }
    }
    if !cur.is_empty() {
        runs.push(cur);
    }
    let mut out: Vec<Trigram> = Vec::new();
    for run in &runs {
        if run.len() >= 3 {
            for w in 0..run.len() - 2 {
                out.push([run[w], run[w + 1], run[w + 2]]);
            }
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// Deterministic trigram inverted index over a corpus of `&str` documents.
///
/// Postings are literal-keyed and store **unique, sorted** doc-ids per trigram.
/// No Bloom filter, no compression ⇒ bitwise reproducibility (blueprint §3).
#[derive(Debug, Clone)]
pub struct TrigramIndex {
    docs: Vec<String>,
    /// trigram -> sorted-unique doc-ids that contain it.
    postings: HashMap<Trigram, Vec<u32>>,
}

impl TrigramIndex {
    /// Build the index over `docs` (doc-id = position in `docs`).
    pub fn new(docs: &[&str]) -> Self {
        let mut postings: HashMap<Trigram, Vec<u32>> = HashMap::new();
        for (id, doc) in docs.iter().enumerate() {
            let id = id as u32;
            let mut seen = trigrams(doc);
            seen.sort_unstable();
            seen.dedup();
            for t in seen {
                postings.entry(t).or_default().push(id);
            }
        }
        // Dedupe + sort each posting list → literal-keyed determinism.
        for v in postings.values_mut() {
            v.sort_unstable();
            v.dedup();
        }
        TrigramIndex {
            docs: docs.iter().map(|s| s.to_string()).collect(),
            postings,
        }
    }

    /// Number of documents in the corpus.
    pub fn len(&self) -> usize {
        self.docs.len()
    }

    /// Number of distinct trigrams indexed (the literal-keyed posting count).
    pub fn postings_len(&self) -> usize {
        self.postings.len()
    }

    /// Candidates = docs containing ALL `trigs` (set intersection).
    /// * empty trigram set ⇒ all docs (no reduction possible);
    /// * a missing trigram ⇒ zero candidates (match impossible).
    ///
    /// Counting is order-independent ⇒ bitwise-deterministic regardless of
    /// HashMap iteration order.
    fn candidates(&self, trigs: &[Trigram]) -> Vec<u32> {
        if trigs.is_empty() {
            return (0..self.docs.len() as u32).collect();
        }
        let mut counts: HashMap<u32, u32> = HashMap::new();
        for t in trigs {
            match self.postings.get(t) {
                None => return Vec::new(),
                Some(post) => {
                    for &d in post {
                        *counts.entry(d).or_insert(0) += 1;
                    }
                }
            }
        }
        let k = trigs.len() as u32;
        let mut cand: Vec<u32> = counts
            .into_iter()
            .filter(|(_, c)| *c == k)
            .map(|(d, _)| d)
            .collect();
        cand.sort_unstable();
        cand
    }

    /// Exact literal substring query. Returns doc-ids that **contain** `needle`,
    /// narrowed by trigram intersection then verified by `str::contains`
    /// (exact ⇒ 0 false positives).
    pub fn query_literal(&self, needle: &str) -> Vec<u32> {
        let cand = self.candidates(&trigrams(needle));
        cand.into_iter()
            .filter(|&d| self.docs[d as usize].contains(needle))
            .collect()
    }

    /// Raw candidate count for a literal needle — used by the benchmark to
    /// measure reduction vs a full linear scan (candidate == corpus size).
    pub fn candidate_count_literal(&self, needle: &str) -> usize {
        self.candidates(&trigrams(needle)).len()
    }

    /// Restricted-pattern query (item 5 replacement for the retired `query_regex`).
    /// Candidates are narrowed by literal trigrams extracted from the *pattern*,
    /// then each candidate is verified by the kernel-owned `Pattern` matcher
    /// ({literal, `.`, `.*`}, unanchored contains-match ⇒ 0 false positives).
    /// Falls back to scanning all docs when the pattern yields no literal
    /// trigrams. Unsupported metacharacters are rejected (typed `PatternError`,
    /// never a silent wrong answer) — degrade-closed.
    pub fn query_pattern(&self, pattern: &str) -> Result<Vec<u32>, super::pattern::PatternError> {
        let compiled = super::pattern::Pattern::compile(pattern)?;
        let cand = self.candidates(&literal_trigrams(pattern));
        Ok(cand
            .into_iter()
            .filter(|&d| compiled.is_match(&self.docs[d as usize]))
            .collect())
    }

    /// Raw candidate count for a restricted pattern (literal-run extraction).
    /// Name-only successor to `candidate_count_regex` — identical logic (never
    /// touched the regex crate).
    pub fn candidate_count_pattern(&self, pattern: &str) -> usize {
        self.candidates(&literal_trigrams(pattern)).len()
    }
}
