//! PURE. Which tools each role's agent is offered, and the route each becomes.
//!
//! **A TOOL IS A ROUTE THE PERSON'S OWN APP ALREADY CALLS.** The owner's are
//! the console's `/api/owner/*`; a member of staff's are the room's
//! `/api/staff/*` (and, for the kitchen, the one owner route `staff_at` admits
//! with `advance`); a courier's are `/api/courier/*`. So the list is a
//! convenience and the ROUTE is the wall: a tool that slipped into the wrong
//! list would still be refused by the handler it calls. The list is filtered
//! anyway, so an agent is never shown a tool that can only fail — a waiter's
//! agent asking for `dashboard` is told there is no such tool.

use dowiz_hub::caps::{Cap, Caps, Preset};
use serde_json::{json, Value};

pub enum Verb {
    Get,
    Post,
}

/// One tool: its schema for `tools/list`, and the route it becomes.
pub struct Tool {
    pub name: &'static str,
    pub description: &'static str,
    /// JSON Schema of the arguments, as a string so the table stays readable.
    pub schema: &'static str,
    pub verb: Verb,
    /// The route, with `{id}`-style holes filled from the arguments.
    pub path: &'static str,
    /// A POST whose handler reads `location_id` from the body. The others
    /// refuse unknown fields or take the venue from the token.
    pub loc_in_body: bool,
    /// For a member of staff: the capability the route asks for. `None` is
    /// every role that has the table (the public menu).
    pub need: Option<Cap>,
}

/// Whose agent is calling. Decided at the door from the principal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Owner,
    Staff(Caps),
    Courier,
}

const NO_ARGS: &str = r#"{"type":"object","properties":{}}"#;
const MENU_SCHEMA: &str = r#"{"type":"object","properties":{"lang":{"type":"string","enum":["sq","en","uk"]}}}"#;
const MENU_PATH: &str = "/api/public/locations/{slug}/menu";
const MENU_TEXT: &str = "The public menu as customers see it: categories, dishes, prices, availability, photos.";

const fn get(name: &'static str, description: &'static str, schema: &'static str, path: &'static str, need: Option<Cap>) -> Tool {
    Tool { name, description, schema, verb: Verb::Get, path, loc_in_body: false, need }
}
const fn post(name: &'static str, description: &'static str, schema: &'static str, path: &'static str, loc_in_body: bool, need: Option<Cap>) -> Tool {
    Tool { name, description, schema, verb: Verb::Post, path, loc_in_body, need }
}

pub const OWNER_TOOLS: &[Tool] = &[
    get("dashboard", "Today's numbers and the venue's state (open/busy/closed, delivery paused).", NO_ARGS, "/api/owner/dashboard", None),
    get("orders", "Every order the hub holds, newest first, each with a live ETA (eta.range in minutes) and its status.",
        r#"{"type":"object","properties":{"status":{"type":"string","description":"Optional filter, e.g. PENDING, PREPARING, READY, IN_DELIVERY"}}}"#, "/api/owner/orders", None),
    post("order_action", "Move an order: confirm, preparing, ready, reject (with reason) or cancel.",
        r#"{"type":"object","required":["id","action"],"properties":{"id":{"type":"string"},"action":{"type":"string","enum":["confirm","preparing","ready","reject","cancel"]},"reason":{"type":"string"}}}"#, "/api/owner/orders/{id}/action", true, None),
    post("assign_courier", "Hand a confirmed/preparing/ready delivery order to a courier by id or phone.",
        r#"{"type":"object","required":["id","courier_id"],"properties":{"id":{"type":"string"},"courier_id":{"type":"string"}}}"#, "/api/owner/orders/{id}/assign", true, None),
    get("couriers", "The venue's couriers, who is on shift, and whether each is active.", NO_ARGS, "/api/owner/couriers", None),
    get("menu", MENU_TEXT, MENU_SCHEMA, MENU_PATH, None),
    post("set_dish", "Change a dish: take it off sale or put it back (with a note customers see), change its price or cooking time.",
        r#"{"type":"object","required":["id"],"properties":{"id":{"type":"string"},"available":{"type":"boolean"},"unavailable_note":{"type":"string"},"price":{"type":"integer"},"cooking_min":{"type":"integer"}}}"#, "/api/owner/products/{id}", true, None),
    get("stock", "Supplies the kitchen tracks, with available and reserved quantities and low marks.", NO_ARGS, "/api/owner/stock", None),
    post("stock_move", "Record a movement: received (qty), wasted (qty) or stocktake (observed).",
        r#"{"type":"object","required":["kind","item"],"properties":{"kind":{"type":"string","enum":["received","wasted","stocktake"]},"item":{"type":"string"},"qty":{"type":"number"},"observed":{"type":"number"},"reason":{"type":"string"}}}"#, "/api/owner/stock/{kind}", false, None),
    get("analytics", "Orders, revenue, average check, by day and by hour, top dishes over the last N days (7 or 30).",
        r#"{"type":"object","properties":{"days":{"type":"integer","enum":[7,30]}}}"#, "/api/owner/analytics", None),
    get("customers", "Returning customers: orders, spend, last seen.", NO_ARGS, "/api/owner/customers", None),
    get("promotions", "Promo codes and how used each is.", NO_ARGS, "/api/owner/promotions", None),
    post("create_promotion", "Create a promo code: percent or a fixed amount off, optional minimum order and use cap.",
        r#"{"type":"object","required":["code","kind","value"],"properties":{"code":{"type":"string"},"kind":{"type":"string","enum":["percent","fixed"]},"value":{"type":"integer"},"minOrder":{"type":"integer"},"maxUses":{"type":"integer"}}}"#, "/api/owner/promotions", false, None),
    post("venue_state", "Open, mark busy or close the venue; pause or resume delivery.",
        r#"{"type":"object","properties":{"status":{"type":"string","enum":["open","busy","closed"]},"delivery_paused":{"type":"boolean"}}}"#, "/api/owner/location", true, None),
    get("inbox", "Customer conversations from WhatsApp and Instagram, newest first, with unread counts.", NO_ARGS, "/api/owner/inbox", None),
    get("inbox_thread", "One conversation with a customer; opening it marks it read.",
        r#"{"type":"object","required":["channel","peer"],"properties":{"channel":{"type":"string","enum":["whatsapp","instagram"]},"peer":{"type":"string"}}}"#, "/api/owner/inbox/{peer}", None),
    post("inbox_reply", "Answer a customer on the channel they wrote from.",
        r#"{"type":"object","required":["channel","peer","text"],"properties":{"channel":{"type":"string","enum":["whatsapp","instagram"]},"peer":{"type":"string"},"text":{"type":"string"}}}"#, "/api/owner/inbox/{peer}", true, None),
    post("backup_to_cloud", "Push a full backup of the venue to its configured S3-compatible bucket now.", NO_ARGS, "/api/owner/backup/cloud", false, None),
];

/// The room's routes. Every POST here reads `location_id` from its body.
pub const STAFF_TOOLS: &[Tool] = &[
    get("menu", MENU_TEXT, MENU_SCHEMA, MENU_PATH, None),
    get("room", "Every open table (sitting) with its rounds, their status, and what is due.", NO_ARGS, "/api/staff/room", Some(Cap::TakeOrders)),
    get("floor", "The floor plan with each table's state: free, seated, held by a booking, or waiting to be cleared.", NO_ARGS, "/api/staff/floor", Some(Cap::TakeOrders)),
    post("table_cleared", "Mark a table cleared after the guests left (only a table waiting to be cleared).",
        r#"{"type":"object","required":["sitting"],"properties":{"sitting":{"type":"string"}}}"#, "/api/staff/floor/{sitting}/cleared", true, Some(Cap::TakeOrders)),
    post("guest_round", "Answer a round a guest ordered from the table's QR code: confirm or reject.",
        r#"{"type":"object","required":["id","action"],"properties":{"id":{"type":"string"},"action":{"type":"string","enum":["confirm","reject"]}}}"#, "/api/staff/orders/{id}/guest", true, Some(Cap::TakeOrders)),
    post("amend_round", "Change a round: add dishes, remove or change a line's quantity, move it to another table. base_seq is the round's seq as last read.",
        r#"{"type":"object","required":["id","base_seq","ops"],"properties":{"id":{"type":"string"},"base_seq":{"type":"integer"},"reason":{"type":"string"},"ops":{"type":"array","items":{"type":"object","required":["op"],"properties":{"op":{"type":"string","enum":["add","remove","set_qty","comp","table"]},"product_id":{"type":"string"},"modifier_ids":{"type":"array","items":{"type":"string"}},"quantity":{"type":"integer"},"line":{"type":"integer"},"qty":{"type":"integer"},"table":{"type":"string"}}}}}}"#,
        "/api/staff/orders/{id}/amend", true, Some(Cap::TakeOrders)),
    post("take_payment", "Record a payment against a round: amount in minor units, method cash or card, optional tip.",
        r#"{"type":"object","required":["id","amount","method"],"properties":{"id":{"type":"string"},"amount":{"type":"integer"},"method":{"type":"string","enum":["cash","card"]},"tip":{"type":"integer"},"base_seq":{"type":"integer"}}}"#,
        "/api/staff/orders/{id}/pay", true, Some(Cap::TakePayment)),
    get("tips", "Tips taken today (or between from_ms and to_ms), per person.",
        r#"{"type":"object","properties":{"from_ms":{"type":"integer"},"to_ms":{"type":"integer"}}}"#, "/api/staff/till/tips", Some(Cap::OpenTill)),
    post("order_action", "Move an order through the kitchen: confirm, preparing, ready.",
        r#"{"type":"object","required":["id","action"],"properties":{"id":{"type":"string"},"action":{"type":"string","enum":["confirm","preparing","ready"]}}}"#, "/api/owner/orders/{id}/action", true, Some(Cap::Advance)),
    post("kitchen_ack", "Record that the kitchen saw an order.",
        r#"{"type":"object","required":["id"],"properties":{"id":{"type":"string"}}}"#, "/api/staff/orders/{id}/kitchen-ack", true, Some(Cap::Advance)),
];

/// The courier's own routes. Every one names the courier by the TOKEN, so a
/// courier's agent can only ever see that courier's runs and the open pool.
pub const COURIER_TOOLS: &[Tool] = &[
    get("tasks", "My runs, and orders ready for pickup that nobody has taken; whether I am on shift.", NO_ARGS, "/api/courier/tasks", None),
    post("shift", "Start (open: true) or end (open: false) my shift.",
        r#"{"type":"object","required":["open"],"properties":{"open":{"type":"boolean"}}}"#, "/api/courier/shift", false, None),
    post("accept", "Take an order offered to me or free in the pool.",
        r#"{"type":"object","required":["id"],"properties":{"id":{"type":"string"}}}"#, "/api/courier/orders/{id}/accept", false, None),
    post("pickup", "Say I picked the order up at the venue.",
        r#"{"type":"object","required":["id"],"properties":{"id":{"type":"string"}}}"#, "/api/courier/orders/{id}/pickup", false, None),
    post("deliver", "Say I delivered it; cash_collected in minor units when the customer paid cash.",
        r#"{"type":"object","required":["id"],"properties":{"id":{"type":"string"},"cash_collected":{"type":"integer"}}}"#, "/api/courier/orders/{id}/deliver", false, None),
    get("earnings", "My shifts: deliveries and cash collected.", NO_ARGS, "/api/courier/earnings", None),
    get("history", "My past deliveries.", NO_ARGS, "/api/courier/history", None),
];

/// The tools this role's agent is offered, in table order.
pub fn tools_for(role: Role) -> Vec<&'static Tool> {
    match role {
        Role::Owner => OWNER_TOOLS.iter().collect(),
        Role::Staff(caps) => STAFF_TOOLS.iter().filter(|t| t.need.is_none_or(|c| caps.allows(c))).collect(),
        Role::Courier => COURIER_TOOLS.iter().collect(),
    }
}

/// A tool by name, ONLY from this role's list.
pub fn find(role: Role, name: &str) -> Option<&'static Tool> {
    tools_for(role).into_iter().find(|t| t.name == name)
}

pub fn listed(role: Role) -> Value {
    json!(tools_for(role)
        .iter()
        .map(|t| json!({
            "name": t.name, "description": t.description,
            "inputSchema": serde_json::from_str::<Value>(t.schema).unwrap_or(json!({ "type": "object" })),
        }))
        .collect::<Vec<_>>())
}

/// Each role word the apps know, with the names and descriptions its agent
/// gets. What `GET /api/mcp` shows every screen, so none of them keeps its
/// own copy of the list.
pub fn by_role() -> Value {
    let one = |role: Role| -> Value {
        json!(tools_for(role).iter().map(|t| json!({ "name": t.name, "description": t.description })).collect::<Vec<_>>())
    };
    let mut out = serde_json::Map::new();
    out.insert("owner".into(), one(Role::Owner));
    for p in [Preset::Waiter, Preset::Kitchen, Preset::CounterManager] {
        out.insert(p.as_str().into(), one(Role::Staff(p.caps())));
    }
    out.insert("courier".into(), one(Role::Courier));
    Value::Object(out)
}

/// RFC 3986 unreserved characters pass; everything else is %XX.
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

/// Fill `{id}`-style holes from the arguments; an argument that fills a hole
/// leaves the body. `{slug}` is the venue's, supplied by the caller.
pub fn fill_path(path: &str, args: &mut serde_json::Map<String, Value>, slug: &str) -> Result<String, String> {
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

/// One tool call, planned: the URL, the body if it is a POST, and the status
/// filter `orders` applies afterwards. PURE: the only thing `call_tool` adds
/// is the request itself.
#[derive(Debug, PartialEq)]
pub struct Plan {
    pub url: String,
    pub body: Option<String>,
    pub status_filter: Option<String>,
}

pub fn plan(tool: &Tool, args: Value, location_id: &str, slug: &str, origin: &str) -> std::result::Result<Plan, String> {
    let mut args = match args {
        Value::Object(m) => m,
        Value::Null => serde_json::Map::new(),
        _ => return Err("arguments must be an object".into()),
    };
    // `orders` filters after the fact; `menu` picks a language; neither is a route argument.
    let status_filter = if tool.name == "orders" { args.remove("status").and_then(|v| v.as_str().map(str::to_uppercase)) } else { None };
    let lang = if tool.name == "menu" { args.remove("lang").and_then(|v| v.as_str().map(str::to_string)) } else { None };
    let path = fill_path(tool.path, &mut args, slug)?;
    Ok(match tool.verb {
        Verb::Get => {
            let mut url = format!("{origin}{path}?location_id={}", enc(location_id));
            for (k, v) in &args {
                let s = match v { Value::String(s) => s.clone(), other => other.to_string() };
                url.push_str(&format!("&{}={}", enc(k), enc(&s)));
            }
            if let Some(l) = &lang {
                url.push_str(&format!("&locale={}", enc(l)));
            }
            Plan { url, body: None, status_filter }
        }
        Verb::Post => {
            // THE VENUE IS THE KEY'S, written over anything the agent sent.
            if tool.loc_in_body {
                args.insert("location_id".into(), json!(location_id));
            }
            Plan { url: format!("{origin}{path}"), body: Some(Value::Object(args).to_string()), status_filter }
        }
    })
}
