//! P40 Task A — the sibling agent-loop HTTP service (DECART-B).
//!
//! RED `agent_loop_service_boots_one_turn`: before this task there was no service
//! module → nothing bound a listener or drove a turn over HTTP. GREEN: bind a
//! localhost listener, POST a scripted `{"prompt":...}`, and assert the service
//! drives EXACTLY ONE bounded turn and returns a typed `LoopOutcome` as JSON,
//! within a wall-time budget (proving no hang).
//!
//! Backend: `llm-adapters`' internal `FakeBackend` is NOT exported, so this test
//! defines a tiny scripted `LlmBackend` (deterministic, offline). The live path
//! (real `OllamaAdapter`) needs a running daemon — that is an OPS precondition,
//! not a code defect; `src/main_service`-style wiring reuses `main.rs`'s adapter.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use agent_facade::{
    Caps, ChatRequest, ChatResponse, EmbedRequest, EmbedResponse, FixtureOrders, LlmBackend,
    LlmError, ReadOrderStatusTool, RerankRequest, RerankResponse, ToolAction, ToolResource,
    ToolScope, Usage,
};
use agent_loop::service;
use agent_loop::LoopOutcome;

/// A scripted, offline `LlmBackend`: healthy, tool-calling capable, and always
/// answers directly (empty `tool_calls`) so the loop returns `Answer` in ONE turn.
struct ScriptedBackend {
    answer: String,
    chat_calls: Arc<AtomicUsize>,
}

impl LlmBackend for ScriptedBackend {
    fn id(&self) -> &str {
        "scripted"
    }
    fn caps(&self) -> Caps {
        Caps {
            chat: true,
            embed: false,
            rerank: false,
            tool_calling: true,
        }
    }
    fn chat(&self, _req: &ChatRequest) -> Result<ChatResponse, LlmError> {
        self.chat_calls.fetch_add(1, Ordering::SeqCst);
        Ok(ChatResponse {
            content: self.answer.clone(),
            usage: Usage {
                prompt_tokens: 3,
                completion_tokens: 4,
                total_tokens: 7,
            },
            tool_calls: Vec::new(),
        })
    }
    fn embed(&self, _req: &EmbedRequest) -> Result<EmbedResponse, LlmError> {
        Err(LlmError::Unsupported)
    }
    fn rerank(&self, _req: &RerankRequest) -> Result<RerankResponse, LlmError> {
        Err(LlmError::Unsupported)
    }
    fn health(&self) -> Result<(), LlmError> {
        Ok(())
    }
}

fn granted_scope() -> ToolScope {
    ToolScope {
        resource: ToolResource::OrderStatus,
        action: ToolAction::Read,
    }
}

/// Raw HTTP POST helper (std-only; mirrors the SPA integration-test discipline).
fn http_post(addr: std::net::SocketAddr, path: &str, body: &str) -> String {
    let mut stream = TcpStream::connect(addr).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.as_bytes().len(),
        body
    );
    stream.write_all(req.as_bytes()).expect("write");
    stream.flush().unwrap();
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).expect("read");
    let text = String::from_utf8_lossy(&buf).into_owned();
    // Return the body (after the header block).
    match text.split_once("\r\n\r\n") {
        Some((_, body)) => body.to_string(),
        None => text,
    }
}

#[test]
fn agent_loop_service_boots_one_turn() {
    let chat_calls = Arc::new(AtomicUsize::new(0));
    let backend = ScriptedBackend {
        answer: "ORD-42 is IN_DELIVERY".to_string(),
        chat_calls: Arc::clone(&chat_calls),
    };
    let orders = FixtureOrders::from_pairs(&[("ORD-42", "IN_DELIVERY")]);
    let tool = ReadOrderStatusTool::new(orders);
    let granted = granted_scope();

    // Bind a real localhost listener on a free ephemeral port.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
    let addr = listener.local_addr().unwrap();

    // Serve exactly one connection on a background thread; capture the outcome.
    let handle =
        std::thread::spawn(move || service::serve_one(&listener, &backend, &tool, granted));

    // Give the accept loop a beat, then POST a scripted prompt with a wall budget.
    std::thread::sleep(Duration::from_millis(50));
    let start = Instant::now();
    let resp_body = http_post(
        addr,
        "/agent",
        r#"{"prompt":"what is the status of ORD-42?"}"#,
    );
    let elapsed = start.elapsed();

    // Bounded: one POST returns well within the wall-time budget (no hang).
    assert!(
        elapsed < Duration::from_secs(5),
        "one turn must return within budget, took {elapsed:?}"
    );

    // Typed outcome came back over the wire as JSON.
    assert!(
        resp_body.contains("\"outcome\":\"answer\""),
        "expected typed Answer outcome, got: {resp_body}"
    );
    assert!(
        resp_body.contains("ORD-42 is IN_DELIVERY"),
        "answer text must be serialized, got: {resp_body}"
    );

    // The service returned the same typed LoopOutcome (not a re-parse).
    let outcome = handle
        .join()
        .expect("service thread")
        .expect("serve_one io");
    match outcome {
        LoopOutcome::Answer { text, .. } => assert!(text.contains("IN_DELIVERY")),
        other => panic!("expected Answer, got {other:?}"),
    }

    // EXACTLY ONE bounded turn — one chat call, never a runaway loop.
    assert_eq!(
        chat_calls.load(Ordering::SeqCst),
        1,
        "exactly one bounded turn (one model call)"
    );
}
