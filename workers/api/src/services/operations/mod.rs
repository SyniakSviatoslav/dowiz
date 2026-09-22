//! What an owner can see and do about their own hub: how full it is, what has
//! been archived, and their own copy of everything.
//!
//! A VENUE'S DATA IS THE VENUE'S. `backup` hands back every image; `restore`
//! puts one into an EMPTY venue only, because writing over a live one is a
//! loss no undo can reach.

use serde_json::{json, Value};
use worker::*;

use crate::owner::now_ms;

/// `GET /api/owner/health` — what this venue is spending, and how close to a limit.
///
/// THE ARENA IS THE LIMIT NOBODY SEES UNTIL IT BITES. A bebop store never
/// reclaims a generation, so an image is spent by the NUMBER OF WRITES as much
/// as by the data — measured at 313 empty commits before a fresh roster
/// refused. When it fills, the hub answers `arena_full` and an order is refused
/// mid-service. This is the gauge that makes that a thing the owner sees coming
/// rather than a thing that happens to them.
///
/// EVERY IMAGE, not just the log, because they fill for different reasons: the
/// log grows with orders, settings with writes, posts with drafts.
///
/// THERE IS NO `dead` FIGURE. The superblock has a `superseded_cells` column
/// and nothing on this write path ever writes it, so a ratio built on it would
/// read 0 forever while looking like a measurement. See `dowiz_hub::Usage`.
pub async fn health(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let (_, loc, (hub, cat)) =
        match crate::owner::owner_beside(&req, &ctx, &db, &place, crate::hubstore::load_both(&place)).await
        {
            Ok(v) => v,
            Err(r) => return Ok(r),
        };
    // The smaller images, beside the two the dashboard already needs.
    let (settings, posts, stock) = futures_util::future::join3(
        crate::hubstore::load_settings(&place),
        crate::hubstore::load_posts(&place),
        crate::hubstore::load_stock(&place),
    )
    .await;

    let mut images = serde_json::Map::new();
    images.insert("log".into(), crate::gauges::gauge(hub.hub.usage()));
    images.insert("catalog".into(), crate::gauges::gauge(cat.catalog.usage()));
    if let Ok(s) = &settings {
        images.insert("settings".into(), crate::gauges::gauge(s.settings.usage()));
    }
    if let Ok(p) = &posts {
        images.insert("posts".into(), crate::gauges::gauge(p.posts.usage()));
    }
    if let Ok(st) = &stock {
        images.insert("stock".into(), crate::gauges::gauge(st.stock.usage()));
    }

    // Which image the owner is being warned about, and which reading is a
    // sawtooth rather than a warning at all. See `crate::gauges::verdict`.
    let (worst, worst_growing, verdict) = crate::gauges::verdict(&images);

    // THE FAILURES THE LOGS NO LONGER CARRY. Workers Logs are sampled at one
    // request in ten and no token on this box can read them at all, so the
    // errors that matter are also rows -- and this is the screen that already
    // answers "is this venue healthy". An empty list is the good answer.
    let audit = crate::errlog::recent(&place.ns, &place.venue, 20).await.unwrap_or_default();

    // THE BREAKERS ARE VISIBLE OR THEY ARE NOT AN INSTRUMENT. A breaker that
    // silently protects a venue is indistinguishable from one that silently
    // does nothing, and this codebase has paid for that distinction before.
    let rails = crate::rail::snapshot(&place, now_ms()).await;

    // Every record the venue's logs hold and this build cannot read. See
    // `crate::quarantine`: a non-zero count is a failing gate, not a warning.
    let quarantined = crate::quarantine::seen(&hub.hub, audit.quarantined);

    Response::from_json(&json!({
        "venue": loc,
        "images": images,
        "worstUsedPerMille": worst,
        "worstGrowingPerMille": worst_growing,
        "verdict": verdict,
        // The claim the root carries. `events` is what the log delivers, and
        // `quarantined` is the difference: the three must add up.
        "orders": hub.hub.len(),
        "events": hub.hub.events().len(),
        "quarantined": quarantined,
        "errors": audit.errors,
        "rails": rails,
    }))
}

/// `GET /api/owner/history` — what has been archived, and what is in one.
///
/// THE HOT LOG IS NOT THE WHOLE HISTORY ANY MORE. Without a route that reads
/// the archives, rotation would be deletion with extra steps: the bytes would
/// exist and nothing could reach them. `?archive=log@<n>` returns that
/// archive's orders, folded exactly as the live ones are.
pub async fn history(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let (_, _loc, settings) = match crate::owner::owner_beside(
        &req,
        &ctx,
        &db,
        &place,
        crate::hubstore::load_settings(&place),
    )
    .await
    {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let archives = crate::hubstore::archives_of(&settings.settings);
    let wanted = req
        .url()
        .ok()
        .and_then(|u| u.query_pairs().find(|(k, _)| k == "archive").map(|(_, v)| v.to_string()));

    let Some(id) = wanted else {
        return Response::from_json(&json!({ "archives": archives, "keepMs": crate::hubstore::HOT_KEEP_MS }));
    };
    match crate::hubstore::archive_orders(&place, &id).await? {
        Some(orders) => {
            let rows: Vec<Value> = orders
                .into_iter()
                .filter_map(|e| serde_json::from_str::<Value>(&e.order_json).ok())
                .collect();
            let mut res = Response::from_json(&json!({ "archive": id, "orders": rows }))?;
            // An archive holds customers' addresses, exactly as the live log
            // does: nothing between here and the owner's screen keeps a copy.
            res.headers_mut().set("cache-control", "private, no-store")?;
            Ok(res)
        }
        None => Response::error("no such archive", 404),
    }
}

/// `POST /api/owner/hub/rotate` — move finished history out of the hot log now.
///
/// The nightly cron does this for every venue; this is the same call for an
/// owner who wants it done before then, and for a test that wants to see it
/// happen.
pub async fn rotate_now(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let loc = match crate::owner::owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    match crate::hubstore::rotate(&place, now_ms()).await {
        Ok(v) => Response::from_json(&v),
        Err(e) => {
            crate::loud!(&place.ns, Some(&place.venue), "hub.rotate", "{e}");
            Response::error(e.to_string(), 500)
        }
    }
}

/// `GET /api/owner/backup` — the venue's own copy of everything.
///
/// A DOWNLOAD, NOT A DASHBOARD. The point is that the file leaves this platform
/// and lands where the venue keeps things. Cloudflare's thirty-day time travel
/// is a fine safety net and is not theirs.
pub async fn backup(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let (_, _loc, bundle) =
        match crate::owner::owner_beside(&req, &ctx, &db, &place, crate::hubstore::export(&place)).await {
            Ok(v) => v,
            Err(r) => return Ok(r),
        };
    let mut res = Response::from_json(&bundle)?;
    let stamp = Date::now().as_millis();
    let h = res.headers_mut();
    h.set("content-disposition", &format!("attachment; filename=\"dowiz-backup-{stamp}.json\""))?;
    // A backup holds every order this venue has ever taken. Nothing between
    // here and the owner's disk may keep a copy.
    h.set("cache-control", "private, no-store")?;
    Ok(res)
}

/// `POST /api/owner/restore` — put one back, into an EMPTY venue only.
///
/// See `hubstore::import` for why the refusal is the feature: a restore that
/// overwrites a live hub is a one-click way to erase a venue's history, and it
/// would be reachable by anything that could reach an owner's token.
pub async fn restore(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let bundle: Value = match req.json().await {
        Ok(v) => v,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let db = ctx.d1("DB")?;
    // Authority first and alone: this one writes, so nothing starts beside it.
    let loc = match crate::owner::owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    match crate::hubstore::import(&place, &bundle).await {
        Ok(written) => Response::from_json(&json!({ "restored": written })),
        // The refusals here are all the caller's to fix -- a damaged file, a
        // venue that is not empty -- so they are 409, with the reason said.
        Err(e) => Response::error(format!("{e}"), 409),
    }
}
