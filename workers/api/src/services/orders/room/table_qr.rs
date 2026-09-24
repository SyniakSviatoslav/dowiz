//! THE OWNER'S TABLE CODES: one QR per table on the plan (L66, A9).
//!
//! `GET /api/owner/tables/qr` — every table as `{zone, zone_name, n, url, svg}`,
//! and `GET /api/owner/tables/:zone/:n/qr.svg` — one code as an image.
//!
//! THE SERVER DRAWS THE CODE (`qrcodegen`, MIT, no dependencies, pure Rust, so
//! it builds for wasm32 like everything else here). The URL inside it carries a
//! signature only the Worker can make (`table_link`), so the browser could not
//! draw a correct one anyway, and a vendored JS generator would be a second
//! QR implementation to trust.
//!
//! ONE VENUE: the one the owner is authorised for (`owner_and_venue`), and the
//! place is built from it (`Place::of_authorised`). The plan and the venue's
//! slug are read from the venue record, never from the request.

use qrcodegen::{QrCode, QrCodeEcc};
use serde_json::{json, Value};
use worker::*;

use super::table_link;
use dowiz_hub::tables::Plan;

/// The quiet zone the QR standard asks for, in modules.
pub const BORDER: i32 = 4;

/// One QR code as a self-contained SVG: one `<path>` of unit squares on a
/// white ground, black modules, scaled by the viewer. `None` only if the text
/// is too long for any QR version, which a table URL never is.
pub fn svg(text: &str) -> Option<String> {
    let qr = QrCode::encode_text(text, QrCodeEcc::Medium).ok()?;
    let size = qr.size();
    let dim = size + 2 * BORDER;
    let mut d = String::new();
    for y in 0..size {
        for x in 0..size {
            if qr.get_module(x, y) {
                d.push_str(&format!("M{},{}h1v1h-1z", x + BORDER, y + BORDER));
            }
        }
    }
    Some(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {dim} {dim}\" shape-rendering=\"crispEdges\">\
         <rect width=\"100%\" height=\"100%\" fill=\"#fff\"/><path d=\"{d}\" fill=\"#000\"/></svg>"
    ))
}

/// Every table on the plan with its link and its code, in the plan's order.
pub fn entries(plan: &Plan, host: &str, key: &[u8], loc: &str) -> Vec<Value> {
    plan.zones
        .iter()
        .flat_map(|z| {
            z.tables.iter().map(move |t| {
                let url = table_link::table_url(host, key, loc, &z.id, t.n);
                json!({ "zone": z.id, "zone_name": z.name, "n": t.n, "svg": svg(&url), "url": url })
            })
        })
        .collect()
}

/// The venue's own host: `<slug>.<platform>`. The printed code must open the
/// venue's storefront wherever the owner happened to be signed in.
pub fn venue_host(slug: &str, platform: &str) -> String {
    format!("{slug}.{platform}")
}

/// Authorise the owner, then read the plan, the slug and the key.
async fn load(req: &Request, ctx: &RouteContext<crate::Req>) -> std::result::Result<(Plan, String, Vec<u8>, String), Response> {
    let loc = crate::owner::owner_and_venue(req, ctx).await.map(|(_, l)| l)?;
    let err = |m: String, s: u16| Response::error(m, s).unwrap();
    let place = crate::hubstore::Place::of_authorised(ctx, &loc).map_err(|e| err(e.to_string(), 500))?;
    let rec = crate::hubstore::venue_record(&place).await.map_err(|e| err(e.to_string(), 500))?;
    let rec = rec.unwrap_or(Value::Null);
    let slug = rec.get("slug").and_then(Value::as_str).unwrap_or("").to_string();
    if slug.is_empty() {
        return Err(err("this venue has no slug yet; its storefront has no address to print".into(), 409));
    }
    let plan = super::placer::plan_of(&rec.to_string());
    let key = crate::auth::signing_key(&ctx.env);
    if key.is_empty() {
        return Err(err("table codes are not configured".into(), 503));
    }
    let platform = ctx.var("PLATFORM_HOST").map(|v| v.to_string()).unwrap_or_else(|_| "dowiz.org".into());
    Ok((plan, venue_host(&slug, &platform), key, loc))
}

/// `GET /api/owner/tables/qr`
pub async fn list(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let (plan, host, key, loc) = match load(&req, &ctx).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    Response::from_json(&json!({ "location_id": loc, "host": host, "tables": entries(&plan, &host, &key, &loc) }))
}

/// `GET /api/owner/tables/:zone/:n/qr.svg`
pub async fn one(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let zone = ctx.param("zone").cloned().unwrap_or_default();
    let Some(n) = ctx.param("n").and_then(|n| n.trim_end_matches(".svg").parse::<i64>().ok()) else {
        return Response::error("a table number is required", 400);
    };
    let (plan, host, key, loc) = match load(&req, &ctx).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    if plan.find(&zone, n).is_none() {
        return Response::error(format!("there is no table {n} in zone {zone:?} on this venue's plan"), 404);
    }
    let Some(body) = svg(&table_link::table_url(&host, &key, &loc, &zone, n)) else {
        return Response::error("that link does not fit in a QR code", 500);
    };
    let mut res = Response::ok(body)?;
    res.headers_mut().set("content-type", "image/svg+xml")?;
    res.headers_mut().set("cache-control", "private, no-store")?;
    Ok(res)
}
