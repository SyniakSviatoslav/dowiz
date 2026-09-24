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
//! READS THREE IMAGES AND WRITES NONE: the order log, the till log, and the
//! settings (for the late-amendment minutes). One venue: the one the owner's
//! token was authorised for (`owner_beside` → `must_be`).
//!
//! `?from=&to=` in epoch ms; default the last day. `?period=till` is the open
//! till's period (or the last closed one).

pub mod alert;
pub mod fold;
pub mod legs;

use serde_json::json;
use worker::*;

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
    let (_, loc, (hub, till, settings, ledger, cat, listed)) = match crate::owner::owner_beside(&req, &ctx, &place, async {
        let (h, t, s, l, c) = futures_util::future::join5(
            crate::hubstore::load(&place),
            crate::hubstore::load_log(&place, crate::command::till::IMAGE_TILL),
            crate::hubstore::load_settings(&place),
            crate::hubstore::load_log(&place, crate::wallet::IMAGE_LEDGER),
            crate::hubstore::load_catalog(&place),
        )
        .await;
        // The orders as the legs route reads them (`hubstore::orders`).
        let o = crate::hubstore::orders(&place).await?;
        Ok((h?.hub, t?.log, s?.settings, l?.log, c?.catalog, o))
    })
    .await
    {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let now = ctx.data.now_ms;
    let q: Vec<(String, String)> = req.url()?.query_pairs().map(|(k, v)| (k.into_owned(), v.into_owned())).collect();
    let entries = till.entries();
    // A till log that does not fold is REFUSED, never read as "no pay-outs".
    let periods = match crate::command::till::periods(&entries) {
        Ok(p) => p,
        Err(e) => return Response::error(format!("the till log does not fold: {e}"), 500),
    };
    let (from, to) = window_of(&q, &periods, now);
    let late = alert::late_ms(settings.get(alert::LATE_KEY).as_deref());
    // THE WALLET LEGS (law 12): the ledger oldest first, the orders as the
    // audit route reads them. A ledger that does not replay is REFUSED.
    let mut rows = ledger.about(crate::wallet::K_TX, None, usize::MAX);
    rows.reverse();
    let rows: Vec<String> = rows.into_iter().map(|e| e.json).collect();
    let orders = legs::orders_json(&listed);
    let legs = match legs::leg_rows(&orders, &rows, &loc, &crate::services::venue::currency_of(&cat)) {
        Ok(r) => r,
        Err(e) => return Response::error(format!("the wallet ledger does not replay: {e}"), 500),
    };
    match report(&hub.events_oldest_first(), &entries, legs, late, from, to) {
        Ok(mut v) => {
            v["venue"] = json!(loc);
            v["threshold"] = json!(alert::threshold(settings.get(alert::THRESHOLD_KEY).as_deref()));
            Response::from_json(&v)
        }
        Err(e) => Response::error(e, 500),
    }
}

#[cfg(test)]
mod tests;
