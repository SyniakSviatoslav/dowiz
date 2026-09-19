//! The hub as an MCP server: `POST /api/mcp`.
//!
//! Model Context Protocol over its Streamable HTTP transport, the JSON-RPC
//! half only: a client POSTs `initialize`, `tools/list`, `tools/call`, and
//! reads one JSON response. No SSE stream is opened, because nothing here
//! pushes; every answer is the answer to a call. That is enough for Claude
//! Desktop, Claude Code, Cursor and any agent runtime that speaks MCP.
//!
//! **Authority is an API key, exchanged at this door.** The key
//! (`dowiz_…`, minted in the console under API keys) is checked once, here,
//! and traded for a five-minute owner token. Every tool is then a plain call
//! to the same `/api/owner/*` route the console uses, made by this Worker to
//! itself with that token. So a tool can do exactly what the console can and
//! nothing the console cannot, and there is one place where "what an agent
//! may do" is decided: the routes.

use serde_json::{json, Value};
use worker::*;

use crate::auth::{self, Claims, Principal};

/// The MCP protocol revision this server speaks.
const PROTOCOL_VERSION: &str = "2025-06-18";
const SERVER_NAME: &str = "dowiz";
/// A tool's token lives this long; a call takes seconds.
const TOOL_TOKEN_TTL_MS: i64 = 5 * 60 * 1000;
/// JSON-RPC error codes, from the specification.
const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
/// What an agent reads on `initialize`.
const INSTRUCTIONS: &str = "This is one restaurant's dowiz hub. Money is an integer in the venue's \
currency minor units (ALL has none: 1500 means 1500 lek). Order actions are \
confirm | preparing | ready | reject | cancel and must follow the order's status. \
Every read is live; nothing is cached.";

enum Verb {
    Get,
    Post,
}

/// One tool: its schema for `tools/list`, and the route it becomes.
struct Tool {
    name: &'static str,
    description: &'static str,
    /// JSON Schema of the arguments, as a string so the table stays readable.
    schema: &'static str,
    verb: Verb,
    /// The route, with `{id}` / `{peer}` / `{kind}` / `{code}` filled from the arguments.
    path: &'static str,
}

const TOOLS: &[Tool] = &[
    Tool { name: "dashboard", description: "Today's numbers and the venue's state (open/busy/closed, delivery paused).",
        schema: r#"{"type":"object","properties":{}}"#, verb: Verb::Get, path: "/api/owner/dashboard" },
    Tool { name: "orders", description: "Every order the hub holds, newest first, each with a live ETA (eta.range in minutes) and its status.",
        schema: r#"{"type":"object","properties":{"status":{"type":"string","description":"Optional filter, e.g. PENDING, PREPARING, READY, IN_DELIVERY"}}}"#, verb: Verb::Get, path: "/api/owner/orders" },
    Tool { name: "order_action", description: "Move an order: confirm, preparing, ready, reject (with reason) or cancel.",
        schema: r#"{"type":"object","required":["id","action"],"properties":{"id":{"type":"string"},"action":{"type":"string","enum":["confirm","preparing","ready","reject","cancel"]},"reason":{"type":"string"}}}"#, verb: Verb::Post, path: "/api/owner/orders/{id}/action" },
    Tool { name: "assign_courier", description: "Hand a confirmed/preparing/ready delivery order to a courier by id or phone.",
        schema: r#"{"type":"object","required":["id","courier_id"],"properties":{"id":{"type":"string"},"courier_id":{"type":"string"}}}"#, verb: Verb::Post, path: "/api/owner/orders/{id}/assign" },
    Tool { name: "couriers", description: "The venue's couriers, who is on shift, and whether each is active.",
        schema: r#"{"type":"object","properties":{}}"#, verb: Verb::Get, path: "/api/owner/couriers" },
    Tool { name: "menu", description: "The public menu as customers see it: categories, dishes, prices, availability, photos.",
        schema: r#"{"type":"object","properties":{"lang":{"type":"string","enum":["sq","en","uk"]}}}"#, verb: Verb::Get, path: "/api/public/locations/{slug}/menu" },
    Tool { name: "set_dish", description: "Change a dish: take it off sale or put it back (with a note customers see), change its price or cooking time.",
        schema: r#"{"type":"object","required":["id"],"properties":{"id":{"type":"string"},"available":{"type":"boolean"},"unavailable_note":{"type":"string"},"price":{"type":"integer"},"cooking_min":{"type":"integer"}}}"#, verb: Verb::Post, path: "/api/owner/products/{id}" },
    Tool { name: "stock", description: "Supplies the kitchen tracks, with available and reserved quantities and low marks.",
        schema: r#"{"type":"object","properties":{}}"#, verb: Verb::Get, path: "/api/owner/stock" },
    Tool { name: "stock_move", description: "Record a movement: received (qty), wasted (qty) or stocktake (observed).",
        schema: r#"{"type":"object","required":["kind","item"],"properties":{"kind":{"type":"string","enum":["received","wasted","stocktake"]},"item":{"type":"string"},"qty":{"type":"number"},"observed":{"type":"number"},"reason":{"type":"string"}}}"#, verb: Verb::Post, path: "/api/owner/stock/{kind}" },
    Tool { name: "analytics", description: "Orders, revenue, average check, by day and by hour, top dishes over the last N days (7 or 30).",
        schema: r#"{"type":"object","properties":{"days":{"type":"integer","enum":[7,30]}}}"#, verb: Verb::Get, path: "/api/owner/analytics" },
    Tool { name: "customers", description: "Returning customers: orders, spend, last seen.",
        schema: r#"{"type":"object","properties":{}}"#, verb: Verb::Get, path: "/api/owner/customers" },
    Tool { name: "promotions", description: "Promo codes and how used each is.",
        schema: r#"{"type":"object","properties":{}}"#, verb: Verb::Get, path: "/api/owner/promotions" },
    Tool { name: "create_promotion", description: "Create a promo code: percent or a fixed amount off, optional minimum order and use cap.",
        schema: r#"{"type":"object","required":["code","kind","value"],"properties":{"code":{"type":"string"},"kind":{"type":"string","enum":["percent","fixed"]},"value":{"type":"integer"},"minOrder":{"type":"integer"},"maxUses":{"type":"integer"}}}"#, verb: Verb::Post, path: "/api/owner/promotions" },
    Tool { name: "venue_state", description: "Open, mark busy or close the venue; pause or resume delivery.",
        schema: r#"{"type":"object","properties":{"status":{"type":"string","enum":["open","busy","closed"]},"delivery_paused":{"type":"boolean"}}}"#, verb: Verb::Post, path: "/api/owner/location" },
    Tool { name: "inbox", description: "Customer conversations from WhatsApp and Instagram, newest first, with unread counts.",
        schema: r#"{"type":"object","properties":{}}"#, verb: Verb::Get, path: "/api/owner/inbox" },
    Tool { name: "inbox_thread", description: "One conversation with a customer; opening it marks it read.",
        schema: r#"{"type":"object","required":["channel","peer"],"properties":{"channel":{"type":"string","enum":["whatsapp","instagram"]},"peer":{"type":"string"}}}"#, verb: Verb::Get, path: "/api/owner/inbox/{peer}" },
    Tool { name: "inbox_reply", description: "Answer a customer on the channel they wrote from.",
        schema: r#"{"type":"object","required":["channel","peer","text"],"properties":{"channel":{"type":"string","enum":["whatsapp","instagram"]},"peer":{"type":"string"},"text":{"type":"string"}}}"#, verb: Verb::Post, path: "/api/owner/inbox/{peer}" },
    Tool { name: "backup_to_cloud", description: "Push a full backup of the venue to its configured S3-compatible bucket now.",
        schema: r#"{"type":"object","properties":{}}"#, verb: Verb::Post, path: "/api/owner/backup/cloud" },
];

/// The POST handlers whose body names the venue. The others refuse unknown
/// fields (`deny_unknown_fields`), so `location_id` is injected only here.
const LOCATION_IN_BODY: &[&str] = &["order_action", "assign_courier", "set_dish", "venue_state", "inbox_reply"];

/// RFC 3986 unreserved characters pass; everything else is %XX. A local
/// twelve-line function rather than a crate, per the feature discipline.
pub(crate) fn enc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// For the integrations screen: how many tools, and their names.
pub(crate) fn tool_count() -> usize { TOOLS.len() }
pub(crate) fn tool_names() -> Vec<&'static str> { TOOLS.iter().map(|t| t.name).collect() }

fn rpc_error(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message.into() } })
}

fn rpc_result(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn tools_list() -> Value {
    json!(TOOLS
        .iter()
        .map(|t| json!({
            "name": t.name, "description": t.description,
            "inputSchema": serde_json::from_str::<Value>(t.schema).unwrap_or(json!({ "type": "object" })),
        }))
        .collect::<Vec<_>>())
}

/// Fill `{id}`-style holes from the arguments; an argument that fills a hole
/// leaves the body. `{slug}` is the venue's, supplied by the caller.
fn fill_path(path: &str, args: &mut serde_json::Map<String, Value>, slug: &str) -> std::result::Result<String, String> {
    let mut segs = Vec::new();
    for seg in path.split('/') {
        if let Some(hole) = seg.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
            let v = if hole == "slug" {
                slug.to_string()
            } else {
                match args.remove(hole) {
                    Some(Value::String(s)) => s,
                    Some(other) => other.to_string(),
                    None => return Err(format!("missing argument: {hole}")),
                }
            };
            segs.push(enc(&v));
        } else {
            segs.push(seg.to_string());
        }
    }
    Ok(segs.join("/"))
}

struct Caller<'a> {
    env: &'a Env,
    origin: String,
    token: String,
    location_id: String,
    slug: String,
}

/// The tool as one call to this Worker's own route, dispatched IN PROCESS.
///
/// Not a fetch of the public URL: a Worker fetching its own hostname is
/// refused by the edge (measured 2026-09-19: every tool answered `522`).
/// `crate::route` is the same router `main` runs, given a synthetic request
/// with the short-lived token, so a tool is exactly one console request.
async fn call_tool(caller: &Caller<'_>, name: &str, args: Value) -> std::result::Result<Value, String> {
    let Some(tool) = TOOLS.iter().find(|t| t.name == name) else {
        return Err(format!("unknown tool: {name}"));
    };
    let mut args = match args {
        Value::Object(m) => m,
        Value::Null => serde_json::Map::new(),
        _ => return Err("arguments must be an object".into()),
    };
    // `orders` filters after the fact; `menu` picks a language; neither is a route argument.
    let status_filter = if name == "orders" { args.remove("status").and_then(|v| v.as_str().map(str::to_uppercase)) } else { None };
    let lang = if name == "menu" { args.remove("lang").and_then(|v| v.as_str().map(str::to_string)) } else { None };
    let path = fill_path(tool.path, &mut args, &caller.slug)?;
    let headers = Headers::new();
    headers.set("authorization", &format!("Bearer {}", caller.token)).map_err(|e| e.to_string())?;
    headers.set("accept", "application/json").map_err(|e| e.to_string())?;
    let (url, method, body) = match tool.verb {
        Verb::Get => {
            let mut url = format!("{}{}?location_id={}", caller.origin, path, enc(&caller.location_id));
            for (k, v) in &args {
                let s = match v { Value::String(s) => s.clone(), other => other.to_string() };
                url.push_str(&format!("&{}={}", enc(k), enc(&s)));
            }
            if let Some(l) = &lang {
                url.push_str(&format!("&locale={}", enc(l)));
            }
            (url, Method::Get, None)
        }
        Verb::Post => {
            if LOCATION_IN_BODY.contains(&name) {
                args.insert("location_id".into(), json!(caller.location_id));
            }
            headers.set("content-type", "application/json").map_err(|e| e.to_string())?;
            (format!("{}{}", caller.origin, path), Method::Post, Some(Value::Object(args).to_string()))
        }
    };
    let mut init = RequestInit::new();
    init.with_method(method).with_headers(headers);
    if let Some(b) = body {
        init.with_body(Some(b.into()));
    }
    let r = Request::new_with_init(&url, &init).map_err(|e| e.to_string())?;
    let mut res = crate::route(r, caller.env.clone()).await.map_err(|e| e.to_string())?;
    let status = res.status_code();
    let text = res.text().await.unwrap_or_default();
    let mut v: Value = serde_json::from_str(&text).unwrap_or(json!({ "text": text }));
    if status >= 400 {
        return Err(format!("{status}: {}", v.get("error").and_then(Value::as_str).or(v.get("text").and_then(Value::as_str)).unwrap_or(&text)));
    }
    if let Some(want) = status_filter {
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
        "tools/list" => Ok(json!({ "tools": tools_list() })),
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

/// `GET /api/mcp` — how to connect, for a person who opened the URL.
pub async fn describe(req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    let origin = req.url().map(|u| u.origin().ascii_serialization()).unwrap_or_default();
    Response::from_json(&json!({
        "name": SERVER_NAME,
        "protocolVersion": PROTOCOL_VERSION,
        "transport": "streamable-http",
        "endpoint": format!("{origin}/api/mcp"),
        "auth": "Authorization: Bearer <API key from the console, starts with dowiz_>",
        "tools": TOOLS.iter().map(|t| t.name).collect::<Vec<_>>(),
    }))
}

/// `POST /api/mcp`
pub async fn rpc(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let now = crate::owner::now_ms();
    let principal = match auth::authenticate(&req, &ctx.env, &db, now).await {
        Ok(p) => p,
        Err(e) => return e.into_response(),
    };
    let Principal::Owner { user_id, active_location_id } = principal else {
        return Response::error("an owner's API key is required", 403);
    };
    let Some(location_id) = active_location_id else {
        return Response::error("this key names no venue", 403);
    };
    #[derive(serde::Deserialize)]
    struct Row {
        slug: String,
    }
    let slug = db
        .prepare("SELECT slug FROM locations WHERE id = ?1 LIMIT 1")
        .bind(&[location_id.clone().into()])?
        .first::<Row>(None)
        .await?
        .map(|r| r.slug)
        .unwrap_or_default();
    if slug.trim().is_empty() {
        return Response::error("this key names no venue", 403);
    }
    let token = match auth::sign(
        &ctx.env,
        &Claims::Owner {
            sub: user_id.clone(),
            user_id,
            active_location_id: Some(location_id.clone()),
            iat: now,
            exp: now + TOOL_TOKEN_TTL_MS,
        },
    ) {
        Ok(t) => t,
        Err(e) => return e.into_response(),
    };
    let caller = Caller {
        env: &ctx.env,
        origin: req.url()?.origin().ascii_serialization(),
        token,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tool_schema_parses_and_names_are_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for t in TOOLS {
            assert!(serde_json::from_str::<Value>(t.schema).is_ok(), "{}", t.name);
            assert!(seen.insert(t.name), "duplicate {}", t.name);
        }
    }

    #[test]
    fn holes_are_filled_from_arguments_and_removed() {
        let mut args = serde_json::Map::new();
        args.insert("id".into(), json!("a b"));
        args.insert("action".into(), json!("confirm"));
        let p = fill_path("/api/owner/orders/{id}/action", &mut args, "s").unwrap();
        assert_eq!(p, "/api/owner/orders/a%20b/action");
        assert!(args.get("id").is_none() && args.get("action").is_some());
        assert_eq!(fill_path("/api/public/locations/{slug}/menu", &mut args, "sushi").unwrap(), "/api/public/locations/sushi/menu");
        assert!(fill_path("/x/{peer}", &mut args, "s").is_err());
    }
}
