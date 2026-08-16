//! Living-knowledge retrieval — the `no_std` contract + recall surface.
//!
//! Decision (operator, 2026-07-14): the living-knowledge engine lives on a DIVERGENT
//! branch (`recover/stash-1-2994e6c8`, JS + bge-small ONNX embedder). This module is the
//! kernel's CONTRACT half: the `LivingKnowledge` trait, its wire types (`Hit`, `DocInput`),
//! and the PRIMARY recall surface (`recall_at_k` / `primary_recall_adapter`) that delegates
//! to the deterministic, pure-`std` BM25+trigram fusion in `crate::retrieval::recall`.
//!
//! The PROCESS-BACKED adapter (`SubprocessLivingKnowledge`, which speaks JSON-over-stdio to a
//! bridge command via `std::process`) stays in the kernel shim (`kernel/src/living_knowledge.rs`)
//! — subprocess spawning is not a no_std capability. The bridge is swappable at runtime via
//! `LK_BRIDGE_CMD`; the real ONNX-backed spike is plugged in without a kernel recompile.
//!
//! Fail-closed: any spawn / I/O / protocol error returns `Err` — retrieval never silently
//! degrades to "empty results".

use alloc::string::String;
use alloc::vec::Vec;

/// A single retrieval hit.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "json-api",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct Hit {
    pub id: String,
    pub score: f64,
}

/// A document handed to the bridge (path/title/text triple).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "json-api", derive(serde::Serialize))]
pub struct DocInput {
    pub rel: String,
    pub title: String,
    pub text: String,
}

/// Retrieval contract the rest of the kernel depends on.
pub trait LivingKnowledge {
    /// Rank `k` documents for `query`. Errors are explicit (fail-closed).
    fn retrieve(&self, query: &str, k: usize) -> Result<Vec<Hit>, String>;
}

/// W18 — PRIMARY recall delegation from `living_knowledge` into the kernel-owned recall path.
///
/// The (formerly JS-stranded) `living_knowledge` recall loop no longer shells out to the purged
/// JS engine: this thin wrapper delegates to the deterministic BM25+trigram fusion in
/// `crate::retrieval::recall` (the PRIMARY recall source). No JS, no network. This is the
/// `recall_at_k` surface the blueprint requires `living_knowledge` to expose; the real ranking
/// work lives in `retrieval::recall`.
pub fn recall_at_k(query: &str, k: usize) -> Vec<(String, f64)> {
    crate::retrieval::recall::PrimaryRecall::new().recall_at_k(query, k)
}

/// W18 — build the `LivingKnowledge`-implementing PRIMARY recall adapter backed by the
/// kernel-owned recall path (not the node subprocess bridge).
pub fn primary_recall_adapter() -> crate::retrieval::recall::PrimaryRecall {
    crate::retrieval::recall::PrimaryRecall::new()
}
