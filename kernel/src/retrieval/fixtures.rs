//! Fixture corpus for the L0 retrieval tests (blueprint M1 / W1-3).
//!
//! 20 strings styled after living-memory note names. Used by the RED→GREEN
//! test to prove candidate reduction + 0 false positives on a realistic corpus.

/// 20 living-memory note names. Deterministic, ASCII, good trigram spread.
pub const FIXTURE: &[&str] = &[
    "MEMORY.md",
    "MEMORY-ATTIC.md",
    "note-salience-decay.md",
    "note-wikilink-graph.md",
    "note-pgrust-schema.md",
    "note-trigram-index.md",
    "note-bm25-fusion.md",
    "note-heat-kernel-recall.md",
    "note-pagerank-local-push.md",
    "note-never-delete-tier.md",
    "note-compression-zstd.md",
    "note-vsa-composite-key.md",
    "note-renormalizer-gate.md",
    "note-entropy-ledger.md",
    "note-field-operator.md",
    "note-divergence-signal.md",
    "note-coherence-fusion.md",
    "note-ttrain-deferred.md",
    "note-quantization-pq.md",
    "note-cdc-dedup.md",
];

/// Deterministic synthetic corpus of `n` docs, each = shared boilerplate +
/// a UNIQUE 12-char marker (from a 26-letter alphabet) + shared boilerplate.
///
/// Querying a unique marker demonstrates ~N× candidate reduction: the marker's
/// trigrams are exclusive to the single planted doc, so intersection collapses
/// to one candidate. Seeded xorshift64* ⇒ fully reproducible across runs.
pub fn synthetic_corpus(n: usize) -> Vec<String> {
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut rng = || {
        // xorshift64*
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let alpha: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
    let mut docs = Vec::with_capacity(n);
    for _ in 0..n {
        let mut marker = String::with_capacity(12);
        for _ in 0..12 {
            let idx = (rng() % 26) as usize;
            marker.push(alpha[idx] as char);
        }
        docs.push(format!("boilerplate-prefix-{}-suffix-boilerplate", marker));
    }
    docs
}
