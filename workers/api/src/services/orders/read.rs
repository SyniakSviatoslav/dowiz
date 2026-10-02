//! `GET /api/order/:id` -- one order, for the three principals that may read it. MOVED here from
//! an inline closure in `lib.rs` (W-COV C2) so it runs under `cargo test`; the body is unchanged.

use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

pub async fn order(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing order id", 400);
    };
    // ── AN ORDER IS NOT READABLE BY WHOEVER KNOWS ITS ID ──
    //
    // This route was public. An id is not a secret -- it appears in a
    // URL, a browser history, a shared screenshot -- and behind it sat
    // the customer's name, phone and street address. That is
    // capability-by-obscurity, and it was live.
    //
    // Three principals may read one order, and each is checked against
    // THIS order rather than against a role: the customer holding the
    // key minted with it, the venue's owner, and the courier whose run
    // it actually is. A courier is not entitled to every customer's
    // address in the venue.
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let Some(order_json) = crate::hubstore::order(&place, &id).await? else {
        return Response::error("order not found", 404);
    };
    let envelope: serde_json::Value =
        serde_json::from_str(&order_json).unwrap_or(serde_json::json!({}));

    // Only the venue's own people see what a dish costs the kitchen
    // (W-PERF P1's stamp); the customer and the courier read the order
    // without it.
    let mut insider = false;
    let allowed = match crate::auth::authenticate(&req, &ctx.env, ctx.data.now_ms).await {
        Ok(crate::auth::Principal::Customer { order_id, .. }) => order_id == id,
        // AN OWNER OF THIS VENUE, not an owner of any venue.
        //
        // `authenticate`'s owner check asks "is this user an owner
        // somewhere", because that is all a role needs. Here the
        // question is about a VENUE: a platform-admin token carries no
        // location at all and this route used to answer `true` for it,
        // so it read any venue's orders -- name, phone and address --
        // on the venue's own host, which `accounts.rs` states in as
        // many words that it cannot do. The claim has to name THIS
        // hub.
        Ok(crate::auth::Principal::Owner { active_location_id, .. }) => {
            insider = active_location_id.as_deref() == Some(place.venue.as_str());
            insider
        }
        Ok(crate::auth::Principal::Courier { courier_id, .. }) => {
            envelope.get("courier_id").and_then(|c| c.as_str()) == Some(courier_id.as_str())
        }
        // Staff of THIS venue who may move or take orders read them.
        Ok(crate::auth::Principal::Staff { active_location_id, caps, .. }) => {
            insider = active_location_id == place.venue
                && (caps.allows(crate::auth::Cap::Advance) || caps.allows(crate::auth::Cap::TakeOrders));
            insider
        }
        Err(_) => false,
    };
    if !allowed {
        return Response::error("this order needs the link you were given", 401);
    }
    // The order as it stands NOW: the time that is left rides with it.
    let mut live = envelope.clone();
    if !insider {
        crate::command::place::cost::strip(&mut live);
    }
    crate::live_eta::attach_one(&place, &mut live, ctx.data.now_ms).await;
    let mut res = Response::ok(serde_json::to_string(&live).unwrap_or(order_json))?;
    res.headers_mut().set("content-type", "application/json; charset=utf-8")?;
    // Never cached by anything between here and the browser: it holds
    // an address.
    res.headers_mut().set("cache-control", "private, no-store")?;
    Ok(res)
}

#[cfg(test)]
mod tests;
