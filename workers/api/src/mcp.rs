//! The hub as an MCP server: `POST /api/mcp`.
//!
//! Model Context Protocol over its Streamable HTTP transport, the JSON-RPC
//! half only: a client POSTs `initialize`, `tools/list`, `tools/call`, and
//! reads one JSON response. No SSE stream is opened, because nothing here
//! pushes; every answer is the answer to a call. That is enough for Claude
//! Desktop, Claude Code, Codex, Gemini CLI, Cursor and any agent runtime that speaks MCP.
//!
//! **Authority is a key, exchanged at this door, and the key decides the role.**
//! An owner's key (`dowiz_…`, the console's API keys) becomes a five-minute
//! owner token; a member of staff's (`dowizs_…`) or a courier's (`dowizc_…`),
//! minted in their own app (`keys.rs`), becomes a five-minute token of THAT
//! person (`rolekey.rs`). Every tool is then a plain call to a route that
//! person's own app uses, made by this Worker to itself with that token, and
//! each role is offered only its own tools (`tools.rs`). So an agent can do
//! exactly what its person can and nothing more, and the routes stay the one
//! place where that is decided.

mod keys;
mod rolekey;
mod tools;
#[cfg(test)]
mod tests;

pub use keys::{courier_list, courier_mint, courier_revoke, owner_list, owner_revoke, staff_list, staff_mint, staff_revoke};
pub(crate) use tools::enc;

use serde_json::{json, Value};
use worker::*;

use crate::auth::{self, Claims, Principal};
use tools::Role;

/// The MCP protocol revision this server speaks.
const PROTOCOL_VERSION: &str = "2025-06-18";
const SERVER_NAME: &str = "dowiz";
/// JSON-RPC error codes, from the specification.
const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
/// What an agent reads on `initialize`.
const INSTRUCTIONS: &str = "This is one restaurant's dowiz hub. Money is an integer in the venue's \
currency minor units (ALL has none: 1500 means 1500 lek). The tools are the ones the key's person \
may use: an owner's, a member of staff's (waiter, kitchen, counter-manager) or a courier's. Order \
actions must follow the order's status. Every read is live; nothing is cached.";

/// For the integrations screen: how many OWNER tools, and their names.
pub(crate) fn tool_count() -> usize { tools::OWNER_TOOLS.len() }
pub(crate) fn tool_names() -> Vec<&'static str> { tools::OWNER_TOOLS.iter().map(|t| t.name).collect() }

/// The principal at the door, as the role whose tools it gets and the venue
/// they act on. PURE, so the refusals are proved without an `Env`.
fn role_of(p: &Principal) -> std::result::Result<(Role, String), (u16, &'static str)> {
    match p {
        Principal::Owner { active_location_id: Some(l), .. } => Ok((Role::Owner, l.clone())),
        Principal::Owner { active_location_id: None, .. } => Err((403, "this key names no venue")),
        Principal::Staff { caps, active_location_id, .. } => Ok((Role::Staff(*caps), active_location_id.clone())),
        Principal::Courier { active_location_id, .. } => Ok((Role::Courier, active_location_id.clone())),
        Principal::Customer { .. } => Err((403, "an agent key is required")),
    }
}

fn rpc_error(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message.into() } })
}

fn rpc_result(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

struct Caller<'a> {
    env: &'a Env,
    origin: String,
    token: String,
    role: Role,
    location_id: String,
    slug: String,
}

/// The tool as one call to this Worker's own route, dispatched IN PROCESS.
///
/// Not a fetch of the public URL: a Worker fetching its own hostname is
/// refused by the edge (measured 2026-09-19: every tool answered `522`).
/// `crate::route` is the same router `main` runs, given a synthetic request
/// with the short-lived token, so a tool is exactly one request of the app.
async fn call_tool(caller: &Caller<'_>, name: &str, args: Value) -> std::result::Result<Value, String> {
    // ONLY THIS ROLE'S LIST: a waiter's agent naming an owner tool is told it
    // does not exist, before any route is asked.
    let Some(tool) = tools::find(caller.role, name) else {
        return Err(format!("unknown tool: {name}"));
    };
    let p = tools::plan(tool, args, &caller.location_id, &caller.slug, &caller.origin)?;
    let headers = Headers::new();
    headers.set("authorization", &format!("Bearer {}", caller.token)).map_err(|e| e.to_string())?;
    headers.set("accept", "application/json").map_err(|e| e.to_string())?;
    let mut init = RequestInit::new();
    match p.body {
        Some(b) => {
            headers.set("content-type", "application/json").map_err(|e| e.to_string())?;
            init.with_method(Method::Post).with_headers(headers).with_body(Some(b.into()));
        }
        None => {
            init.with_method(Method::Get).with_headers(headers);
        }
    }
    let r = Request::new_with_init(&p.url, &init).map_err(|e| e.to_string())?;
    let mut res = crate::route(r, caller.env.clone()).await.map_err(|e| e.to_string())?;
    let status = res.status_code();
    let text = res.text().await.unwrap_or_default();
    let mut v: Value = serde_json::from_str(&text).unwrap_or(json!({ "text": text }));
    if status >= 400 {
        return Err(format!("{status}: {}", v.get("error").and_then(Value::as_str).or(v.get("text").and_then(Value::as_str)).unwrap_or(&text)));
    }
    if let Some(want) = p.status_filter {
        if let Some(list) = v.get_mut("orders").and_then(Value::as_array_mut) {
            list.retain(|o| o.get("status").and_then(Value::as_str) == Some(want.as_str()));
        }
    }
    Ok(v)
}

async fn handle_one(caller: &Caller<'_>, msg: &Value) -> Option<Value> {
    let id = msg.get("id").cloned().unwrap_or(Value::Null);
    let Some(method) = msg.get("method").and_then(Value::as_str) else {
        return Some(rpc_error(id, INVALID_REQUEST, "no method"));
    };
    let params = msg.get("params").cloned().unwrap_or(Value::Null);
    // A notification carries no id and gets no answer.
    let is_notification = msg.get("id").is_none();
    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": SERVER_NAME, "version": env!("CARGO_PKG_VERSION") },
            "instructions": INSTRUCTIONS,
        })),
        "notifications/initialized" | "notifications/cancelled" => return None,
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools::listed(caller.role) })),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(Value::Null);
            if name.is_empty() {
                Err((INVALID_PARAMS, "tools/call needs a name".to_string()))
            } else {
                match call_tool(caller, name, args).await {
                    Ok(v) => Ok(json!({
                        "content": [{ "type": "text", "text": v.to_string() }],
                        "structuredContent": v,
                        "isError": false,
                    })),
                    // A tool's failure is a RESULT with isError, per the spec: the
                    // model reads it and tries again; a protocol error would stop it.
                    Err(e) => Ok(json!({ "content": [{ "type": "text", "text": e }], "isError": true })),
                }
            }
        }
        _ => Err((METHOD_NOT_FOUND, format!("unknown method: {method}"))),
    };
    if is_notification {
        return None;
    }
    Some(match result {
        Ok(v) => rpc_result(id, v),
        Err((code, m)) => rpc_error(id, code, m),
    })
}

/// `GET /api/mcp` — how to connect, for a person who opened the URL, and
/// the tools each role's key gets (every app's MCP panel reads `roles`).
pub async fn describe(req: Request, _ctx: RouteContext<crate::Req>) -> Result<Response> {
    let origin = req.url().map(|u| u.origin().ascii_serialization()).unwrap_or_default();
    Response::from_json(&describe_at(&origin))
}

fn describe_at(origin: &str) -> Value {
    json!({
        "name": SERVER_NAME,
        "protocolVersion": PROTOCOL_VERSION,
        "transport": "streamable-http",
        "endpoint": format!("{origin}/api/mcp"),
        "auth": "Authorization: Bearer <key>: dowiz_ (owner, console API keys), dowizs_ (staff) or dowizc_ (courier), minted in that person's own app",
        "tools": tool_names(),
        "roles": tools::by_role(),
    })
}

/// `POST /api/mcp`
pub async fn rpc(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let now = ctx.data.now_ms;
    let raw = match auth::bearer(&req) {
        Ok(r) => r,
        Err(e) => return e.into_response(),
    };
    // A PERSON'S KEY is exchanged for a token of that person; anything else
    // (an owner's key, or a token an app already holds) is authenticated as
    // itself.
    let (principal, minted) = match rolekey::parse(&raw) {
        Some((h, id, secret)) => match keys::exchange(&ctx.env, h, id, secret, now).await {
            Ok((p, t)) => (p, Some(t)),
            Err(e) => return e.into_response(),
        },
        None => match auth::authenticate_token(&raw, &ctx.env, now).await {
            Ok(p) => (p, None),
            Err(e) => return e.into_response(),
        },
    };
    let (role, location_id) = match role_of(&principal) {
        Ok(v) => v,
        Err((status, why)) => return Response::error(why, status),
    };
    let slug = crate::identity_store::rec(
        &crate::identity_store::registry(&ctx.env).await?,
        crate::identity_store::K_LOC,
        &location_id,
    )
    .map(|r| crate::identity_store::s_of(&r, "slug"))
    .unwrap_or_default();
    if slug.trim().is_empty() {
        return Response::error("this key names no venue", 403);
    }
    // THE TOKEN THE TOOLS CARRY. An owner's key is never forwarded as itself:
    // it is traded for a five-minute owner token, as before.
    let token = match (&principal, minted) {
        (_, Some(t)) => t,
        (Principal::Owner { user_id, .. }, None) => match auth::sign(
            &ctx.env,
            &Claims::Owner {
                sub: user_id.clone(),
                user_id: user_id.clone(),
                active_location_id: Some(location_id.clone()),
                iat: now,
                exp: now + rolekey::TOOL_TOKEN_TTL_MS,
            },
        ) {
            Ok(t) => t,
            Err(e) => return e.into_response(),
        },
        (_, None) => raw,
    };
    let caller = Caller {
        env: &ctx.env,
        origin: req.url()?.origin().ascii_serialization(),
        token,
        role,
        location_id,
        slug,
    };
    let body: Value = match req.json().await {
        Ok(v) => v,
        Err(_) => {
            let mut r = Response::from_json(&rpc_error(Value::Null, PARSE_ERROR, "the body is not JSON"))?;
            r.headers_mut().set("content-type", "application/json")?;
            return Ok(r);
        }
    };
    // A batch is answered as a batch; a lone notification gets 202 and nothing.
    let out = match body {
        Value::Array(list) => {
            let mut answers = Vec::new();
            for m in &list {
                if let Some(a) = handle_one(&caller, m).await {
                    answers.push(a);
                }
            }
            if answers.is_empty() { None } else { Some(Value::Array(answers)) }
        }
        other => handle_one(&caller, &other).await,
    };
    match out {
        None => Ok(Response::empty()?.with_status(202)),
        Some(v) => {
            let mut r = Response::from_json(&v)?;
            r.headers_mut().set("content-type", "application/json")?;
            Ok(r)
        }
    }
}
