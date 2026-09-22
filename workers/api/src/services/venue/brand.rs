//! The five tokens a venue's surfaces are drawn from, and the presets that
//! set them all at once.
//!
//! A PALETTE IS DERIVED AGAINST THE PAPER IT IS PRINTED ON. See
//! `dowiz_hub::brand` for the rule; this is the doorway to it.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use crate::owner::owner_and_venue;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BrandIn {
    primary: String,
    #[serde(default)]
    ink: Option<String>,
    #[serde(default)]
    paper: Option<String>,
    #[serde(default)]
    type_pair: Option<String>,
    #[serde(default)]
    radius: Option<i64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PresetIn {
    preset: String,
}

/// `GET /api/owner/branding?location_id=`
///
/// The presets and the type pairs come from the HUB rather than being written
/// into the pane, so the console and the storefront cannot disagree about which
/// pairs exist -- and a pair not on this list is not one the storefront renders.
pub async fn branding(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    use dowiz_hub::brand::{Brand, PRESETS, RADIUS_MAX, TYPE_PAIRS};
    // The membership query and this read do not depend on each other, so
    // `owner_beside` runs them together. The token is still verified before
    // either is issued -- see it for why that order matters.
    let (_, _loc, loaded) =
        match crate::owner::owner_beside(&req, &ctx, &place, crate::hubstore::load_catalog(&place)).await {
            Ok(v) => v,
            Err(r) => return Ok(r),
        };
    let stored = loaded
        .catalog
        .location()
        .and_then(|j| serde_json::from_str::<Value>(&j).ok())
        .and_then(|l| l.get("theme").cloned())
        .unwrap_or(Value::Null);
    let b = match &stored {
        Value::Null => Brand::shipped(),
        v => Brand::parse(&v.to_string()),
    };
    Response::from_json(&json!({
        "brand": { "primary": b.accent.hex(), "ink": b.ink.hex(), "paper": b.paper.hex(),
                   "typePair": b.type_pair, "radius": b.radius },
        "presets": PRESETS.iter().map(|p| json!({
            "id": p.id, "label": p.label, "primary": p.accent, "ink": p.ink,
            "paper": p.paper, "typePair": p.type_pair, "radius": p.radius })).collect::<Vec<_>>(),
        "typePairs": TYPE_PAIRS.iter().map(|t| json!({ "id": t.id, "label": t.label }))
            .collect::<Vec<_>>(),
        "radiusMax": RADIUS_MAX,
    }))
}

async fn store_brand(place: &crate::hubstore::Place, b: dowiz_hub::brand::Brand) -> Result<Response> {
    let theme = b.theme();
    // The derivation already walks each colour until every pair passes; this is
    // the belt to that braces. Shipping a failing theme would make text
    // unreadable for every customer of this venue.
    if let Some(bad) = theme.contrast_report().into_iter().find(|(_, got, want)| got < want) {
        return Response::error(
            format!("derived theme fails WCAG: {} is {:.2}:1, needs {}:1", bad.0, bad.1, bad.2),
            409,
        );
    }
    let stored = json!({
        "seed": b.accent.hex(), "ink": b.ink.hex(), "paper": b.paper.hex(),
        "typePair": b.type_pair, "radius": b.radius,
        "light": theme.as_css_tokens(), "dark": theme.as_dark_css_tokens(),
    });
    let payload = stored.clone();
    crate::hubstore::with_catalog(&place, move |cat| {
        let raw = cat.location().ok_or_else(|| Error::RustError("no venue".into()))?;
        let mut loc: Value = serde_json::from_str(&raw).unwrap_or(json!({}));
        loc["theme"] = payload.clone();
        cat.set_location(&serde_json::to_string(&loc).unwrap_or(raw));
        Ok(())
    })
    .await?;
    Response::from_json(&json!({
        "primary": theme.primary.hex(),
        "primaryAdjustedPct": theme.primary_adjusted_pct,
        "typePair": b.type_pair, "radius": b.radius,
        "light": stored["light"], "dark": stored["dark"],
        "contrast": theme.contrast_report().into_iter().map(|(what, got, want)| json!({
            "pair": what, "ratio": (got * 100.0).round() / 100.0,
            "required": want, "passes": got >= want })).collect::<Vec<_>>(),
    }))
}

/// `POST /api/owner/branding?location_id=`
pub async fn set_branding(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: BrandIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    use dowiz_hub::brand::{type_pair, Brand, RADIUS_MAX};
    use dowiz_hub::palette::Rgb;
    let d = Brand::shipped();
    let colour = |what: &str, v: &str| -> std::result::Result<Rgb, String> {
        Rgb::from_hex(v).ok_or_else(|| format!("{what}: {v:?} is not a colour"))
    };
    let accent = match colour("accent", &body.primary) {
        Ok(c) => c,
        Err(e) => return Response::error(e, 400),
    };
    let ink = match &body.ink {
        Some(v) => match colour("ink", v) {
            Ok(c) => c,
            Err(e) => return Response::error(e, 400),
        },
        None => d.ink,
    };
    let paper = match &body.paper {
        Some(v) => match colour("paper", v) {
            Ok(c) => c,
            Err(e) => return Response::error(e, 400),
        },
        None => d.paper,
    };
    // An unknown pair is REFUSED rather than defaulted: an owner looking at a
    // font they did not pick deserves a reason.
    let pair = match &body.type_pair {
        Some(v) => match type_pair(v) {
            Some(p) => p.id,
            None => return Response::error(format!("{v:?} is not a type pair"), 400),
        },
        None => d.type_pair,
    };
    let radius = match body.radius {
        Some(r) if !(0..=RADIUS_MAX).contains(&r) => {
            return Response::error(format!("a radius is 0 to {RADIUS_MAX} px"), 400)
        }
        Some(r) => r,
        None => d.radius,
    };
    store_brand(&place, Brand { accent, ink, paper, type_pair: pair, radius }).await
}

/// `POST /api/owner/branding/preset?location_id=` — a whole look at once.
pub async fn set_preset(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: PresetIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let Some(b) = dowiz_hub::brand::preset(&body.preset) else {
        return Response::error(format!("{:?} is not a preset", body.preset), 400);
    };
    store_brand(&place, b).await
}
