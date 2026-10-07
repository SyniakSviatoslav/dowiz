//! `POST /api/platform/compact[?pin=0]` -- the Worker's half of `hubdo/compact.rs`: every venue's
//! object, one after another, compacted and pinned to v2 (or unpinned). PLATFORM ADMINISTRATORS
//! ONLY (`platform::admin_only`: an owner is answered 404, exactly as for `/api/platform/hubs`), no
//! console button: `tools/deploy/rollback.sh` calls it before it switches the Worker's version.
//!
//! 200 only when EVERY venue answered 200; otherwise 502 with the same per-venue list, so a script
//! that reads the status alone cannot mistake a half-compacted platform for a ready one.

use crate::edge::Ctx;
use crate::wire::{Call, Reply};
use serde_json::json;
use worker::{Method, Result};

/// The objects to compact: every venue in the registry, and the unnamed one `bootstrap` fills.
fn venues(rows: Vec<(String, String)>) -> Vec<String> {
    let mut out: Vec<String> = rows
        .into_iter()
        .filter_map(|(_, j)| serde_json::from_str::<serde_json::Value>(&j).ok())
        .filter_map(|v| v["id"].as_str().map(str::to_string))
        .collect();
    out.push(crate::hubstore::UNNAMED_VENUE.to_string());
    out.sort();
    out.dedup();
    out
}

pub(crate) async fn platform_compact(req: Call, ctx: Ctx<crate::Req>) -> Result<Reply> {
    if let Err(r) = crate::platform::admin_only(&req, &ctx).await {
        return Ok(r);
    }
    let unpin = req.url()?.query_pairs().any(|(k, v)| k == "pin" && v == "0");
    let path = if unpin { "https://hub/fold/compact?pin=0" } else { "https://hub/fold/compact" };
    let rows = crate::identity_store::registry(&ctx.env).await?.all(crate::identity_store::K_LOC);
    let ns = ctx.env.durable_object("HUB")?;
    let (mut out, mut failed) = (Vec::new(), 0usize);
    for venue in venues(rows) {
        let answered = async { ns.id_from_name(&venue)?.get_stub()?.fetch_with_request(Call::new(path, Method::Post)?).await }.await;
        let (status, body) = match answered {
            Ok(r) => (r.status_code(), serde_json::from_slice(r.body()).unwrap_or_else(|_| json!(String::from_utf8_lossy(r.body())))),
            Err(e) => (500, json!(e.to_string())),
        };
        failed += usize::from(status != 200);
        out.push(json!({ "venue": venue, "status": status, "answer": body }));
    }
    let res = Reply::from_json(&json!({ "venues": out, "failed": failed, "pinned": !unpin }))?;
    Ok(if failed == 0 { res } else { res.with_status(502) })
}
