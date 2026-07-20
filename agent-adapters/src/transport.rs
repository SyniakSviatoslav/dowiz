//! transport.rs — one generic JSON-RPC 2.0 transport, shared by every agent bridge.
//!
//! Holds NO framework knowledge (mirrors `OpenAiCompatTransport`): all MCP/A2A deltas
//! come from the `AgentQuirks` the caller passes. The byte IO is behind a [`RpcChannel`]
//! seam (stdio pipe, streamable-HTTP, or an in-memory test double) so the transport
//! itself is transport-agnostic and offline-testable. Maps JSON-RPC errors to typed
//! [`AgentError`] — never a mock.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use dowiz_kernel::ports::agent::AgentError;
use serde_json::{json, Value};

/// The byte-IO seam under the JSON-RPC framing. A real bridge supplies a stdio-pipe or
/// streamable-HTTP channel; tests supply [`MockChannel`].
pub trait RpcChannel {
    /// Send a framed JSON-RPC request (bytes) and return the response bytes. `Err` on any
    /// transport failure.
    fn request(&self, raw: &[u8]) -> Result<Vec<u8>, AgentError>;
}

/// A synchronous JSON-RPC 2.0 transport over any [`RpcChannel`].
pub struct JsonRpcTransport<C: RpcChannel> {
    channel: C,
    next_id: AtomicU64,
}

impl<C: RpcChannel> JsonRpcTransport<C> {
    /// Wrap a channel.
    pub fn new(channel: C) -> Self {
        JsonRpcTransport {
            channel,
            next_id: AtomicU64::new(1),
        }
    }

    /// Perform one JSON-RPC 2.0 call. Maps a JSON-RPC `error` object to a typed
    /// [`AgentError`]; returns the `result` value on success.
    pub fn call(&self, method: &str, params: Value) -> Result<Value, AgentError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let req = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        let bytes = serde_json::to_vec(&req).map_err(|e| AgentError::BadRequest(e.to_string()))?;
        let raw = self.channel.request(&bytes)?;
        let resp: Value =
            serde_json::from_slice(&raw).map_err(|e| AgentError::BadRequest(e.to_string()))?;
        if let Some(err) = resp.get("error") {
            let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(0);
            let msg = err
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("")
                .to_string();
            // JSON-RPC error → typed AgentError (a coarse, honest mapping).
            return Err(match code {
                -32601 => AgentError::Unsupported, // method not found
                -32602 => AgentError::BadRequest(msg),
                _ => AgentError::Refused(format!("jsonrpc error {code}: {msg}")),
            });
        }
        resp.get("result")
            .cloned()
            .ok_or_else(|| AgentError::BadRequest("missing result".into()))
    }

    /// Borrow the underlying channel (tests inspect the mock).
    pub fn channel(&self) -> &C {
        &self.channel
    }
}

/// An in-memory `RpcChannel` for offline tests. Canned responses per method + a call log.
/// Mutable state (responses/log) is behind a `Mutex` so `request(&self, ...)` matches the
/// real (shared-ref) trait shape.
#[derive(Default)]
pub struct MockChannel {
    inner: Mutex<MockState>,
}

#[derive(Default)]
struct MockState {
    /// method → canned JSON `result` value.
    responses: std::collections::BTreeMap<String, Value>,
    /// method → canned JSON-RPC `error` object.
    errors: std::collections::BTreeMap<String, Value>,
    /// Ordered log of called methods.
    calls: Vec<String>,
}

impl MockChannel {
    /// Empty mock.
    pub fn new() -> Self {
        MockChannel::default()
    }
    /// Register a canned `result` for `method`.
    pub fn with_result(self, method: &str, result: Value) -> Self {
        self.inner
            .lock()
            .unwrap()
            .responses
            .insert(method.to_string(), result);
        self
    }
    /// Register a canned JSON-RPC error object for `method`.
    pub fn with_error(self, method: &str, code: i64, message: &str) -> Self {
        self.inner.lock().unwrap().errors.insert(
            method.to_string(),
            json!({ "code": code, "message": message }),
        );
        self
    }
    /// Replace a method's canned result at runtime (e.g. the server changes its tool list).
    pub fn set_result(&self, method: &str, result: Value) {
        self.inner
            .lock()
            .unwrap()
            .responses
            .insert(method.to_string(), result);
    }
    /// The ordered method-call log.
    pub fn calls(&self) -> Vec<String> {
        self.inner.lock().unwrap().calls.clone()
    }
}

impl RpcChannel for MockChannel {
    fn request(&self, raw: &[u8]) -> Result<Vec<u8>, AgentError> {
        let req: Value =
            serde_json::from_slice(raw).map_err(|e| AgentError::BadRequest(e.to_string()))?;
        let method = req
            .get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("")
            .to_string();
        let id = req.get("id").cloned().unwrap_or(json!(0));
        let mut st = self.inner.lock().unwrap();
        st.calls.push(method.clone());
        if let Some(err) = st.errors.get(&method).cloned() {
            let resp = json!({ "jsonrpc": "2.0", "id": id, "error": err });
            return Ok(serde_json::to_vec(&resp).unwrap());
        }
        let result = st.responses.get(&method).cloned().unwrap_or(json!(null));
        let resp = json!({ "jsonrpc": "2.0", "id": id, "result": result });
        Ok(serde_json::to_vec(&resp).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jsonrpc_result_and_error_mapping() {
        let ch = MockChannel::new()
            .with_result("ping", json!({"ok": true}))
            .with_error("secret", -32601, "method not found");
        let t = JsonRpcTransport::new(ch);
        assert_eq!(t.call("ping", json!({})).unwrap(), json!({"ok": true}));
        assert_eq!(t.call("secret", json!({})), Err(AgentError::Unsupported));
        assert_eq!(
            t.channel().calls(),
            vec!["ping".to_string(), "secret".to_string()]
        );
    }
}
