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
    /// Tool declarations for this call (OpenAI `tools` array). Empty for a plain
    /// chat. Extend-don't-rewrite: `Default` seeds `Vec::new()` so every existing
    /// call site compiles and behaves identically (no tool calls requested). The
    /// adapter serializes this into the wire `tools` array.
    pub tools: Vec<ToolDecl>,
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
            tools: Vec::new(),
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
    /// Tool calls the model returned this turn (OpenAI `message.tool_calls`).
    /// Empty when the model answered directly. Iterating on a single tool call is
    /// the loop's job; the kernel carries all of them verbatim, unparsed.
    pub tool_calls: Vec<ToolCallReq>,
}

/// A tool the backend may be asked to call — the kernel-side declaration. The
/// adapter serializes this into the OpenAI `tools` array; the struct carries
/// owned `String`s (the kernel port stays `'static`-free at the wire boundary).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolDecl {
    pub name: String,
    pub description: String,
    pub arg_name: String,
}

/// A single tool call the model returned — parsed by the adapter from
/// `message.tool_calls[].function`. Carries the raw argument JSON so the port
/// impl (not the kernel) owns argument parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCallReq {
    pub name: String,
    pub arguments_json: String,
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

/// The three operating modes (P41 C-b). This enum is the operator's three-mode
/// directive AS A TYPE. It is constructed ONLY by `BackendConfig::from_env` — the
/// presence of a single non-test construction site is what makes silent
/// mode-escalation unrepresentable (blueprint §3.2 / §4.1). Default: `Off`, the
/// fail-closed mode — no backend is constructed, no data egresses.
///
/// NOTE: the canonical home for `AiMode`/`BackendConfig` is the `llm-adapters`
/// crate (which owns all HTTP/JSON). This kernel-side mirror is landed here first
/// per the P41 wave; `llm-adapters` reuses it by import. The kernel still never
/// *constructs* a backend or touches the network — this type only parses operator
/// intent and pins loopback for `LocalOffline`. No new crate dependency is added
/// (compile-firewall: `std::env`/`std::fs` only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiMode {
    /// Mode 1 — no AI. The agent surface is absent; no backend is constructed.
    Off,
    /// Mode 2 — local offline. A loopback Ollama-style backend. Never leaves the node.
    LocalOffline,
    /// Mode 3 — connected. An explicit managed/remote backend with an explicit key.
    /// Both must be present or `from_env` refuses (never a partial fallback).
    Connected,
}

impl Default for AiMode {
    /// Fail-closed default: unset `DOWIZ_AI_MODE` ⇒ `Off`.
    fn default() -> Self {
        AiMode::Off
    }
}

/// Typed configuration failure surfaced at composition time — never a panic,
/// never a silent default-to-something-else (blueprint §2 `ConfigError`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// `DOWIZ_AI_MODE` set to an unrecognized value.
    UnknownMode(String),
    /// `Connected` requested without `DOWIZ_LLM_BASE_URL`.
    MissingBaseUrl,
    /// `Connected` requested without a readable key file at `DOWIZ_LLM_API_KEY_FILE`.
    MissingApiKey,
    /// `LocalOffline` with a non-loopback base URL — "local" that egresses is refused.
    NonLoopbackLocal(String),
}

#[cfg(feature = "std")]
impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::UnknownMode(m) => write!(f, "unknown DOWIZ_AI_MODE value: {m:?}"),
            ConfigError::MissingBaseUrl => {
                write!(f, "DOWIZ_AI_MODE=connected requires DOWIZ_LLM_BASE_URL")
            }
            ConfigError::MissingApiKey => write!(
                f,
                "DOWIZ_AI_MODE=connected requires DOWIZ_LLM_API_KEY_FILE (a readable key file)"
            ),
            ConfigError::NonLoopbackLocal(u) => {
                write!(f, "DOWIZ_AI_MODE=local base URL is not loopback: {u}")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ConfigError {}

/// Backend-selection config (P41 C-b). Resolved once per process from the
/// operator environment; the selected `LlmBackend` is built by the consumer
/// (`llm-adapters`' composition layer) — this type carries only resolved intent.
///
/// Environment contract (single authority):
/// * `DOWIZ_AI_MODE` = `"off"` (DEFAULT when unset) | `"local"` | `"connected"`
/// * `DOWIZ_LLM_BASE_URL` — `Connected` only; required there. `LocalOffline` pins
///   loopback; a non-loopback URL under `local` is a typed `NonLoopbackLocal` error.
/// * `DOWIZ_LLM_API_KEY_FILE` — `Connected` only; path to a key file (never the key
///   itself in env — process-listing hygiene).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendConfig {
    pub mode: AiMode,
    /// Loopback (Local) or explicit remote (Connected). Empty for `Off`.
    pub base_url: String,
    /// `Connected` only, loaded from the key file. `None` for `Off`/`LocalOffline`.
    pub api_key: Option<String>,
}

impl BackendConfig {
    /// THE mode constructor. Unset `DOWIZ_AI_MODE` ⇒ `Off` (fail-closed default).
    /// Partial `Connected` config ⇒ `Err` — never "fall back to local", never
    /// "try the default remote" (both would be silent mode changes).
    ///
    /// This is the ONLY place `AiMode::Connected` is constructed from real
    /// environment input; combined with the anti-escalation grep guard that is
    /// the structural guarantee that mode cannot auto-escalate.
    #[cfg(feature = "std")]
    pub fn from_env() -> Result<BackendConfig, ConfigError> {
        Self::from_env_get(|k| std::env::var(k).ok())
    }

    /// Resolver core used by `from_env` and by the tests (dependency-injected read,
    /// so parsing is fully deterministic and free of global env-var mutation).
    /// Always available (no_std-safe): the only std dependency is `from_env` itself.
    pub fn from_env_get(
        read: impl Fn(&str) -> Option<String>,
    ) -> Result<BackendConfig, ConfigError> {
        let mode = match read("DOWIZ_AI_MODE").as_deref() {
            None | Some("") | Some("off") => AiMode::Off,
            Some("local") => AiMode::LocalOffline,
            Some("connected") => AiMode::Connected,
            Some(other) => return Err(ConfigError::UnknownMode(other.to_string())),
        };

        match mode {
            AiMode::Off => Ok(BackendConfig {
                mode,
                base_url: String::new(),
                api_key: None,
            }),
            AiMode::LocalOffline => {
                // Local pins loopback. If a base URL is supplied it MUST be loopback;
                // a non-loopback "local" is a lie the type refuses.
                let base = read("DOWIZ_LLM_BASE_URL")
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "http://127.0.0.1:11434".to_string());
                if !is_loopback(&base) {
                    return Err(ConfigError::NonLoopbackLocal(base));
                }
                Ok(BackendConfig {
                    mode,
                    base_url: base,
                    api_key: None,
                })
            }
            AiMode::Connected => {
                let base = read("DOWIZ_LLM_BASE_URL")
                    .filter(|s| !s.is_empty())
                    .ok_or(ConfigError::MissingBaseUrl)?;
                let key_file = read("DOWIZ_LLM_API_KEY_FILE")
                    .filter(|s| !s.is_empty())
                    .ok_or(ConfigError::MissingApiKey)?;
                let api_key = read_key_file(&key_file).ok_or(ConfigError::MissingApiKey)?;
                Ok(BackendConfig {
                    mode,
                    base_url: base,
                    api_key: Some(api_key),
                })
            }
        }
    }
}

/// True iff `url` resolves to a loopback host (`127.0.0.1`, `localhost`, `::1`).
/// Used to enforce the `LocalOffline` no-egress invariant without any DNS (pure std).
#[cfg(feature = "std")]
fn is_loopback(url: &str) -> bool {
    match host_of(url) {
        Some(host) => {
            host == "127.0.0.1" || host == "localhost" || host == "::1" || host == "[::1]"
        }
        None => false,
    }
}

/// Extract the host portion of a `scheme://host[:port][/...]` URL with no regex/crate.
/// Returns `None` for malformed input (treated as "not loopback" → refused by caller).
#[cfg(feature = "std")]
fn host_of(url: &str) -> Option<String> {
    let without_scheme = url.split_once("://")?.1;
    let authority = without_scheme.split('/').next().unwrap_or("");
    let host = authority.split(':').next().unwrap_or("");
    if host.is_empty() {
        None
    } else {
        Some(host.to_string())
    }
}

/// Read a key from a file path. Fail-closed: an unreadable file yields `None`
/// (→ `ConfigError::MissingApiKey`); the key is never placed in the process env.
#[cfg(feature = "std")]
fn read_key_file(path: &str) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Empty env reader — models "no DOWIZ_* variables set anywhere".
    fn empty(_: &str) -> Option<String> {
        None
    }

    /// Env reader backed by a fixed map; models an explicit operator environment.
    fn env<'a>(map: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |k: &str| map.iter().find(|(key, _)| *key == k).map(|(_, v)| v.to_string())
    }

    // ── (a) default Off when env absent ────────────────────────────────────────
    #[test]
    fn unset_mode_is_off_fail_closed_default() {
        let cfg = BackendConfig::from_env_get(empty).expect("unset env must parse");
        assert_eq!(cfg.mode, AiMode::Off, "unset DOWIZ_AI_MODE ⇒ Off");
        assert_eq!(cfg.base_url, "");
        assert_eq!(cfg.api_key, None);
    }

    // ── (b) partial env → Off (fail-closed) ────────────────────────────────────
    // Leftover backend vars WITHOUT an explicit mode must NOT infer `Connected`
    // or `LocalOffline`; they stay Off (the safe default). This is the
    // fail-closed default, not a silent mode change.
    #[test]
    fn partial_env_with_extra_vars_still_off() {
        // base url + key file present, but no DOWIZ_AI_MODE ⇒ Off.
        let cfg = BackendConfig::from_env_get(env(&[
            ("DOWIZ_LLM_BASE_URL", "https://api.example.com/v1"),
            ("DOWIZ_LLM_API_KEY_FILE", "/tmp/never-read.key"),
        ]))
        .expect("leftover vars without mode must parse to Off, never escalate");
        assert_eq!(cfg.mode, AiMode::Off, "no mode set ⇒ Off even with trailing vars");
        assert_eq!(cfg.api_key, None, "key file is never read when mode is Off");
    }

    // ── (c) explicit mode parses correctly ────────────────────────────────────
    #[test]
    fn explicit_off_local_connected_parse() {
        // explicit off
        let off = BackendConfig::from_env_get(env(&[("DOWIZ_AI_MODE", "off")])).unwrap();
        assert_eq!(off.mode, AiMode::Off);

        // local, no explicit base ⇒ loopback default pinned
        let local = BackendConfig::from_env_get(env(&[("DOWIZ_AI_MODE", "local")])).unwrap();
        assert_eq!(local.mode, AiMode::LocalOffline);
        assert!(is_loopback(&local.base_url), "local default must be loopback");
        assert_eq!(local.api_key, None);

        // local, explicit loopback base
        let local2 = BackendConfig::from_env_get(env(&[
            ("DOWIZ_AI_MODE", "local"),
            ("DOWIZ_LLM_BASE_URL", "http://localhost:11434"),
        ]))
        .unwrap();
        assert_eq!(local2.mode, AiMode::LocalOffline);
        assert_eq!(local2.base_url, "http://localhost:11434");
    }

    #[test]
    fn explicit_connected_with_key_file_parses() {
        // write a temp key file so the connected path is fully satisfied.
        let dir = std::env::temp_dir();
        let key_path = dir.join("p41_aimode_test_key.txt");
        std::fs::write(&key_path, "  sk-test-123\n  ").unwrap();
        let cfg = BackendConfig::from_env_get(env(&[
            ("DOWIZ_AI_MODE", "connected"),
            ("DOWIZ_LLM_BASE_URL", "https://api.example.com/v1"),
            (
                "DOWIZ_LLM_API_KEY_FILE",
                key_path.to_str().unwrap(),
            ),
        ]))
        .expect("fully-specified connected config must parse");
        assert_eq!(cfg.mode, AiMode::Connected);
        assert_eq!(cfg.base_url, "https://api.example.com/v1");
        assert_eq!(cfg.api_key.as_deref(), Some("sk-test-123"));
        let _ = std::fs::remove_file(&key_path);
    }

    // ── fail-closed teeth: partial Connected ⇒ typed Err, NEVER a silent fallback ─
    #[test]
    fn junk_mode_is_typed_error() {
        let err = BackendConfig::from_env_get(env(&[("DOWIZ_AI_MODE", "turbo")])).unwrap_err();
        assert_eq!(err, ConfigError::UnknownMode("turbo".to_string()));
    }

    #[test]
    fn connected_without_base_url_refused_never_local_fallback() {
        let dir = std::env::temp_dir();
        let key_path = dir.join("p41_aimode_test_key2.txt");
        std::fs::write(&key_path, "sk-test-456").unwrap();
        let err = BackendConfig::from_env_get(env(&[
            ("DOWIZ_AI_MODE", "connected"),
            ("DOWIZ_LLM_API_KEY_FILE", key_path.to_str().unwrap()),
        ]))
        .unwrap_err();
        assert_eq!(
            err,
            ConfigError::MissingBaseUrl,
            "connected without base url must be refused, never fall back to local"
        );
        let _ = std::fs::remove_file(&key_path);
    }

    #[test]
    fn connected_without_key_refused() {
        let err = BackendConfig::from_env_get(env(&[
            ("DOWIZ_AI_MODE", "connected"),
            ("DOWIZ_LLM_BASE_URL", "https://api.example.com/v1"),
        ]))
        .unwrap_err();
        assert_eq!(err, ConfigError::MissingApiKey);
    }

    #[test]
    fn connected_with_unreadable_key_file_refused() {
        let err = BackendConfig::from_env_get(env(&[
            ("DOWIZ_AI_MODE", "connected"),
            ("DOWIZ_LLM_BASE_URL", "https://api.example.com/v1"),
            ("DOWIZ_LLM_API_KEY_FILE", "/nonexistent/and/unreadable/key.file"),
        ]))
        .unwrap_err();
        assert_eq!(err, ConfigError::MissingApiKey);
    }

    #[test]
    fn local_with_remote_url_refused() {
        let err = BackendConfig::from_env_get(env(&[
            ("DOWIZ_AI_MODE", "local"),
            ("DOWIZ_LLM_BASE_URL", "https://api.example.com"),
        ]))
        .unwrap_err();
        assert_eq!(
            err,
            ConfigError::NonLoopbackLocal("https://api.example.com".to_string()),
            "\"local\" that egresses is refused by type"
        );
    }

    // ── (d) Unavailable error still degrades per P40's contract ────────────────
    // The degradation primitive is unchanged: a backend whose `health()` fails
    // yields `LlmError::Unavailable` (fail-closed), propagated verbatim — never
    // a mock, never a panic. The agent loop (P40) maps this to
    // `LoopOutcome::AssistantUnavailable`.
    struct DeadBackend;

    impl LlmBackend for DeadBackend {
        fn id(&self) -> &str {
            "test:dead"
        }
        fn caps(&self) -> Caps {
            Caps::default()
        }
        fn chat(&self, _req: &ChatRequest) -> Result<ChatResponse, LlmError> {
            Err(LlmError::Unavailable)
        }
        fn embed(&self, _req: &EmbedRequest) -> Result<EmbedResponse, LlmError> {
            Err(LlmError::Unavailable)
        }
        fn rerank(&self, _req: &RerankRequest) -> Result<RerankResponse, LlmError> {
            Err(LlmError::Unavailable)
        }
        fn health(&self) -> Result<(), LlmError> {
            Err(LlmError::Unavailable)
        }
    }

    #[test]
    fn unavailable_error_propagates_fail_closed() {
        let b = DeadBackend;
        // The typed variant is exactly `Unavailable` (semantics untouched).
        assert_eq!(LlmError::Unavailable, LlmError::Unavailable);
        // health failure surfaces as Unavailable verbatim.
        assert_eq!(b.health(), Err(LlmError::Unavailable));
        // A chat attempt also degrades to Unavailable — no silent fallback value.
        let req = ChatRequest {
            task_class: TaskClass::General,
            ..Default::default()
        };
        assert!(
            matches!(b.chat(&req), Err(LlmError::Unavailable)),
            "chat degrades to Unavailable — no silent fallback value"
        );
    }
}
