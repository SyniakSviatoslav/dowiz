//! Wallet legs against wallet payments (conservation law 12), and the repair
//! of a lost leg. The rules are `command::pay::legs`; this file reads the two
//! images and, only when asked with `apply: true`, writes ONE: the ledger.
//!
//! `GET  /api/owner/wallet/legs`        — the ledger's spends and the audit.
//! `POST /api/owner/wallet/legs/repair` — `{apply?}`: a DRY RUN by default,
//!   answering what would be written and what is refused; with `apply: true`
//!   it appends the missing legs, re-deciding against the ledger as it stands
//!   inside the write guard. Idempotent: a leg's id is derived, so a second
//!   apply finds nothing missing and writes nothing.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use crate::command::pay::legs;
use crate::wallet::{IMAGE_LEDGER, K_TX};

/// The venue's orders (the log is read BEFORE the ledger: a leg is written
/// after its `Paid`, so a `Paid` seen here has had its chance to land) and
/// its currency.
async fn read(place: &crate::hubstore::Place) -> Result<(Vec<Value>, String)> {
    let orders: Vec<Value> = crate::hubstore::orders(place)
        .await?
        .into_iter()
        .filter_map(|o| serde_json::from_str(&o.order_json).ok())
        .collect();
    let cat = crate::hubstore::load_catalog(place).await?;
    Ok((orders, crate::services::venue::currency_of(&cat.catalog)))
}

/// The ledger's records, oldest first.
fn rows_of(log: &dowiz_hub::logimage::LogImage) -> Vec<String> {
    let mut es = log.about(K_TX, None, usize::MAX);
    es.reverse();
    es.into_iter().map(|e| e.json).collect()
}

pub async fn audit(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let loc = match crate::owner::owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let (orders, currency) = read(&place).await?;
    let rows = rows_of(&crate::hubstore::load_log(&place, IMAGE_LEDGER).await?.log);
    let spends: Vec<Value> = rows
        .iter()
        .filter_map(|j| serde_json::from_str::<Value>(j).ok())
        .filter(legs::is_spend)
        .collect();
    let a = legs::audit(&orders, &rows, &loc, &currency);
    let mut res = Response::from_json(&json!({ "venue": loc, "spends": spends, "audit": a, "holds": a.holds() }))?;
    res.headers_mut().set("cache-control", "private, no-store")?;
    Ok(res)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RepairIn {
    #[serde(default)]
    apply: bool,
}

pub async fn repair(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: RepairIn = match req.text().await.map(|t| if t.trim().is_empty() { "{}".into() } else { t }) {
        Ok(t) => match serde_json::from_str(&t) {
            Ok(b) => b,
            Err(e) => return Response::error(format!("bad request body: {e}"), 400),
        },
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let loc = match crate::owner::owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let (orders, currency) = read(&place).await?;
    let refused_of = |p: &legs::Plan| -> Vec<Value> {
        p.refused.iter().map(|(l, why)| json!({ "leg": l, "why": why })).collect()
    };
    if !body.apply {
        let rows = rows_of(&crate::hubstore::load_log(&place, IMAGE_LEDGER).await?.log);
        let plan = match legs::repair(&orders, &rows, &loc, &currency) {
            Ok(p) => p,
            Err(r) => return Response::error(r.message().to_string(), r.status()),
        };
        let would: Vec<&str> = plan.write.iter().map(|d| d.tx_id.as_str()).collect();
        return Response::from_json(&json!({ "applied": false, "wouldWrite": would, "refused": refused_of(&plan) }));
    }
    // DECIDED AGAIN INSIDE THE GUARD, against the ledger as it is now; a
    // retried closure re-decides rather than replaying a stale plan.
    let (loc2, cur2) = (loc.clone(), currency.clone());
    let plan = crate::hubstore::with_log(&place, IMAGE_LEDGER, move |log| {
        let plan = legs::repair(&orders, &rows_of(log), &loc2, &cur2)
            .map_err(|r| Error::RustError(r.message().to_string()))?;
        for d in &plan.write {
            log.append(K_TX, &d.tx_id, &d.record).map_err(|e| Error::RustError(format!("ledger: {e:?}")))?;
        }
        Ok(plan)
    })
    .await?;
    let wrote: Vec<&str> = plan.write.iter().map(|d| d.tx_id.as_str()).collect();
    Response::from_json(&json!({ "applied": true, "written": wrote, "refused": refused_of(&plan) }))
}
