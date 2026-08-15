//! `retrieval/` — pure no_std retrieval primitives (L0 exact search).
//!
//! The deterministic, vectorless L0 layer: the frozen living-memory corpus
//! ([`fixtures`]), the byte-level trigram inverted index ([`index`]), and the
//! kernel-owned restricted wildcard matcher ([`pattern`], the item-5 regex
//! retirement). All three are pure (core + alloc only); the std layers
//! (`bm25`, `ppr`, `diffusion`, `recall`, `memory_store`, `spine`) stay in the
//! kernel shim.

pub mod bm25;
pub mod diffusion;
pub mod fixtures;
pub mod index;
pub mod pattern;
pub mod ppr;
pub mod spine;
