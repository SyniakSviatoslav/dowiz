//! The consent routes (§3.2). ORCHESTRATION ONLY: the acts are built by the
//! pure functions in `consent_log`, and written by `consent::log::write`.

use serde_json::json;
use worker::*;

use super::consent_log::{self, OwnerActIn};
use super::record_routes::is_key;
use crate::owner::owner_and_venue;

/// `GET /api/public/consent/wordings` — the sentences beside the checkout's
/// box, with their ids. SERVED BY THE HUB so the page shows the exact bytes
/// the id was computed over: a sentence copied into the storefront would be a
/// second copy that drifts, and then the id proves nothing.
pub async fn wordings(_req: Request, _ctx: RouteContext<crate::Req>) -> Result<Response> {
    use dowiz_hub::consent::{wording_id, WORDINGS};
    let out: Vec<_> = WORDINGS
        .iter()
        .map(|(l, text)| json!({ "lang": l, "id": wording_id(l), "text": text }))
        .collect();
    let mut res = Response::from_json(&json!({ "wordings": out }))?;
    res.headers_mut().set("cache-control", "public, max-age=300")?;
    Ok(res)
}

/// `POST /api/owner/customers/:key/consent?location_id=` — the owner files a
/// withdrawal (or an evidenced grant) on the customer's behalf.
///
/// THE WITHDRAWAL ROUTE. A STOP said at the counter, on the phone or in a
/// message the webhook did not see is filed here, and the fold stops the next
/// send. Nothing is removed: the grant stays as the proof of what the venue
/// was allowed to do before.
pub async fn owner_act(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: OwnerActIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let (who, loc) = match owner_and_venue(&req, &ctx).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let Some(key) = ctx.param("key").cloned().filter(|k| is_key(k)) else {
        return Response::error("not a customer key", 400);
    };
    let act = match consent_log::owner_act(&key, &body, &who, ctx.data.now_ms) {
        Ok(a) => a,
        Err(why) => return Response::error(why, 400),
    };
    match consent_log::file(&place, &act).await {
        Ok(()) => Response::from_json(&json!({ "key": key, "state": act.state.as_str(), "atMs": act.at_ms })),
        Err(why) => Response::error(why, 500),
    }
}
