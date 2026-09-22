//! Where this venue delivers, and how far that actually reaches.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use crate::owner::owner_and_venue;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ZonesIn {
    /// The service area as the owner drew it. An EMPTY list removes every
    /// restriction, which is how a venue turns the check off.
    zones: Vec<Value>,
}

/// `POST /api/owner/zones`
///
/// PARSED BACK BEFORE IT IS STORED. A zone the reader cannot understand is
/// treated as NO zone at all -- and no zones means every address is accepted --
/// so a configuration that silently means nothing would quietly turn the check
/// off while the owner believed they had drawn a boundary.
pub async fn set_zones(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: ZonesIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let db = ctx.d1("DB")?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let raw = serde_json::to_string(&body.zones).unwrap_or_else(|_| "[]".into());
    let parsed = dowiz_hub::zone::from_json(&raw);
    if parsed.len() != body.zones.len() {
        return Response::error(
            format!(
                "{} of {} zones could not be read. A circle is \
                 {{\"kind\":\"circle\",\"lat\":<micro-degrees>,\"lon\":<micro-degrees>,\
                 \"radius_m\":<metres>}}; a polygon is {{\"kind\":\"polygon\",\
                 \"points\":[[lat,lon],…]}} with at least three points",
                body.zones.len() - parsed.len(),
                body.zones.len()
            ),
            400,
        );
    }
    let zones = body.zones.clone();
    crate::hubstore::with_catalog(&place, move |cat| {
        let raw = cat.location().ok_or_else(|| Error::RustError("no venue".into()))?;
        let mut l: Value = serde_json::from_str(&raw).unwrap_or(json!({}));
        l["delivery_zones"] = json!(zones);
        cat.set_location(&serde_json::to_string(&l).unwrap_or(raw));
        Ok(())
    })
    .await?;
    Response::from_json(&json!({ "ok": true, "zones": parsed.len() }))
}

/// `GET /api/public/reach?lat_udeg=&lon_udeg=` — will you deliver here?
///
/// ASKED BEFORE THE ORDER, so a refusal costs one sale rather than a courier
/// sent forty minutes out of town and every order behind it late. Micro-degrees
/// because the whole system holds coordinates as integers: a float crossing
/// into this path is what MANIFESTO C2 forbids.
pub async fn reach(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let q = |name: &str| -> Option<i64> {
        req.url()
            .ok()?
            .query_pairs()
            .find(|(k, _)| k == name)
            .and_then(|(_, v)| v.parse::<i64>().ok())
    };
    let point = q("lat_udeg").zip(q("lon_udeg"));
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let cat = crate::hubstore::load_catalog(&place).await?.catalog;
    let zones = cat
        .location()
        .and_then(|j| serde_json::from_str::<Value>(&j).ok())
        .and_then(|l| l.get("delivery_zones").cloned())
        .map(|z| dowiz_hub::zone::from_json(&z.to_string()))
        .unwrap_or_default();
    let r = dowiz_hub::zone::reach(&zones, point);
    use dowiz_hub::zone::Reach;
    Response::from_json(&json!({
        "reach": match r {
            Reach::Unrestricted => "unrestricted",
            Reach::Inside => "inside",
            Reach::Outside { .. } => "outside",
            Reach::Unknown => "unknown",
        },
        // How far outside, so the storefront can say "two streets over" rather
        // than a flat no.
        "metresAway": match r { Reach::Outside { nearest_m } => json!(nearest_m), _ => Value::Null },
    }))
}
