//! leak_gate.rs — §3.3 Layer-B (semantic) leakage gate. Native (always compiled, zero-dep).
//!
//! Holds the embedding of every already-minted instance and rejects a fresh instance whose cosine
//! similarity to ANY held instance is ≥ `SEMANTIC_LEAK_THRESHOLD`. Layer-A (exact FNV-1a) lives in
//! `evals::MintLog`; this is the advisory cosine companion.
//!
//! The embedding model is INJECTED via `&dyn LlmBackend` (never imported) — the kernel stays
//! zero-dep; the live bridge is `OllamaAdapter::embed` in the `llm-adapters` crate. On a backend
//! error the gate DOWNGRADES to exact-only (does not freeze generation) — fail-closed.

use crate::ports::llm::{EmbedRequest, LlmBackend};

/// §3.3 Layer-B threshold: near-duplicate if cosine ≥ this.
pub const SEMANTIC_LEAK_THRESHOLD: f64 = 0.9;

/// A holder of instance embeddings + the cosine near-duplicate check.
#[derive(Debug, Default)]
pub struct LeakGate {
    store: Vec<Vec<f32>>,
}

impl LeakGate {
    pub fn new() -> Self {
        Self::default()
    }

    /// Cosine similarity between two equal-length vectors (0.0 if either is empty/mismatched).
    pub fn cosine(a: &[f32], b: &[f32]) -> f64 {
        if a.is_empty() || b.is_empty() || a.len() != b.len() {
            return 0.0;
        }
        let dot: f64 = a.iter().zip(b).map(|(x, y)| *x as f64 * *y as f64).sum();
        let na: f64 = a.iter().map(|x| (*x as f64) * (*x as f64)).sum::<f64>().sqrt();
        let nb: f64 = b.iter().map(|x| (*x as f64) * (*x as f64)).sum::<f64>().sqrt();
        if na == 0.0 || nb == 0.0 {
            return 0.0;
        }
        (dot / (na * nb)).clamp(-1.0, 1.0)
    }

    /// Returns `true` if `embedding` is a near-duplicate of any held instance (cosine ≥ threshold).
    pub fn is_leak(&self, embedding: &[f32]) -> bool {
        self.store
            .iter()
            .any(|e| Self::cosine(embedding, e) >= SEMANTIC_LEAK_THRESHOLD)
    }

    /// Mint semantically: embed `instance_text` via `backend` (if any), reject on near-duplicate,
    /// otherwise record the embedding and return `true`. With `backend = None` the gate is a no-op
    /// pass (exact-only is enforced upstream in `MintLog`). On backend error → downgrade to pass
    /// (do not freeze generation).
    pub fn accept(&mut self, instance_text: &str, backend: Option<&dyn LlmBackend>) -> bool {
        let Some(be) = backend else {
            return true;
        };
        match be.embed(&EmbedRequest {
            model_id: "nomic-embed-text".to_string(),
            input: instance_text.to_string(),
        }) {
            Ok(resp) => {
                if self.is_leak(&resp.embedding) {
                    return false; // semantic near-duplicate leakage → reject.
                }
                self.store.push(resp.embedding);
                true
            }
            Err(_) => true, // fail-closed downgrade: embeddings outage must not freeze generation.
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::llm::{Caps, ChatRequest, ChatResponse, EmbedRequest, EmbedResponse, LlmBackend, LlmError, RerankRequest, RerankResponse};

    /// Deterministic fake backend: one-hot on a hash of the input → identical cos1, different cos0.
    struct FakeEmbedder {
        dim: usize,
    }
    impl LlmBackend for FakeEmbedder {
        fn id(&self) -> &str { "fake" }
        fn caps(&self) -> Caps { Caps { chat: false, embed: true, rerank: false, tool_calling: false } }
        fn chat(&self, _: &ChatRequest) -> Result<ChatResponse, LlmError> { Err(LlmError::Unsupported) }
        fn embed(&self, req: &EmbedRequest) -> Result<EmbedResponse, LlmError> {
            let h = req.input.bytes().fold(0usize, |a, b| a.wrapping_add(b as usize)) % self.dim;
            let mut v = vec![0.0f32; self.dim];
            v[h] = 1.0;
            Ok(EmbedResponse { embedding: v })
        }
        fn rerank(&self, _: &RerankRequest) -> Result<RerankResponse, LlmError> { Err(LlmError::Unsupported) }
        fn health(&self) -> Result<(), LlmError> { Ok(()) }
    }

    #[test]
    fn rejects_near_duplicate_passes_distinct() {
        let be = FakeEmbedder { dim: 64 };
        let mut gate = LeakGate::new();
        assert!(gate.accept("the cat sat on the mat", Some(&be)));
        // Same text → cos=1.0 ≥ 0.9 → rejected.
        assert!(!gate.accept("the cat sat on the mat", Some(&be)), "near-duplicate must be rejected");
        // Different text → cos=0 < 0.9 → accepted.
        assert!(gate.accept("a totally different sentence about rockets", Some(&be)));
    }

    #[test]
    fn no_backend_is_pass() {
        let mut gate = LeakGate::new();
        assert!(gate.accept("same text", None));
        assert!(gate.accept("same text", None));
    }

    #[test]
    fn backend_error_downgrades_to_pass() {
        struct Broken;
        impl LlmBackend for Broken {
            fn id(&self) -> &str { "broken" }
            fn caps(&self) -> Caps { Caps { chat: false, embed: true, rerank: false, tool_calling: false } }
            fn chat(&self, _: &ChatRequest) -> Result<ChatResponse, LlmError> { Err(LlmError::Unsupported) }
            fn embed(&self, _: &EmbedRequest) -> Result<EmbedResponse, LlmError> { Err(LlmError::Unavailable) }
            fn rerank(&self, _: &RerankRequest) -> Result<RerankResponse, LlmError> { Err(LlmError::Unsupported) }
            fn health(&self) -> Result<(), LlmError> { Err(LlmError::Unavailable) }
        }
        let mut gate = LeakGate::new();
        // A backend error must NOT freeze generation (fail-closed downgrade to pass).
        assert!(gate.accept("anything", Some(&Broken)));
    }

    #[test]
    fn cosine_basics() {
        assert_eq!(LeakGate::cosine(&[1.0, 0.0], &[0.0, 1.0]), 0.0);
        assert!((LeakGate::cosine(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 1e-9);
        assert_eq!(LeakGate::cosine(&[], &[1.0]), 0.0);
    }
}

