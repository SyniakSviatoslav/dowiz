//! ports/llm.rs — the `LlmBackend` port (trait + value types) for the harness.
//!
//! Compile firewall: this module has ZERO network / HTTP / JSON / serde. It defines only the
//! abstract contract a backend must satisfy and the plain value structs passed across it. The
//! concrete adapter crate (`llm-adapters`, repo root) owns all HTTP/JSON and converts wire
//! shapes into these structs. `cargo tree -p dowiz-kernel` must show no HTTP client here after
//! implementation (verified by the WAVE-0 done-check).
//!
//! Per M5 (hub-autonomy): backend choice is configuration on the consumer side, never a kernel
//! recompile. This trait is the seam — `OllamaAdapter`, `VllmAdapter`, `ManagedApiAdapter` are
//! all `&dyn LlmBackend` behind a config-selected constructor.

use std::fmt::Debug;

/// Fail-closed feature discovery for a backend. A capability the backend does not expose is `false`;
/// the caller must NOT assume presence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Caps {
    pub chat: bool,
    pub embed: bool,
    pub rerank: bool,
    pub tool_calling: bool,
}

/// Drives within-adapter model routing (e.g. `OllamaAdapter`: Code→qwen2.5-coder:7b,
/// General→llama3.1:8b, Embedding→nomic-embed-text). The kernel never knows the model names;
/// the adapter maps `TaskClass` to its concrete `model_id`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskClass {
    Code,
    General,
    Embedding,
}

/// A chat turn. Plain struct, no serde — the adapter serializes it.
#[derive(Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// A chat completion request.
#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub model_id: String,
    pub messages: Vec<Message>,
    pub temperature: f32,
    pub top_p: f32,
    pub max_tokens: u32,
    pub seed: Option<u64>,
    pub task_class: TaskClass,
    /// Cache policy for this call. `Exact` (default) only consults the exact-match cache;
    /// `SemanticOk` additionally allows the advisory near-duplicate Layer-B hit; `NoCache` bypasses
    /// both. Gate-critical callers MUST use `Exact`/`NoCache` — never `SemanticOk`.
    pub cache_policy: CachePolicy,
    /// Backend-specific options surfaced verbatim (Ollama `keep_alive`/`num_ctx`/`think`, etc.).
    /// Passed through untouched; parsed per-adapter in the transport layer.
    pub options: std::collections::BTreeMap<String, String>,
}

impl Default for ChatRequest {
    fn default() -> Self {
        ChatRequest {
            model_id: String::new(),
            messages: Vec::new(),
            temperature: 0.0,
            top_p: 1.0,
            max_tokens: 1024,
            seed: None,
            task_class: TaskClass::General,
            cache_policy: CachePolicy::Exact,
            options: std::collections::BTreeMap::new(),
        }
    }
}

/// Cache policy — a TYPE, not a convention (§3.3's hard boundary is enforced structurally).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CachePolicy {
    /// Exact-match sha3 cache only (Layer A). Safe for gate-critical calls.
    #[default]
    Exact,
    /// Layers A then B (semantic near-duplicate, advisory-only). For tolerant/exploratory tasks ONLY.
    SemanticOk,
    /// Bypass both cache layers — always hit the model.
    NoCache,
}

/// Token usage returned by a completion. Mirrors OpenAI's `usage` object (verified live on Ollama).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl Usage {
    /// Cost of this call against a `TokenBucket` budget (1 token = 1 unit).
    pub fn cost(&self) -> u64 {
        self.total_tokens as u64
    }
}

/// A chat completion response.
#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub content: String,
    pub usage: Usage,
}

/// An embedding request.
#[derive(Debug, Clone)]
pub struct EmbedRequest {
    pub model_id: String,
    pub input: String,
}

/// An embedding response.
#[derive(Debug, Clone)]
pub struct EmbedResponse {
    pub embedding: Vec<f32>,
}

/// A rerank request (optional backend capability).
#[derive(Debug, Clone)]
pub struct RerankRequest {
    pub model_id: String,
    pub query: String,
    pub documents: Vec<String>,
}

/// A rerank response (optional backend capability).
#[derive(Debug, Clone)]
pub struct RerankResponse {
    pub scores: Vec<f32>,
}

/// Typed backend error. `health()` and any capability probe return a typed `Err` when the backend
/// is absent or refuses — never a mock, never a panic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmError {
    /// Backend process/endpoint is not reachable (health failed). Fail-closed.
    Unavailable,
    /// Requested capability (rerank, tool-calling, …) is not offered by this backend.
    Unsupported,
    /// Malformed request (e.g. empty messages, unknown `:tag` model id).
    BadRequest(String),
    /// Request timed out (transport-level deadline).
    Timeout,
}

/// The pluggable LLM backend port. Implemented by `OllamaAdapter`, `VllmAdapter`,
/// `ManagedApiAdapter` in the `llm-adapters` crate.
pub trait LlmBackend {
    /// Stable backend id, e.g. `"ollama:llama3.1:8b"`. Used in cache keys + telemetry rows.
    fn id(&self) -> &str;
    /// Fail-closed capability discovery.
    fn caps(&self) -> Caps;
    /// Chat completion. `Err` on any failure — never a mock response.
    fn chat(&self, req: &ChatRequest) -> Result<ChatResponse, LlmError>;
    /// Embedding. `Err(Unsupported)` if the backend has no embedding model wired.
    fn embed(&self, req: &EmbedRequest) -> Result<EmbedResponse, LlmError>;
    /// Rerank. `Err(Unsupported)` is a VALID response for backends without rerank — the caller
    /// must handle it (fall back to cosine, etc.).
    fn rerank(&self, req: &RerankRequest) -> Result<RerankResponse, LlmError>;
    /// Typed health probe. `Ok(())` iff the endpoint is reachable; `Err(Unavailable)` otherwise.
    /// Never fabricates liveness.
    fn health(&self) -> Result<(), LlmError>;
}
