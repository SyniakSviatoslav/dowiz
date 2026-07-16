//! Integration test: real (non-mocked) roundtrip against the already-running Ollama daemon.
//! Requires Ollama live at 127.0.0.1:11434 (it is, per the plan's ground truth §1.2).
//! Run: `cargo test --test ollama_roundtrip` from the llm-adapters dir.

use llm_adapters::ollama::OllamaAdapter;
use llm_adapters::transport::message;
use dowiz_kernel::ports::llm::{ChatRequest, EmbedRequest, LlmBackend, LlmError, TaskClass};

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
    let resp = o.chat(&req).expect("chat should succeed against live Ollama");
    assert!(!resp.content.is_empty(), "response content must be non-empty");
    assert!(resp.usage.total_tokens > 0, "usage.total_tokens must be reported by Ollama");
}

#[test]
fn ollama_embed_roundtrip() {
    let o = OllamaAdapter::new(BASE);
    let req = EmbedRequest {
        model_id: "nomic-embed-text".to_string(),
        input: "the quick brown fox".to_string(),
    };
    let resp = o.embed(&req).expect("embed should succeed against live Ollama");
    assert!(!resp.embedding.is_empty(), "embedding vector must be non-empty");
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
