//! RED→GREEN tests for the L0 trigram index (blueprint M1 / W1-3).
//!
//! RED semantics: a test that asserts the *verify* step is doing real work — if
//! we returned raw candidates (superset), the `matches == linear-oracle`
//! assertions would FAIL. GREEN: after the exact verify, matches equal a
//! linear-scan oracle and false positives == 0.

use super::fixtures::{synthetic_corpus, FIXTURE};
use super::index::TrigramIndex;

fn build() -> TrigramIndex {
    TrigramIndex::new(FIXTURE)
}

/// Naive linear-scan oracle — the ground truth the index must reproduce
/// exactly (the honest "grep every doc" baseline from the manifest).
fn linear_literal(docs: &[&str], needle: &str) -> Vec<u32> {
    docs.iter()
        .enumerate()
        .filter(|(_, d)| d.contains(needle))
        .map(|(i, _)| i as u32)
        .collect()
}

#[test]
fn fixture_index_builds_with_20_docs() {
    let idx = build();
    assert_eq!(idx.len(), 20);
    assert!(idx.postings_len() > 0, "index must contain trigrams");
    // Determinism: a rebuild yields the identical literal-keyed posting count.
    assert_eq!(TrigramIndex::new(FIXTURE).postings_len(), idx.postings_len());
}

#[test]
fn literal_query_exact_and_zero_false_positives() {
    let idx = build();
    // "decay" appears only in "note-salience-decay.md" (doc 2).
    let got = idx.query_literal("decay");
    let oracle = linear_literal(FIXTURE, "decay");
    assert_eq!(got, oracle, "matches must equal the linear-scan oracle");
    assert_eq!(got, vec![2]);
    // 0 false positives: every returned doc really contains the needle.
    for &d in &got {
        assert!(FIXTURE[d as usize].contains("decay"));
    }
}

#[test]
fn candidate_reduction_strictly_smaller_than_corpus() {
    let idx = build();
    let needle = "recall";
    let cand = idx.candidate_count_literal(needle);
    let matches = idx.query_literal(needle);
    assert!(cand <= 20, "candidates must be ≤ corpus size");
    assert!(
        matches.len() <= cand,
        "verify can only shrink the candidate set (0 false positives)"
    );
    // A rarer needle must reduce candidates further and hit exactly one doc.
    let rare = "quantization";
    let c_rare = idx.candidate_count_literal(rare);
    assert!(c_rare < 20, "rare needle must reduce candidates");
    assert_eq!(idx.query_literal(rare), vec![18]);
}

#[test]
fn verify_filters_overbroad_candidates_no_false_positives() {
    let idx = build();
    // "trigram-index" shares trigrams with others, but verify must reject
    // docs that merely share a trigram yet lack the full needle.
    let needle = "trigram-index";
    let cand = idx.candidate_count_literal(needle);
    let matches = idx.query_literal(needle);
    assert!(cand >= matches.len(), "candidates are a superset of matches");
    // matches == oracle, i.e. exactly the docs containing the needle.
    assert_eq!(matches, linear_literal(FIXTURE, needle));
    assert_eq!(matches, vec![5]);
}

#[test]
fn regex_query_exact_and_zero_false_positives() {
    let idx = build();
    // Pattern with a literal trigram run → candidate reduction + regex verify.
    let pat = r"note-.*-recall";
    let got = idx.query_regex(pat).expect("valid regex");
    let re = regex::Regex::new(pat).unwrap();
    let oracle: Vec<u32> = FIXTURE
        .iter()
        .enumerate()
        .filter(|(_, d)| re.is_match(d))
        .map(|(i, _)| i as u32)
        .collect();
    assert_eq!(got, oracle);
    assert_eq!(got, vec![7]); // note-heat-kernel-recall.md
    for &d in &got {
        assert!(re.is_match(FIXTURE[d as usize]));
    }
}

#[test]
fn candidate_reduction_factor_on_synthetic_corpus() {
    // N docs, each with a UNIQUE 12-char marker ⇒ querying the marker must
    // reduce candidates to ~1 (the planted doc) ⇒ ~N× reduction.
    let n = 2000usize;
    let corpus = synthetic_corpus(n);
    let docs: Vec<&str> = corpus.iter().map(|s| s.as_str()).collect();
    let idx = TrigramIndex::new(&docs);
    let marker = corpus[1234]
        .trim_start_matches("boilerplate-prefix-")
        .trim_end_matches("-suffix-boilerplate");
    let cand = idx.candidate_count_literal(marker);
    let matches = idx.query_literal(marker);
    let factor = n as f64 / cand.max(1) as f64;
    println!(
        "[retrieval::bench] synthetic n={} candidates={} matches={} reduction={:.1}x",
        n, cand, matches.len(), factor
    );
    assert_eq!(matches, vec![1234]);
    assert!(cand <= 1, "unique marker ⇒ ≤1 candidate");
    assert!(factor >= 50.0, "expected ≥50× reduction, got {:.1}x", factor);
}

#[test]
fn full_suite_parity_with_linear_oracle() {
    // Exhaustively check a battery of needles against the linear oracle.
    let idx = build();
    let needles = [
        "md", "note", "schema", "push", "tier", "zstd", "vsa", "entropy", "field",
        "divergence", "coherence", "ttrain", "pq", "cdc", "graph", "index", "decay",
        "recall", "fusion", "gate", "ledger",
    ];
    for nd in needles {
        assert_eq!(
            idx.query_literal(nd),
            linear_literal(FIXTURE, nd),
            "needle '{}' must match the linear oracle exactly",
            nd
        );
    }
}
