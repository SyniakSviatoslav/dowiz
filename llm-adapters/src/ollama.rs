//! ollama.rs — `OllamaAdapter`: the live Tier-1 backend (already running on this host).
//!
//! Implements `dowiz_kernel::ports::llm::LlmBackend`. Model routing by `TaskClass` (§2.2):
//! Code→qwen2.5-coder:7b, General→llama3.1:8b, Embedding→nomic-embed-text (qwen3-embedding:0.6b
//! as the higher-quality option). `:tag` ids pass through verbatim; `fp_ollama` sentinel is ignored.

use std::collections::BTreeMap;
use std::sync::Mutex;

use dowiz_kernel::ports::llm::{
    Caps, ChatRequest, ChatResponse, EmbedRequest, EmbedResponse, LlmBackend, LlmError,
    RerankRequest, RerankResponse, TaskClass,
};

use crate::quirks::Quirks;
use crate::transport::OpenAiCompatTransport;

/// The Ollama adapter. Construct with a base URL (default `http://127.0.0.1:11434`).
#[derive(Clone)]
pub struct OllamaAdapter {
    transport: OpenAiCompatTransport,
    /// Memoized per-model `tool_calling` probe results. The probe runs lazily on
    /// the first `caps()` call (never at construction, so building an adapter while
    /// the daemon is down stays cheap and non-failing). A probe failure is recorded
    /// as `false` (fail-closed) — never a panic, never a stale `true`.
    toolkit: std::sync::Arc<Mutex<BTreeMap<String, bool>>>,
}

impl OllamaAdapter {
    pub fn new(base_url: &str) -> Self {
        OllamaAdapter {
            transport: OpenAiCompatTransport::new(base_url, Quirks::ollama()),
            toolkit: std::sync::Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    /// Default constructor pointing at the already-running local daemon.
    pub fn local() -> Self {
        Self::new("http://127.0.0.1:11434")
    }

    /// Map `TaskClass` → concrete Ollama model id (within-adapter routing, distinct from Phase-5's
    /// `gov_route` dev-tooling router). Embedding defaults to `nomic-embed-text`; the caller may
    /// override `model_id` directly on the request for the higher-quality `qwen3-embedding:0.6b`.
    fn route_model(&self, req: &ChatRequest) -> String {
        if !req.model_id.is_empty() {
            return req.model_id.clone();
        }
        match req.task_class {
            TaskClass::Code => "qwen2.5-coder:7b".to_string(),
            TaskClass::General => "llama3.1:8b".to_string(),
            TaskClass::Embedding => "nomic-embed-text".to_string(),
        }
    }

    /// Probe (and memoize) whether `model_id` supports tool calling, via Ollama's
    /// native `/api/show` capabilities. Fail-closed: any failure ⇒ `false`
    /// (recorded so the same model is never re-probed into a `true`).
    fn tool_calling_for(&self, model_id: &str) -> bool {
        // Fast path: reuse a prior probe.
        if let Some(&v) = self.toolkit.lock().unwrap().get(model_id) {
            return v;
        }
        let ok = self
            .transport
            .show_capabilities(model_id)
            .map(|caps| caps.iter().any(|c| c.eq_ignore_ascii_case("tools")))
            .unwrap_or(false);
        self.toolkit
            .lock()
            .unwrap()
            .insert(model_id.to_string(), ok);
        ok
    }

    /// Public probe entry point used by tests and callers wanting to check an
    /// arbitrary model id (not just the General route). Shares the memoized logic
    /// and the same fail-closed semantics as `caps()`.
    pub fn probe_tool_calling(&self, model_id: &str) -> bool {
        self.tool_calling_for(model_id)
    }
}

impl LlmBackend for OllamaAdapter {
    fn id(&self) -> &str {
        "ollama"
    }

    fn caps(&self) -> Caps {
        // Ollama on this host: chat + embed confirmed live (§1.2). Rerank not wired.
        // `tool_calling` is NO LONGER hard-pinned: it is discovered live per the
        // General-route model the loop actually uses (llama3.1:8b), with a
        // fail-closed probe (failure ⇒ false, never fail-open).
        let model = self.route_model(&ChatRequest {
            task_class: TaskClass::General,
            ..Default::default()
        });
        Caps {
            chat: true,
            embed: true,
            rerank: false,
            tool_calling: self.tool_calling_for(&model),
        }
    }

    fn chat(&self, req: &ChatRequest) -> Result<ChatResponse, LlmError> {
        let mut routed = req.clone();
        routed.model_id = self.route_model(req);
        self.transport.chat(&routed)
    }

    fn embed(&self, req: &EmbedRequest) -> Result<EmbedResponse, LlmError> {
        self.transport.embed(req)
    }

    fn rerank(&self, _req: &RerankRequest) -> Result<RerankResponse, LlmError> {
        // Ollama (this host) has no rerank endpoint wired → fail closed, caller falls back.
        Err(LlmError::Unsupported)
    }

    fn health(&self) -> Result<(), LlmError> {
        self.transport.health()
    }
}

/// Backwards-compatible alias so the plan's `OllamaQuirks` reference resolves (the adapter uses
/// `Quirks::ollama()` directly; this type is a thin marker, kept so the public surface matches docs).
pub type OllamaQuirks = Quirks;
