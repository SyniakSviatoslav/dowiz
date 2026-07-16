//! telemetry.rs — self-improvement loop: recurring-pattern surface over the
//! tool-outcome token stream.
//!
//! W19 integration point for `trigram.rs` (T2-β). The self-improvement loop
//! records every tool outcome as a token and periodically asks: "which triples
//! of outcomes recur most?" — a deterministic, zero-dep signal of loops/habits
//! in the agent's own behaviour (the same hot path `markov.rs` models as a
//! first-order chain; here we surface the *recurring 3-grams* with a
//! lexicographic tie-break so the answer is byte-identical on every machine).
//!
//! Fail-closed: an empty token stream yields ZERO trigrams (never a synthetic
//! or panicking result). Ranking is deterministic (count DESC, then key ASC).
//!
//! ZERO new dependencies (reuses `crate::trigram`, which is `std` HashMap only).

use crate::trigram::{count, most_common, NGrams};

/// A recurring trigram key: three outcome tokens.
pub type Tri = [String; 3];

/// A single recurring-pattern surface result: top-k trigrams + their counts.
#[derive(Debug, Clone)]
pub struct PatternSurface {
    pub top: Vec<(Tri, u64)>,
    /// Total number of trigram windows emitted (0 for short/empty streams).
    pub trigram_total: u64,
}

/// The self-improvement loop's pattern surface.
///
/// Consume `crate::trigram` directly (do NOT re-implement n-gram counting or
/// any EMA here — that is the kernel `kalman`/`trigram` substrate's job). Given
/// the tool-outcome `tokens`, returns the `k` most common trigrams, ranked
/// deterministically.
///
/// Fail-closed: empty/short `tokens` → `top` is empty and `trigram_total == 0`.
pub fn surface_recurring_patterns(tokens: &[&str], k: usize) -> PatternSurface {
    let ng: NGrams = count(tokens);
    let top = most_common(&ng, k);
    PatternSurface {
        top,
        trigram_total: ng.trigram_total,
    }
}

/// Convenience: the single most recurring trigram (the loop's headline signal).
/// `None` when there are no trigrams (empty/short stream).
pub fn top_pattern(tokens: &[&str]) -> Option<(Tri, u64)> {
    surface_recurring_patterns(tokens, 1).top.into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Hand oracle (matches trigram.rs): a b c a b c a b d  (9 tokens)
    //   trigram windows: abc, bca, cab, abc, bca, cab, abd  => 7 windows
    //   counts: abc×2, bca×2, cab×2, abd×1  => tie at 2 → "abc" wins lex.
    const TOK: &[&str] = &["a", "b", "c", "a", "b", "c", "a", "b", "d"];

    // ── W19 GREEN (b): deterministic top-1 trigram ──
    #[test]
    fn green_top_pattern_is_deterministic() {
        let top = top_pattern(TOK).expect("non-empty stream yields a pattern");
        assert_eq!(top.0, ["a", "b", "c"].map(str::to_string));
        assert_eq!(top.1, 2);
    }

    #[test]
    fn green_surface_top_k_ranks_deterministically() {
        let surf = surface_recurring_patterns(TOK, 3);
        assert_eq!(surf.trigram_total, 7);
        assert_eq!(surf.top.len(), 3);
        // count-2 keys, lexicographic order: abc, bca, cab.
        assert_eq!(surf.top[0].0, ["a", "b", "c"].map(str::to_string));
        assert_eq!(surf.top[1].0, ["b", "c", "a"].map(str::to_string));
        assert_eq!(surf.top[2].0, ["c", "a", "b"].map(str::to_string));
    }

    // ── W19 fail-closed: empty stream → 0 trigrams ──
    #[test]
    fn green_empty_stream_yields_zero_trigrams() {
        let surf = surface_recurring_patterns(&[], 5);
        assert_eq!(surf.trigram_total, 0);
        assert!(surf.top.is_empty());
        assert!(top_pattern(&[]).is_none());
        // also: 2 tokens (< 3) ⇒ no trigrams.
        let short = surface_recurring_patterns(&["x", "y"], 5);
        assert_eq!(short.trigram_total, 0);
        assert!(short.top.is_empty());
    }

    // ── Determinism invariant: same input ⇒ byte-identical top-1 every run ──
    #[test]
    fn green_determinism_invariant() {
        let a = top_pattern(TOK);
        let b = top_pattern(TOK);
        assert_eq!(a, b, "top_pattern must be deterministic");
    }
}
