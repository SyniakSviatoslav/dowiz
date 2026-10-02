//! `GET /api/owner/exceptions` — the exception report (P1-5), read-only.
//!
//! BLUEPRINT-OPERATIONAL-BLIND-SPOTS-2026-09-23 §2.5. The owner sees the events
//! the chained logs already hold — voids after the kitchen, comps, late
//! amendments, refunds, cash outside a till, pay-outs, over/short — one row
//! each with its signer, grouped by kind+reason and by round. The fold is
//! `fold.rs` (pure); the alert is `alert.rs` (pure) and runs in the object
//! (`hubdo/exceptions.rs`). NOTHING IS PERSISTED and NOTHING IS SCORED: no
//! number is ever computed per person (DECISIONS OD-8, `no-scoring.sh`).
//!
//! READS FIVE IMAGES AND WRITES NONE -- the order log, the till log, the
//! settings (for the late-amendment minutes), the wallet ledger and the
//! venue's currency -- IN THE VENUE'S OBJECT (`answer`, pure; BN1), which
//! answers the report. One venue: the one the owner's token was authorised
//! for (`owner_beside` → `must_be`).
//!
//! `?from=&to=` in epoch ms; default the last day. `?period=till` is the open
//! till's period (or the last closed one).

pub mod alert;
pub mod fold;
pub mod legs;

use serde_json::json;
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

/// Parse `from`/`to`/`period` into a window. PURE.
pub fn window_of(q: &[(String, String)], periods: &[crate::command::till::Period], now_ms: i64) -> (i64, i64) {
    let get = |k: &str| q.iter().find(|(a, _)| a == k).map(|(_, v)| v.as_str());
    if get("period") == Some("till") {
        if let Some(p) = periods.last() {
            return (p.opened_at, p.closed_at.unwrap_or(now_ms));
        }
    }
    let to = get("to").and_then(|v| v.parse().ok()).unwrap_or(now_ms);
    let from = get("from").and_then(|v| v.parse().ok()).unwrap_or(to - alert::WINDOW_MS);
    (from, to)
}

/// Every exception row for a venue's logs, windowed. PURE: the route and the
/// tests call this one function. `extra`: rows folded elsewhere — the
/// wallet-leg findings (`legs::leg_rows`).
pub fn report(
    events: &[dowiz_hub::Event],
    till: &[dowiz_hub::logimage::Entry],
    extra: Vec<fold::Row>,
    late_ms: i64,
    from: i64,
    to: i64,
) -> std::result::Result<serde_json::Value, String> {
    let periods = crate::command::till::periods(till)?;
    let mut rows = fold::order_rows(events, late_ms);
    rows.extend(fold::till_rows(till, &periods));
    rows.extend(extra);
    let rows = fold::window(rows, from, to);
    let rounds: Vec<_> = fold::by_round(&rows).into_iter().map(|(id, kinds)| json!({ "orderId": id, "kinds": kinds })).collect();
    Ok(json!({
        "from": from,
        "to": to,
        "lateMinutes": late_ms / 60_000,
        "groups": fold::by_reason(&rows),
        "rounds": rounds,
        "rows": rows,
    }))
}

pub async fn exceptions(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    // THE REPORT IS FOLDED IN THE OBJECT (BN1, `/fold/exceptions`,
    // `hubdo/exceptions.rs`): the five images are there; `answer` folds them
    // and the report alone crosses the hop. `owner_beside` still runs the
    // membership query beside it and checks the venue read is the venue
    // authorised (`Place::must_be`). Only the names are the Worker's: they
    // are the platform's identity table, not the venue's images.
    let q: Vec<(String, String)> = req.url()?.query_pairs().map(|(k, v)| (k.into_owned(), v.into_owned())).collect();
    let mut url = format!("https://hub/fold/exceptions?venue={}&now={}", crate::mcp::enc(&place.venue), ctx.data.now_ms);
    for (k, v) in &q {
        if matches!(k.as_str(), "from" | "to" | "period") {
            url.push_str(&format!("&{k}={}", crate::mcp::enc(v)));
        }
    }
    let (_, loc, (status, text)) = match crate::owner::owner_beside(&req, &ctx, &place, crate::fold::ask::text(&place, &url)).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    if status != 200 {
        return Response::error(text, status);
    }
    let mut v: serde_json::Value = serde_json::from_str(&text).map_err(|e| Error::RustError(format!("exceptions: unreadable answer: {e}")))?;
    v["names"] = names(&ctx.env, &loc, &v["rows"]).await?;
    Response::from_json(&v)
}

/// The report but the names, over the venue's images. PURE: the venue's
/// object calls this (`/fold/exceptions`) with the images it holds. A refusal
/// is a status and its words, the ones the route gave.
#[allow(clippy::too_many_arguments)]
pub fn answer(
    hub: &dowiz_hub::Hub,
    till: &dowiz_hub::logimage::LogImage,
    settings: &dowiz_hub::settings::Settings,
    ledger: &dowiz_hub::logimage::LogImage,
    listed: &[crate::hubdo::OrderView],
    currency: &str,
    loc: &str,
    q: &[(String, String)],
    now: i64,
) -> std::result::Result<serde_json::Value, (u16, String)> {
    let entries = till.entries();
    // A till log that does not fold is REFUSED, never read as "no pay-outs".
    let periods = crate::command::till::periods(&entries).map_err(|e| (500, format!("the till log does not fold: {e}")))?;
    let (from, to) = window_of(q, &periods, now);
    let late = alert::late_ms(settings.get(alert::LATE_KEY).as_deref());
    // THE WALLET LEGS (law 12): the ledger oldest first, the orders as the
    // audit route reads them. A ledger that does not replay is REFUSED.
    let mut rows = ledger.about(crate::wallet::K_TX, None, usize::MAX);
    rows.reverse();
    let rows: Vec<String> = rows.into_iter().map(|e| e.json).collect();
    let orders = legs::orders_json(listed);
    let legs = legs::leg_rows(&orders, &rows, loc, currency).map_err(|e| (500, format!("the wallet ledger does not replay: {e}")))?;
    let mut v = report(&hub.events_oldest_first(), &entries, legs, late, from, to).map_err(|e| (500, e))?;
    v["venue"] = json!(loc);
    v["threshold"] = json!(alert::threshold(settings.get(alert::THRESHOLD_KEY).as_deref()));
    // WHERE THE ALERT GOES: without a chat it goes nowhere, and the
    // pane says so instead of promising a message nobody will get.
    v["alertChat"] = json!(!settings.known("notify.telegram.chat").trim().is_empty());
    Ok(v)
}

/// THE SIGNER'S NAME for each `by` on the rows: the report promised "the
/// name of who did it" and printed a user id. Only a MEMBER of this venue is
/// named -- a user id that is not one stays an id, never another venue's name.
pub(crate) async fn names(env: &Env, venue: &str, rows: &serde_json::Value) -> Result<serde_json::Value> {
    let mut ids: Vec<&str> = rows.as_array().map_or_else(Vec::new, |r| r.iter().filter_map(|x| x["by"].as_str()).collect());
    ids.sort_unstable();
    ids.dedup();
    let mut out = serde_json::Map::new();
    if ids.is_empty() {
        return Ok(serde_json::Value::Object(out));
    }
    let t = crate::identity_store::identity(env).await?;
    for id in ids {
        if crate::identity_store::membership(&t, venue, id).is_none() {
            continue;
        }
        let name = crate::identity_store::rec(&t, crate::identity_store::K_USER, id)
            .map(|u| crate::identity_store::s_of(&u, "display_name"))
            .unwrap_or_default();
        if !name.trim().is_empty() {
            out.insert(id.to_string(), json!(name));
        }
    }
    Ok(serde_json::Value::Object(out))
}

#[cfg(test)]
mod tests;
