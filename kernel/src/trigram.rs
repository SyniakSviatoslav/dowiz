//! trigram.rs — deterministic n-gram (bigram + trigram) frequency extraction
//! over a token stream.
//!
//! GROWTH-SUBSTRATE PRIMITIVE (P9 / T2-β). The self-improvement loop
//! needs to surface *recurring* token patterns (tool-outcome triples, edit
//! sequences) without any ML dependency. This is the deterministic core:
//! slide a fixed window, count, rank. No float, no I/O, no external dep.
//!
//! All ranking is deterministic: ties in count break lexicographically on the
//! key, so the SAME input always yields the SAME top-k bytes.
//!
//! ZERO new dependencies (std `HashMap` only).

use std::collections::HashMap;

type Bi = [String; 2];
type Tri = [String; 3];

/// All n-gram counts over a token stream.
#[derive(Debug, Clone)]
pub struct NGrams {
    pub bigrams: HashMap<Bi, u64>,
    pub trigrams: HashMap<Tri, u64>,
    /// Number of emitted trigram windows (= max(tokens.len().saturating_sub(2), 0)).
    pub trigram_total: u64,
}

/// Count bigrams + trigrams over `tokens` via a sliding window.
/// `tokens.len() < 3` ⇒ no trigrams (trigram_total = 0), bigrams need ≥2.
pub fn count(tokens: &[&str]) -> NGrams {
    let mut bigrams: HashMap<Bi, u64> = HashMap::new();
    let mut trigrams: HashMap<Tri, u64> = HashMap::new();
    let n = tokens.len();
    for w in 0..n.saturating_sub(1) {
        let key: Bi = [tokens[w].to_string(), tokens[w + 1].to_string()];
        *bigrams.entry(key).or_insert(0) += 1;
    }
    for w in 0..n.saturating_sub(2) {
        let key: Tri = [
            tokens[w].to_string(),
            tokens[w + 1].to_string(),
            tokens[w + 2].to_string(),
        ];
        *trigrams.entry(key).or_insert(0) += 1;
    }
    NGrams {
        bigrams,
        trigrams,
        trigram_total: (n.saturating_sub(2)) as u64,
    }
}

/// Probability of a specific trigram under the empirical distribution.
/// Returns 0.0 if no trigram windows were emitted.
pub fn probability(ng: &NGrams, tri: &Tri) -> f64 {
    if ng.trigram_total == 0 {
        return 0.0;
    }
    let c = ng.trigrams.get(tri).copied().unwrap_or(0);
    c as f64 / ng.trigram_total as f64
}

/// Top-k trigrams by count. Deterministic: sort DESC by count, then ASC by
/// lexicographic key, so ties resolve identically on any machine.
pub fn most_common(ng: &NGrams, k: usize) -> Vec<(Tri, u64)> {
    let mut v: Vec<(Tri, u64)> = ng.trigrams.iter().map(|(key, c)| (key.clone(), *c)).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    v.truncate(k);
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    // Hand oracle. tokens = a b c a b c a b d  (9 tokens)
    //   trigram windows (i..i+3), i in 0..7  (9-2 = 7 windows):
    //     abc, bca, cab, abc, bca, cab, abd
    //   counts: abc×2, bca×2, cab×2, abd×1  ⇒ trigram_total = 7
    //   most_common(1) ties at count 2 ⇒ lexicographic smallest = abc
    //   P("a,b,c") = 2/7
    const TOK: &[&str] = &["a", "b", "c", "a", "b", "c", "a", "b", "d"];

    #[test]
    fn count_basics() {
        let ng = count(TOK);
        assert_eq!(ng.trigram_total, 7);
        // 9-1 = 8 bigram WINDOWS; 4 DISTINCT bigrams (ab×3, bc×2, ca×2, bd×1)
        let bigram_windows: u64 = ng.bigrams.values().sum();
        assert_eq!(bigram_windows, 8);
        assert_eq!(ng.bigrams.len(), 4);
        assert_eq!(ng.trigrams.get(&["a", "b", "c"].map(str::to_string)).copied(), Some(2));
        assert_eq!(ng.trigrams.get(&["b", "c", "a"].map(str::to_string)).copied(), Some(2));
        assert_eq!(ng.trigrams.get(&["c", "a", "b"].map(str::to_string)).copied(), Some(2));
        assert_eq!(ng.trigrams.get(&["a", "b", "d"].map(str::to_string)).copied(), Some(1));
    }

    #[test]
    fn probability_oracle() {
        let ng = count(TOK);
        let abc = ["a", "b", "c"].map(str::to_string);
        assert!((probability(&ng, &abc) - 2.0 / 7.0).abs() < 1e-12);
        // unseen trigram ⇒ 0
        let zzz = ["z", "z", "z"].map(str::to_string);
        assert_eq!(probability(&ng, &zzz), 0.0);
    }

    #[test]
    fn most_common_deterministic() {
        let ng = count(TOK);
        // tie at count 2 ⇒ lexicographically smallest key "abc" wins
        let top = most_common(&ng, 1);
        assert_eq!(top.len(), 1);
        assert_eq!(top[0].0, ["a", "b", "c"].map(str::to_string));
        assert_eq!(top[0].1, 2);
        // top-3 returns all three count-2 keys, lexicographic order
        let top3 = most_common(&ng, 3);
        assert_eq!(top3.len(), 3);
        assert_eq!(top3[0].0, ["a", "b", "c"].map(str::to_string));
        assert_eq!(top3[1].0, ["b", "c", "a"].map(str::to_string));
        assert_eq!(top3[2].0, ["c", "a", "b"].map(str::to_string));
    }

    #[test]
    fn short_inputs_safe() {
        // < 3 tokens ⇒ no trigrams
        let ng = count(&["x", "y"]);
        assert_eq!(ng.trigram_total, 0);
        assert_eq!(ng.trigrams.len(), 0);
        assert_eq!(ng.bigrams.len(), 1);
        assert_eq!(probability(&ng, &["x", "y", "z"].map(str::to_string)), 0.0);
        assert!(most_common(&ng, 5).is_empty());
        // empty
        let empty = count(&[]);
        assert_eq!(empty.trigram_total, 0);
        assert_eq!(empty.bigrams.len(), 0);
    }
}
