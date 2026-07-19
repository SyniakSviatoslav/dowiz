//! agent-loop sibling HTTP service (P40 DECART-B).
//!
//! A tiny std-only localhost HTTP surface over the bounded [`crate::AgentLoop`].
//! `native-spa-server` PROXIES `/api/agent` to THIS service (choice B), so the
//! SPA binary stays zero-OCI: the heavy `llm-adapters` dependency lives ONLY in
//! this crate/process, never in the SPA's Cargo graph.
//!
//! Wire contract (minimal): `POST /agent` with body `{"prompt":"..."}` →
//! `{"outcome": "...", "text": <string|null>, "log": [ ... ]}`. Every response is
//! a typed [`crate::LoopOutcome`] serialized; the executor bounds itself by
//! `MAX_AGENT_ITERATIONS` + `TokenBucket`, so one request = one bounded turn.
//!
//! No new dependency: the HTTP parse + JSON emit are std-only (matching the
//! `native-spa-server` integration tests' own raw `TcpStream` discipline). The
//! `dowiz-kernel` firewall is untouched — this module imports ONLY `agent_facade`
//! port types + `crate`.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

use agent_facade::{LlmBackend, ToolPort, ToolScope};

use crate::{AgentLoop, LoopEventKind, LoopLogEntry, LoopOutcome};

/// Serve exactly ONE connection: read a `POST /agent` request, drive one bounded
/// `AgentLoop::run`, write the typed outcome as JSON, close. Returns the outcome
/// so a test can assert on it without re-parsing the wire.
///
/// Bounded by construction: `AgentLoop::run` cannot loop past `MAX_AGENT_ITERATIONS`.
pub fn serve_one(
    listener: &TcpListener,
    backend: &dyn LlmBackend,
    tool: &dyn ToolPort,
    granted: ToolScope,
) -> std::io::Result<LoopOutcome> {
    let (stream, _peer) = listener.accept()?;
    handle_connection(stream, backend, tool, granted)
}

/// Serve requests forever (the sibling-service main loop). Each accepted
/// connection drives exactly one bounded turn; a per-connection error is logged
/// and the loop continues (no unbounded state, no self-scheduling).
pub fn serve_forever(
    listener: &TcpListener,
    backend: &dyn LlmBackend,
    tool: &dyn ToolPort,
    granted: ToolScope,
) -> std::io::Result<()> {
    for conn in listener.incoming() {
        match conn {
            Ok(stream) => {
                if let Err(e) = handle_connection(stream, backend, tool, granted) {
                    eprintln!("[agent-service] connection error: {e}");
                }
            }
            Err(e) => eprintln!("[agent-service] accept error: {e}"),
        }
    }
    Ok(())
}

/// Handle one HTTP connection: parse body → run one turn → emit JSON.
pub fn handle_connection(
    mut stream: TcpStream,
    backend: &dyn LlmBackend,
    tool: &dyn ToolPort,
    granted: ToolScope,
) -> std::io::Result<LoopOutcome> {
    let body = read_http_request_body(&mut stream)?;
    let prompt = extract_json_string_field(&body, "prompt").unwrap_or_default();

    let agent = AgentLoop::new(backend, tool, granted);
    let outcome = agent.run(&prompt);

    let json = outcome_to_json(&outcome);
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        json.as_bytes().len(),
        json
    );
    stream.write_all(resp.as_bytes())?;
    stream.flush()?;
    Ok(outcome)
}

/// Read an HTTP/1.1 request off the stream and return its body bytes as a String.
/// Honors `Content-Length`; std-only, no HTTP crate.
fn read_http_request_body(stream: &mut TcpStream) -> std::io::Result<String> {
    let mut buf: Vec<u8> = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];

    // Read until we have the full header block.
    let header_end = loop {
        if let Some(pos) = find_subslice(&buf, b"\r\n\r\n") {
            break pos + 4;
        }
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            // Connection closed before headers completed.
            return Ok(String::new());
        }
        buf.extend_from_slice(&chunk[..n]);
    };

    let headers = String::from_utf8_lossy(&buf[..header_end]);
    let content_length = headers
        .lines()
        .find_map(|l| {
            let (k, v) = l.split_once(':')?;
            if k.trim().eq_ignore_ascii_case("content-length") {
                v.trim().parse::<usize>().ok()
            } else {
                None
            }
        })
        .unwrap_or(0);

    // We already may have read part (or all) of the body.
    let mut body: Vec<u8> = buf[header_end..].to_vec();
    while body.len() < content_length {
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
    }
    body.truncate(content_length.max(body.len().min(content_length)));
    // If content_length was 0 but a body arrived, keep what we have.
    let take = if content_length == 0 { body.len() } else { content_length.min(body.len()) };
    Ok(String::from_utf8_lossy(&body[..take]).into_owned())
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|w| w == needle)
}

/// Extract a JSON string field value by key from a small JSON object. Handles
/// standard escapes (`\"`, `\\`, `\n`, `\r`, `\t`, `\/`). Std-only, minimal —
/// sufficient for the `{"prompt":"..."}` request shape.
fn extract_json_string_field(json: &str, key: &str) -> Option<String> {
    let bytes = json.as_bytes();
    let needle = format!("\"{key}\"");
    let mut i = json.find(&needle)? + needle.len();
    // skip whitespace and ':'
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b':' || bytes[i] == b'\n' || bytes[i] == b'\r') {
        i += 1;
    }
    if i >= bytes.len() || bytes[i] != b'"' {
        return None;
    }
    i += 1; // opening quote
    let mut out = String::new();
    while i < bytes.len() {
        let c = bytes[i];
        match c {
            b'"' => return Some(out),
            b'\\' => {
                i += 1;
                if i >= bytes.len() {
                    break;
                }
                match bytes[i] {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    other => out.push(other as char),
                }
            }
            _ => out.push(c as char),
        }
        i += 1;
    }
    None
}

/// Escape a string for inclusion in a JSON double-quoted value.
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Render one loop log entry to a human-readable line (event-driven, debug-friendly).
fn log_entry_to_string(e: &LoopLogEntry) -> String {
    let ev = match &e.event {
        LoopEventKind::ModelReply { content, total_tokens } => {
            format!("model_reply tokens={total_tokens} content={content:?}")
        }
        LoopEventKind::ToolCallParsed { tool_name, raw_arg } => {
            format!("tool_call_parsed {tool_name} arg={raw_arg:?}")
        }
        LoopEventKind::ToolCallMalformed { raw, reason } => {
            format!("tool_call_malformed reason={reason:?} raw={raw:?}")
        }
        LoopEventKind::ToolResult { tool_name, output } => {
            format!("tool_result {tool_name} output={output:?}")
        }
        LoopEventKind::ToolFailed { tool_name, error } => {
            format!("tool_failed {tool_name} error={error:?}")
        }
    };
    format!("iter{}: {ev}", e.iteration)
}

/// Serialize a [`LoopOutcome`] to the `AgentResponse` JSON shape
/// (`{"outcome","text","log"}`). NO money vocabulary appears here (P54 firewall).
pub fn outcome_to_json(outcome: &LoopOutcome) -> String {
    let (kind, text, log): (&str, Option<String>, &[LoopLogEntry]) = match outcome {
        LoopOutcome::Answer { text, log } => ("answer", Some(text.clone()), log.as_slice()),
        LoopOutcome::AssistantUnavailable { reason, log } => {
            ("unavailable", Some(reason.clone()), log.as_slice())
        }
        LoopOutcome::ToolCallingUnsupported { backend_id } => {
            ("tool_calling_unsupported", Some(backend_id.clone()), &[])
        }
        LoopOutcome::IterationCapExceeded { log } => ("cap_exceeded", None, log.as_slice()),
    };

    let text_json = match text {
        Some(t) => format!("\"{}\"", json_escape(&t)),
        None => "null".to_string(),
    };
    let log_json = log
        .iter()
        .map(|e| format!("\"{}\"", json_escape(&log_entry_to_string(e))))
        .collect::<Vec<_>>()
        .join(",");

    format!(
        "{{\"outcome\":\"{}\",\"text\":{},\"log\":[{}]}}",
        kind, text_json, log_json
    )
}
