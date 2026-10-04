//! I/O ONLY. The bag insert's four doors (W-QR); the rules are `bag` and
//! `welcome`, pure and tested natively.
//!
//! * `GET  /api/owner/bag?c=`             the card: link, QR, offer, commission, counts
//! * `POST /api/owner/bag`                set the offer and the commission percent
//! * `GET  /api/owner/bag/qr.svg?c=`      the QR alone, as an image to download
//! * `GET  /api/public/locations/:slug/welcome`  the offer, for the storefront banner
//!
//! THE QR IS THE TABLE CODES' ENCODER (`orders::room::table_qr::svg`,
//! `qrcodegen`, MIT): one QR implementation in the tree, drawn by the server,
//! no vendored JS and no CDN (the CSP allows neither).
//!
//! ONE VENUE: the one the owner is authorised for (`owner_and_venue`), the
//! place built from it (`Place::of_authorised`), its host read from its record.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

use super::bag;
use super::welcome::{self, Offer, OfferIn, WelcomeIn};
use crate::hubstore::{Place, IMAGE_PEOPLE, PEOPLE_BYTES};
use crate::services::customers::alias::Aliases;
use crate::services::customers::handlers::{customer_key, signing_secret};
use crate::services::orders::room::table_qr;

fn err(m: impl Into<String>, s: u16) -> Response {
    Response::error(m.into(), s).unwrap()
}

/// The `c` query parameter, checked.
fn campaign_of(req: &Request) -> std::result::Result<Option<String>, Response> {
    let raw = req.url().ok().and_then(|u| u.query_pairs().find(|(k, _)| k == "c").map(|(_, v)| v.to_string()));
    bag::campaign(raw.as_deref()).map_err(|e| err(e, 400))
}

/// Authorise the owner; the place, the venue's own host.
async fn venue(req: &Request, ctx: &RouteContext<crate::Req>) -> std::result::Result<(Place, String, String), Response> {
    let loc = crate::owner::owner_and_venue(req, ctx).await.map(|(_, l)| l)?;
    let place = Place::of_authorised(ctx, &loc).map_err(|e| err(e.to_string(), 500))?;
    let rec = crate::hubstore::venue_record(&place).await.map_err(|e| err(e.to_string(), 500))?.unwrap_or(Value::Null);
    let slug = rec.get("slug").and_then(Value::as_str).unwrap_or("").to_string();
    if slug.is_empty() {
        return Err(err("this venue has no slug yet; its storefront has no address to print", 409));
    }
    let platform = ctx.var("PLATFORM_HOST").map(|v| v.to_string()).unwrap_or_else(|_| "dowiz.org".into());
    Ok((place, table_qr::venue_host(&slug, &platform), loc))
}

/// `GET /api/owner/bag?c=`
pub async fn card(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let c = match campaign_of(&req) { Ok(c) => c, Err(r) => return Ok(r) };
    let (place, host, loc) = match venue(&req, &ctx).await { Ok(v) => v, Err(r) => return Ok(r) };
    let (settings, listed) =
        futures_util::future::try_join(crate::hubstore::load_settings(&place), crate::hubstore::orders(&place)).await?;
    let known = |k: &str| settings.settings.known(k);
    let offer = Offer::parse(&known(welcome::OFFER));
    let pct = welcome::check_pct(&known(welcome::COMMISSION)).ok().flatten();
    let secret = signing_secret(&ctx.env);
    let memo = std::cell::RefCell::new(BTreeMap::<String, String>::new());
    let key_of = |p: &str| memo.borrow_mut().entry(p.to_string()).or_insert_with(|| customer_key(&secret, p)).clone();
    let stats = bag::stats(&listed, &loc, key_of, pct);
    let url = bag::landing_url(&host, c.as_deref());
    let mut res = Response::from_json(&json!({
        "host": host, "url": url, "c": c, "svg": table_qr::svg(&url),
        "offer": offer.as_ref().map(Offer::to_json), "commission_pct": pct,
        "stamps_on": super::stamps::config(known).is_some(),
        "stats": stats.to_json(),
    }))?;
    res.headers_mut().set("cache-control", "private, no-store")?;
    Ok(res)
}

/// The owner's form. STRICT: an unknown field is a 400 that names it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BagIn {
    pub offer: OfferIn,
    #[serde(default)]
    pub commission_pct: String,
}

/// `POST /api/owner/bag`
pub async fn set(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let loc = match crate::owner::owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // AUTHORITY BEFORE THE BODY: nobody's JSON is parsed before the door.
    let body: BagIn = match crate::body::strict(&mut req).await {
        Ok(b) => b,
        Err(r) => return Ok(r),
    };
    let place = Place::of_authorised(&ctx, &loc)?;
    let settings = crate::hubstore::load_settings(&place).await?;
    let stamps_on = super::stamps::config(|k| settings.settings.known(k)).is_some();
    let offer = match welcome::check(&body.offer, stamps_on) {
        Ok(o) => o,
        Err(why) => return Response::error(why, 400),
    };
    let pct = match welcome::check_pct(&body.commission_pct) {
        Ok(p) => p,
        Err(why) => return Response::error(why, 400),
    };
    // A GIFT IS A DISH ON THIS MENU: refused while the owner can still fix it.
    if let Some(Offer::Gift { product }) = &offer {
        let nodes = crate::services::ordering::basket::ask(&place, std::slice::from_ref(product), None).await?;
        if nodes.product(product).is_none() {
            return Response::error("bag_gift: that dish is not on this menu", 400);
        }
    }
    let stored = offer.as_ref().map(|o| o.to_json().to_string()).unwrap_or_default();
    let pct_s = pct.map(|p| p.to_string()).unwrap_or_default();
    crate::hubstore::with_settings(&place, move |s| {
        s.set(welcome::OFFER, &stored);
        s.set(welcome::COMMISSION, &pct_s);
        Ok(())
    })
    .await?;
    Response::from_json(&json!({ "ok": true, "offer": offer.as_ref().map(Offer::to_json), "commission_pct": pct }))
}

/// `GET /api/owner/bag/qr.svg?c=`
pub async fn qr(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let c = match campaign_of(&req) { Ok(c) => c, Err(r) => return Ok(r) };
    let (_, host, _) = match venue(&req, &ctx).await { Ok(v) => v, Err(r) => return Ok(r) };
    let Some(body) = table_qr::svg(&bag::landing_url(&host, c.as_deref())) else {
        return Response::error("that link does not fit in a QR code", 500);
    };
    let mut res = Response::ok(body)?;
    res.headers_mut().set("content-type", "image/svg+xml")?;
    res.headers_mut().set("cache-control", "private, no-store")?;
    Ok(res)
}

/// `GET /api/public/locations/:slug/welcome` -- the PUBLIC offer, the same for
/// everyone: `{on:false}` or `{on, kind, value?, min?, gift?}`. Nothing about
/// any guest, and no write: reading it counts nothing.
pub async fn public(_req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let Some(slug) = ctx.param("slug").cloned() else { return Response::error("missing slug", 400) };
    let place = Place::of_slug(&ctx, &slug).await?;
    let settings = crate::hubstore::load_settings(&place).await?;
    let body = match Offer::parse(&settings.settings.known(welcome::OFFER)) {
        None => json!({ "on": false }),
        Some(Offer::Gift { product }) => {
            let nodes = crate::services::ordering::basket::ask(&place, std::slice::from_ref(&product), None).await?;
            match nodes.product(&product).and_then(|j| serde_json::from_str::<Value>(&j).ok()) {
                Some(p) => json!({ "on": true, "kind": "gift", "gift": { "id": product, "name": p.get("name"), "price": p.get("price") } }),
                None => json!({ "on": false }),
            }
        }
        Some(o) => {
            let mut v = o.to_json();
            v["on"] = json!(true);
            v
        }
    };
    let mut res = Response::from_json(&body)?;
    res.headers_mut().set("cache-control", "public, max-age=30")?;
    Ok(res)
}

/// What `storefront::place` puts in `PlaceIn.welcome`: `None` when the guest
/// did not land from a bag, typed no phone, or the venue has no offer. An
/// order is never refused for a welcome offer; a failure is recorded, loudly.
pub async fn at_placement(place: &Place, secret: &[u8], location_id: &str, phone: &str, campaign: Option<String>) -> Option<WelcomeIn> {
    let read = futures_util::future::try_join3(
        crate::hubstore::load_settings(place),
        crate::hubstore::orders(place),
        crate::hubstore::load_table(place, IMAGE_PEOPLE, PEOPLE_BYTES),
    )
    .await;
    let (settings, listed, people) = match read {
        Ok(v) => v,
        Err(e) => {
            crate::loud!(&place.ns, Some(&place.venue), "bag.place", "welcome unread: {e}");
            return None;
        }
    };
    let offer = Offer::parse(&settings.settings.known(welcome::OFFER))?;
    let pending = crate::services::customers::identity::alias_at_placement(secret, phone);
    let circle = Aliases::of(&people.table).circle(&customer_key(secret, phone), pending.as_deref());
    let phones = super::stamps::spellings(&listed, location_id, super::handlers::in_circle(secret, &circle), phone);
    Some(WelcomeIn { offer, location_id: location_id.to_string(), phones, campaign })
}

#[cfg(test)]
mod tests;
