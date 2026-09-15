//! The venue's own MCP server.
//!
//! WHAT IT IS FOR. The operator asked whether each owner can have their own API
//! and MCP and connect to them. This is that: one hub is one venue, so its MCP
//! server IS the owner's — there is no multi-tenant routing to get wrong, no
//! shared namespace, and nothing another restaurant could reach. An owner points
//! their own MCP client at their own hub with their own key and gets tools over
//! their own data.
//!
//! IT IS THE SAME AUTHORITY AS EVERY OTHER SURFACE. A tool call goes through the
//! same handlers the admin pane uses, so the kernel still decides every
//! transition, the write lock is still the only writer, and an action taken from
//! a chat client lands in the same event log with the same actor recorded. There
//! is deliberately no back door here: whatever an MCP client cannot do through
//! the API, it cannot do at all.
//!
//! JSON-RPC 2.0 OVER ONE POST. The Streamable HTTP transport's core is a POST
//! that takes a request and returns a response; SSE streaming is for
//! server-initiated messages, which nothing here sends. Implementing the half
//! that is used, and refusing the half that is not, is smaller and more honest
//! than a partial stream nobody reads.

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{json, Value};

use crate::hub::{HubHttpError, Shared};
use crate::hubauth::Caller;
use dowiz_hub::token::Role;

/// The protocol revision this speaks.
const PROTOCOL_VERSION: &str = "2025-06-18";

/// What the tools are, declared once.
///
/// The SCHEMA is what an MCP client shows its user and validates against, so a
/// wrong one here is a tool that looks callable and is not. Written out rather
/// than derived, because there is no schema-generating dependency and because
/// the descriptions are the part a model actually reads -- they are documentation
/// for a reader who cannot ask a follow-up question.
fn tools() -> Vec<Value> {
    vec![
        json!({
            "name": "list_orders",
            "description": "List this venue's orders, newest first. Use for questions about \
                            what is happening now: how many are waiting, what is in the kitchen, \
                            what is out for delivery.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "status": { "type": "string",
                        "description": "Optional filter, e.g. PENDING, CONFIRMED, PREPARING, READY, IN_DELIVERY." },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 100,
                        "description": "How many to return. Defaults to 20." }
                }
            }
        }),
        json!({
            "name": "get_order",
            "description": "One order in full, including the customer's contact details and address.",
            "inputSchema": {
                "type": "object",
                "properties": { "id": { "type": "string" } },
                "required": ["id"]
            }
        }),
        json!({
            "name": "order_action",
            "description": "Move an order: confirm, preparing, ready, reject or cancel. The kernel \
                            decides whether the transition is legal and will refuse an illegal one. \
                            A rejection must carry a reason, which the customer is shown.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "action": { "type": "string",
                        "enum": ["confirm", "preparing", "ready", "reject", "cancel"] },
                    "reason": { "type": "string", "description": "Required for reject." }
                },
                "required": ["id", "action"]
            }
        }),
        json!({
            "name": "dashboard",
            "description": "Today's counts and takings: orders today, waiting, active, revenue in \
                            minor currency units.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "search_menu",
            "description": "Find dishes by name. Returns id, name, price and whether it is available.",
            "inputSchema": {
                "type": "object",
                "properties": { "query": { "type": "string" } },
                "required": ["query"]
            }
        }),
        json!({
            "name": "set_availability",
            "description": "Take a dish off the menu or put it back. The note is shown to \
                            customers while it is off.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "available": { "type": "boolean" },
                    "note": { "type": "string" }
                },
                "required": ["id", "available"]
            }
        }),
    ]
}

/// JSON-RPC error codes, as the spec numbers them.
const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
const INTERNAL_ERROR: i64 = -32603;

fn rpc_error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn rpc_ok(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

/// A tool result, in the shape MCP expects.
///
/// `isError` rather than a JSON-RPC error for a tool that ran and refused: the
/// distinction matters to a client, which shows a protocol error as "something
/// is broken" and a tool error as "that did not work, here is why". A kernel
/// refusing an illegal transition is the second kind.
fn tool_text(text: String, is_error: bool) -> Value {
    json!({ "content": [{ "type": "text", "text": text }], "isError": is_error })
}

/// `POST /mcp` — the whole transport.
pub async fn endpoint(
    State(st): State<Shared>,
    caller: Caller,
    body: String,
) -> Response {
    // MCP is the OWNER's surface. A courier token authenticates perfectly well
    // and must still be refused here: these tools reprice menus and move orders.
    if caller.person.role != Role::Owner {
        return (
            axum::http::StatusCode::FORBIDDEN,
            Json(rpc_error(Value::Null, INVALID_REQUEST, "this endpoint is for the venue owner")),
        )
            .into_response();
    }

    let req: Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => return Json(rpc_error(Value::Null, PARSE_ERROR, &e.to_string())).into_response(),
    };

    // A batch is an array. Handled by recursion rather than refused, because a
    // client that batches `initialize` with `tools/list` is doing something the
    // spec allows.
    if let Some(items) = req.as_array() {
        let mut out = Vec::new();
        for item in items {
            if let Some(r) = handle_one(&st, &caller, item).await {
                out.push(r);
            }
        }
        if out.is_empty() {
            return axum::http::StatusCode::ACCEPTED.into_response();
        }
        return Json(Value::Array(out)).into_response();
    }

    match handle_one(&st, &caller, &req).await {
        Some(r) => Json(r).into_response(),
        // A notification gets no body. 202 is what the transport specifies for
        // a request that produces no response.
        None => axum::http::StatusCode::ACCEPTED.into_response(),
    }
}

/// One JSON-RPC message. `None` for a notification, which must not be answered.
async fn handle_one(st: &Shared, caller: &Caller, req: &Value) -> Option<Value> {
    let id = req.get("id").cloned();
    let method = req.get("method").and_then(Value::as_str).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or(json!({}));

    // A message with no id is a NOTIFICATION. Answering one is a protocol
    // violation that some clients treat as a fatal error.
    let is_notification = id.is_none();
    let id = id.unwrap_or(Value::Null);

    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": "dowiz-hub", "version": env!("CARGO_PKG_VERSION") },
            "instructions": "This server belongs to ONE restaurant. Every tool acts on that \
                             venue's live data. Order transitions are decided by the kernel, not \
                             by this server, so an illegal one is refused rather than applied."
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools() })),
        "tools/call" => call_tool(st, caller, &params).await,
        // Declared unsupported rather than answered with an empty list: an empty
        // list says "this venue has no prompts", which is a different claim from
        // "this server does not do prompts".
        "notifications/initialized" | "notifications/cancelled" => Ok(json!({})),
        other => Err((METHOD_NOT_FOUND, format!("no method {other:?}"))),
    };

    if is_notification {
        return None;
    }
    Some(match result {
        Ok(v) => rpc_ok(id, v),
        Err((code, msg)) => rpc_error(id, code, &msg),
    })
}

type ToolResult = Result<Value, (i64, String)>;

async fn call_tool(st: &Shared, caller: &Caller, params: &Value) -> ToolResult {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or((INVALID_PARAMS, "no tool name".to_string()))?;
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    // A tool that RAN and refused reports through `isError`, not through a
    // JSON-RPC error -- see `tool_text`.
    let refused = |e: HubHttpError| Ok(tool_text(format!("{e:?}"), true));

    match name {
        "list_orders" => {
            let want = args.get("status").and_then(Value::as_str);
            let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(20).clamp(1, 100) as usize;
            let hub = match st.read_log() {
                Ok(h) => h,
                Err(e) => return refused(e),
            };
            let mut list: Vec<Value> = hub
                .orders()
                .iter()
                .filter_map(|ev| serde_json::from_str::<Value>(&ev.order_json).ok())
                .filter(|o| {
                    want.is_none_or(|w| {
                        o.get("status").and_then(Value::as_str).is_some_and(|s| s.eq_ignore_ascii_case(w))
                    })
                })
                .collect();
            list.sort_by_key(|o| -o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0));
            list.truncate(limit);
            Ok(tool_text(
                serde_json::to_string_pretty(&json!({ "orders": list })).unwrap_or_default(),
                false,
            ))
        }
        "get_order" => {
            let id = args.get("id").and_then(Value::as_str).ok_or((INVALID_PARAMS, "no id".into()))?;
            let hub = match st.read_log() {
                Ok(h) => h,
                Err(e) => return refused(e),
            };
            match hub.order(id) {
                Ok(raw) => Ok(tool_text(raw, false)),
                Err(_) => Ok(tool_text(format!("no order {id}"), true)),
            }
        }
        "order_action" => {
            let id = args.get("id").and_then(Value::as_str).ok_or((INVALID_PARAMS, "no id".into()))?;
            let action = args
                .get("action")
                .and_then(Value::as_str)
                .ok_or((INVALID_PARAMS, "no action".into()))?;
            let reason = args.get("reason").and_then(Value::as_str).map(str::to_string);
            // THE SAME HANDLER the admin pane calls. Not a parallel path: a
            // second implementation would be a second set of rules, and the one
            // reachable from a chat client would be the less-tested one.
            match crate::hubowner::apply_owner_action(st, &caller.person.id, id, action, reason).await
            {
                Ok(v) => Ok(tool_text(serde_json::to_string_pretty(&v).unwrap_or_default(), false)),
                Err(e) => refused(e),
            }
        }
        "dashboard" => match crate::hubowner::dashboard_facts(st).await {
            Ok(v) => Ok(tool_text(serde_json::to_string_pretty(&v).unwrap_or_default(), false)),
            Err(e) => refused(e),
        },
        "search_menu" => {
            let q = args
                .get("query")
                .and_then(Value::as_str)
                .ok_or((INVALID_PARAMS, "no query".into()))?
                .to_lowercase();
            let cat = match st.read_catalog() {
                Ok(c) => c,
                Err(e) => return refused(e),
            };
            let hits: Vec<Value> = cat
                .products()
                .into_iter()
                .filter_map(|(_, j)| serde_json::from_str::<Value>(&j).ok())
                .filter(|p| {
                    p.get("name")
                        .and_then(Value::as_str)
                        .is_some_and(|n| n.to_lowercase().contains(&q))
                })
                .map(|p| {
                    json!({
                        "id": p.get("id").cloned().unwrap_or(Value::Null),
                        "name": p.get("name").cloned().unwrap_or(Value::Null),
                        "price": p.get("price").cloned().unwrap_or(Value::Null),
                        "available": p.get("available").cloned().unwrap_or(Value::Null),
                    })
                })
                .take(50)
                .collect();
            Ok(tool_text(
                serde_json::to_string_pretty(&json!({ "matches": hits })).unwrap_or_default(),
                false,
            ))
        }
        "set_availability" => {
            let id = args.get("id").and_then(Value::as_str).ok_or((INVALID_PARAMS, "no id".into()))?;
            let available = args
                .get("available")
                .and_then(Value::as_bool)
                .ok_or((INVALID_PARAMS, "no available".into()))?;
            let note = args.get("note").and_then(Value::as_str).map(str::to_string);
            match crate::hubowner::set_product_availability(st, id, available, note).await {
                Ok(v) => Ok(tool_text(serde_json::to_string_pretty(&v).unwrap_or_default(), false)),
                Err(e) => refused(e),
            }
        }
        other => Err((INVALID_PARAMS, format!("no tool {other:?}"))),
    }
    .map(|v| json!(v))
}

pub fn routes(state: Shared) -> Router {
    Router::new().route("/mcp", post(endpoint)).with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every tool must declare a schema an MCP client can validate against, and
    /// a description a model can act on. A tool with an empty description is one
    /// that will be called wrongly.
    #[test]
    fn every_tool_is_fully_declared() {
        let t = tools();
        assert!(t.len() >= 6);
        for tool in &t {
            let name = tool["name"].as_str().expect("name");
            assert!(!name.is_empty());
            let desc = tool["description"].as_str().expect("description");
            assert!(desc.len() > 40, "{name} needs a real description, got {desc:?}");
            assert_eq!(tool["inputSchema"]["type"], "object", "{name}");
            // A declared `required` field must exist in `properties`, or the
            // client validates against a schema that can never be satisfied.
            if let Some(req) = tool["inputSchema"].get("required").and_then(Value::as_array) {
                for r in req {
                    let k = r.as_str().expect("required entry");
                    assert!(
                        tool["inputSchema"]["properties"].get(k).is_some(),
                        "{name}: required {k:?} is not in properties"
                    );
                }
            }
        }
    }

    #[test]
    fn tool_names_are_unique() {
        let all = tools();
        let mut names: Vec<&str> = all.iter().map(|t| t["name"].as_str().unwrap()).collect();
        names.sort_unstable();
        let n = names.len();
        names.dedup();
        assert_eq!(names.len(), n, "two tools share a name");
    }

    #[test]
    fn errors_use_the_spec_s_own_codes() {
        let e = rpc_error(json!(1), METHOD_NOT_FOUND, "nope");
        assert_eq!(e["jsonrpc"], "2.0");
        assert_eq!(e["id"], 1);
        assert_eq!(e["error"]["code"], -32601);
        assert_eq!(rpc_ok(json!("x"), json!({}))["jsonrpc"], "2.0");
    }

    /// A tool that ran and refused is NOT a protocol error: a client shows the
    /// first as "that did not work, here is why" and the second as "the server
    /// is broken".
    #[test]
    fn a_refusal_is_a_tool_error_not_a_protocol_error() {
        let t = tool_text("Illegal transition".into(), true);
        assert_eq!(t["isError"], true);
        assert_eq!(t["content"][0]["type"], "text");
        assert!(t["content"][0]["text"].as_str().unwrap().contains("Illegal"));
    }
}
