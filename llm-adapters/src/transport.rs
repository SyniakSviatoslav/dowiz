//! transport.rs — one ureq-based OpenAI-compatible transport, shared by every adapter.
//!
//! Holds NO vendor knowledge: all Ollama/vLLM deltas come from the `Quirks` the caller passes.
//! Synchronous (ureq, no tokio). Maps transport failures to typed `LlmError` — never a mock.

use dowiz_kernel::ports::llm::{
    ChatRequest, ChatResponse, EmbedRequest, EmbedResponse, LlmError, Message, Usage,
};
use serde_json::{json, Value};

use crate::quirks::Quirks;

/// A synchronous OpenAI-compatible transport over `ureq`. Cheap to construct (clones the base URL
/// + quirks); the underlying agent is created per-request (ureq 2 pools connections internally).
#[derive(Debug, Clone)]
pub struct OpenAiCompatTransport {
    base_url: String,
    quirks: Quirks,
}

impl OpenAiCompatTransport {
    pub fn new(base_url: impl Into<String>, quirks: Quirks) -> Self {
        let mut base = base_url.into();
        // Normalize: no trailing slash (we append paths).
        while base.ends_with('/') {
            base.pop();
        }
        OpenAiCompatTransport { base_url: base, quirks }
    }

    /// Chat completion → `/v1/chat/completions`.
    pub fn chat(&self, req: &ChatRequest) -> Result<ChatResponse, LlmError> {
        if req.messages.is_empty() {
            return Err(LlmError::BadRequest("empty messages".into()));
        }
        let messages: Vec<Value> = req
            .messages
            .iter()
            .map(|m| json!({ "role": m.role, "content": m.content }))
            .collect();
        let mut body = json!({
            "model": req.model_id,
            "messages": messages,
            "temperature": req.temperature,
            "top_p": req.top_p,
            "max_tokens": req.max_tokens,
            "stream": false,
        });
        if let Some(seed) = req.seed {
            body["seed"] = json!(seed);
        }
        // Ollama-only options (keep_alive/num_ctx/think) surfaced verbatim when the adapter wants.
        if self.quirks.surface_options && !req.options.is_empty() {
            let mut opts = serde_json::Map::new();
            for (k, v) in &req.options {
                // Values are plain strings per the kernel type; parse bool/int when possible.
                if let Ok(b) = v.parse::<bool>() {
                    opts.insert(k.clone(), json!(b));
                } else if let Ok(i) = v.parse::<i64>() {
                    opts.insert(k.clone(), json!(i));
                } else {
                    opts.insert(k.clone(), json!(v));
                }
            }
            body["options"] = Value::Object(opts);
        }

        let raw = self.post("/v1/chat/completions", &body)?;
        parse_chat(&raw)
    }

    /// Embedding → the adapter's `embeddings_path` (Ollama `/v1/embeddings`; vLLM same).
    pub fn embed(&self, req: &EmbedRequest) -> Result<EmbedResponse, LlmError> {
        if req.input.trim().is_empty() {
            return Err(LlmError::BadRequest("empty embedding input".into()));
        }
        let body = json!({ "model": req.model_id, "input": req.input });
        let raw = self.post(&self.quirks.embeddings_path, &body)?;
        parse_embed(&raw)
    }

    /// Send a POST, returning the parsed JSON body. Translates HTTP/transport failures to `LlmError`.
    fn post(&self, path: &str, body: &Value) -> Result<Value, LlmError> {
        let url = format!("{}{}", self.base_url, path);
        let mut reqb = ureq::post(&url).timeout(std::time::Duration::from_secs(120));
        for (k, v) in &self.quirks.extra_headers {
            reqb = reqb.set(k, v);
        }
        let resp = reqb.send_json(body.clone());
        let resp = match resp {
            Ok(r) => r,
            Err(e) => {
                // ureq error kinds: status (non-2xx), transport (DNS/conn/TLS), etc.
                if let ureq::Error::Status(code, _r) = &e {
                    if *code == 404 {
                        return Err(LlmError::Unsupported);
                    }
                    if (500..=599).contains(code) {
                        return Err(LlmError::Unavailable);
                    }
                    return Err(LlmError::BadRequest(format!("HTTP {}", code)));
                }
                // Transport-level (connection refused, timeout, TLS) ⇒ backend unavailable.
                return Err(LlmError::Unavailable);
            }
        };
        resp.into_json().map_err(|_| LlmError::Unavailable)
    }

    /// Typed health probe: a cheap `/v1/models` GET. `Err(Unavailable)` when unreachable.
    pub fn health(&self) -> Result<(), LlmError> {
        let url = format!("{}/v1/models", self.base_url);
        let mut reqb = ureq::get(&url).timeout(std::time::Duration::from_secs(10));
        for (k, v) in &self.quirks.extra_headers {
            reqb = reqb.set(k, v);
        }
        match reqb.call() {
            Ok(_) => Ok(()),
            Err(_) => Err(LlmError::Unavailable),
        }
    }
}

/// Parse an OpenAI chat/completions response into `ChatResponse`.
fn parse_chat(raw: &Value) -> Result<ChatResponse, LlmError> {
    let content = raw
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .ok_or_else(|| LlmError::BadRequest("missing choices[0].message.content".into()))?;
    let usage = raw.get("usage").map(parse_usage).unwrap_or_default();
    Ok(ChatResponse {
        content: content.to_string(),
        usage,
    })
}

/// Parse an OpenAI `/v1/embeddings` response (`data:[{embedding:[…]}]`).
fn parse_embed(raw: &Value) -> Result<EmbedResponse, LlmError> {
    let arr = raw
        .get("data")
        .and_then(|d| d.get(0))
        .and_then(|d| d.get("embedding"))
        .and_then(|e| e.as_array())
        .ok_or_else(|| LlmError::BadRequest("missing data[0].embedding".into()))?;
    let embedding = arr
        .iter()
        .filter_map(|v| v.as_f64().map(|f| f as f32))
        .collect::<Vec<f32>>();
    if embedding.is_empty() {
        return Err(LlmError::BadRequest("empty embedding vector".into()));
    }
    Ok(EmbedResponse { embedding })
}

fn parse_usage(v: &Value) -> Usage {
    Usage {
        prompt_tokens: v.get("prompt_tokens").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
        completion_tokens: v.get("completion_tokens").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
        total_tokens: v.get("total_tokens").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
    }
}

/// Helper for adapters/tests: build a `Message`.
pub fn message(role: &str, content: &str) -> Message {
    Message {
        role: role.to_string(),
        content: content.to_string(),
    }
}
