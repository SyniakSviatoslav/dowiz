//! Photographs, and the venue's place on a map.
//!
//! AN IMAGE IS STORED IN THE VENUE'S OWN IMAGE, not on a CDN somebody else
//! owns: `dowiz_hub::media` decides what may be stored and how big it may be,
//! and this is the doorway to it.

use serde_json::{json, Value};
use worker::*;

use crate::owner::owner_and_venue;

//
// THE BYTES ARE SNIFFED, NEVER TRUSTED. A `content-type` header is what the
// uploader says; the magic bytes are what the file is. Anything that is not one
// of the four image formats the hub recognises is refused, so a page that later
// renders these URLs cannot be handed a script with a .jpg name.
//
// The key is the SHA-256 of the bytes. The same photo uploaded twice is stored
// once, and a URL that names its own content can be cached forever -- there is
// no version of it that could later be different.

/// `POST /api/owner/products/:id/image` — body is the image.
pub async fn set_product_image(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing product", 400);
    };
    // The product must exist BEFORE a blob is written, or a typo in an id
    // leaves an orphan nothing will ever reference or clean up.
    let cat = crate::hubstore::load_catalog(&place).await?.catalog;
    if cat.product(&id).is_none() {
        return Response::error("not found", 404);
    }

    let bytes = req.bytes().await?;
    let stored = match dowiz_hub::media::prepare(&bytes) {
        Ok(s) => s,
        Err(e) => return Response::error(e.to_string(), 400),
    };
    let url = stored.url();
    let key = url.trim_start_matches("/media/").to_string();

    let kv = ctx.kv("MEDIA")?;
    kv.put_bytes(&key, &bytes)?
        // No expiry. A dish photo is referenced by orders that are already
        // placed; letting it lapse would blank the picture on a receipt.
        .execute()
        .await?;
    // The media type is stored beside the blob rather than guessed at read
    // time: sniffing twice is two chances to disagree.
    kv.put(&format!("{key}#type"), stored.kind.mime())?.execute().await?;

    // WHICH PICTURE OF THE DISH THIS IS. `?variant=small` is the grid's card --
    // about 480 px, a tenth of the bytes -- and the sheet's photograph is
    // everything else. The menu emits a `srcset` when both exist, so a cold
    // visit that used to pull eighteen full photographs pulls eighteen cards.
    // Both are content-addressed blobs under the same rules; only the field in
    // the catalogue differs.
    let small = matches!(req.url()?.query_pairs().find(|(k, _)| k == "variant"), Some((_, v)) if v == "small");
    let field = if small { "imageUrlSmall" } else { "imageUrl" };
    let (pid, u) = (id.clone(), url.clone());
    crate::hubstore::with_catalog(&place, move |cat| {
        let Some(raw) = cat.product(&pid) else {
            return Err(Error::RustError("unknown product".into()));
        };
        let mut p: Value = serde_json::from_str(&raw).unwrap_or(json!({}));
        // The PREVIOUS image is not deleted. Another product may reference the
        // same bytes -- content addressing makes that likely, not rare -- and an
        // order placed an hour ago still names the dish it was sold as.
        p[field] = json!(u);
        // A NEW FULL PHOTOGRAPH DROPS THE OLD CARD. They are two renderings of
        // one picture, so leaving the previous small one beside a new large one
        // would show the customer the dish that was replaced.
        if !small {
            if let Some(obj) = p.as_object_mut() {
                obj.remove("imageUrlSmall");
            }
        }
        cat.set_product(&pid, &serde_json::to_string(&p).unwrap_or(raw));
        Ok(())
    })
    .await?;
    Response::from_json(&json!({
        field: url, "bytes": stored.bytes, "type": stored.kind.mime()
    }))
}

/// `POST /api/owner/logo`
///
/// THE VENUE'S OWN MARK. `locations.logo_url` has existed since the catalogue
/// migration and NOTHING HAS EVER READ OR WRITTEN IT -- the column was created,
/// documented, and left dead, which is why every venue's storefront carried the
/// platform's name and none carried its own. A logo is the one thing a customer
/// recognises before they read anything, so it belongs beside the venue's
/// colours in the location record, not in a column no route touches.
///
/// It is stored exactly as a dish photograph is: sniffed, checked for
/// completeness, content-addressed, written to MEDIA with its media type beside
/// it. The same rules apply for the same reasons -- a truncated logo is a
/// broken logo on every screen at once.
pub async fn set_venue_logo(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let _ = &loc;

    let bytes = req.bytes().await?;
    let stored = match dowiz_hub::media::prepare(&bytes) {
        Ok(s) => s,
        Err(e) => return Response::error(e.to_string(), 400),
    };
    let url = stored.url();
    let key = url.trim_start_matches("/media/").to_string();

    let kv = ctx.kv("MEDIA")?;
    // No expiry, for the reason a dish photo has none: a receipt printed last
    // week still names this mark.
    kv.put_bytes(&key, &bytes)?.execute().await?;
    kv.put(&format!("{key}#type"), stored.kind.mime())?.execute().await?;

    let u = url.clone();
    crate::hubstore::with_catalog(&place, move |cat| {
        let raw = cat.location().ok_or_else(|| Error::RustError("no venue".into()))?;
        let mut l: Value = serde_json::from_str(&raw).unwrap_or(json!({}));
        // The PREVIOUS logo is not deleted, exactly as a dish's previous photo
        // is not: the bytes are content-addressed and may be referenced by
        // anything already printed.
        l["logo_url"] = json!(u);
        cat.set_location(&serde_json::to_string(&l).unwrap_or(raw));
        Ok(())
    })
    .await?;
    Response::from_json(&json!({
        "logoUrl": url, "bytes": stored.bytes, "type": stored.kind.mime()
    }))
}

/// `POST /api/owner/logo/clear`
pub async fn clear_venue_logo(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    crate::hubstore::with_catalog(&place, move |cat| {
        let raw = cat.location().ok_or_else(|| Error::RustError("no venue".into()))?;
        let mut l: Value = serde_json::from_str(&raw).unwrap_or(json!({}));
        l["logo_url"] = Value::Null;
        cat.set_location(&serde_json::to_string(&l).unwrap_or(raw));
        Ok(())
    })
    .await?;
    Response::from_json(&json!({ "ok": true }))
}

/// `POST /api/owner/products/:id/image/clear`
pub async fn clear_product_image(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing product", 400);
    };
    // The blob STAYS. Clearing a dish's photo is not a statement about every
    // other dish that might share those bytes, nor about the orders that
    // already carry the URL.
    crate::hubstore::with_catalog(&place, move |cat| {
        let Some(raw) = cat.product(&id) else {
            return Err(Error::RustError("unknown product".into()));
        };
        let mut p: Value = serde_json::from_str(&raw).unwrap_or(json!({}));
        p["imageUrl"] = Value::Null;
        // Both renderings go: a card left behind would be the only picture the
        // grid still had, of a dish whose photograph the owner just removed.
        if let Some(obj) = p.as_object_mut() {
            obj.remove("imageUrlSmall");
        }
        cat.set_product(&id, &serde_json::to_string(&p).unwrap_or(raw));
        Ok(())
    })
    .await?;
    Response::from_json(&json!({ "ok": true }))
}

/// `GET /media/:name` — serve one.
///
/// PUBLIC AND IMMUTABLE. The name is a content hash, so the bytes behind it can
/// never change and the cache can hold them for a year. That is the whole
/// benefit of content addressing and it is why the header is written here
/// rather than left to a default.
pub async fn media(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let Some(name) = ctx.param("name").cloned() else {
        return Response::error("not found", 404);
    };
    // A path that is not a hash and an extension cannot be one of ours, and
    // refusing early keeps anything with a slash or a dot-dot out of the key.
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '.') || name.len() > 80 {
        return Response::error("not found", 404);
    }
    // THE EDGE KEEPS IT. A Worker's response is not edge-cached unless the
    // Worker puts it there (see `storefront::menu`), so the year of
    // `immutable` below was a promise kept only by browsers: every new device
    // paid two KV reads per photograph. Content-addressed and immutable is the
    // easiest thing in the world to cache, so cache it.
    let cache = Cache::default();
    let key = req.url()?.to_string();
    if let Some(hit) = cache.get(&key, false).await? {
        return Ok(hit);
    }
    let kv = ctx.kv("MEDIA")?;
    let Some(bytes) = kv.get(&name).bytes().await? else {
        return Response::error("not found", 404);
    };
    let kind = kv.get(&format!("{name}#type")).text().await?;
    let mut res = Response::from_bytes(bytes)?;
    let h = res.headers_mut();
    h.set("content-type", kind.as_deref().unwrap_or("application/octet-stream"))?;
    h.set("cache-control", "public, max-age=31536000, immutable")?;
    // A stored blob is data, not a document: a browser must never be talked
    // into running one.
    h.set("x-content-type-options", "nosniff")?;
    h.set("content-security-policy", "default-src 'none'; sandbox")?;
    // Stored after the response is built, so a store that fails cannot fail
    // the image.
    if let Ok(copy) = res.cloned() {
        let _ = cache.put(&key, copy).await;
    }
    Ok(res)
}
