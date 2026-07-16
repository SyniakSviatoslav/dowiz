//! Native benchmark harness for llm-adapters — OFFLINE paths only.
//!
//! Per the AGENTS.md rule, harness benches must NOT depend on the live Ollama daemon
//! (host/noisy → not a committed baseline). The cache exact-hit decode is the deterministic
//! hot path: every repeated chat prompt short-circuits here, so its cost MUST be tracked.
//! Live Ollama latency is covered by the `ollama_roundtrip` integration test (a probe, pass/fail),
//! not by a baseline-gated bench.
//!
//! Run: `cargo bench` (from llm-adapters/) — parsed by ../kernel/benches/bench_track.py.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dowiz_kernel::backup::MemStore;
use dowiz_kernel::ports::llm::{CachePolicy, ChatRequest, LlmBackend, LlmError, Message, TaskClass};
use llm_adapters::cache::CachingBackend;
use std::collections::BTreeMap;

/// Build a deterministic chat request (temperature 0 → cacheable under `Exact`).
fn req() -> ChatRequest {
    ChatRequest {
        model_id: "qwen2.5-coder:7b".into(),
        messages: vec![Message {
            role: "user".into(),
            content: "deterministic cache decode probe".into(),
        }],
        max_tokens: 8,
        temperature: 0.0,
        top_p: 1.0,
        seed: None,
        task_class: TaskClass::General,
        cache_policy: CachePolicy::Exact,
        options: BTreeMap::new(),
        ..Default::default()
    }
}

/// The cache HIT path: encode → store → decode. Isolates the (de)serialization cost that every
/// repeated prompt pays instead of an HTTP round-trip. Offline, deterministic.
fn bench_cache_decode(c: &mut Criterion) {
    let backend = CachingBackend::with_store(FakeBackend, MemStore::new());
    let r = req();
    // Prime the cache once (encode + store), then bench the decode-on-hit path directly.
    let _ = backend.chat(&r);
    c.bench_function("cache/exact_hit_decode", |b| {
        b.iter(|| black_box(backend.chat(&r)))
    });
}

/// A backend that returns a fixed response and counts calls (not used for timing, just to prime).
struct FakeBackend;
impl LlmBackend for FakeBackend {
    fn id(&self) -> &str {
        "fake"
    }
    fn caps(&self) -> dowiz_kernel::ports::llm::Caps {
        dowiz_kernel::ports::llm::Caps {
            chat: true,
            embed: false,
            rerank: false,
            tool_calling: false,
        }
    }
    fn chat(
        &self,
        _req: &ChatRequest,
    ) -> Result<dowiz_kernel::ports::llm::ChatResponse, LlmError> {
        Ok(dowiz_kernel::ports::llm::ChatResponse {
            content: "cached-answer".into(),
            usage: dowiz_kernel::ports::llm::Usage {
                prompt_tokens: 4,
                completion_tokens: 2,
                total_tokens: 6,
            },
        })
    }
    fn embed(
        &self,
        _req: &dowiz_kernel::ports::llm::EmbedRequest,
    ) -> Result<dowiz_kernel::ports::llm::EmbedResponse, LlmError> {
        Err(LlmError::Unsupported)
    }
    fn rerank(
        &self,
        _req: &dowiz_kernel::ports::llm::RerankRequest,
    ) -> Result<dowiz_kernel::ports::llm::RerankResponse, LlmError> {
        Err(LlmError::Unsupported)
    }
    fn health(&self) -> Result<(), LlmError> {
        Ok(())
    }
}

criterion_group!(benches, bench_cache_decode);
criterion_main!(benches);
