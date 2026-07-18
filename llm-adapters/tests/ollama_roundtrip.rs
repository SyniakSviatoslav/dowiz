//! Integration test: real (non-mocked) roundtrip against the already-running Ollama daemon.
//! Requires Ollama live at 127.0.0.1:11434 (it is, per the plan's ground truth §1.2).
//! Run: `cargo test --test ollama_roundtrip` from the llm-adapters dir.

use dowiz_kernel::ports::llm::{ChatRequest, EmbedRequest, LlmBackend, LlmError, TaskClass};
use llm_adapters::ollama::OllamaAdapter;
use llm_adapters::transport::message;

const BASE: &str = "http://127.0.0.1:11434";

#[test]
fn ollama_chat_roundtrip() {
    let o = OllamaAdapter::new(BASE);
    // health must be Ok when the daemon is up.
    assert_eq!(o.health(), Ok(()), "Ollama should be live at {BASE}");

    let mut req = ChatRequest {
        task_class: TaskClass::General,
        messages: vec![message("user", "Reply with exactly the word: PONG")],
        temperature: 0.0,
        max_tokens: 16,
        ..Default::default()
    };
    req.model_id = "llama3.1:8b".to_string(); // :tag id passes through verbatim (quirk).
    let resp = o
        .chat(&req)
        .expect("chat should succeed against live Ollama");
    assert!(
        !resp.content.is_empty(),
        "response content must be non-empty"
    );
    assert!(
        resp.usage.total_tokens > 0,
        "usage.total_tokens must be reported by Ollama"
    );
}

#[test]
fn ollama_embed_roundtrip() {
    let o = OllamaAdapter::new(BASE);
    let req = EmbedRequest {
        model_id: "nomic-embed-text".to_string(),
        input: "the quick brown fox".to_string(),
    };
    let resp = o
        .embed(&req)
        .expect("embed should succeed against live Ollama");
    assert!(
        !resp.embedding.is_empty(),
        "embedding vector must be non-empty"
    );
    assert_eq!(resp.embedding.len(), 768, "nomic-embed-text is 768-dim");
}

#[test]
fn ollama_rerank_unsupported_fail_closed() {
    let o = OllamaAdapter::new(BASE);
    // No rerank endpoint wired → typed Err(Unsupported), never a panic/mock.
    let r = o.rerank(&dowiz_kernel::ports::llm::RerankRequest {
        model_id: "llama3.1:8b".to_string(),
        query: "x".into(),
        documents: vec!["a".into(), "b".into()],
    });
    assert!(matches!(r, Err(LlmError::Unsupported)));
}

// ── B-b: Caps.tool_calling un-pinned via a live per-model probe (§3.2) ────────
// The General-route model (llama3.1:8b) on this host advertises `tools` in its
// /api/show capabilities ⇒ caps().tool_calling must now be TRUE (the old pin was
// false). This is the GREEN side of the un-pin.
#[test]
fn tool_calling_probed_true_for_llama31() {
    let o = OllamaAdapter::new(BASE);
    // First, health must be up so the probe can run at all.
    assert_eq!(o.health(), Ok(()), "Ollama must be live for the probe");
    let caps = o.caps();
    assert!(
        caps.tool_calling,
        "llama3.1:8b advertises the 'tools' capability → tool_calling must be true"
    );
}

// nomic-embed-text is an embedding model with no `tools` capability, but a probe
// for it still must NOT error — it yields `false` (fail-closed discovery, not a
// panic). We probe the transport path directly so the test targets the probe
// semantics rather than the adapter's General-route routing.
#[test]
fn tool_calling_probed_false_for_embed_model() {
    use llm_adapters::transport::OpenAiCompatTransport;
    let t = OpenAiCompatTransport::new(BASE, llm_adapters::quirks::Quirks::ollama());
    // A live embed model's capabilities do not include "tools".
    let caps = t.show_capabilities("nomic-embed-text");
    match caps {
        Ok(list) => assert!(
            !list.iter().any(|c| c.eq_ignore_ascii_case("tools")),
            "nomic-embed-text must not advertise 'tools'"
        ),
        // A model that does not exist would fail the probe ⇒ Unsupported. That is
        // still fail-closed (we never assert a false positive), so tolerate it.
        Err(LlmError::Unsupported) => {}
        Err(e) => panic!("probe returned an unexpected error: {e:?}"),
    }
}

// Fail-closed on a probe that cannot succeed: a nonexistent model id makes
// `/api/show` fail, which the adapter maps to `false` (never a panic, never a
// stale `true`). This is the fail-closed half of the un-pin — a probe failure is
// indistinguishable from "no capability".
#[test]
fn probe_failure_maps_to_false_not_panic() {
    let o = OllamaAdapter::new(BASE);
    let bad_model_caps = o.probe_tool_calling("definitely-not-a-real-model:latest");
    assert!(
        !bad_model_caps,
        "probe failure must map to false (fail-closed)"
    );
}
