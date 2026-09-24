//! The assistant's two doors, and the retrieval behind them.
//!
//! NOTHING HERE DECIDES ANYTHING. Both routes gather FACTS out of the hub and
//! hand them to a model with a system prompt; the model never reads the store
//! and never writes to it. `graph` exposes the same retrieval on its own,
//! because an answer is only as good as what was shown to produce it, and an
//! owner who cannot see what was shown cannot tell a wrong answer from a wrong
//! retrieval.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use crate::services::orders::mine::of_venue as orders_of;


#[derive(Deserialize)]
struct AskIn {
    question: String,
}

/// `POST /api/owner/assist` — a question about this venue's own live data.

/// What the hub knows that bears on a question, as facts.
///
/// THE ASSISTANT USED TO SEE ONLY LIVE ORDERS, which answers "what is late"
/// and nothing else. An owner asking "which dishes need salmon" or "what has
/// Eni been carrying" was asking about relations that exist in the data and
/// nowhere in any single record — the ids are in the JSON, the meaning is in
/// how they connect. `Graph::of` folds those relations out of the log and the
/// catalogue, and `hybrid` finds the part of that graph the question is about.
///
/// HYBRID, not a keyword search, for the reason the graph module gives at
/// length: an ingredient's name appears in no order's text, so words alone
/// cannot reach the orders it touched. The walk can. The two are fused by rank
/// rather than by score, so neither needs a weight anyone has to tune.
///
/// NEIGHBOURS ARE INCLUDED because a node alone is a name. "dish:item-41" says
/// nothing; "Panko Shrimps, in snacks, uses shrimp, was in order ord-2" is an
/// answer. The model is given the relations and told, as everywhere on this
/// path, that these are the truth and it is not.

/// What the shelf currently holds, for the graph's ingredient nodes.
///
/// COURIER AND CUSTOMER NAMES ARE DELIBERATELY ABSENT, and the reason is in the
/// schema: the column is `full_name_encrypted`. A courier's name is encrypted at
/// rest on purpose, and the one place it must never be decrypted into is a
/// retrieval index that is then pasted into a model's prompt — which may be a
/// hosted provider's. The graph therefore knows WHICH courier carried an order
/// and not who they are; the console maps the id to a name at display time,
/// where the decrypt already lives and stays on the owner's screen.
///
/// A SEARCH FOR A COURIER BY NAME THEREFORE FINDS NOTHING, and that is the
/// correct behaviour rather than a gap. It cost one round of building the
/// wrong thing to see it.
///
/// A stock level is not a person. "salmon: 4 on hand, 1 reserved" is exactly
/// what turns "which dishes use salmon" into "and here is what running out
/// costs you", which is the question an owner actually asks.
async fn shelf_labels(place: &crate::hubstore::Place) -> std::collections::HashMap<String, String> {
    let Ok(loaded) = crate::hubstore::load_stock(place).await else {
        return std::collections::HashMap::new();
    };
    let Ok(ledger) = loaded.stock.ledger() else {
        return std::collections::HashMap::new();
    };
    ledger
        .items()
        .into_iter()
        .map(|(item, lvl)| {
            (
                format!("ingredient:{item}"),
                format!("на складі {} зарезервовано {}", lvl.on_hand, lvl.reserved),
            )
        })
        .collect()
}

fn graph_facts(
    hub: &dowiz_hub::Hub,
    cat: &dowiz_hub::catalog::Catalog,
    labels: &std::collections::HashMap<String, String>,
    question: &str,
    limit: usize,
) -> Value {
    use dowiz_hub::graph::Graph;
    let g = Graph::of_with(hub, cat, labels);
    let hits = g.hybrid(question, limit);
    let found: Vec<Value> = hits
        .iter()
        .filter_map(|(i, score)| {
            let n = g.node(*i)?;
            let related: Vec<Value> = g
                .neighbours(*i)
                .into_iter()
                .take(12)
                .filter_map(|(rel, j, forward)| {
                    let m = g.node(j)?;
                    Some(json!({
                        "how": rel.tag(),
                        "direction": if forward { "to" } else { "from" },
                        "kind": m.kind.tag(),
                        "id": m.id,
                        "label": m.label,
                    }))
                })
                .collect();
            Some(json!({
                "kind": n.kind.tag(),
                "id": n.id,
                "label": n.label,
                "relevance": score,
                "related": related,
            }))
        })
        .collect();
    json!({ "nodes": g.len(), "relations": g.edge_count(), "found": found })
}




/// EVERYTHING THE OWNER'S ASSISTANT IS SHOWN, as one value: what is sent to
/// the model is exactly this, so a test of this is a test of the payload.
///
/// THE FACTS ARE COMPUTED HERE and handed over. The model is told plainly that
/// they are the truth and it is not; a model that invented a number would have
/// an owner phoning a customer about an order that does not exist.
pub fn owner_facts(
    hub: &dowiz_hub::Hub,
    cat: &dowiz_hub::catalog::Catalog,
    labels: &std::collections::HashMap<String, String>,
    loc: &str,
    question: &str,
    now: i64,
) -> Value {
    let listed: Vec<crate::hubdo::OrderView> =
        crate::hubstore::orders_state(hub).into_iter().map(crate::hubdo::OrderView::of_event).collect();
    let mut live: Vec<Value> = orders_of(listed, loc)
        .into_iter()
        .filter(|o| crate::services::customers::forget::LIVE.contains(&o.get("status").and_then(Value::as_str).unwrap_or("")))
        .map(|o| owner_order_fact(&o, now))
        .collect();
    live.sort_by_key(|o| -o["waiting_minutes"].as_i64().unwrap_or(0));
    let off: Vec<Value> = cat
        .products()
        .into_iter()
        .filter_map(|(_, j)| {
            let v: Value = serde_json::from_str(&j).ok()?;
            if v.get("available").and_then(Value::as_bool).unwrap_or(true) {
                return None;
            }
            Some(json!({ "name": v.get("name").cloned().unwrap_or(Value::Null),
                         "why": v.get("unavailableNote").cloned().unwrap_or(Value::Null) }))
        })
        .collect();
    // Everything else the hub knows that bears on what was asked.
    let knows = graph_facts(hub, cat, labels, question, 12);
    json!({ "now_ms": now, "live_orders": live, "off_the_menu": off, "hub_knows": knows })
}

/// ONE LIVE ORDER AS THE OWNER'S ASSISTANT SEES IT (P11, GDPR Art. 5(1)(c)).
///
/// The facts go to whatever model the venue configured (`ai.endpoint`), which
/// is a processor the customer was never told about. So the order is there --
/// its id, status, money, age, kind, courier and lines, which is what "what is
/// late" and "who is carrying what" are answered from -- and the PERSON is
/// not: no `contact` (name, phone) and no `address` (a home is a person). An
/// owner who needs to ring somebody opens the order on their own screen.
pub fn owner_order_fact(o: &Value, now: i64) -> Value {
    let created = o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(now);
    let field = |k: &str| o.get(k).cloned().unwrap_or(Value::Null);
    json!({
        "id": field("id"),
        "status": field("status"),
        "total": field("total"),
        "waiting_minutes": (now - created) / 60_000,
        "fulfilment": o.get("fulfilment").and_then(|f| f.get("kind")).cloned().unwrap_or(Value::Null),
        "courier_id": field("courier_id"),
        "items": field("items"),
    })
}

/// ONE ORDER OF THE COURIER'S OWN RUN, for the courier's assistant (P11). The
/// courier is going to the door, so the address LINE stays -- it is the one
/// thing "which do I take first" needs -- but not its parts, coordinates or
/// note, and never the customer's name or phone: the courier app has those,
/// and the model does not need them to plan a route.
pub fn courier_run_fact(o: &Value, now: i64) -> Value {
    let created = o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(now);
    let field = |k: &str| o.get(k).cloned().unwrap_or(Value::Null);
    let line = o.pointer("/fulfilment/address/line").cloned().unwrap_or(Value::Null);
    json!({
        "id": field("id"),
        "status": field("status"),
        "total": field("total"),
        "payment": field("payment"),
        "waiting_minutes": (now - created) / 60_000,
        "address": line,
    })
}

/// `GET /api/owner/graph?q=` — what the hub knows, directly.
///
/// THE SAME RETRIEVAL THE ASSISTANT USES, exposed on its own. An answer a model
/// gives is only as good as what it was shown, and an owner who cannot see what
/// it was shown cannot tell a wrong answer from a wrong retrieval. This is also
/// how the retrieval is tested without a model in the loop.
///
/// With no `q` it reports the shape — how many nodes and relations — which is
/// the cheapest way to see that the fold is working at all.
pub async fn graph(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let (_, _loc, (loaded, loaded_cat)) =
        match crate::owner::owner_beside(&req, &ctx, &place, crate::hubstore::load_both(&place)).await
        {
            Ok(v) => v,
            Err(r) => return Ok(r),
        };
    let q = req
        .url()
        .ok()
        .and_then(|u| u.query_pairs().find(|(k, _)| k == "q").map(|(_, v)| v.to_string()))
        .unwrap_or_default();
    let limit = req
        .url()
        .ok()
        .and_then(|u| u.query_pairs().find(|(k, _)| k == "limit").map(|(_, v)| v.to_string()))
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(12)
        .clamp(1, 50);
    let labels = shelf_labels(&place).await;
    Response::from_json(&graph_facts(&loaded.hub, &loaded_cat.catalog, &labels, &q, limit))
}

pub async fn owner_assist(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: AskIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    // The membership query and this read do not depend on each other, so
    // `owner_beside` runs them together. The token is still verified before
    // either is issued -- see it for why that order matters.
    // THE IMAGE, not the projection, and for once that is right: `graph_facts`
    // walks the hub itself -- reveals, stock, the shape of the log -- to answer
    // a question in words. The orders are folded out of the same image rather
    // than asked for a second time.
    let (_, loc, loaded) =
        match crate::owner::owner_beside(&req, &ctx, &place, crate::hubstore::load(&place)).await {
            Ok(v) => v,
            Err(r) => return Ok(r),
        };
    let now = ctx.data.now_ms;
    let cat = crate::hubstore::load_catalog(&place).await?.catalog;
    let labels = shelf_labels(&place).await;
    let facts = owner_facts(&loaded.hub, &cat, &labels, &loc, &body.question, now);
    crate::assist::ask(&place, crate::assist::SYSTEM_OWNER, facts, &body.question).await
}

/// `POST /api/courier/assist` — a question about this courier's own run.
pub async fn courier_assist(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: AskIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let me = match crate::auth::authenticate(&req, &ctx.env, ctx.data.now_ms).await {
        Ok(crate::auth::Principal::Courier { courier_id, .. }) => courier_id,
        Ok(_) => return Response::error("forbidden role", 403),
        Err(e) => return e.into_response(),
    };
    let now = ctx.data.now_ms;
    // THEIR OWN RUN AND NOTHING ELSE. A courier asking the assistant must not
    // be able to reach a neighbour's address through it.
    let mine: Vec<Value> = crate::hubstore::orders(&place)
        .await?
        .into_iter()
        .filter_map(|e| serde_json::from_str::<Value>(&e.order_json).ok())
        .filter(|o| o.get("courier_id").and_then(Value::as_str) == Some(me.as_str()))
        .filter(|o| {
            !matches!(
                o.get("status").and_then(Value::as_str),
                Some("DELIVERED" | "CANCELLED" | "REJECTED")
            )
        })
        .map(|o| courier_run_fact(&o, now))
        .collect();
    let facts = json!({ "now_ms": now, "my_runs": mine });
    crate::assist::ask(&place, crate::assist::SYSTEM_COURIER, facts, &body.question).await
}

#[cfg(test)]
mod tests;
