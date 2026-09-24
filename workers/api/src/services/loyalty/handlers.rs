//! I/O ONLY. The two places the stamp card touches a port: what a placement
//! hands the object, and what the customer's own order page reads. The rules
//! are `stamps`, pure and tested natively.

use std::collections::BTreeMap;

use serde_json::{json, Value};
use worker::*;

use super::stamps::{self, StampIn};
use crate::auth::{self, Claims};
use crate::hubstore::{Place, IMAGE_PEOPLE, PEOPLE_BYTES};
use crate::services::customers::alias::Aliases;
use crate::services::customers::handlers::customer_key;

/// `mine` for the Worker: every order whose phone's key is in `circle`. One
/// HMAC per distinct spelling, not per order.
fn in_circle<'a>(secret: &'a [u8], circle: &'a [String]) -> impl Fn(&str) -> bool + 'a {
    let memo = std::cell::RefCell::new(BTreeMap::<String, bool>::new());
    move |p: &str| {
        if let Some(hit) = memo.borrow().get(p) {
            return *hit;
        }
        let hit = circle.contains(&customer_key(secret, p));
        memo.borrow_mut().insert(p.to_string(), hit);
        hit
    }
}

/// What `storefront::place` puts in `PlaceIn.stamps`: `None` when the venue
/// has no card, and when anything needed cannot be read -- an order is never
/// refused for a stamp card. The failure is recorded, loudly.
///
/// The spellings include the rule's link THIS placement is about to write
/// (`identity::alias_at_placement`), so a customer's first `069…` order is on
/// their `+355…` card already.
pub async fn at_placement(place: &Place, secret: &[u8], location_id: &str, phone: &str) -> Option<StampIn> {
    let card = match crate::hubstore::load_settings(place).await {
        Ok(s) => stamps::config(|k| s.settings.known(k))?,
        Err(e) => {
            crate::loud!(&place.ns, Some(&place.venue), "loyalty.place", "settings unread: {e}");
            return None;
        }
    };
    let read = futures_util::future::try_join(
        crate::hubstore::orders(place),
        crate::hubstore::load_table(place, IMAGE_PEOPLE, PEOPLE_BYTES),
    )
    .await;
    let (listed, people) = match read {
        Ok(v) => v,
        Err(e) => {
            crate::loud!(&place.ns, Some(&place.venue), "loyalty.place", "card unread: {e}");
            return None;
        }
    };
    let pending = crate::services::customers::identity::alias_at_placement(secret, phone);
    let circle = Aliases::of(&people.table).circle(&customer_key(secret, phone), pending.as_deref());
    let phones = stamps::spellings(&listed, location_id, in_circle(secret, &circle), phone);
    Some(StampIn { n: card.n, reward_minor: card.reward_minor, location_id: location_id.to_string(), phones })
}

/// `GET /api/order/:id/stamps` — the card, for the customer holding the key
/// minted with THIS order. `{on:false}` when the venue has no card or the
/// order carries no phone; otherwise `{on, have, n, reward, used}`.
pub async fn order_stamps(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing order id", 400);
    };
    let claims = auth::bearer(&req).and_then(|t| auth::verify(&ctx.env, &t, ctx.data.now_ms));
    let location_id = match claims {
        Ok(Claims::Customer { order_id, location_id, .. }) if order_id == id => location_id,
        _ => return Response::error("this card needs the link you were given with the order", 401),
    };
    let place = Place::of_authorised(&ctx, &location_id)?;
    let (settings, listed, people) = futures_util::future::try_join3(
        crate::hubstore::load_settings(&place),
        crate::hubstore::orders(&place),
        crate::hubstore::load_table(&place, IMAGE_PEOPLE, PEOPLE_BYTES),
    )
    .await?;
    let this: Option<Value> = listed
        .iter()
        .find(|v| v.order_id == id)
        .and_then(|v| serde_json::from_str(&v.order_json).ok());
    let Some(this) = this else { return Response::error("not found", 404) };
    let card = stamps::config(|k| settings.settings.known(k));
    let body = match (card, stamps::phone_of(&this)) {
        (Some(card), Some(phone)) => {
            let secret = crate::services::customers::handlers::signing_secret(&ctx.env);
            let circle = Aliases::of(&people.table).circle(&customer_key(&secret, phone), None);
            let hit = in_circle(&secret, &circle);
            let mine = |o: &Value| {
                o.get("location_id").and_then(Value::as_str) == Some(location_id.as_str())
                    && stamps::phone_of(o).is_some_and(&hit)
            };
            json!({
                "on": true,
                "have": stamps::shown(&listed, mine, card.n, &this),
                "n": card.n,
                "reward": card.reward_minor,
                "used": stamps::used(&this),
            })
        }
        _ => json!({ "on": false }),
    };
    let mut res = Response::from_json(&body)?;
    res.headers_mut().set("cache-control", "private, no-store")?;
    Ok(res)
}
