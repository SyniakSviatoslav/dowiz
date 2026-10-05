//! THE GUEST'S TASTE ON THE SERVER: its objection, its writes and its four routes (W-MR0 row MR8).
//! I/O ONLY: every rule is `taste.rs` (the profile) or `dowiz_hub::consent` (the objection log),
//! tested natively. DECISIONS.md D0 (amended 2026-10-04) is why this exists; `tools/gates/no-scoring.sh`
//! is why the scoring itself may live only in `taste.rs`.
//!
//! OPERATOR RULING 2026-10-04: AUTOMATIC, NO CHECKBOX. The profile is kept under the phone the guest
//! gives at checkout, ON by default, on legitimate interest (GDPR 6(1)(f), Recital 47 -- the balancing
//! test is docs/privacy/DPIA-personalisation.md), and stopped by ONE tap: an objection (Art. 21) filed
//! in the consent log as a `personalisation` withdrawal, which also deletes the profile.
//!
//!   at placement   `prepare` before the order (a `taste_sync` that is not the closed shape, or from a
//!                  guest who objected: 400 by name; `taste_off` = the objection the phone kept),
//!                  `after` once the order exists (file the objection and delete, or fold this order in)
//!   GET  /api/owner/customers/:key/taste?location_id=   the owner's view of one guest
//!   GET  /api/owner/customers/taste/segments?location_id=   the venue's counts per segment
//!   GET  /api/order/:id/taste      the guest's own view (their order's link), which is the export
//!   POST /api/order/:id/taste/withdraw   Art. 21: object, and the profile is deleted at once

use serde_json::json;
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

use super::taste::{self, Line, SyncIn, IMAGE_TASTE, KIND, TASTE_BYTES};
use crate::hubstore::Place;
use dowiz_hub::consent::{Act, Method, State, CHANNEL_STOREFRONT, PURPOSE_PERSONALISATION};

const DAY_MS: i64 = 86_400_000;
pub fn day_of(now_ms: i64) -> i64 {
    now_ms.div_euclid(DAY_MS)
}

/// What a placement carries into `after`.
#[derive(Debug, Default)]
pub struct Prepared {
    pub key: Option<String>,
    /// The objection to file (the phone's "turn off", sent as `taste_off`).
    pub objection: Option<Act>,
    pub sync: Option<SyncIn>,
    /// W-SENSE: the moment at the venue the order was placed in (band, day, weather), or none.
    pub at: Option<super::taste::senses::At>,
}

/// PURE. The venue's moment as the profile files it.
pub fn at_of(b: &crate::services::venue::context::Bucket) -> super::taste::senses::At {
    super::taste::senses::At { keys: b.keys(), when: b.when() }
}

/// PURE. The guest's objection: a personalisation withdrawal by their own tap, no sentence needed.
pub fn objection_of(key: &str, now_ms: i64, via: &str) -> Act {
    Act {
        key: key.to_string(),
        purpose: PURPOSE_PERSONALISATION.into(),
        channel: CHANNEL_STOREFRONT.into(),
        state: State::Withdrawn,
        at_ms: now_ms,
        method: Method::Objection,
        evidence: String::new(),
        wording_id: String::new(),
        via: via.to_string(),
    }
}

/// Has this guest objected, by the log?
async fn objected(place: &Place, key: &str) -> Result<bool> {
    let log = crate::hubstore::load_log(place, super::consent_log::IMAGE_CONSENT).await?;
    Ok(dowiz_hub::consent::objected(&log.log.entries(), key))
}

/// PURE. The placement's taste fields, judged before anything is read: the vector's closed shape.
/// `Err(why)` is a 400 by name. A vector or an objection with no phone has nobody to belong to and
/// is dropped (an order needs no phone).
pub fn judge(key: Option<String>, taste_off: bool, sync: Option<&SyncIn>, now_ms: i64) -> std::result::Result<Prepared, String> {
    if let Some(s) = sync {
        s.validate()?;
    }
    let Some(k) = key else { return Ok(Prepared::default()) };
    let objection = taste_off.then(|| objection_of(&k, now_ms, ""));
    // An objection in the same body wins: the vector is not kept.
    let sync = if taste_off { None } else { sync.cloned() };
    Ok(Prepared { key: Some(k), objection, sync, at: None })
}

/// BEFORE THE ORDER. `Ok(Err(why))` is a 400 the caller answers by name: a `taste_sync` that is not
/// the closed shape, or one from a guest who objected (the phone should have stopped sending).
pub async fn prepare(
    place: &Place,
    env: &Env,
    record: &serde_json::Value,
    key: Option<String>,
    taste_off: bool,
    sync: Option<&SyncIn>,
    now_ms: i64,
) -> Result<std::result::Result<Prepared, String>> {
    let mut prep = match judge(key, taste_off, sync, now_ms) {
        Ok(p) => p,
        Err(why) => return Ok(Err(why)),
    };
    // The VENUE's clock and the VENUE's weather, from the cache only: a placement never waits on
    // a provider, and a cold cache is simply no weather (`venue::context`).
    // Only a guest whose profile will be written gets a moment: no phone or an objection, no read.
    if prep.key.is_some() && prep.objection.is_none() {
        let weather = crate::services::venue::context::weather(env, record, false).await;
        let b = crate::services::venue::context::Bucket::at(crate::hubstore::zone_of(Some(record)), now_ms, weather);
        prep.at = Some(at_of(&b));
    }
    if let (Some(k), Some(_)) = (&prep.key, &prep.sync) {
        if objected(place, k).await? {
            return Ok(Err("taste_sync is refused: this guest turned personalisation off".into()));
        }
    }
    Ok(Ok(prep))
}

/// Delete the guest's profile; answers whether one was there.
async fn delete_profile(place: &Place, key: &str) -> Result<bool> {
    let k = key.to_string();
    crate::hubstore::with_table(place, IMAGE_TASTE, TASTE_BYTES, move |t| Ok(t.remove(KIND, &k))).await
}

/// AFTER THE ORDER, and unable to fail it: an objection is filed and the profile deleted; otherwise,
/// unless the guest objected before, this order's dishes (and the device's vector, if sent) are folded
/// into the profile. `lines` are the order's dishes as `taste::line_of` reads them.
pub async fn after(place: &Place, prep: Prepared, lines: Vec<Line>, order_id: &str, now_ms: i64) {
    let Some(key) = prep.key else { return };
    if let Some(mut act) = prep.objection {
        act.via = order_id.to_string();
        if super::consent_log::file(place, &act).await.is_ok() {
            if let Err(e) = delete_profile(place, &key).await {
                crate::loud!(&place.ns, Some(&place.venue), "customers.taste", "cust:{key} objection filed, profile not deleted: {e}");
            }
        }
        return; // `file` has recorded why, loudly, when it failed
    }
    match objected(place, &key).await {
        Ok(false) => {}
        Ok(true) => return,
        Err(e) => {
            crate::loud!(&place.ns, Some(&place.venue), "customers.taste", "cust:{key} objection log unread: {e}");
            return;
        }
    }
    let day = day_of(now_ms);
    let sync = prep.sync;
    let at = prep.at;
    let wrote = crate::hubstore::with_table(place, IMAGE_TASTE, TASTE_BYTES, move |t| {
        let prev = t.get(KIND, &key).as_deref().and_then(taste::parse);
        let p = taste::apply_order_at(prev, &lines, sync.as_ref(), day, at.as_ref());
        let json = serde_json::to_string(&p).map_err(|e| Error::RustError(format!("taste: {e}")))?;
        t.put(KIND, &key, &json, &[], &[]).map_err(|e| Error::RustError(format!("taste: {e:?}")))?;
        Ok(())
    })
    .await;
    if let Err(e) = wrote {
        crate::loud!(&place.ns, Some(&place.venue), "customers.taste", "not filed: {e}");
    }
}

/// `GET /api/owner/customers/:key/taste?location_id=` -- one guest, as the owner may see it.
pub async fn owner_view(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let (_who, loc) = match crate::owner::owner_and_venue(&req, &ctx).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let place = Place::of_authorised(&ctx, &loc)?;
    let Some(key) = ctx.param("key").cloned().filter(|k| super::record_routes::is_key(k)) else {
        return Response::error("not a customer key", 400);
    };
    let people = crate::hubstore::load_table(&place, IMAGE_TASTE, TASTE_BYTES).await?;
    let today = day_of(ctx.data.now_ms);
    let p = people.table.get(KIND, &key).as_deref().and_then(taste::parse).filter(|p| !taste::expired(p, today));
    Response::from_json(&json!({ "key": key, "taste": p.map(|p| taste::view(&p, today)) }))
}

/// `GET /api/owner/customers/taste/segments?location_id=` -- how many guests in each segment.
pub async fn owner_segments(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let (_who, loc) = match crate::owner::owner_and_venue(&req, &ctx).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let place = Place::of_authorised(&ctx, &loc)?;
    let people = crate::hubstore::load_table(&place, IMAGE_TASTE, TASTE_BYTES).await?;
    let profiles: Vec<taste::Profile> = people.table.all(KIND).iter().filter_map(|(_, j)| taste::parse(j)).collect();
    let counts = taste::segment_counts(profiles.iter(), day_of(ctx.data.now_ms));
    Response::from_json(&json!({ "contract": "customers.taste-segments.v1", "segments": counts,
        "rules": { "atRiskDays": taste::AT_RISK_DAYS, "lapsedDays": taste::LAPSED_DAYS } }))
}

/// The guest behind THIS order's link, as their key and the order id the link was bound to, or the
/// 4xx that says why not.
async fn guest_key(req: &Request, ctx: &RouteContext<crate::Req>, place: &Place) -> Result<std::result::Result<(String, String), Response>> {
    let Some(id) = ctx.param("id").cloned() else { return Ok(Err(Response::error("missing order", 400)?)) };
    match crate::auth::authenticate(req, &ctx.env, ctx.data.now_ms).await {
        Ok(crate::auth::Principal::Customer { order_id, .. }) if order_id == id => {}
        Ok(_) => return Ok(Err(Response::error("that link is not for this order", 401)?)),
        Err(_) => return Ok(Err(Response::error("this order needs the link you were given", 401)?)),
    }
    let Some(order) = crate::hubstore::order(place, &id).await? else {
        return Ok(Err(Response::error("no such order", 404)?));
    };
    let o: serde_json::Value = serde_json::from_str(&order).unwrap_or_default();
    let phone = o.get("contact").and_then(|c| c.get("phone")).and_then(|p| p.as_str()).unwrap_or("");
    if !super::roll::names_a_person(phone) {
        return Ok(Err(Response::error("this order has no phone, so nothing is kept about its taste", 404)?));
    }
    Ok(Ok((super::handlers::customer_key(&super::handlers::signing_secret(&ctx.env), phone), id)))
}

/// `GET /api/order/:id/taste` -- what the venue keeps about the guest's taste: their own copy of
/// everything held (the export), whether they objected, and (on the page) how to object.
pub async fn guest_view(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let place = Place::of_any(&req, &ctx).await?;
    let key = match guest_key(&req, &ctx, &place).await? {
        Ok((k, _)) => k,
        Err(r) => return Ok(r),
    };
    let off = objected(&place, &key).await?;
    let people = crate::hubstore::load_table(&place, IMAGE_TASTE, TASTE_BYTES).await?;
    let today = day_of(ctx.data.now_ms);
    let p = people.table.get(KIND, &key).as_deref().and_then(taste::parse).filter(|p| !taste::expired(p, today));
    Response::from_json(&json!({ "objected": off, "taste": p.map(|p| taste::view(&p, today)) }))
}

/// `POST /api/order/:id/taste/withdraw` -- the guest objects (Art. 21) and the profile is deleted, in
/// one tap. The objection is filed even when nothing was held, so the next order folds nothing.
pub async fn guest_withdraw(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let place = Place::of_any(&req, &ctx).await?;
    let (key, order_id) = match guest_key(&req, &ctx, &place).await? {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let act = objection_of(&key, ctx.data.now_ms, &order_id);
    if let Err(why) = super::consent_log::file(&place, &act).await {
        return Response::error(why, 500);
    }
    let removed = delete_profile(&place, &key).await?;
    Response::from_json(&json!({ "objected": true, "deleted": removed }))
}

#[cfg(test)]
#[path = "taste_routes/tests.rs"]
mod tests;
