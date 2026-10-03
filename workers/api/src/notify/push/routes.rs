//! THE ROUTES: a phone says "tell me", "stop telling me", "are you telling me".
//!
//!   GET  /api/push/key          the VAPID public key the browser subscribes with
//!   POST /api/push/subscribe    PushSubscription.toJSON() + lang
//!   POST /api/push/unsubscribe  {endpoint}
//!   POST /api/push/state        {endpoint} -> {on}
//!
//! WHO IS TOLD WHAT IS DECIDED BY THE TOKEN, never by the body: a customer's
//! token is for ONE order and subscribes to that order; a courier's to the
//! orders handed to them; an owner's or a staff member's to the venue's new
//! orders. The body names a device, never a person, an order or a venue.

use serde::Deserialize;
use serde_json::json;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};
use worker::*;

use super::subs::{self, SubIn, K_COURIER, K_CUSTOMER, K_STAFF};
use crate::auth::Principal;

/// `{endpoint}`, for unsubscribe and state.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EndpointIn {
    pub endpoint: String,
}

/// The venue, the record kind and the key a principal subscribes under. PURE.
pub fn whose(p: &Principal) -> std::result::Result<(String, &'static str, String), &'static str> {
    match p {
        Principal::Customer { order_id, location_id, .. } => Ok((location_id.clone(), K_CUSTOMER, order_id.clone())),
        Principal::Courier { courier_id, active_location_id, .. } => Ok((active_location_id.clone(), K_COURIER, courier_id.clone())),
        Principal::Staff { person_id, active_location_id, .. } => Ok((active_location_id.clone(), K_STAFF, person_id.clone())),
        Principal::Owner { user_id, active_location_id: Some(v) } => Ok((v.clone(), K_STAFF, user_id.clone())),
        Principal::Owner { active_location_id: None, .. } => Err("open a venue first"),
    }
}

async fn caller(req: &Request, ctx: &RouteContext<crate::Req>) -> std::result::Result<(crate::hubstore::Place, &'static str, String), Response> {
    let p = match crate::auth::authenticate(req, &ctx.env, ctx.data.now_ms).await {
        Ok(p) => p,
        Err(e) => return Err(e.into_response().unwrap_or_else(|_| Response::error("unauthorised", 401).unwrap())),
    };
    let (venue, kind, key) = whose(&p).map_err(|m| Response::error(m, 403).unwrap())?;
    let place = crate::hubstore::Place::of_authorised(ctx, &venue).map_err(|e| Response::error(e.to_string(), 500).unwrap())?;
    Ok((place, kind, key))
}

/// `GET /api/push/key`.
pub async fn key(_req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    Response::from_json(&json!({ "key": super::vapid::public_key(&ctx.env) }))
}

/// `POST /api/push/subscribe`.
pub async fn subscribe(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: SubIn = match crate::body::strict(&mut req).await {
        Ok(b) => b,
        Err(r) => return Ok(r),
    };
    let (place, kind, key) = match caller(&req, &ctx).await {
        Ok(c) => c,
        Err(r) => return Ok(r),
    };
    let now = ctx.data.now_ms;
    let sub = match body.check(now) {
        Ok(s) => s,
        Err(m) => return Response::error(m, 400),
    };
    let id = subs::record_id(&key, &sub.endpoint);
    let rec = serde_json::to_string(&sub).map_err(|e| Error::RustError(e.to_string()))?;
    crate::hubstore::with_table(&place, subs::IMAGE_PUSH, subs::PUSH_BYTES, move |t| {
        let all = t.all(kind);
        let mut drop = subs::over_cap(&subs::of_key(&all, &key), &id);
        if kind == K_CUSTOMER {
            drop.extend(subs::stale_customers(&all, now));
        }
        for d in &drop {
            t.remove(kind, d);
        }
        t.put(kind, &id, &rec, &[], &[]).map_err(|e| Error::RustError(format!("push: {e:?}")))
    })
    .await?;
    Response::from_json(&json!({ "ok": true, "on": true }))
}

/// `POST /api/push/unsubscribe`.
pub async fn unsubscribe(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: EndpointIn = match crate::body::strict(&mut req).await {
        Ok(b) => b,
        Err(r) => return Ok(r),
    };
    let (place, kind, key) = match caller(&req, &ctx).await {
        Ok(c) => c,
        Err(r) => return Ok(r),
    };
    let id = subs::record_id(&key, body.endpoint.trim());
    let removed = crate::hubstore::with_table(&place, subs::IMAGE_PUSH, subs::PUSH_BYTES, move |t| Ok(t.remove(kind, &id))).await?;
    Response::from_json(&json!({ "ok": true, "on": false, "removed": removed }))
}

/// `POST /api/push/state`: is THIS device told, for this caller?
pub async fn state(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: EndpointIn = match crate::body::strict(&mut req).await {
        Ok(b) => b,
        Err(r) => return Ok(r),
    };
    let (place, kind, key) = match caller(&req, &ctx).await {
        Ok(c) => c,
        Err(r) => return Ok(r),
    };
    let id = subs::record_id(&key, body.endpoint.trim());
    let table = crate::hubstore::load_table(&place, subs::IMAGE_PUSH, subs::PUSH_BYTES).await?.table;
    Response::from_json(&json!({ "on": table.get(kind, &id).is_some() }))
}

#[cfg(test)]
mod tests;
