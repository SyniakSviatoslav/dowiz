//! `GET /api/owner/activation` — can this venue open?
//!
//! EVERY ANSWER IS DERIVED FROM THE VENUE'S OWN RECORD, never stored: a stored
//! "ready" flag goes stale the moment somebody deletes the last dish.

use serde_json::{json, Value};
use worker::*;


/// `GET /api/owner/activation?location_id=`
///
/// Three things, and an order is useless without all of them: something to
/// sell, somebody who hears the order land, and a way to get it there. The leg
/// that catches real venues is the second -- nobody notices nothing is bound to
/// the bot until an order has sat unanswered for forty minutes.
pub async fn activation(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    // The membership query and this read do not depend on each other, so
    // `owner_beside` runs them together. The token is still verified before
    // either is issued -- see it for why that order matters.
    let (_, _loc, loaded) =
        match crate::owner::owner_beside(&req, &ctx, &place, crate::hubstore::load_catalog(&place)).await {
            Ok(v) => v,
            Err(r) => return Ok(r),
        };
    let raw: Value = loaded
        .catalog
        .location()
        .and_then(|j| serde_json::from_str(&j).ok())
        .unwrap_or(json!({}));
    let sellable = loaded
        .catalog
        .products()
        .into_iter()
        .filter(|(_, pj)| {
            serde_json::from_str::<Value>(pj)
                .ok()
                .map(|p| {
                    p.get("available").and_then(Value::as_bool).unwrap_or(false)
                        && p.get("price").and_then(Value::as_i64).unwrap_or(0) > 0
                })
                .unwrap_or(false)
        })
        .count();
    let settings = crate::hubstore::load_settings(&place).await?.settings;
    let phone = raw.get("phone").and_then(Value::as_str).unwrap_or("");
    let f = dowiz_hub::activation::Facts {
        sellable_dishes: sellable,
        telegram_chats: telegram_chats(ctx.env.secret("TELEGRAM_BOT_TOKEN").is_ok(), &settings),
        has_venue_phone: phone.chars().filter(char::is_ascii_digit).count() >= 8,
        delivery_configured: delivery_configured(&raw),
        pickup_enabled: raw.get("pickup").and_then(Value::as_bool).unwrap_or(false),
    };
    let missing = dowiz_hub::activation::missing(&f);
    Response::from_json(&json!({
        // `missing.is_empty()` WAS WRITTEN OUT HERE, and identically in the
        // twin server, while `activation::can_open` -- whose whole body is
        // that expression -- had four tests and no caller anywhere. Three
        // copies of a one-line rule is still three rules.
        "canOpen": dowiz_hub::activation::can_open(&f),
        "missing": missing.iter().map(|r| json!({ "key": r.key(), "why": r.as_str() }))
            .collect::<Vec<_>>(),
        "facts": {
            "sellableDishes": f.sellable_dishes, "telegramChats": f.telegram_chats,
            "hasVenuePhone": f.has_venue_phone, "deliveryConfigured": f.delivery_configured,
            "pickupEnabled": f.pickup_enabled,
        }
    }))
}

/// WOULD `/api/public/reach` RESTRICT ANYTHING? The storefront's
/// `hasDeliveryZones` and this module's `delivery_configured` both ask, and
/// both used to answer `delivery_zones.is_some()` -- true for the `[]` that
/// `POST /api/owner/zones` stores to CLEAR the area, while `reach` answered
/// "unrestricted". Derived exactly as `reach` derives it, so the two can
/// never disagree again: a list that parses to no zone is no zone.
pub fn has_delivery_zones(raw: &Value) -> bool {
    raw.get("delivery_zones")
        .is_some_and(|z| !dowiz_hub::zone::from_json(&z.to_string()).is_empty())
}

/// The area as stored when it restricts anything, else `[]` -- what the
/// owner console's delivery-area editor (`admin/zones.js`) starts from. The
/// storefront menu carries it beside `hasDeliveryZones`; the circles are the
/// venue's public promise of where it delivers, which `reach` already answers.
pub fn delivery_zones(raw: &Value) -> Value {
    match raw.get("delivery_zones") {
        Some(z) if has_delivery_zones(raw) => z.clone(),
        _ => json!([]),
    }
}

/// A fee or a real area. Either tells the customer what delivery costs them.
pub fn delivery_configured(raw: &Value) -> bool {
    raw.get("delivery_fee").is_some() || has_delivery_zones(raw)
}

/// Would anything HEAR an order? `notify.rs` sends with the venue's own bot
/// (`notify.telegram.token`, falling back to the platform's) to
/// `notify.telegram.chat`. A token with nowhere to send is nothing heard; a
/// venue-owned bot with a chat is heard even when the platform has no bot.
pub fn telegram_chats(platform_token: bool, s: &dowiz_hub::settings::Settings) -> usize {
    let set = |k: &str| !s.known(k).trim().is_empty();
    usize::from((platform_token || set("notify.telegram.token")) && set("notify.telegram.chat"))
}

#[cfg(test)]
mod tests;
