//! managed.rs — `ManagedApiAdapter`: the thin managed/remote backend (Connected mode).
//!
//! This is the constructor blueprint §2.2 says `Connected` mode "necessarily creates": the
//! `OpenAiCompatTransport` already does the wire work; `ManagedApiAdapter` is the `LlmBackend`
//! shell around it carrying `Quirks::managed_api(api_key)` (bearer auth + standard OpenAI
//! envelope). It is NOT a new crate and NOT a new dependency — exactly the "Quirks preset +
//! thin constructor" extension mechanism the blueprint locks in (§1.5). `VllmAdapter` would be
//! the same shape with `Quirks::vllm()`.

use dowiz_kernel::ports::llm::{
    Caps, ChatRequest, ChatResponse, EmbedRequest, EmbedResponse, LlmBackend, LlmError,
    RerankRequest, RerankResponse,
};

use crate::quirks::Quirks;
use crate::transport::OpenAiCompatTransport;

/// A managed/remote OpenAI-compatible backend (Connected mode). Wraps the shared
/// `OpenAiCompatTransport`; the only delta vs `OllamaAdapter` is the `Quirks` preset
/// (`managed_api`), so this is a constructor, not a backend build.
#[derive(Clone)]
pub struct ManagedApiAdapter {
    transport: OpenAiCompatTransport,
}

impl ManagedApiAdapter {
    /// Build a managed backend over `base_url` with `api_key` (bearer auth). The key is supplied
    /// by the caller (resolved from `DOWIZ_LLM_API_KEY_FILE` by `BackendConfig`); it is never read
    /// from the process env.
    pub fn new(base_url: impl Into<String>, api_key: &str) -> Self {
        ManagedApiAdapter {
            transport: OpenAiCompatTransport::new(base_url, Quirks::managed_api(api_key)),
        }
    }
}

impl LlmBackend for ManagedApiAdapter {
    fn id(&self) -> &str {
        "openai-compat"
    }

    fn caps(&self) -> Caps {
        // A managed API is assumed to support chat + embeddings; rerank is not standard OpenAI and
        // is not wired here. Tool calling passes through the standard OpenAI envelope.
        Caps {
            chat: true,
            embed: true,
            rerank: false,
            tool_calling: true,
        }
    }

    fn chat(&self, req: &ChatRequest) -> Result<ChatResponse, LlmError> {
        self.transport.chat(req)
    }

    fn embed(&self, req: &EmbedRequest) -> Result<EmbedResponse, LlmError> {
        self.transport.embed(req)
    }

    fn rerank(&self, _req: &RerankRequest) -> Result<RerankResponse, LlmError> {
        // No rerank endpoint in the standard OpenAI surface → fail closed.
        Err(LlmError::Unsupported)
    }

    fn health(&self) -> Result<(), LlmError> {
        self.transport.health()
    }
}
