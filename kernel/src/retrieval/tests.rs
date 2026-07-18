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
    assert_eq!(
        TrigramIndex::new(FIXTURE).postings_len(),
        idx.postings_len()
    );
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
    assert!(
        cand >= matches.len(),
        "candidates are a superset of matches"
    );
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
        n,
        cand,
        matches.len(),
        factor
    );
    assert_eq!(matches, vec![1234]);
    assert!(cand <= 1, "unique marker ⇒ ≤1 candidate");
    assert!(
        factor >= 50.0,
        "expected ≥50× reduction, got {:.1}x",
        factor
    );
}

#[test]
fn full_suite_parity_with_linear_oracle() {
    // Exhaustively check a battery of needles against the linear oracle.
    let idx = build();
    let needles = [
        "md",
        "note",
        "schema",
        "push",
        "tier",
        "zstd",
        "vsa",
        "entropy",
        "field",
        "divergence",
        "coherence",
        "ttrain",
        "pq",
        "cdc",
        "graph",
        "index",
        "decay",
        "recall",
        "fusion",
        "gate",
        "ledger",
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

// ── W18 — living_knowledge Rust adapter as PRIMARY recall source ──
// Deterministic recall@k integration test. Does NOT depend on the deleted
// 324-file JS corpus: it asserts the wired `retrieval::recall_at_k` PRIMARY
// recall API returns the expected top-k over the kernel's own deterministic
// fixture corpus (recall@5 == 1.0 over a hand-verified oracle). 0 JS.

#[test]
fn w18_primary_recall_at_k_returns_expected_top_k() {
    // GREEN gate: call the wired PRIMARY recall API and assert the top-k
    // matches a known deterministic fixture. We verify on the kernel's own
    // FIXTURE_CORPUS (the PRIMARY source) using hand-verified queries whose
    // relevant doc is doc-id 0..=11; we check that recall_at_k surfaces the
    // correct doc at rank 1 with a positive score (deterministic, no JS).
    let hits = super::recall::recall_at_k("how is the order total calculated", 5);
    assert!(!hits.is_empty(), "recall must return at least one hit");
    assert_eq!(hits.len(), 5, "recall@5 must return exactly 5 ranked hits");
    // PrimaryRecall ids are `lk:<position>`; doc 0 ("pricing") must be rank 1.
    assert_eq!(hits[0].0, "lk:0", "primary recall must surface lk:0 first");
    assert!(hits[0].1 > 0.0, "top hit must carry a positive BM25 score");
    // Scores must be in non-increasing order (descending ranking).
    for w in hits.windows(2) {
        assert!(
            w[0].1 >= w[1].1,
            "recall ranking must be non-increasing in score"
        );
    }
}

#[test]
fn w18_primary_recall_at_5_is_one_point_zero_on_deterministic_fixture() {
    // recall@5 == 1.0 over a hand-verified oracle derived from the kernel's
    // PRIMARY corpus, certified by the kernel's own `csr::recall_at_k`. This
    // is the headline property the blueprint requires (deterministic, 0 JS).
    use crate::csr::recall_at_k;
    const K: usize = 5;
    let oracle: Vec<(&str, usize)> = vec![
        ("how is the order total calculated", 0),
        ("when does the package get delivered", 1),
        ("can i get my money back", 2),
        ("where are product prices defined", 3),
        ("how does exact substring search work", 4),
        ("what ranks documents by word frequency", 5),
        ("which algorithm measures node importance in a graph", 6),
        ("how do related memories get surfaced", 7),
        ("why do old notes lose weight", 8),
        ("how is stored memory made smaller", 9),
        ("how are embeddings compressed", 10),
        ("what tracks information gain of improvements", 11),
    ];
    let mut total = 0.0f64;
    let mut successes = 0u64;
    for (q, relevant) in &oracle {
        let hits = super::recall::recall_at_k(q, K);
        // Encode the ranking as a doc-id-indexed score vector for certify.
        let mut scores = vec![0.0f64; 12];
        for (pos, (id, _score)) in hits.iter().enumerate() {
            let d: usize = id.strip_prefix("lk:").unwrap().parse().unwrap();
            scores[d] = (hits.len() - pos) as f64;
        }
        let r = recall_at_k(&scores, &[*relevant], K);
        assert_eq!(
            r, 1.0,
            "W18 primary recall: query '{}' must recall relevant doc lk:{} in top-{}",
            q, relevant, K
        );
        total += r;
        // Each query has exactly one relevant doc ⇒ a Bernoulli trial; r==1.0 is a
        // success. mean recall@5 == 1.0 therefore means successes/n == 12/12.
        if r == 1.0 {
            successes += 1;
        }
    }
    let n = oracle.len() as u64;
    let mean = total / oracle.len() as f64;
    assert_eq!(mean, 1.0, "mean recall@5 over the W18 oracle must be 1.0");

    // E2 (§4 criterion 3 / D4): the bare `1.0` is unfalsifiable as displayed — a
    // reader cannot tell whether it rests on 12 trials or 12 000. Attach the honest
    // 95% Wilson lower bound. At p̂=1.0 the Wald interval degenerates to [1,1]; Wilson
    // does not — for k=n it is n/(n+z²), computed here (NOT hardcoded from the doc).
    assert_eq!(
        successes, 12,
        "12/12 Bernoulli successes back the 1.0 headline"
    );
    let (lo, hi) = crate::stats::wilson_interval(successes, n, 1.96);
    let closed = n as f64 / (n as f64 + 1.96 * 1.96);
    assert!(
        (lo - closed).abs() < 1e-12,
        "12/12 Wilson lower must equal n/(n+z²)={closed}"
    );
    assert!(
        (lo - 0.7575).abs() < 1e-4,
        "recall@5 12-query Wilson 95% lower bound must be ≈0.7575, got {lo}"
    );
    assert!(
        (hi - 1.0).abs() < 1e-12,
        "Wilson upper clamps to 1.0 at full success"
    );
    // A failing query (11/12) MUST move the floor — the interval reacts to evidence,
    // it is not an assertable constant. (§4 criterion 3, D4.)
    let (lo_miss, _) = crate::stats::wilson_interval(11, n, 1.96);
    assert!(
        lo_miss < lo,
        "an 11/12 result must drop the Wilson floor below {lo}"
    );
    // The reported string now carries the interval, not a bare 1.0.
    println!(
        "[living_knowledge] recall@5 = {:.3}  [Wilson 95% {:.4}, {:.4}]  n={} (successes={})",
        mean, lo, hi, n, successes
    );
}
