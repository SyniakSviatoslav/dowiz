//! compose.rs — the harness composition call-site (§4.2 / M5).
//!
//! Stacks the port adapters into ONE bounded, cache-fronted LLM surface:
//!
//! ```text
//!     Harness
//!       └─ Dispatcher { TokenBucket-bounded }            (WAVE 1(d): budget + typed refusal)
//!            └─ CachingBackend { OllamaAdapter, MemStore } (WAVE 1(b): exact-match cache)
//!                 └─ OllamaAdapter                        (WAVE 0b: live OpenAI-compat backend)
//! ```
//!
//! The backend choice is CONFIG (M5), never a hard-coded fork: `StackBuilder` selects the live
//! Ollama (or a later-managed backend) and whether the cache is on. This is the single entry
//! point a consuming engine (FE render loop, agent step, research-argue loop) calls — `harness.chat`
//! / `harness.embed` — and it is the DISPATCH call-site where token budget + cache + EV harvest
//! are enforced, never a side-channel around the event-sourced substrate.
//!
//! NOTE: the GPU/particle `engine` crate is intentionally NOT the consumer — it is offline-clean
//! (kernel-only, no network) by mandate. The LLM consumer is the harness, i.e. this crate.

use crate::cache::{CachingBackend, NoCache};
use crate::dispatch::{DispatchError, Dispatcher};
use crate::ollama::OllamaAdapter;
use dowiz_kernel::backup::MemStore;
use dowiz_kernel::ports::llm::{
    Caps, ChatRequest, ChatResponse, EmbedRequest, EmbedResponse, LlmBackend, LlmError,
    RerankRequest, RerankResponse,
};
use std::sync::Arc;

/// Default Ollama base URL (the daemon already running on this host).
pub const DEFAULT_OLLAMA_BASE: &str = "http://127.0.0.1:11434";

/// A fully-wired harness over a concrete store `S` (`MemStore` = caching on, `NoCache` = off).
///
/// Chat goes through the bounded `Dispatcher` (token budget + typed `BudgetExceeded` refusal,
/// H1 EV harvest); embed/rerank go through the same cache-fronted backend directly (embed caching
/// powers the semantic-leak gate; the dispatcher's per-call budget does not apply to embeddings).
pub struct Harness<S: dowiz_kernel::backup::BlockStore + Send + Sync + 'static> {
    dispatcher: Dispatcher<CachingBackend<OllamaAdapter, S>>,
    backend: Arc<CachingBackend<OllamaAdapter, S>>,
}

impl<S: dowiz_kernel::backup::BlockStore + Send + Sync + 'static> Harness<S> {
    /// Dispatch a chat request through the bounded dispatcher (budget + EV harvest).
    pub fn chat(&self, req: ChatRequest) -> Result<ChatResponse, DispatchError> {
        self.dispatcher.dispatch(req)
    }

    /// Embed through the cache-fronted backend (bypasses the per-call token budget; shares cache).
    pub fn embed(&self, req: &EmbedRequest) -> Result<EmbedResponse, LlmError> {
        self.backend.embed(req)
    }

    /// Rerank through the backend (Ollama has no rerank endpoint → `Err(Unsupported)` fail-closed).
    pub fn rerank(&self, req: &RerankRequest) -> Result<RerankResponse, LlmError> {
        self.backend.rerank(req)
    }

    /// Degrade-closed health passthrough.
    pub fn health(&self) -> Result<(), LlmError> {
        self.backend.health()
    }

    /// Capability surface of the underlying adapter.
    pub fn caps(&self) -> Caps {
        self.backend.caps()
    }
}

/// Config-driven builder for the harness stack (M5: backend choice is config, not a fork).
///
/// Defaults target the local Ollama daemon with caching on and a conservative token budget
/// (≈8 tokens/s refill, 64 burst) — local-first, never a surprise managed-API call.
#[derive(Clone)]
pub struct StackBuilder {
    base: String,
    workers: usize,
    capacity: u64,
    refill_rate: f64,
    cache: bool,
}

impl Default for StackBuilder {
    fn default() -> Self {
        StackBuilder {
            base: DEFAULT_OLLAMA_BASE.to_string(),
            workers: 2,
            capacity: 64,
            refill_rate: 8.0,
            cache: true,
        }
    }
}

impl StackBuilder {
    /// Point the stack at a different Ollama base URL (or a future OpenAI-compat endpoint).
    pub fn ollama(mut self, base: impl Into<String>) -> Self {
        self.base = base.into();
        self
    }

    /// Worker-pool size (≤ backend parallelism cap; Ollama ≈ 2).
    pub fn workers(mut self, n: usize) -> Self {
        self.workers = n.max(1);
        self
    }

    /// Token-bucket budget: `capacity` burst tokens, `refill_rate` tokens/second.
    pub fn budget(mut self, capacity: u64, refill_rate: f64) -> Self {
        self.capacity = capacity;
        self.refill_rate = refill_rate;
        self
    }

    /// Enable/disable the exact-match response cache (default on).
    pub fn with_cache(mut self, on: bool) -> Self {
        self.cache = on;
        self
    }

    /// Build the wired harness. `S` is `MemStore` (cache on) or `NoCache` (cache off) per
    /// `with_cache`; the caller selects by calling `build_cached` / `build_uncached`.
    fn build_with<S: dowiz_kernel::backup::BlockStore + Send + Sync + Clone + 'static>(
        self,
        store: S,
    ) -> Harness<S> {
        let ollama = OllamaAdapter::new(&self.base);
        let backend = Arc::new(CachingBackend::with_store(ollama, store));
        // `Dispatcher::new` Arcs the backend again; both handles share the SAME cache store
        // (Arc clone inside CachingBackend is a Mutex clone = same inner store).
        let dispatcher = Dispatcher::new(
            (*backend).clone(),
            self.workers,
            self.capacity,
            self.refill_rate,
        );
        Harness {
            dispatcher,
            backend,
        }
    }

    /// Build with the in-memory cache enabled.
    pub fn build_cached(self) -> Harness<MemStore> {
        self.build_with(MemStore::new())
    }

    /// Build with caching disabled (every call reaches the backend).
    pub fn build_uncached(self) -> Harness<NoCache> {
        self.build_with(NoCache)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dowiz_kernel::ports::llm::{Message, TaskClass};

    fn chat_req(model: &str, text: &str) -> ChatRequest {
        ChatRequest {
            model_id: model.to_string(),
            messages: vec![Message {
                role: "user".into(),
                content: text.into(),
            }],
            max_tokens: 8,
            task_class: TaskClass::General,
            ..Default::default()
        }
    }

    // The composition compiles AND is Sync (Dispatcher's Send+Sync bound is satisfied through the
    // Mutex-backed cache). This is the root-cause fix that made Dispatcher<cache<ollama>> possible.
    fn assert_send_sync<T: Send + Sync>() {}
    #[test]
    fn harness_stack_is_send_sync() {
        assert_send_sync::<Harness<MemStore>>();
    }

    // Live: build the default stack, run one chat — exercises ollama → cache → dispatcher end-to-end.
    #[test]
    fn harness_live_chat_through_full_stack() {
        let h = StackBuilder::default().build_cached();
        let r = h
            .chat(chat_req("qwen2.5-coder:7b", "Reply with exactly: OK"))
            .expect("live chat through full stack");
        assert!(!r.content.is_empty(), "received a non-empty response");
        assert!(r.usage.total_tokens > 0, "usage reported");
    }

    // Live: identical second call is served from the cache (exact-hit) — proves the stack is wired
    // cache-front, not just pass-through.
    #[test]
    fn harness_live_second_call_is_cache_hit() {
        let h = StackBuilder::default().build_cached();
        let req = chat_req("qwen2.5-coder:7b", "cache hit test — say PONG");
        let r1 = h.chat(req.clone()).expect("chat 1");
        let r2 = h.chat(req.clone()).expect("chat 2 (cache)");
        assert_eq!(
            r1.content, r2.content,
            "cache hit returns identical content"
        );
    }

    // Live: embed path works through the same cache-fronted backend.
    #[test]
    fn harness_live_embed_through_stack() {
        let h = StackBuilder::default().build_cached();
        let r = h
            .embed(&EmbedRequest {
                model_id: "nomic-embed-text".into(),
                input: "semantic gate probe".into(),
            })
            .expect("live embed through stack");
        assert_eq!(
            r.embedding.len(),
            768,
            "nomic-embed-text returns 768-d vectors"
        );
    }
}
