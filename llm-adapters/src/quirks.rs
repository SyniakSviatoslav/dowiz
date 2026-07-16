//! quirks.rs — per-adapter behavior deltas.
//!
//! Live probes (HARNESS-LLM-BACKEND.md §2.2) found concrete Ollama quirks a naive OpenAI client
//! would trip on. This struct encodes them so the single `OpenAiCompatTransport` can serve every
//! backend by passing a different `Quirks`. This is the Hermes-proven `Quirks`-pattern the repo
//! already cites.

use std::collections::BTreeMap;

/// Per-adapter wire deltas. The transport reads these; it holds no vendor knowledge itself.
#[derive(Debug, Clone)]
pub struct Quirks {
    /// Pass `:tag` model ids through verbatim — never strip/normalize after `:`
    /// (Ollama `llama3.1:8b`). Default: true (safe).
    pub pass_tag_ids_verbatim: bool,
    /// `system_fingerprint:"fp_ollama"` is a constant sentinel — never key caching/dedup on it.
    /// The transport already strips it; this flag documents intent + lets vLLM assert absence.
    pub strip_sentinel_fingerprint: bool,
    /// Embeddings endpoint path. Ollama: `/v1/embeddings` (OpenAI-parity). vLLM: `/v1/embeddings`.
    pub embeddings_path: String,
    /// Native (non-`/v1`) embeddings fallback path, if the adapter wants it (Ollama `/api/embed`).
    pub native_embeddings_path: Option<String>,
    /// Surface `options` (Ollama-only knobs: `keep_alive`, `num_ctx`, `think`). When false, the
    /// transport drops `options` from the wire envelope entirely (vLLM/managed-API).
    pub surface_options: bool,
    /// Extra headers (e.g. `Authorization: Bearer …` for managed-API). Sent on every request.
    pub extra_headers: BTreeMap<String, String>,
}

impl Default for Quirks {
    fn default() -> Self {
        Quirks {
            pass_tag_ids_verbatim: true,
            strip_sentinel_fingerprint: true,
            embeddings_path: "/v1/embeddings".to_string(),
            native_embeddings_path: None,
            surface_options: false,
            extra_headers: BTreeMap::new(),
        }
    }
}

impl Quirks {
    /// Ollama-specific quirks (live-verified §1.2 / §2.2).
    pub fn ollama() -> Self {
        Quirks {
            pass_tag_ids_verbatim: true,
            strip_sentinel_fingerprint: true,
            embeddings_path: "/v1/embeddings".to_string(),
            native_embeddings_path: Some("/api/embed".to_string()),
            surface_options: true,
            extra_headers: BTreeMap::new(),
        }
    }

    /// vLLM (Tier-2, GPU-gated) — native OpenAI-compat, no `:tag` normalization needed, no
    /// Ollama-only options.
    pub fn vllm() -> Self {
        Quirks {
            pass_tag_ids_verbatim: false,
            strip_sentinel_fingerprint: false,
            embeddings_path: "/v1/embeddings".to_string(),
            native_embeddings_path: None,
            surface_options: false,
            extra_headers: BTreeMap::new(),
        }
    }

    /// Managed-API proxy (Tier-0 default) — standard OpenAI envelope + bearer auth.
    pub fn managed_api(api_key: &str) -> Self {
        let mut headers = BTreeMap::new();
        headers.insert("Authorization".to_string(), format!("Bearer {}", api_key));
        Quirks {
            pass_tag_ids_verbatim: false,
            strip_sentinel_fingerprint: false,
            embeddings_path: "/v1/embeddings".to_string(),
            native_embeddings_path: None,
            surface_options: false,
            extra_headers: headers,
        }
    }
}
