//! retrieval/ — internal-retrieval layers (vectorless, deterministic).
//!
//! Two complementary layers over the same 20-doc living-memory corpus:
//!   * L0 (M1) exact byte+pattern search (restricted {literal, `.`, `.*`}
//!     wildcard subset; `regex` retired item 5) — sibling subagent (W1-3):
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

/// L2 lexical ranker — pure-`std` Okapi BM25 (M2).
pub mod bm25;
/// L3 RELATEDNESS — wikilink fixture graph + "what relates to X" diffusion.
pub mod diffusion;
/// L0 exact-search corpus + synthetic generator (W1-3).
pub mod fixtures;
/// L0 deterministic trigram inverted index + exact verify (W1-3).
pub mod index;
/// M4/W4-1 — native std-only content-addressed living-memory store (default)
/// + feature-gated `pgrust` SQL adapter boundary (OFF by default).
pub mod memory_store;
/// L0 kernel-owned restricted wildcard matcher ({literal, `.`, `.*`}) — the
/// regex-retirement replacement for the `query_regex` verify step (item 5).
pub mod pattern;
/// L3 PPR power-iteration engine — mirrors `kernel/src/markov.rs` determinism.
pub mod ppr;
/// L2+L0 fusion — BM25 + trigram index, living-knowledge recall@5=1.0
/// (un-strands the spike engine's lexical capability into the kernel, M2/A2).
///
/// W18 — `recall` ALSO hosts the PRIMARY recall source (`PrimaryRecall` +
/// `recall_at_k`) that the self-improvement loop uses. The (wasm-gated)
/// `crate::living_knowledge` adapter delegates its lexical recall to this
/// kernel-owned, std-only path instead of the purged JS engine.
pub mod recall;
/// W18 — wire the kernel-owned PRIMARY recall source (`recall::PrimaryRecall`)
/// as the lexical `LivingKnowledge` adapter. The (wasm-gated) `crate::living_knowledge`
/// recall loop delegates here instead of the purged JS engine. This is a real
/// (non-test) consumer of `living_knowledge` registered from `retrieval`.
#[cfg(all(feature = "wasm", not(target_arch = "wasm32")))]
pub use crate::living_knowledge::LivingKnowledge as PrimaryRecallLivingKnowledge;
/// Construct a `LivingKnowledge`-implementing PRIMARY recall source (wasm only).
#[cfg(all(feature = "wasm", not(target_arch = "wasm32")))]
pub fn primary_recall_adapter() -> recall::PrimaryRecall {
    recall::PrimaryRecall::new()
}
/// W3-3 P1 — knowledge-spine frontmatter validator + MAP.md generator organ.
pub mod spine;
/// L0 RED→GREEN exact-match / 0-false-positive tests (W1-3).
pub mod tests;
