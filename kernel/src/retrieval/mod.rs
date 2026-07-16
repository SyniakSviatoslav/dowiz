//! retrieval/ — internal-retrieval layers (vectorless, deterministic).
//!
//! Two complementary layers over the same 20-doc living-memory corpus:
//!   * L0 (M1) exact byte+regex search — sibling subagent (W1-3):
//!       - `fixtures` — the frozen 20-doc corpus + synthetic generator;
//!       - `index`    — deterministic trigram inverted index + exact verify;
//!       - `tests`    — RED→GREEN exact-match / 0-false-positive suite.
//!   * L3 (M3) RELATEDNESS — this task:
//!       - `ppr`       — Personalized-PageRank power-iteration engine, reusing
//!                       `kernel/src/markov.rs`' bitwise-deterministic recurrence;
//!       - `diffusion` — a 20-node / 41-edge wikilink fixture + "what relates to
//!                       X" ranking (NOT exact search).
//!
//! Shared contract (red-line): `markov.rs` is NEVER modified — its proven
//! deterministic accumulation order is *mirrored* in `ppr.rs`. No
//! eigendecomposition. Same input ⇒ same bytes (fixed K, fixed summation order).

/// L0 exact-search corpus + synthetic generator (W1-3).
pub mod fixtures;
/// L0 deterministic trigram inverted index + exact verify (W1-3).
pub mod index;
/// L0 RED→GREEN exact-match / 0-false-positive tests (W1-3).
pub mod tests;
/// L3 PPR power-iteration engine — mirrors `kernel/src/markov.rs` determinism.
pub mod ppr;
/// L3 RELATEDNESS — wikilink fixture graph + "what relates to X" diffusion.
pub mod diffusion;
