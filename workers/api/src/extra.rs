//! The surfaces built natively and ported here, on the SHARED logic.
//!
//! WHAT THIS MODULE IS AND WHY IT IS NOT A SECOND IMPLEMENTATION. Every decision
//! below is made by `dowiz_hub` -- `promo::redeem`, `allergens::read`,
//! `activation::missing`, `brand::Brand` -- exactly as the native adapter makes
//! it. What is duplicated is the HTTP plumbing and the storage call, because a
//! Worker cannot run axum and the native adapter cannot run on D1. The rule that
//! keeps the two from drifting is that nothing here decides anything: if a
//! refusal is worded differently from the native one, that is a bug in this file.
//!
//! WHAT IS DELIBERATELY NOT HERE. Courier invites live on the hub's roster, and
//! this Worker authenticates against D1 `users`/`memberships` -- a different
//! identity model. Porting them means moving the roster into D1, which is its
//! own change with its own migration, and doing it in passing would leave two
//! half-built identity systems instead of one.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::wasm_bindgen::JsValue;
use worker::*;

use crate::owner::{now_ms, owner_and_venue};

// ── helpers ─────────────────────────────────────────────────────────────────

fn start_of_day_ms(now: i64) -> i64 {
    // Europe/Tirane. A UTC day boundary would reset an owner's takings at one or
    // two in the morning -- during service on a Saturday.
    let off = 2 * 60 * 60 * 1000;
    ((now + off) / 86_400_000) * 86_400_000 - off
}

fn currency_of(cat: &dowiz_hub::catalog::Catalog) -> String {
    cat.location()
        .and_then(|j| serde_json::from_str::<Value>(&j).ok())
        .and_then(|l| {
            l.get("currency_code")
                .or_else(|| l.get("currency"))
                .and_then(Value::as_str)
                .map(String::from)
        })
        .unwrap_or_else(|| "ALL".into())
}

/// Orders belonging to this venue. The Worker is multi-tenant in its tables even
/// though a hub is not, so every fold filters -- an unfiltered one would show a
/// neighbouring venue's takings.
fn orders_of(loaded: &crate::hubstore::Loaded, loc: &str) -> Vec<Value> {
    loaded
        .hub
        .orders()
        .into_iter()
        .filter_map(|e| serde_json::from_str::<Value>(&e.order_json).ok())
        .filter(|o| {
            o.get("location_id").and_then(Value::as_str).map(|l| l == loc).unwrap_or(true)
        })
        .collect()
}

// ── public: what a code would take off this basket ──────────────────────────

#[derive(Deserialize)]
struct PromoCheckIn {
    code: String,
    items: Vec<crate::storefront::LineIn>,
}

/// `POST /api/promo/check`
///
/// The basket arrives as product ids, never as a subtotal: the hub prices it
/// with the catalogue, which is the same source the order uses. A preview that
/// trusted a number from the browser would quote whatever the browser asked for.
///
/// It is a PREVIEW and says so: the order re-checks under the write, so a code
/// on its last use can be quoted here and refused at checkout. That is the right
/// way round -- the alternative gives the same last use away twice.
pub async fn promo_check(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: PromoCheckIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    // TWO IMAGES, one hub: the log and the catalogue have different roots and
    // cannot share one, so a route that reads both loads both.
    // ONE ROUND TRIP for both images: see `load_both`.
    let (loaded, cat) = crate::hubstore::load_both(&place).await?;
    let cat = cat.catalog;
    let code = dowiz_hub::promo::normalise(&body.code);
    let Some(p) = cat.promo(&code).as_deref().and_then(dowiz_hub::promo::Promo::parse)
    else {
        return Response::error(dowiz_hub::promo::Refusal::Unknown.as_str(), 400);
    };

    let mut subtotal = 0i64;
    for it in &body.items {
        let Some(pj) = cat.product(&it.product_id) else {
            return Response::error(format!("unknown product: {}", it.product_id), 400);
        };
        let v: Value = serde_json::from_str(&pj).unwrap_or(json!({}));
        let base = v.get("price").and_then(Value::as_i64).unwrap_or(0);
        let groups = dowiz_hub::modifiers::groups_of(&pj);
        let delta = dowiz_hub::modifiers::price(&groups, &it.modifier_ids)
            .map(|c| c.delta)
            .unwrap_or(0);
        subtotal += (base + delta).max(0) * it.quantity.clamp(1, 99);
    }

    let used = crate::hubstore::promo_uses(&loaded.hub, &code);
    match p.redeem(subtotal, now_ms(), used) {
        Ok(cut) => Response::from_json(&json!({
            "code": p.code, "discount": cut, "subtotal": subtotal, "total": subtotal - cut
        })),
        Err(r) => Response::error(r.as_str(), 409),
    }
}

// ── public: what the customer thought ───────────────────────────────────────

#[derive(Deserialize)]
struct FeedbackIn {
    text: String,
}

/// `POST /api/order/:id/feedback` — a sentence to the venue about one order.
///
/// NO STARS, NO SCORE, NOT ON ANYONE. A number attached to an order becomes a
/// number attached to whoever carried it the moment anybody joins the two, and
/// dowiz does not rank the people who work through it. A kitchen can act on "the
/// rice was cold"; it can do nothing with a three.
pub async fn feedback(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing order", 400);
    };
    let body: FeedbackIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let text = body.text.trim().to_string();
    if text.is_empty() {
        return Response::error("say something, or say nothing", 400);
    }
    if text.chars().count() > 600 {
        return Response::error("that is longer than a note about an order", 400);
    }
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    // The customer's own token for THIS order and nothing else. The owner's is
    // refused too: this is the customer's voice and not the venue's.
    match crate::auth::authenticate(&req, &ctx.env, &db, now_ms()).await {
        Ok(crate::auth::Principal::Customer { order_id, .. }) if order_id == id => {}
        Ok(_) => return Response::error("that link is not for this order", 401),
        Err(_) => return Response::error("this order needs the link you were given", 401),
    }

    let at = now_ms();
    let oid = id.clone();
    let outcome = crate::hubstore::with_hub(&place, move |hub| {
        let current = hub
            .order(&oid)
            .map_err(|_| Error::RustError("no such order".into()))?;
        let mut o: Value = serde_json::from_str(&current).unwrap_or(json!({}));
        if o.get("feedback").is_some() {
            return Err(Error::RustError("already".into()));
        }
        let status = o.get("status").and_then(Value::as_str).unwrap_or("");
        if !matches!(status, "DELIVERED" | "REJECTED" | "CANCELLED") {
            return Err(Error::RustError("running".into()));
        }
        o["feedback"] = json!({ "text": text, "at": at });
        let stored = serde_json::to_string(&o).unwrap_or(current);
        // `Noted`, not `Advanced`: a note is not a transition the order machine
        // decided, and writing it as one puts an edge in the log that does not
        // exist.
        hub.append(dowiz_hub::EventKind::Noted, &oid, &stored, at as u64, [0u8; 32])
            .map_err(|e| Error::RustError(format!("{e:?}")))?;
        Ok(())
    })
    .await;
    match outcome {
        Ok(()) => Response::from_json(&json!({ "ok": true })),
        Err(e) if e.to_string().contains("already") => {
            Response::error("you have already left a note", 409)
        }
        Err(e) if e.to_string().contains("running") => Response::error(
            "this order is still running -- call the venue if something is wrong",
            409,
        ),
        Err(e) if e.to_string().contains("no such order") => Response::error("not found", 404),
        Err(e) => Err(e),
    }
}

// ── owner: the numbers, folded from the orders ──────────────────────────────

/// `GET /api/owner/analytics?location_id=&days=7|30`
///
/// No analytics store. A second table of pre-aggregated numbers is a second
/// thing that can disagree with the orders, and at one restaurant the fold is a
/// loop over a few hundred envelopes.
pub async fn analytics(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let days: i64 = req
        .url()
        .ok()
        .and_then(|u| {
            u.query_pairs().find(|(k, _)| k == "days").and_then(|(_, v)| v.parse::<i64>().ok())
        })
        .map(|d| if d >= 30 { 30 } else { 7 })
        .unwrap_or(7);

    let now = now_ms();
    let day_ms = 86_400_000;
    let from = start_of_day_ms(now) - (days - 1) * day_ms;
    // ONE ROUND TRIP for both images: see `load_both`.
    let (loaded, cat) = crate::hubstore::load_both(&place).await?;
    let cat = cat.catalog;

    let mut by_day: Vec<(i64, i64, i64)> = (0..days).map(|i| (from + i * day_ms, 0, 0)).collect();
    let mut by_hour = [0i64; 24];
    let mut products: Vec<(String, i64, i64)> = Vec::new();
    let (mut orders, mut revenue, mut rejected) = (0i64, 0i64, 0i64);
    let (mut delivery, mut pickup) = (0i64, 0i64);

    for o in orders_of(&loaded, &loc) {
        let at = o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0);
        if at < from {
            continue;
        }
        let status = o.get("status").and_then(Value::as_str).unwrap_or("");
        let refused = matches!(status, "REJECTED" | "CANCELLED");
        orders += 1;
        if refused {
            rejected += 1;
        }
        // Tips excluded: they are the courier's money passing through.
        let took = if refused {
            0
        } else {
            o.get("total").and_then(Value::as_i64).unwrap_or(0)
                - o.get("tip").and_then(Value::as_i64).unwrap_or(0)
        };
        revenue += took;
        if o.get("fulfilment").and_then(|f| f.get("kind")).and_then(Value::as_str)
            == Some("pickup")
        {
            pickup += 1;
        } else {
            delivery += 1;
        }
        let idx = ((at - from) / day_ms).clamp(0, days - 1) as usize;
        by_day[idx].1 += 1;
        by_day[idx].2 += took;
        let local_h = (((at + 2 * 60 * 60 * 1000) % day_ms) / 3_600_000).clamp(0, 23) as usize;
        by_hour[local_h] += 1;

        if refused {
            continue;
        }
        for it in o.get("items").and_then(Value::as_array).into_iter().flatten() {
            let id = it.get("product_id").and_then(Value::as_str).unwrap_or("").to_string();
            let q = it.get("quantity").and_then(Value::as_i64).unwrap_or(0);
            let money = it.get("unit_price").and_then(Value::as_i64).unwrap_or(0) * q;
            match products.iter_mut().find(|p| p.0 == id) {
                Some(p) => {
                    p.1 += q;
                    p.2 += money;
                }
                None => products.push((id, q, money)),
            }
        }
    }
    // By money, then by id so a tie is stable across reads rather than
    // whichever way the fold happened to land.
    products.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
    products.truncate(8);

    Response::from_json(&json!({
        "days": days,
        "orders": orders,
        "revenue": revenue,
        "rejected": rejected,
        // INTEGER DIVISION. Money never becomes a float in this system, and a
        // mean that rounds down by one lek is honest in a way 2649.9999 is not.
        "averageOrder": if orders > rejected { revenue / (orders - rejected) } else { 0 },
        "delivery": delivery,
        "pickup": pickup,
        "byDay": by_day.iter().map(|(at, n, r)| json!({ "at": at, "orders": n, "revenue": r }))
            .collect::<Vec<_>>(),
        "byHour": by_hour.to_vec(),
        "topProducts": products.iter().map(|(id, q, m)| json!({
            "id": id,
            "name": cat.product(id)
                .and_then(|j| serde_json::from_str::<Value>(&j).ok())
                .and_then(|p| p.get("name").cloned())
                .unwrap_or(json!(id)),
            "quantity": q, "revenue": m
        })).collect::<Vec<_>>(),
        "currency": currency_of(&cat),
    }))
}

// ── owner: promo codes ──────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PromoIn {
    code: String,
    kind: String,
    value: i64,
    #[serde(default)]
    min_order: Option<i64>,
    #[serde(default)]
    from_ms: Option<i64>,
    #[serde(default)]
    until_ms: Option<i64>,
    #[serde(default)]
    max_uses: Option<i64>,
    #[serde(default)]
    active: Option<bool>,
}

/// `GET /api/owner/promotions?location_id=`
pub async fn promotions(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    // The membership query and the image read do not depend on each other;
    // `owner_with_hub` runs them together. See it for why the token is still
    // verified before either is issued.
    let (_, loc, (loaded, cat)) =
        match crate::owner::owner_beside(&req, &ctx, &db, crate::hubstore::load_both(&place)).await {
            Ok(v) => v,
            Err(r) => return Ok(r),
        };
    let cat = cat.catalog;
    let now = now_ms();
    let mut rows: Vec<Value> = cat
        .promos()
        .into_iter()
        .filter_map(|(_, j)| dowiz_hub::promo::Promo::parse(&j))
        .map(|p| {
            // Status is DERIVED, never stored: a stored one goes stale the
            // moment the clock passes the window, and the owner would be
            // reading a label that no longer describes the code.
            let used = crate::hubstore::promo_uses(&loaded.hub, &p.code);
            json!({
                "code": p.code, "kind": p.kind.as_str(), "value": p.value,
                "minOrder": p.min_order, "fromMs": p.from_ms, "untilMs": p.until_ms,
                "maxUses": p.max_uses, "active": p.active,
                "used": used, "status": p.status(now, used).as_str(),
            })
        })
        .collect();
    // Alphabetical: a code is looked up by its name, and sorting by status would
    // move a row under the owner's cursor the moment a window closed.
    rows.sort_by(|a, b| a["code"].as_str().cmp(&b["code"].as_str()));
    Response::from_json(&json!({ "promotions": rows, "currency": currency_of(&cat) }))
}

/// `POST /api/owner/promotions?location_id=` — create or replace.
///
/// AN UNKNOWN FIELD IS A REFUSAL. serde's default is to ignore what it does not
/// recognise, and for a promo that gives money away: a client sending `until`
/// instead of `untilMs` gets a code with no expiry, silently, for ever.
/// Deserialised by hand so the refusal names the field instead of arriving as a
/// bare 422 with an empty body.
pub async fn set_promotion(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let raw: Value = match req.json().await {
        Ok(v) => v,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let body: PromoIn = match serde_json::from_value(raw) {
        Ok(b) => b,
        Err(e) => return Response::error(e.to_string(), 400),
    };
    use dowiz_hub::promo::{normalise, valid_code, valid_value, Kind, Promo};
    let code = normalise(&body.code);
    if !valid_code(&code) {
        return Response::error("a code is 3 to 16 letters or digits", 400);
    }
    let Some(kind) = Kind::parse(&body.kind) else {
        return Response::error("a code takes off a percent or a fixed amount", 400);
    };
    if !valid_value(kind, body.value) {
        return Response::error(
            match kind {
                Kind::Percent => "a percentage is between 1 and 100",
                Kind::Fixed => "a fixed discount must be more than nothing",
            },
            400,
        );
    }
    if let (Some(f), Some(u)) = (body.from_ms, body.until_ms) {
        if u <= f {
            return Response::error("that window ends before it starts", 400);
        }
    }
    let p = Promo {
        code: code.clone(),
        kind,
        value: body.value,
        min_order: body.min_order.unwrap_or(0).max(0),
        from_ms: body.from_ms,
        until_ms: body.until_ms,
        max_uses: body.max_uses.filter(|n| *n > 0),
        active: body.active.unwrap_or(true),
    };
    let stored = p.to_json();
    crate::hubstore::with_catalog(&place, move |cat| {
        cat.set_promo(&code, &stored);
        Ok(())
    })
    .await?;
    Response::from_json(&json!({ "ok": true, "code": p.code }))
}

/// `POST /api/owner/promotions/:code/delete?location_id=` — a real delete.
///
/// Distinct from the active switch: switching off is reversible and keeps the
/// dates, deleting frees the word.
pub async fn delete_promotion(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let Some(code) = ctx.param("code").cloned() else {
        return Response::error("missing code", 400);
    };
    let code = dowiz_hub::promo::normalise(&code);
    let gone = crate::hubstore::with_catalog(&place, move |cat| Ok(cat.remove_promo(&code))).await?;
    if !gone {
        return Response::error("not found", 404);
    }
    Response::from_json(&json!({ "ok": true }))
}

// ── owner: can this venue open? ─────────────────────────────────────────────

/// `GET /api/owner/activation?location_id=`
///
/// Three things, and an order is useless without all of them: something to
/// sell, somebody who hears the order land, and a way to get it there. The leg
/// that catches real venues is the second -- nobody notices nothing is bound to
/// the bot until an order has sat unanswered for forty minutes.
pub async fn activation(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    // The membership query and this read do not depend on each other, so
    // `owner_beside` runs them together. The token is still verified before
    // either is issued -- see it for why that order matters.
    let (_, loc, loaded) =
        match crate::owner::owner_beside(&req, &ctx, &db, crate::hubstore::load_catalog(&place)).await {
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
    let phone = raw.get("phone").and_then(Value::as_str).unwrap_or("");
    let f = dowiz_hub::activation::Facts {
        sellable_dishes: sellable,
        // The Worker reaches Telegram through a configured bot rather than the
        // hub's own subscription store, so the question "would anything hear
        // this" is answered by whether a bot is configured at all.
        telegram_chats: usize::from(ctx.env.secret("TELEGRAM_BOT_TOKEN").is_ok()),
        has_venue_phone: phone.chars().filter(char::is_ascii_digit).count() >= 8,
        delivery_configured: raw.get("delivery_fee").is_some()
            || raw.get("delivery_zones").is_some(),
        pickup_enabled: raw.get("pickup").and_then(Value::as_bool).unwrap_or(false),
    };
    let missing = dowiz_hub::activation::missing(&f);
    Response::from_json(&json!({
        "canOpen": missing.is_empty(),
        "missing": missing.iter().map(|r| json!({ "key": r.key(), "why": r.as_str() }))
            .collect::<Vec<_>>(),
        "facts": {
            "sellableDishes": f.sellable_dishes, "telegramChats": f.telegram_chats,
            "hasVenuePhone": f.has_venue_phone, "deliveryConfigured": f.delivery_configured,
            "pickupEnabled": f.pickup_enabled,
        }
    }))
}

// ── owner: the five tokens ──────────────────────────────────────────────────

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
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    use dowiz_hub::brand::{Brand, PRESETS, RADIUS_MAX, TYPE_PAIRS};
    // The membership query and this read do not depend on each other, so
    // `owner_beside` runs them together. The token is still verified before
    // either is issued -- see it for why that order matters.
    let (_, loc, loaded) =
        match crate::owner::owner_beside(&req, &ctx, &db, crate::hubstore::load_catalog(&place)).await {
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
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
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
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let Some(b) = dowiz_hub::brand::preset(&body.preset) else {
        return Response::error(format!("{:?} is not a preset", body.preset), 400);
    };
    store_brand(&place, b).await
}

// ── owner: who orders here, without saying who they are ────────────────────
//
// THERE IS NO CUSTOMER REGISTRY BEHIND THIS. It is a fold over the orders,
// computed per request and stored nowhere, so the venue holds exactly what it
// held before. What a registry would add is not the data but the CONVENIENCE --
// a ready-made list, sorted by value, one click from export. So the protection
// lives where it can still do work: redacted by default, un-redacting is
// deliberate, and the act is written into the append-only log.

/// Enough to recognise a number you already know; not enough to dial one you do
/// not.
fn mask_phone(p: &str) -> String {
    let d: Vec<char> = p.chars().filter(|c| c.is_ascii_digit()).collect();
    if d.len() < 4 {
        return "•".repeat(d.len().max(1));
    }
    format!(
        "+{}•••••{}",
        d[..3].iter().collect::<String>(),
        d[d.len() - 2..].iter().collect::<String>()
    )
}

fn mask_name(n: &str) -> String {
    let parts: Vec<String> = n
        .split_whitespace()
        .filter_map(|w| w.chars().next())
        .map(|c| format!("{}.", c.to_uppercase()))
        .collect();
    if parts.is_empty() {
        "—".into()
    } else {
        parts.join(" ")
    }
}

/// A stable, non-reversible handle for a phone. The audit entry must not carry
/// the number it is about, and a URL holding a phone number puts it in every
/// proxy log between here and the browser.
fn customer_key(secret: &[u8], phone: &str) -> String {
    let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
    let mac = dowiz_hub::crypto::hmac_sha256(secret, digits.as_bytes());
    dowiz_hub::crypto::hex(&mac[..8])
}

fn signing_secret(env: &Env) -> Vec<u8> {
    env.secret("AUTH_SIGNING_KEY")
        .map(|v| v.to_string().into_bytes())
        // A hub with no configured key still masks consistently WITHIN itself:
        // the key only has to be stable and secret, and an unconfigured Worker
        // has bigger problems than a predictable mask.
        .unwrap_or_else(|_| b"dowiz-unconfigured".to_vec())
}

/// `GET /api/owner/customers?location_id=&sort=spent|orders`
pub async fn customers(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    // The membership query and the image read do not depend on each other, so
    // `owner_beside` runs them together. Measured against this very handler
    // before it was converted: 293 ms of server time against the dashboard's
    // 235 for the same 655 KB, on the same deployment at the same minute.
    let (_, loc, (loaded, loaded_cat)) =
        match crate::owner::owner_beside(&req, &ctx, &db, crate::hubstore::load_both(&place)).await {
            Ok(v) => v,
            Err(r) => return Ok(r),
        };
    let sort = req
        .url()
        .ok()
        .and_then(|u| u.query_pairs().find(|(k, _)| k == "sort").map(|(_, v)| v.to_string()));
    let secret = signing_secret(&ctx.env);

    let mut rows: Vec<(String, String, String, i64, i64, i64)> = Vec::new();
    for o in orders_of(&loaded, &loc) {
        let Some(phone) = o.get("contact").and_then(|c| c.get("phone")).and_then(Value::as_str)
        else {
            continue;
        };
        let name = o.get("contact").and_then(|c| c.get("name")).and_then(Value::as_str).unwrap_or("");
        let at = o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0);
        // What they spent WITH THE VENUE: a refused order is not money taken,
        // and the tip went to the courier.
        let spent = match o.get("status").and_then(Value::as_str) {
            Some("REJECTED" | "CANCELLED") => 0,
            _ => {
                o.get("total").and_then(Value::as_i64).unwrap_or(0)
                    - o.get("tip").and_then(Value::as_i64).unwrap_or(0)
            }
        };
        let key = customer_key(&secret, phone);
        match rows.iter_mut().find(|r| r.0 == key) {
            Some(r) => {
                r.3 += 1;
                r.4 += spent;
                r.5 = r.5.max(at);
            }
            None => rows.push((key, mask_name(name), mask_phone(phone), 1, spent, at)),
        }
    }
    match sort.as_deref() {
        Some("spent") => rows.sort_by(|a, b| b.4.cmp(&a.4).then(b.5.cmp(&a.5))),
        Some("orders") => rows.sort_by(|a, b| b.3.cmp(&a.3).then(b.5.cmp(&a.5))),
        // Newest first: the question at the end of a shift is who has just been
        // in, not who is worth the most.
        _ => rows.sort_by(|a, b| b.5.cmp(&a.5)),
    }
    let cat = loaded_cat.catalog;
    Response::from_json(&json!({
        "customers": rows.iter().map(|r| json!({
            "key": r.0, "name": r.1, "phone": r.2,
            "orders": r.3, "spent": r.4, "lastAt": r.5 })).collect::<Vec<_>>(),
        "currency": currency_of(&cat),
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RevealIn {
    reason: String,
}

/// `POST /api/owner/customers/:key/reveal?location_id=`
///
/// The audit entry is appended BEFORE the answer is returned. An un-auditable
/// reveal is the one thing this route must not do, and answering first would
/// make the log best-effort.
pub async fn reveal_customer(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: RevealIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let (who, loc) = match owner_and_venue(&req, &ctx, &db).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let reason = body.reason.trim().to_string();
    if reason.len() < 3 {
        return Response::error("say why you are looking", 400);
    }
    let Some(key) = ctx.param("key").cloned() else {
        return Response::error("missing customer", 400);
    };

    let secret = signing_secret(&ctx.env);
    let loaded = crate::hubstore::load(&place).await?;
    let mut found: Option<(String, String, Vec<Value>)> = None;
    for o in orders_of(&loaded, &loc) {
        let Some(phone) = o.get("contact").and_then(|c| c.get("phone")).and_then(Value::as_str)
        else {
            continue;
        };
        if customer_key(&secret, phone) != key {
            continue;
        }
        let name = o
            .get("contact")
            .and_then(|c| c.get("name"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let row = json!({
            "id": o.get("id").cloned().unwrap_or(Value::Null),
            "at": o.get("created_at_ms").cloned().unwrap_or(json!(0)),
            "status": o.get("status").cloned().unwrap_or(Value::Null),
            "total": o.get("total").cloned().unwrap_or(json!(0)),
            "address": o.get("fulfilment").and_then(|f| f.get("address"))
                .and_then(|a| a.get("line")).cloned().unwrap_or(Value::Null),
        });
        match &mut found {
            Some((_, _, rows)) => rows.push(row),
            None => found = Some((name, phone.to_string(), vec![row])),
        }
    }
    let Some((name, phone, orders)) = found else {
        return Response::error("not found", 404);
    };

    let at = now_ms();
    let entry = json!({ "by": who, "at": at, "reason": reason }).to_string();
    let subject = format!("cust:{key}");
    crate::hubstore::with_hub(&place, move |hub| {
        hub.append(dowiz_hub::EventKind::Revealed, &subject, &entry, at as u64, [0u8; 32])
            .map_err(|e| Error::RustError(format!("{e:?}")))
    })
    .await?;

    Response::from_json(&json!({ "name": name, "phone": phone, "orders": orders }))
}

/// `GET /api/owner/customers/reveals?location_id=` — who has been looking.
pub async fn reveals(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    // The membership query and this read do not depend on each other, so
    // `owner_beside` runs them together. The token is still verified before
    // either is issued -- see it for why that order matters.
    let (_, loc, loaded) =
        match crate::owner::owner_beside(&req, &ctx, &db, crate::hubstore::load(&place)).await {
            Ok(v) => v,
            Err(r) => return Ok(r),
        };
    let out: Vec<Value> = loaded
        .hub
        .reveals()
        .into_iter()
        .take(200)
        .filter_map(|e| {
            let v: Value = serde_json::from_str(&e.order_json).ok()?;
            Some(json!({
                "customer": e.order_id.strip_prefix("cust:").unwrap_or(&e.order_id),
                "by": v.get("by").cloned().unwrap_or(Value::Null),
                "at": v.get("at").cloned().unwrap_or(json!(0)),
                "reason": v.get("reason").cloned().unwrap_or(Value::Null),
            }))
        })
        .collect();
    Response::from_json(&json!({ "reveals": out }))
}

// ── the offer window ────────────────────────────────────────────────────────

/// How long a courier has to answer an assignment before it goes back to the
/// pool.
///
/// FIVE MINUTES, and the number is a judgement rather than a constant somebody
/// liked. A minute is not enough: a courier is holding a bike, or a door, or the
/// previous order's change. An evening is far too long -- the food is cold and
/// the customer has phoned twice.
///
/// NOTHING IS AUTO-DECLINED. Lapsing refuses nothing on the courier's behalf and
/// counts against them in no way; it only stops the order being exclusively
/// theirs.
pub const OFFER_WINDOW_MS: i64 = 5 * 60 * 1000;

/// Has this order been offered to somebody who has not answered?
///
/// An ACCEPTED order never lapses however long it takes, and an order assigned
/// before this field existed is not retroactively reopened by one deploy.
pub fn offer_lapsed(o: &Value, now: i64) -> bool {
    if o.get("accepted_at_ms").and_then(Value::as_i64).is_some() {
        return false;
    }
    match o.get("assigned_at_ms").and_then(Value::as_i64) {
        None => false,
        Some(at) => now.saturating_sub(at) >= OFFER_WINDOW_MS,
    }
}

/// `GET /api/courier/history` — what this courier has finished.
///
/// A fold over the orders, like everything else that counts. No history table:
/// a second list of the same deliveries is a second thing that can disagree.
pub async fn courier_history(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let me = match crate::auth::authenticate(&req, &ctx.env, &db, now_ms()).await {
        Ok(crate::auth::Principal::Courier { courier_id, .. }) => courier_id,
        Ok(_) => return Response::error("forbidden role", 403),
        Err(e) => return e.into_response(),
    };
    let loaded = crate::hubstore::load(&place).await?;
    let mut rows: Vec<Value> = loaded
        .hub
        .orders()
        .into_iter()
        .filter_map(|e| serde_json::from_str::<Value>(&e.order_json).ok())
        .filter(|o| o.get("courier_id").and_then(Value::as_str) == Some(me.as_str()))
        .filter(|o| {
            matches!(
                o.get("status").and_then(Value::as_str),
                Some("DELIVERED" | "CANCELLED" | "REJECTED")
            )
        })
        .map(|o| {
            json!({
                "id": o.get("id").cloned().unwrap_or(Value::Null),
                "at": o.get("created_at_ms").cloned().unwrap_or(json!(0)),
                "status": o.get("status").cloned().unwrap_or(Value::Null),
                "total": o.get("total").cloned().unwrap_or(json!(0)),
                "cashCollected": o.get("cash_collected").cloned().unwrap_or(json!(0)),
                "street": o.get("fulfilment").and_then(|f| f.get("address"))
                    .and_then(|a| a.get("line")).cloned().unwrap_or(Value::Null),
            })
        })
        .collect();
    rows.sort_by(|a, b| b["at"].as_i64().cmp(&a["at"].as_i64()));
    rows.truncate(50);
    Response::from_json(&json!({ "history": rows }))
}

// ── owner: the venue's own configuration ────────────────────────────────────

/// `GET /api/owner/settings` — the declared keys, their values and their hints.
///
/// The KEY SPACE IS CLOSED. An open one would make this a place to stash
/// arbitrary data that nothing ever reads back, and the console renders the
/// list the hub declares rather than a list of its own.
pub async fn settings(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let loaded = crate::hubstore::load_settings(&place).await?;
    let values: Value = serde_json::from_str(&loaded.settings.as_json()).unwrap_or(json!({}));
    Response::from_json(&json!({
        "values": values,
        "known": dowiz_hub::settings::KNOWN.iter().map(|k| json!({
            "key": k.key, "label": k.label, "hint": k.hint, "default": k.default,
            "secret": dowiz_hub::settings::is_secret(k.key),
        })).collect::<Vec<_>>(),
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingIn {
    key: String,
    /// An empty value CLEARS the setting, which is how an owner removes a token
    /// they can no longer see.
    value: String,
}

/// `POST /api/owner/settings`
pub async fn set_setting(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: SettingIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    if !dowiz_hub::settings::KNOWN.iter().any(|k| k.key == body.key) {
        return Response::error(format!("unknown setting {:?}", body.key), 400);
    }
    if body.value.len() > 4096 {
        return Response::error("value too long", 400);
    }
    // An endpoint is checked HERE, while the owner can still fix it, rather
    // than at the first question when they are mid-rush. Plain http is refused
    // outright: a Worker has no loopback, so the native adapter's "only to an
    // address on this machine" exemption cannot apply and would only be a way
    // to send a venue's token in the clear.
    if body.key == "ai.endpoint" && !body.value.trim().is_empty() {
        let v = body.value.trim();
        if !v.starts_with("https://") {
            return Response::error("the endpoint must be https from a Worker", 400);
        }
    }
    let (key, value) = (body.key.clone(), body.value.clone());
    crate::hubstore::with_settings(&place, move |s| {
        s.set(&key, &value);
        Ok(())
    })
    .await?;
    Response::from_json(&json!({ "ok": true, "key": body.key }))
}

/// `GET /api/owner/features` — what can be switched, and what it costs.
///
/// The list comes from the HUB. A console that held its own copy would show a
/// switch for something that no longer exists, or miss one that does.
pub async fn features(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let s = crate::hubstore::load_settings(&place).await?.settings;
    Response::from_json(&json!({
        "features": dowiz_hub::features::all(&s).into_iter().map(|(f, on)| json!({
            "key": f.key, "label": f.label, "hint": f.hint,
            "surface": f.surface, "on": on, "defaultOn": f.default_on,
        })).collect::<Vec<_>>(),
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FeatureIn {
    key: String,
    on: bool,
}

/// `POST /api/owner/features`
pub async fn set_feature(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: FeatureIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // Only a DECLARED flag. An open key space would make this a way to write
    // arbitrary settings, and nothing would ever read them back.
    if dowiz_hub::features::get(&body.key).is_none() {
        return Response::error(format!("{:?} is not a feature", body.key), 400);
    }
    let (key, value) = (body.key.clone(), if body.on { "1" } else { "0" });
    crate::hubstore::with_settings(&place, move |s| {
        s.set(&key, value);
        Ok(())
    })
    .await?;
    Response::from_json(&json!({ "ok": true, "key": body.key, "on": body.on }))
}

// ── owner: a spreadsheet becomes a menu ─────────────────────────────────────

/// `POST /api/owner/menu/import?apply=true&retire=true` — body is the CSV.
///
/// PREVIEW BY DEFAULT. An import that applies on the first click is one the
/// owner cannot inspect first, and a menu is the thing customers buy from.
pub async fn import_menu(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let flag = |name: &str| {
        req.url()
            .ok()
            .and_then(|u| {
                u.query_pairs().find(|(k, _)| k == name).map(|(_, v)| v.to_string())
            })
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false)
    };
    let (apply, retire) = (flag("apply"), flag("retire"));
    let text = req.text().await?;
    let draft = dowiz_hub::import::from_csv(&text);

    let cat = crate::hubstore::load_catalog(&place).await?.catalog;
    let existing: Vec<(String, String)> = cat
        .products()
        .into_iter()
        .filter_map(|(id, j)| {
            let v: Value = serde_json::from_str(&j).ok()?;
            Some((id, v.get("name")?.as_str()?.to_string()))
        })
        .collect();
    // What is on the menu now but not in the file. Reported either way, so the
    // owner sees the consequence before choosing to act on it.
    let missing: Vec<Value> = existing
        .iter()
        .filter(|(id, _)| !draft.products.iter().any(|p| &p.id == id))
        .map(|(id, name)| json!({ "id": id, "name": name }))
        .collect();

    let mut summary = json!({
        "applied": apply,
        "categories": draft.categories.len(),
        "products": draft.products.len(),
        "warnings": draft.warnings,
        "notInFile": missing,
        "retired": if apply && retire { missing.len() } else { 0 },
    });
    if !apply {
        summary["draft"] = serde_json::from_str(&draft.as_json()).unwrap_or(Value::Null);
        return Response::from_json(&summary);
    }
    // REFUSE to apply a file that produced nothing: applying an empty draft
    // would wipe a working menu because of a wrong separator or a missing
    // header, which is the exact failure the parser warns about.
    if draft.products.is_empty() {
        return Response::error(
            format!("nothing to import: {}", draft.warnings.join("; ")),
            400,
        );
    }

    let missing_ids: Vec<String> = missing
        .iter()
        .filter_map(|m| m.get("id").and_then(Value::as_str).map(String::from))
        .collect();
    crate::hubstore::with_catalog(&place, move |cat| {
        for c in &draft.categories {
            cat.set_category(
                &c.id,
                &json!({ "id": c.id, "name": c.name, "sortOrder": c.sort_order }).to_string(),
            );
        }
        for p in &draft.products {
            // AN EXISTING DISH KEEPS WHAT THE FILE HAS NO COLUMN FOR: its
            // photo, its measured size, its option groups and its ALLERGENS.
            // Blanking the last of those is the worst: a re-imported price list
            // would make every declared dish undeclared, the publish gate would
            // then refuse to keep them on sale, and a venue would find its whole
            // menu stopped by an import that looked like it only touched prices.
            let old = cat.product(&p.id).and_then(|j| serde_json::from_str::<Value>(&j).ok());
            let keep = |k: &str| {
                old.as_ref().and_then(|v| v.get(k).cloned()).unwrap_or(Value::Null)
            };
            cat.set_product(
                &p.id,
                &json!({
                    "id": p.id, "categoryId": p.category_id, "name": p.name,
                    "description": p.description, "price": p.price,
                    "available": p.available, "sortOrder": p.sort_order,
                    "imageUrl": keep("imageUrl"), "sizeCm": keep("sizeCm"),
                    "modifierGroups": keep("modifierGroups"), "allergens": keep("allergens")
                })
                .to_string(),
            );
        }
        if retire {
            for id in &missing_ids {
                let Some(raw) = cat.product(id) else { continue };
                let Ok(mut v) = serde_json::from_str::<Value>(&raw) else { continue };
                v["available"] = json!(false);
                v["unavailableNote"] = json!("not on the current menu");
                cat.set_product(id, &v.to_string());
            }
        }
        // Any catalogue write moves the menu version, which is how a client
        // notices its cart went stale.
        if let Some(lj) = cat.location() {
            if let Ok(mut l) = serde_json::from_str::<Value>(&lj) {
                let v = l.get("menu_version").and_then(|x| x.as_i64()).unwrap_or(1);
                l["menu_version"] = json!(v + 1);
                cat.set_location(&serde_json::to_string(&l).unwrap_or(lj));
            }
        }
        Ok(())
    })
    .await?;
    Response::from_json(&summary)
}

// ── owner: the people who carry the orders ─────────────────────────────────

/// `GET /api/owner/couriers` — the roster, and the invites still outstanding.
///
/// PENDING INVITES SIT IN THE SAME LIST as the people. An owner asking who
/// delivers for them counts the person they invited yesterday among the answer,
/// and a separate panel for invites is a panel nobody opens.
pub async fn couriers(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    #[derive(Deserialize)]
    struct C {
        id: String,
        name: Option<String>,
        phone: Option<String>,
        status: String,
        on_shift: i64,
    }
    let rows: Vec<C> = db
        .prepare(
            "SELECT c.id, c.full_name_encrypted AS name, c.phone_encrypted AS phone, c.status, \
             (SELECT COUNT(*) FROM courier_shifts s WHERE s.courier_id = c.id \
              AND s.ended_at_ms IS NULL) AS on_shift \
             FROM couriers c JOIN courier_locations cl ON cl.courier_id = c.id \
             WHERE cl.location_id = ?1 ORDER BY c.created_at_ms",
        )
        .bind(&[loc.clone().into()])?
        .all()
        .await?
        .results()?;

    #[derive(Deserialize)]
    struct I {
        id: String,
        invited_name: Option<String>,
        expires_at_ms: i64,
        created_at_ms: i64,
    }
    // Used and revoked invites are gone from this list: an invite that has been
    // spent is a courier, and it appears as one two lines above.
    let invites: Vec<I> = db
        .prepare(
            "SELECT id, invited_name, expires_at_ms, created_at_ms FROM courier_invites \
             WHERE location_id = ?1 AND used_at_ms IS NULL AND revoked_at_ms IS NULL \
             ORDER BY created_at_ms",
        )
        .bind(&[loc.into()])?
        .all()
        .await?
        .results()?;

    let now = now_ms();
    Response::from_json(&json!({
        "couriers": rows.iter().map(|c| json!({
            // THE REAL ID, and the phone beside it. This sent the phone AS the id,
            // and an assignment by that "id" found no courier.
            "id": c.id,
            "phone": c.phone.clone().unwrap_or_default(),
            "name": c.name.clone().unwrap_or_default(),
            "active": c.status == "active",
            "onShift": c.on_shift > 0,
        })).collect::<Vec<_>>(),
        "invites": invites.iter().map(|i| json!({
            "id": i.id, "name": i.invited_name.clone().unwrap_or_default(),
            "madeMs": i.created_at_ms, "untilMs": i.expires_at_ms,
            // An expired invite is still LISTED: the owner needs to see that the
            // code they sent has run out, which is the answer to "they say it
            // does not work".
            "expired": now >= i.expires_at_ms,
        })).collect::<Vec<_>>(),
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InviteIn {
    phone: String,
    name: String,
}

/// `GET /api/owner/couriers/:id` -- one courier: who, whether on shift, where
/// they were last seen, what they did today.
pub async fn courier_detail(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing courier id", 400);
    };
    let db = ctx.d1("DB")?;
    let (_, loc) = match owner_and_venue(&req, &ctx, &db).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    #[derive(Deserialize)]
    struct C {
        id: String,
        name: Option<String>,
        phone: Option<String>,
        status: String,
        on_shift: i64,
    }
    let c: Option<C> = db
        .prepare(
            "SELECT c.id, c.full_name_encrypted AS name, c.phone_encrypted AS phone, c.status, \
             EXISTS(SELECT 1 FROM courier_shifts s WHERE s.courier_id = c.id AND s.location_id = ?2 \
                    AND s.ended_at_ms IS NULL) AS on_shift \
             FROM couriers c JOIN courier_locations cl ON cl.courier_id = c.id \
             WHERE (c.id = ?1 OR c.phone_encrypted = ?1) AND cl.location_id = ?2",
        )
        .bind(&[id.clone().into(), loc.clone().into()])?
        .first(None)
        .await?;
    let Some(c) = c else {
        return Response::error("not found", 404);
    };
    let now = now_ms();
    let fix = crate::live_eta::fixes(&db, &loc, now).await.into_iter().find(|f| f.courier_id == c.id);
    #[derive(Deserialize)]
    struct Today {
        deliveries: f64,
        cash: f64,
    }
    let day_start = now - now.rem_euclid(24 * 60 * 60 * 1000);
    let today: Option<Today> = db
        .prepare(
            "SELECT COUNT(*) AS deliveries, COALESCE(SUM(cash_collected),0) AS cash \
             FROM courier_assignments WHERE courier_id = ?1 AND location_id = ?2 \
             AND delivered_at_ms >= ?3",
        )
        .bind(&[c.id.clone().into(), loc.clone().into(), JsValue::from_f64(day_start as f64)])?
        .first(None)
        .await?;
    // The old console's two other tiles, restored: a month of runs, and what
    // is on the road right now (assigned, not yet delivered). Counts only --
    // there is deliberately no average, no rank (DECISIONS D0: trust is a
    // capability, never a score).
    const THIRTY_DAYS_MS: i64 = 30 * 24 * 60 * 60 * 1000;
    #[derive(Deserialize)]
    struct Span {
        delivered30d: f64,
        in_flight: f64,
    }
    let span: Option<Span> = db
        .prepare(
            "SELECT COALESCE(SUM(CASE WHEN delivered_at_ms >= ?3 THEN 1 ELSE 0 END),0) AS delivered30d, \
             COALESCE(SUM(CASE WHEN delivered_at_ms IS NULL THEN 1 ELSE 0 END),0) AS in_flight \
             FROM courier_assignments WHERE courier_id = ?1 AND location_id = ?2",
        )
        .bind(&[c.id.clone().into(), loc.clone().into(), JsValue::from_f64((now - THIRTY_DAYS_MS) as f64)])?
        .first(None)
        .await?;
    Response::from_json(&json!({
        "id": c.id, "phone": c.phone, "name": c.name.clone().unwrap_or_default(), "active": c.status == "active", "onShift": c.on_shift > 0,
        "lastFix": fix.map(|f| json!({ "latUdeg": f.lat_udeg, "lonUdeg": f.lon_udeg, "recordedAtMs": f.recorded_at_ms })),
        "today": { "deliveries": today.as_ref().map(|t| t.deliveries as i64).unwrap_or(0),
                   "cashCollected": today.as_ref().map(|t| t.cash as i64).unwrap_or(0) },
        "delivered30d": span.as_ref().map(|s| s.delivered30d as i64).unwrap_or(0),
        "inFlight": span.as_ref().map(|s| s.in_flight as i64).unwrap_or(0),
    }))
}

/// `POST /api/owner/couriers/invite` — mint a code, shown ONCE.
///
/// The code is stored HASHED, exactly like a password, because that is what it
/// is: until it is claimed, whoever holds it can become this courier. A copied
/// database would otherwise hand over every pending account.
pub async fn invite_courier(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    /// A week. Long enough for a courier who starts next Monday, short enough
    /// that a code found in an old message no longer opens anything.
    const TTL_MS: i64 = 7 * 24 * 60 * 60 * 1000;

    let body: InviteIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let db = ctx.d1("DB")?;
    let (owner, loc) = match owner_and_venue(&req, &ctx, &db).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let phone = body.phone.trim().to_string();
    if phone.chars().filter(char::is_ascii_digit).count() < 8 {
        return Response::error("that does not look like a phone number", 400);
    }
    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Response::error("who is this code for?", 400);
    }
    let phone_hash = crate::auth::sha256_hex(&phone);

    #[derive(Deserialize)]
    struct Row {
        id: String,
    }
    let existing: Option<Row> = db
        .prepare("SELECT id FROM couriers WHERE phone_hash = ?1")
        .bind(&[phone_hash.clone().into()])?
        .first(None)
        .await?;
    if existing.is_some() {
        return Response::error("that phone already has an account", 409);
    }

    // THE BYTES ARE THE PLATFORM'S, the alphabet is the hub's.
    //
    // `new_invite_code` reads /dev/urandom, which a Worker does not have -- so
    // it failed here with "no randomness available" while an owner was trying
    // to hire somebody. Two UUIDs from the platform CSPRNG give 32 bytes; the
    // hub renders 16 of them through the one alphabet both implementations
    // share, so a code minted here is indistinguishable from one minted
    // natively.
    let Some(entropy) = crate::edge_id()
        .zip(crate::edge_id())
        .map(|(a, b)| format!("{a}{b}").replace('-', ""))
    else {
        return Response::error("no platform CSPRNG", 500);
    };
    let raw: Vec<u8> = entropy
        .as_bytes()
        .chunks(2)
        .filter_map(|c| u8::from_str_radix(std::str::from_utf8(c).ok()?, 16).ok())
        .collect();
    let Some(code) = dowiz_hub::roster::invite_code_from(&raw) else {
        return Response::error("no platform CSPRNG", 500);
    };
    let Some(id) = crate::edge_id() else {
        return Response::error("no platform CSPRNG", 500);
    };
    let now = now_ms();
    // Inviting the same phone twice REPLACES the pending invite rather than
    // leaving two codes alive for one person -- the first would keep working
    // after the owner believed they had replaced it.
    db.prepare(
        "UPDATE courier_invites SET revoked_at_ms = ?3 WHERE location_id = ?1 \
         AND invited_phone_hash = ?2 AND used_at_ms IS NULL AND revoked_at_ms IS NULL",
    )
    .bind(&[
        loc.clone().into(),
        phone_hash.clone().into(),
        JsValue::from_f64(now as f64),
    ])?
    .run()
    .await?;
    db.prepare(
        "INSERT INTO courier_invites (id,location_id,created_by_owner_id,role,\
         invited_email_hash,invited_phone_hash,invited_name,code_hash,expires_at_ms,created_at_ms) \
         VALUES (?1,?2,?3,'courier',?4,?4,?5,?6,?7,?8)",
    )
    .bind(&[
        id.into(),
        loc.into(),
        owner.into(),
        phone_hash.into(),
        name.into(),
        // Hashed with the same one-way function the phone uses. A 16-character
        // code from a 32-symbol alphabet is 80 bits, so a plain digest is not
        // brute-forceable the way a human password would be.
        crate::auth::sha256_hex(&code).into(),
        JsValue::from_f64((now + TTL_MS) as f64),
        JsValue::from_f64(now as f64),
    ])?
    .run()
    .await?;

    Response::from_json(&json!({ "code": code, "expiresMs": now + TTL_MS }))
}

/// `POST /api/owner/couriers/:id/uninvite` — withdraw a pending code.
pub async fn uninvite_courier(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing invite", 400);
    };
    let res = db
        .prepare(
            "UPDATE courier_invites SET revoked_at_ms = ?3 WHERE id = ?1 AND location_id = ?2 \
             AND used_at_ms IS NULL AND revoked_at_ms IS NULL",
        )
        .bind(&[id.into(), loc.into(), JsValue::from_f64(now_ms() as f64)])?
        .run()
        .await?;
    if res.meta()?.and_then(|m| m.changes).unwrap_or(0) == 0 {
        return Response::error("not found", 404);
    }
    Response::from_json(&json!({ "ok": true }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ActiveIn {
    active: bool,
}

/// `POST /api/owner/couriers/:id/active` — a courier who has left.
///
/// Their record STAYS -- an order they delivered still names them -- and every
/// session they hold dies, because somebody who has left must not keep a working
/// app in their pocket.
pub async fn set_courier_active(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: ActiveIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let db = ctx.d1("DB")?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let Some(ident) = ctx.param("id").cloned() else {
        return Response::error("missing courier", 400);
    };
    // The console addresses a courier by the phone it displays, which is the
    // only handle it has; the id is internal.
    let hash = crate::auth::sha256_hex(&ident);
    #[derive(Deserialize)]
    struct C {
        id: String,
    }
    let row: Option<C> = db
        .prepare(
            "SELECT c.id FROM couriers c JOIN courier_locations cl ON cl.courier_id = c.id \
             WHERE cl.location_id = ?1 AND (c.id = ?2 OR c.phone_hash = ?3)",
        )
        .bind(&[loc.into(), ident.clone().into(), hash.into()])?
        .first(None)
        .await?;
    let Some(row) = row else {
        return Response::error("not found", 404);
    };
    let status = if body.active { "active" } else { "deactivated" };
    db.prepare("UPDATE couriers SET status = ?2 WHERE id = ?1")
        .bind(&[row.id.clone().into(), status.into()])?
        .run()
        .await?;
    let mut revoked = 0usize;
    if !body.active {
        let res = db
            .prepare(
                "UPDATE courier_sessions SET revoked_at_ms = ?2 WHERE courier_id = ?1 \
                 AND revoked_at_ms IS NULL",
            )
            .bind(&[row.id.into(), JsValue::from_f64(now_ms() as f64)])?
            .run()
            .await?;
        revoked = res.meta()?.and_then(|m| m.changes).unwrap_or(0);
    }
    Response::from_json(&json!({ "ok": true, "active": body.active, "sessionsRevoked": revoked }))
}

// ── owner: the venue's public voice ─────────────────────────────────────────
//
// NOTHING IS PUBLISHED WITHOUT A PERSON. The assistant drafts; the owner reads,
// edits and approves. That is the whole shape, and it is the reason the drafts
// are stored at all rather than posted as they are written.

/// `GET /api/owner/posts`
pub async fn posts(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // Two small images, still two queries. They are 16 KB and 64 KB, so the
    // round trip is the cost rather than the bytes -- and this pane is opened
    // rarely enough that a third query for a pane nobody has open would be the
    // worse trade.
    let posts = crate::hubstore::load_posts(&place).await?.posts;
    let s = crate::hubstore::load_settings(&place).await?.settings;
    Response::from_json(&json!({
        "posts": posts.all().into_iter().map(|p| json!({
            "id": p.id, "text": p.text, "state": p.state.as_str(),
            "channel": p.channel.as_str(), "about": p.subject_tag,
            "createdMs": p.created_ms, "error": p.error,
        })).collect::<Vec<_>>(),
        "enabled": s.flag("social.enabled"),
        // Told plainly rather than discovered at publish time.
        "channel": s.known("social.telegram.channel"),
    }))
}

/// `POST /api/owner/posts/draft` — look for something worth saying, and say it.
///
/// THE SUBJECTS ARE DERIVED FROM FACTS, never invented: a dish that came back,
/// a dish that went off, the week's most-ordered plate counted from the log, a
/// venue that reopened. The model writes the sentence; it never decides what is
/// true. A draft the checker finds unusable -- an invented discount, a
/// manufactured urgency -- is dropped and its subject is NOT marked seen, so it
/// can be tried again.
pub async fn draft_post(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    use dowiz_hub::post::{self, Post, State as PostState};

    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let s = crate::hubstore::load_settings(&place).await?.settings;
    if !s.flag("social.enabled") {
        return Response::error("social drafting is switched off", 409);
    }

    let cat = crate::hubstore::load_catalog(&place).await?.catalog;
    let current: Vec<(String, String, bool)> = cat
        .products()
        .into_iter()
        .filter_map(|(id, j)| {
            let v: Value = serde_json::from_str(&j).ok()?;
            Some((
                id,
                v.get("name")?.as_str()?.to_string(),
                v.get("available").and_then(Value::as_bool).unwrap_or(false),
            ))
        })
        .collect();
    let venue: Value =
        cat.location().and_then(|j| serde_json::from_str(&j).ok()).unwrap_or(json!({}));
    let venue_name = venue.get("name").and_then(Value::as_str).unwrap_or("the restaurant");
    let is_open = venue.get("status").and_then(Value::as_str) == Some("open");
    let lang = venue.get("default_locale").and_then(Value::as_str).unwrap_or("sq").to_string();

    // The week's most-ordered dish, COUNTED HERE. The model never counts; it is
    // handed the number.
    let (loaded, _) = crate::hubstore::load_both(&place).await?;
    let week_ago = now_ms() - 7 * 24 * 60 * 60 * 1000;
    let mut tally: Vec<(String, i64)> = Vec::new();
    for o in orders_of(&loaded, &loc) {
        if o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0) < week_ago {
            continue;
        }
        // A rejected order is not a dish people wanted served.
        if matches!(o.get("status").and_then(Value::as_str), Some("REJECTED" | "CANCELLED")) {
            continue;
        }
        for it in o.get("items").and_then(Value::as_array).into_iter().flatten() {
            let Some(pid) = it.get("product_id").and_then(Value::as_str) else { continue };
            let q = it.get("quantity").and_then(Value::as_i64).unwrap_or(1);
            match tally.iter_mut().find(|t| t.0 == pid) {
                Some(t) => t.1 += q,
                None => tally.push((pid.to_string(), q)),
            }
        }
    }
    tally.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let top = tally.first().cloned();

    let posts_img = crate::hubstore::load_posts(&place).await?.posts;
    let previous = posts_img.catalogue_snapshot();
    // A Worker holds no memory between requests, so "was the venue closed last
    // time we looked" cannot be an in-process flag as it is natively. The
    // snapshot in the posts image is the only thing that persists, and a
    // reopening is announced from the venue's current status alone -- which
    // means it can be drafted once per snapshot rather than once per reopening.
    let was_closed = previous.is_empty();
    let subjects = post::derive_subjects(&previous, &current, top, was_closed, is_open, &|k| {
        posts_img.already_seen(k)
    });

    let snapshot: Vec<(String, bool)> =
        current.iter().map(|(id, _, a)| (id.clone(), *a)).collect();
    if subjects.is_empty() {
        let snap = snapshot.clone();
        crate::hubstore::with_posts(&place, move |p| {
            p.set_catalogue_snapshot(&snap);
            Ok(())
        })
        .await?;
        return Response::from_json(&json!({ "drafted": 0, "note": "nothing new to say" }));
    }

    let mut written = Vec::new();
    for subject in &subjects {
        let prompt = post::prompt_for(subject, venue_name, &lang);
        let mut res = crate::assist::ask(&place, post::SYSTEM_POST, json!({}), &prompt).await?;
        if res.status_code() >= 400 {
            let why = res.text().await.unwrap_or_default();
            return Response::error(
                format!("the assistant is needed to write a post: {why}"),
                409,
            );
        }
        let v: Value = res.json().await?;
        let text = v.get("answer").and_then(Value::as_str).unwrap_or("").to_string();
        // A draft that fails the checker is DROPPED and its subject is not
        // marked seen, so nothing invented reaches the owner and the subject can
        // be tried again.
        if post::unusable(&text).is_some() {
            continue;
        }
        let Some(id) = crate::edge_id() else {
            return Response::error("no platform CSPRNG", 500);
        };
        let p = Post {
            id,
            text,
            subject_tag: subject.fact(),
            subject_key: subject.key(),
            state: PostState::Draft,
            channel: dowiz_hub::post::Channel::Telegram,
            created_ms: now_ms(),
            decided_ms: 0,
            error: String::new(),
        };
        let stored = p.clone();
        // Storing the draft IS marking the subject seen: `already_seen` reads
        // the posts themselves, so a second list of keys would be a second fact
        // that could disagree with them.
        crate::hubstore::with_posts(&place, move |posts| {
            posts.put(&stored);
            Ok(())
        })
        .await?;
        written.push(json!({ "id": p.id, "text": p.text, "about": p.subject_tag }));
    }
    let snap = snapshot.clone();
    crate::hubstore::with_posts(&place, move |p| {
        p.set_catalogue_snapshot(&snap);
        Ok(())
    })
    .await?;
    Response::from_json(&json!({ "drafted": written.len(), "posts": written }))
}

#[derive(Deserialize)]
#[serde(default)]
struct ApproveIn {
    /// The owner's own words, if they edited the draft. An assistant that
    /// cannot be overruled is one that gets switched off.
    text: Option<String>,
}

impl Default for ApproveIn {
    fn default() -> Self {
        ApproveIn { text: None }
    }
}

/// `POST /api/owner/posts/:id/approve` — publish it, in the owner's words.
pub async fn approve_post(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    use dowiz_hub::post::State as PostState;

    let body: ApproveIn = req.json().await.unwrap_or_default();
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing post", 400);
    };
    let posts = crate::hubstore::load_posts(&place).await?.posts;
    let Some(mut p) = posts.get(&id) else {
        return Response::error("not found", 404);
    };
    if p.state == PostState::Published {
        return Response::error("that post is already out", 409);
    }
    if let Some(t) = body.text.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        p.text = t.to_string();
    }

    let s = crate::hubstore::load_settings(&place).await?.settings;
    let channel = s.known("social.telegram.channel");
    let token = crate::notify::bot_token(&ctx.env, &s);
    let (state, error) = match (token, channel.trim().is_empty()) {
        (None, _) => (PostState::Failed, Some("no Telegram bot is configured".to_string())),
        (_, true) => (PostState::Failed, Some("no channel is set".to_string())),
        (Some(token), false) => {
            let url = format!("https://api.telegram.org/bot{token}/sendMessage");
            let headers = Headers::new();
            headers.set("content-type", "application/json")?;
            let payload =
                json!({ "chat_id": channel, "text": p.text, "disable_web_page_preview": true });
            let r = Request::new_with_init(
                &url,
                RequestInit::new()
                    .with_method(Method::Post)
                    .with_headers(headers)
                    .with_body(Some(payload.to_string().into())),
            )?;
            let mut res = Fetch::Request(r).send().await?;
            if res.status_code() < 400 {
                (PostState::Published, None)
            } else {
                // Telegram's own words. "Publishing failed" sends an owner to a
                // forum; "bot is not a member of the channel chat" sends them to
                // the channel's admin list, which is where the fix is.
                let body = res.text().await.unwrap_or_default();
                (PostState::Failed, Some(body[..body.len().min(200)].to_string()))
            }
        }
    };
    // ── Instagram, when the venue connected one ──
    //
    // A post about a dish carries that dish's photo; Instagram has no
    // text-only posts, so a post whose subject has no photo is Telegram-only
    // and says so. The two channels are judged together: published if either
    // took it, and the error names the one that did not.
    let (state, error) = match crate::channels::instagram_cfg(&s) {
        None => (state, error),
        Some(ig) => {
            let subject = p.subject_key.split_once(':').map(|(_, v)| v).unwrap_or(&p.subject_key).to_string();
            let origin = req.url().map(|u| u.origin().ascii_serialization()).unwrap_or_default();
            let photo = crate::hubstore::load_catalog(&place).await.ok().and_then(|c| {
                c.catalog.products().into_iter().find_map(|(id, pj)| {
                    let v: Value = serde_json::from_str(&pj).ok()?;
                    let name = v.get("name").and_then(Value::as_str).unwrap_or("");
                    if id != subject && name != subject {
                        return None;
                    }
                    let url = v.get("imageUrl").and_then(Value::as_str)?;
                    Some(if url.starts_with("http") { url.to_string() } else { format!("{origin}{url}") })
                })
            });
            let ig_result = match photo {
                None => Err("Instagram: the post's dish has no photo".to_string()),
                Some(url) => crate::channels::instagram_publish(&ig, &url, &p.text).await.map_err(|e| format!("Instagram: {e}")),
            };
            match (state, ig_result) {
                (PostState::Published, Ok(_)) => (PostState::Published, None),
                (PostState::Published, Err(e)) => (PostState::Published, Some(e)),
                (_, Ok(_)) => (PostState::Published, error.map(|e| format!("Telegram: {e}"))),
                (st, Err(e)) => (st, Some(format!("{} · {e}", error.unwrap_or_default()))),
            }
        }
    };
    p.state = state;
    p.error = error.clone().unwrap_or_default();
    p.decided_ms = now_ms();
    let stored = p.clone();
    crate::hubstore::with_posts(&place, move |ps| {
        ps.put(&stored);
        Ok(())
    })
    .await?;
    if p.state == PostState::Failed {
        return Response::error(error.unwrap_or_else(|| "publishing failed".into()), 502);
    }
    Response::from_json(&json!({ "ok": true, "state": p.state.as_str() }))
}

/// `POST /api/owner/posts/:id/reject` — not this one.
pub async fn reject_post(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    use dowiz_hub::post::State as PostState;

    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing post", 400);
    };
    let posts = crate::hubstore::load_posts(&place).await?.posts;
    let Some(mut p) = posts.get(&id) else {
        return Response::error("not found", 404);
    };
    if p.state == PostState::Published {
        return Response::error("that post is already out", 409);
    }
    p.state = PostState::Rejected;
    crate::hubstore::with_posts(&place, move |ps| {
        ps.put(&p);
        Ok(())
    })
    .await?;
    Response::from_json(&json!({ "ok": true }))
}

// ── the assistant's two doors ───────────────────────────────────────────────

#[derive(Deserialize)]
struct AskIn {
    question: String,
}

/// `POST /api/owner/assist` — a question about this venue's own live data.

/// What the hub knows that bears on a question, as facts.
///
/// THE ASSISTANT USED TO SEE ONLY LIVE ORDERS, which answers "what is late"
/// and nothing else. An owner asking "which dishes need salmon" or "what has
/// Eni been carrying" was asking about relations that exist in the data and
/// nowhere in any single record — the ids are in the JSON, the meaning is in
/// how they connect. `Graph::of` folds those relations out of the log and the
/// catalogue, and `hybrid` finds the part of that graph the question is about.
///
/// HYBRID, not a keyword search, for the reason the graph module gives at
/// length: an ingredient's name appears in no order's text, so words alone
/// cannot reach the orders it touched. The walk can. The two are fused by rank
/// rather than by score, so neither needs a weight anyone has to tune.
///
/// NEIGHBOURS ARE INCLUDED because a node alone is a name. "dish:item-41" says
/// nothing; "Panko Shrimps, in snacks, uses shrimp, was in order ord-2" is an
/// answer. The model is given the relations and told, as everywhere on this
/// path, that these are the truth and it is not.

/// What the shelf currently holds, for the graph's ingredient nodes.
///
/// COURIER AND CUSTOMER NAMES ARE DELIBERATELY ABSENT, and the reason is in the
/// schema: the column is `full_name_encrypted`. A courier's name is encrypted at
/// rest on purpose, and the one place it must never be decrypted into is a
/// retrieval index that is then pasted into a model's prompt — which may be a
/// hosted provider's. The graph therefore knows WHICH courier carried an order
/// and not who they are; the console maps the id to a name at display time,
/// where the decrypt already lives and stays on the owner's screen.
///
/// A SEARCH FOR A COURIER BY NAME THEREFORE FINDS NOTHING, and that is the
/// correct behaviour rather than a gap. It cost one round of building the
/// wrong thing to see it.
///
/// A stock level is not a person. "salmon: 4 on hand, 1 reserved" is exactly
/// what turns "which dishes use salmon" into "and here is what running out
/// costs you", which is the question an owner actually asks.
async fn shelf_labels(place: &crate::hubstore::Place) -> std::collections::HashMap<String, String> {
    let Ok(loaded) = crate::hubstore::load_stock(place).await else {
        return std::collections::HashMap::new();
    };
    let Ok(ledger) = loaded.stock.ledger() else {
        return std::collections::HashMap::new();
    };
    ledger
        .items()
        .into_iter()
        .map(|(item, lvl)| {
            (
                format!("ingredient:{item}"),
                format!("на складі {} зарезервовано {}", lvl.on_hand, lvl.reserved),
            )
        })
        .collect()
}

fn graph_facts(
    hub: &dowiz_hub::Hub,
    cat: &dowiz_hub::catalog::Catalog,
    labels: &std::collections::HashMap<String, String>,
    question: &str,
    limit: usize,
) -> Value {
    use dowiz_hub::graph::Graph;
    let g = Graph::of_with(hub, cat, labels);
    let hits = g.hybrid(question, limit);
    let found: Vec<Value> = hits
        .iter()
        .filter_map(|(i, score)| {
            let n = g.node(*i)?;
            let related: Vec<Value> = g
                .neighbours(*i)
                .into_iter()
                .take(12)
                .filter_map(|(rel, j, forward)| {
                    let m = g.node(j)?;
                    Some(json!({
                        "how": rel.tag(),
                        "direction": if forward { "to" } else { "from" },
                        "kind": m.kind.tag(),
                        "id": m.id,
                        "label": m.label,
                    }))
                })
                .collect();
            Some(json!({
                "kind": n.kind.tag(),
                "id": n.id,
                "label": n.label,
                "relevance": score,
                "related": related,
            }))
        })
        .collect();
    json!({ "nodes": g.len(), "relations": g.edge_count(), "found": found })
}




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
        match crate::owner::owner_beside(&req, &ctx, &db, crate::hubstore::load_both(&place)).await
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

    fn gauge(u: dowiz_hub::Usage) -> Value {
        json!({
            "generation": u.generation,
            "usedCells": u.used_cells,
            // What the image holds now, and the most it may ever hold. These
            // differ for the compacted images: see `dowiz_hub::Usage`. The
            // ceiling is what `usedPerMille` measures against, because the
            // capacity of a compacted image is re-chosen on every save and a
            // ratio against it falls by half exactly when the image grows.
            "capacityCells": u.capacity_cells,
            "ceilingCells": u.ceiling_cells,
            // Per mille rather than a fraction: the kernel keeps no floats and
            // a percentage with one decimal is what a gauge shows anyway.
            "usedPerMille": u.used_per_mille(),
            // Does this image double itself instead of refusing? See
            // `dowiz_hub::Usage`. A reading of 900 means opposite things.
            "grows": u.grows,
        })
    }

    let mut images = serde_json::Map::new();
    images.insert("log".into(), gauge(hub.hub.usage()));
    images.insert("catalog".into(), gauge(cat.catalog.usage()));
    if let Ok(s) = &settings {
        images.insert("settings".into(), gauge(s.settings.usage()));
    }
    if let Ok(p) = &posts {
        images.insert("posts".into(), gauge(p.posts.usage()));
    }
    if let Ok(st) = &stock {
        images.insert("stock".into(), gauge(st.stock.usage()));
    }

    // THE VERDICT COMES ONLY FROM THE IMAGES THAT CAN ACTUALLY REFUSE.
    //
    // Measured: a stock log grew from 7168 cells to 523264 over four thousand
    // events and never refused once, and the order log doubles the same way.
    // For those, a reading near full predicts a DOUBLING -- a few milliseconds
    // of copying -- and counting it as the venue's worst problem put
    // `dubin-durres` on "watch" for a stock image in no danger at all, while
    // the advice attached to it, "compact", is something an append log cannot
    // do. The compacted KV images are the ones with a real ceiling, so they are
    // the ones the verdict is about.
    let worst = images
        .values()
        .filter(|v| v.get("grows").and_then(Value::as_bool) != Some(true))
        .filter_map(|v| v.get("usedPerMille").and_then(Value::as_i64))
        .max()
        .unwrap_or(0);
    // Still reported, because an image doubling every week is worth seeing even
    // though it is not an emergency.
    let worst_growing = images
        .values()
        .filter(|v| v.get("grows").and_then(Value::as_bool) == Some(true))
        .filter_map(|v| v.get("usedPerMille").and_then(Value::as_i64))
        .max()
        .unwrap_or(0);
    let verdict = match worst {
        0..=699 => "ok",
        700..=899 => "watch",
        _ => "compact",
    };

    Response::from_json(&json!({
        "venue": loc,
        "images": images,
        "worstUsedPerMille": worst,
        "worstGrowingPerMille": worst_growing,
        "verdict": verdict,
        "orders": hub.hub.len(),
    }))
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
        match crate::owner::owner_beside(&req, &ctx, &db, crate::hubstore::export(&place)).await {
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
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    // Authority first and alone: this one writes, so nothing starts beside it.
    if let Err(r) = crate::owner::owner_and_venue(&req, &ctx, &db).await {
        return Ok(r);
    }
    match crate::hubstore::import(&place, &bundle).await {
        Ok(written) => Response::from_json(&json!({ "restored": written })),
        // The refusals here are all the caller's to fix -- a damaged file, a
        // venue that is not empty -- so they are 409, with the reason said.
        Err(e) => Response::error(format!("{e}"), 409),
    }
}

/// `GET /api/owner/graph?q=` — what the hub knows, directly.
///
/// THE SAME RETRIEVAL THE ASSISTANT USES, exposed on its own. An answer a model
/// gives is only as good as what it was shown, and an owner who cannot see what
/// it was shown cannot tell a wrong answer from a wrong retrieval. This is also
/// how the retrieval is tested without a model in the loop.
///
/// With no `q` it reports the shape — how many nodes and relations — which is
/// the cheapest way to see that the fold is working at all.
pub async fn graph(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let (_, _loc, (loaded, loaded_cat)) =
        match crate::owner::owner_beside(&req, &ctx, &db, crate::hubstore::load_both(&place)).await
        {
            Ok(v) => v,
            Err(r) => return Ok(r),
        };
    let q = req
        .url()
        .ok()
        .and_then(|u| u.query_pairs().find(|(k, _)| k == "q").map(|(_, v)| v.to_string()))
        .unwrap_or_default();
    let limit = req
        .url()
        .ok()
        .and_then(|u| u.query_pairs().find(|(k, _)| k == "limit").map(|(_, v)| v.to_string()))
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(12)
        .clamp(1, 50);
    let labels = shelf_labels(&place).await;
    Response::from_json(&graph_facts(&loaded.hub, &loaded_cat.catalog, &labels, &q, limit))
}

pub async fn owner_assist(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: AskIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    // The membership query and this read do not depend on each other, so
    // `owner_beside` runs them together. The token is still verified before
    // either is issued -- see it for why that order matters.
    let (_, loc, loaded) =
        match crate::owner::owner_beside(&req, &ctx, &db, crate::hubstore::load(&place)).await {
            Ok(v) => v,
            Err(r) => return Ok(r),
        };
    let now = now_ms();
    // THE FACTS ARE COMPUTED HERE and handed over. The model is told plainly
    // that they are the truth and it is not; a model that invented a number
    // would have an owner phoning a customer about an order that does not exist.
    let mut live: Vec<Value> = orders_of(&loaded, &loc)
        .into_iter()
        .filter(|o| {
            matches!(
                o.get("status").and_then(Value::as_str),
                Some("PENDING" | "CONFIRMED" | "PREPARING" | "READY" | "IN_DELIVERY")
            )
        })
        .map(|o| {
            let created = o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(now);
            json!({
                "id": o.get("id").cloned().unwrap_or(Value::Null),
                "status": o.get("status").cloned().unwrap_or(Value::Null),
                "total": o.get("total").cloned().unwrap_or(Value::Null),
                "waiting_minutes": (now - created) / 60_000,
                "fulfilment": o.get("fulfilment").and_then(|f| f.get("kind")).cloned()
                    .unwrap_or(Value::Null),
                "courier_id": o.get("courier_id").cloned().unwrap_or(Value::Null),
                "contact": o.get("contact").cloned().unwrap_or(Value::Null),
                "address": o.get("fulfilment").and_then(|f| f.get("address")).cloned()
                    .unwrap_or(Value::Null),
                "items": o.get("items").cloned().unwrap_or(Value::Null),
            })
        })
        .collect();
    live.sort_by_key(|o| -o["waiting_minutes"].as_i64().unwrap_or(0));
    let cat = crate::hubstore::load_catalog(&place).await?.catalog;
    let off: Vec<Value> = cat
        .products()
        .into_iter()
        .filter_map(|(_, j)| {
            let v: Value = serde_json::from_str(&j).ok()?;
            if v.get("available").and_then(Value::as_bool).unwrap_or(true) {
                return None;
            }
            Some(json!({ "name": v.get("name").cloned().unwrap_or(Value::Null),
                         "why": v.get("unavailableNote").cloned().unwrap_or(Value::Null) }))
        })
        .collect();
    // Everything else the hub knows that bears on what was asked.
    let labels = shelf_labels(&place).await;
    let knows = graph_facts(&loaded.hub, &cat, &labels, &body.question, 12);
    let facts =
        json!({ "now_ms": now, "live_orders": live, "off_the_menu": off, "hub_knows": knows });
    crate::assist::ask(&place, crate::assist::SYSTEM_OWNER, facts, &body.question).await
}

/// `POST /api/courier/assist` — a question about this courier's own run.
pub async fn courier_assist(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: AskIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let me = match crate::auth::authenticate(&req, &ctx.env, &db, now_ms()).await {
        Ok(crate::auth::Principal::Courier { courier_id, .. }) => courier_id,
        Ok(_) => return Response::error("forbidden role", 403),
        Err(e) => return e.into_response(),
    };
    let loaded = crate::hubstore::load(&place).await?;
    let now = now_ms();
    // THEIR OWN RUN AND NOTHING ELSE. A courier asking the assistant must not
    // be able to reach a neighbour's address through it.
    let mine: Vec<Value> = loaded
        .hub
        .orders()
        .into_iter()
        .filter_map(|e| serde_json::from_str::<Value>(&e.order_json).ok())
        .filter(|o| o.get("courier_id").and_then(Value::as_str) == Some(me.as_str()))
        .filter(|o| {
            !matches!(
                o.get("status").and_then(Value::as_str),
                Some("DELIVERED" | "CANCELLED" | "REJECTED")
            )
        })
        .map(|o| {
            let created = o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(now);
            json!({
                "id": o.get("id").cloned().unwrap_or(Value::Null),
                "status": o.get("status").cloned().unwrap_or(Value::Null),
                "total": o.get("total").cloned().unwrap_or(Value::Null),
                "payment": o.get("payment").cloned().unwrap_or(Value::Null),
                "waiting_minutes": (now - created) / 60_000,
                "address": o.get("fulfilment").and_then(|f| f.get("address")).cloned()
                    .unwrap_or(Value::Null),
                "contact": o.get("contact").cloned().unwrap_or(Value::Null),
            })
        })
        .collect();
    let facts = json!({ "now_ms": now, "my_runs": mine });
    crate::assist::ask(&place, crate::assist::SYSTEM_COURIER, facts, &body.question).await
}

// ── owner: keys for their own tools ─────────────────────────────────────────

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KeyIn {
    label: String,
}

/// `POST /api/owner/apikeys` — mint one, shown ONCE.
///
/// A YEAR, because the thing holding it is a script on somebody's machine and a
/// credential that expires in an hour is one that gets replaced by a password
/// in a config file. Revocable individually: an owner who suspects one key
/// should not have to invalidate the rest.
pub async fn create_api_key(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    const YEAR_MS: i64 = 365 * 24 * 60 * 60 * 1000;

    let body: KeyIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let db = ctx.d1("DB")?;
    let (owner, loc) = match owner_and_venue(&req, &ctx, &db).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let label = body.label.trim().to_string();
    if label.is_empty() || label.chars().count() > 80 {
        // A key with no label is a key nobody can decide about later. The list
        // is read months after the keys were made.
        return Response::error("say what this key is for", 400);
    }
    let (Some(id), Some(secret)) = (crate::edge_id(), crate::edge_id()) else {
        return Response::error("no platform CSPRNG", 500);
    };
    let secret = secret.replace('-', "");
    // Ours, not a person's: 122 bits from the platform CSPRNG. See
    // `auth::hash_opaque` for why argon2 would be the wrong primitive here and
    // what it cost when it was used for the session secrets.
    let hash = crate::auth::hash_opaque(&secret);
    let now = now_ms();
    db.prepare(
        "INSERT INTO owner_api_keys (id,location_id,owner_id,label,key_hash,\
         created_at_ms,expires_at_ms) VALUES (?1,?2,?3,?4,?5,?6,?7)",
    )
    .bind(&[
        id.clone().into(),
        loc.into(),
        owner.into(),
        label.clone().into(),
        hash.into(),
        JsValue::from_f64(now as f64),
        JsValue::from_f64((now + YEAR_MS) as f64),
    ])?
    .run()
    .await?;
    // Shown once. The hub stores a hash and genuinely cannot show it again,
    // which the console says rather than letting the owner assume otherwise.
    Response::from_json(&json!({
        "key": format!("dowiz_{id}.{secret}"),
        "id": id, "label": label, "expiresMs": now + YEAR_MS,
    }))
}

/// `GET /api/owner/apikeys` — which keys exist, and whether anything uses them.
pub async fn list_api_keys(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    #[derive(Deserialize)]
    struct K {
        id: String,
        label: String,
        created_at_ms: i64,
        expires_at_ms: i64,
        last_used_ms: Option<i64>,
    }
    let rows: Vec<K> = db
        .prepare(
            "SELECT id,label,created_at_ms,expires_at_ms,last_used_ms FROM owner_api_keys \
             WHERE location_id = ?1 AND revoked_at_ms IS NULL ORDER BY created_at_ms DESC",
        )
        .bind(&[loc.into()])?
        .all()
        .await?
        .results()?;
    Response::from_json(&json!({
        "keys": rows.iter().map(|k| json!({
            "id": k.id, "label": k.label, "createdMs": k.created_at_ms,
            "expiresMs": k.expires_at_ms, "lastUsedMs": k.last_used_ms,
        })).collect::<Vec<_>>(),
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RevokeIn {
    id: String,
}

/// `POST /api/owner/apikeys/revoke`
pub async fn revoke_api_key(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: RevokeIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let db = ctx.d1("DB")?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // Revoked, not deleted: the row is the record that this key existed and
    // when it stopped, which is the question asked after an incident.
    let res = db
        .prepare(
            "UPDATE owner_api_keys SET revoked_at_ms = ?3 WHERE id = ?1 AND location_id = ?2 \
             AND revoked_at_ms IS NULL",
        )
        .bind(&[body.id.into(), loc.into(), JsValue::from_f64(now_ms() as f64)])?
        .run()
        .await?;
    if res.meta()?.and_then(|m| m.changes).unwrap_or(0) == 0 {
        return Response::error("not found", 404);
    }
    Response::from_json(&json!({ "ok": true }))
}

// ── photographs ─────────────────────────────────────────────────────────────
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
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
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

    let (pid, u) = (id.clone(), url.clone());
    crate::hubstore::with_catalog(&place, move |cat| {
        let Some(raw) = cat.product(&pid) else {
            return Err(Error::RustError("unknown product".into()));
        };
        let mut p: Value = serde_json::from_str(&raw).unwrap_or(json!({}));
        // The PREVIOUS image is not deleted. Another product may reference the
        // same bytes -- content addressing makes that likely, not rare -- and an
        // order placed an hour ago still names the dish it was sold as.
        p["imageUrl"] = json!(u);
        cat.set_product(&pid, &serde_json::to_string(&p).unwrap_or(raw));
        Ok(())
    })
    .await?;
    Response::from_json(&json!({
        "imageUrl": url, "bytes": stored.bytes, "type": stored.kind.mime()
    }))
}

/// `POST /api/owner/place`
///
/// WHERE THE VENUE IS, WHEN IT OPENS, AND WHAT ITS NEIGHBOURS SAY.
///
/// Three of these fields were readable and unwritable. `hours` has been read on
/// every menu request since the schedule landed -- `is_open_at` decides whether
/// the storefront says "open" -- and NOTHING HAS EVER WRITTEN IT, so every
/// venue fell through to the empty schedule that is open forever. `address` was
/// served and never set. Coordinates did not exist at all, which is why a
/// storefront could not draw a map of the place it sells from.
///
/// The BODY IS ALREADY NORMALISED: seven arrays of minute windows, Monday
/// first, exactly as `dowiz_hub::hours::from_json` reads them. The conversion
/// from "пʼятниця 11:00–23:00" happens in the harvester, where the language of
/// the source is known; a Worker that guessed at weekday names in an unknown
/// locale would open a venue on the wrong day.
///
/// THE GOOGLE BLOCK IS SOMEBODY ELSE'S MATERIAL and is stored as such: the
/// listing's URL travels with it so every surface that shows a rating can say
/// where it came from and link back to it.
pub async fn set_place(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    #[derive(Deserialize)]
    struct Win {
        open: i64,
        close: i64,
    }
    #[derive(Deserialize)]
    struct In {
        #[serde(default)]
        address: Option<String>,
        #[serde(default)]
        lat: Option<f64>,
        #[serde(default)]
        lng: Option<f64>,
        #[serde(default)]
        hours: Option<Vec<Vec<Win>>>,
        #[serde(default)]
        google: Option<Value>,
    }
    let body: In = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    if let Err(r) = owner_and_venue(&req, &ctx, &db).await {
        return Ok(r);
    }

    // A COORDINATE IS REFUSED, NOT CLAMPED. A latitude of 91 is a bug in
    // whatever sent it, and clamping it to 90 puts the venue at the North Pole
    // with no error anybody will see.
    if let Some(v) = body.lat {
        if !(-90.0..=90.0).contains(&v) {
            return Response::error("a latitude is between -90 and 90", 400);
        }
    }
    if let Some(v) = body.lng {
        if !(-180.0..=180.0).contains(&v) {
            return Response::error("a longitude is between -180 and 180", 400);
        }
    }
    if body.lat.is_some() != body.lng.is_some() {
        return Response::error("a latitude without a longitude is not a place", 400);
    }

    let hours_json = match &body.hours {
        None => None,
        Some(days) => {
            if days.len() != 7 {
                return Response::error("a week has seven days", 400);
            }
            for d in days {
                for w in d {
                    if !(0..1440).contains(&w.open) || !(0..=1440).contains(&w.close) {
                        return Response::error("a window is minutes from 0 to 1440", 400);
                    }
                    if w.open == w.close {
                        return Response::error("a window of zero length is not a window", 400);
                    }
                }
            }
            Some(json!(days
                .iter()
                .map(|d| d.iter().map(|w| json!({"open": w.open, "close": w.close})).collect::<Vec<_>>())
                .collect::<Vec<_>>()))
        }
    };

    let (addr, lat, lng, goog) = (body.address.clone(), body.lat, body.lng, body.google.clone());
    crate::hubstore::with_catalog(&place, move |cat| {
        let raw = cat.location().ok_or_else(|| Error::RustError("no venue".into()))?;
        let mut l: Value = serde_json::from_str(&raw).unwrap_or(json!({}));
        // Each field is written only when it was sent. A harvest that could not
        // read the telephone number must not erase the one the owner typed.
        if let Some(a) = &addr {
            l["address"] = if a.trim().is_empty() { Value::Null } else { json!(a.trim()) };
        }
        if let (Some(la), Some(ln)) = (lat, lng) {
            l["lat"] = json!(la);
            l["lng"] = json!(ln);
        }
        if let Some(h) = &hours_json {
            l["hours"] = h.clone();
        }
        if let Some(g) = &goog {
            l["google"] = g.clone();
        }
        cat.set_location(&serde_json::to_string(&l).unwrap_or(raw));
        Ok(())
    })
    .await?;
    Response::from_json(&json!({ "ok": true }))
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
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
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
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    if let Err(r) = owner_and_venue(&req, &ctx, &db).await {
        return Ok(r);
    }
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
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
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
pub async fn media(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let Some(name) = ctx.param("name").cloned() else {
        return Response::error("not found", 404);
    };
    // A path that is not a hash and an extension cannot be one of ours, and
    // refusing early keeps anything with a slash or a dot-dot out of the key.
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '.') || name.len() > 80 {
        return Response::error("not found", 404);
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
    Ok(res)
}

// ── the shelf ───────────────────────────────────────────────────────────────
//
// §4's deterministic ledger, reachable at last. A stock level is a FOLD over
// what happened to the shelf -- received, reserved, consumed, released, wasted,
// counted -- never a number somebody edits. That is what makes the refusal at
// checkout trustworthy: when an order cannot be made, the reason is in the log
// and can be replayed.
//
// THE REFUSAL IS THE AUTOMATED 86. A venue that models its ingredients gets a
// basket refused before the order exists, naming the ingredient. A venue that
// models none reserves nothing and this is a no-op -- stock control that must
// be complete before anything can be sold is stock control nobody switches on.

/// `GET /api/owner/stock` — what is on the shelf, and what is running out.
pub async fn stock(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    // The membership query and this read do not depend on each other, so
    // `owner_beside` runs them together. The token is still verified before
    // either is issued -- see it for why that order matters.
    // Two images, and they do not depend on each other either.
    let (_, loc, (cat, log)) = match crate::owner::owner_beside(
        &req,
        &ctx,
        &db,
        async {
            let (c, s) = futures_util::future::join(
                crate::hubstore::load_catalog(&place),
                crate::hubstore::load_stock(&place),
            )
            .await;
            Ok((c?.catalog, s?.stock))
        },
    )
    .await
    {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let led = match log.ledger() {
        Ok(l) => l,
        Err(e) => return Response::error(e.to_string(), 500),
    };
    let rows: Vec<Value> = cat
        .supplies()
        .into_iter()
        .filter_map(|(id, j)| {
            let v: Value = serde_json::from_str(&j).ok()?;
            // A retired supply keeps its ledger history and leaves the list.
            if v.get("active").and_then(Value::as_bool) == Some(false) {
                return None;
            }
            let level = led.level(&id);
            let low_at = v.get("lowAt").and_then(Value::as_i64).unwrap_or(0);
            let take = |k: &str| v.get(k).cloned().unwrap_or(Value::Null);
            Some(json!({
                "id": id,
                "name": v.get("name").cloned().unwrap_or(Value::Null),
                "unit": v.get("unit").cloned().unwrap_or(json!("g")),
                "kind": v.get("kind").cloned().unwrap_or(json!(crate::recipe::KINDS[0])),
                "category": v.get("category").cloned().unwrap_or(json!("")),
                "kcalPer100": take("kcalPer100"), "proteinPer100": take("proteinPer100"), "fatPer100": take("fatPer100"), "carbsPer100": take("carbsPer100"),
                "costPerBasis": take("costPerBasis"), "weightPerUnit": take("weightPerUnit"),
                "nutritionConfirmed": v.get("nutritionConfirmed").and_then(Value::as_bool).unwrap_or(false),
                "onHand": level.on_hand,
                "reserved": level.reserved,
                "available": level.available(),
                "lowAt": low_at,
                "low": low_at > 0 && level.available() <= low_at,
            }))
        })
        .collect();
    // Reservations whose order never settled. Surfaced rather than swept: a
    // stranded hold makes a kitchen believe it is out of something it has.
    let stranded: Vec<Value> = led
        .stranded()
        .into_iter()
        .map(|(order, item, qty)| json!({ "order": order, "item": item, "qty": qty }))
        .collect();
    Response::from_json(&json!({ "supplies": rows, "stranded": stranded }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SupplyIn {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    unit: Option<String>,
    #[serde(default)]
    low_at: Option<i64>,
    /// food_ingredient | condiment | packaging | utensil (`recipe::KINDS`).
    #[serde(default)]
    kind: Option<String>,
    /// Free text: "Fish", "Sauces", "Containers"…
    #[serde(default)]
    category: Option<String>,
    /// Per 100 g/ml, or per piece: kcal and grams of macros.
    #[serde(default)]
    kcal_per100: Option<f64>,
    #[serde(default)]
    protein_per100: Option<f64>,
    #[serde(default)]
    fat_per100: Option<f64>,
    #[serde(default)]
    carbs_per100: Option<f64>,
    /// Minor units per 100 g/ml, or per piece.
    #[serde(default)]
    cost_per_basis: Option<i64>,
    /// Grams per piece, for supplies counted in units.
    #[serde(default)]
    weight_per_unit: Option<f64>,
    #[serde(default)]
    nutrition_confirmed: Option<bool>,
    #[serde(default)]
    active: Option<bool>,
}

/// `POST /api/owner/supplies` — add or edit an ingredient.
pub async fn set_supply(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: SupplyIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let id = body.id.trim().to_string();
    if id.is_empty() || id.len() > 64 {
        return Response::error("an ingredient needs a short id", 400);
    }
    if body.low_at.is_some_and(|v| v < 0) {
        return Response::error("a threshold cannot be negative", 400);
    }
    if let Some(k) = &body.kind {
        if !crate::recipe::KINDS.contains(&k.as_str()) {
            return Response::error("kind is food_ingredient, condiment, packaging or utensil", 400);
        }
    }
    if let Some(u) = &body.unit {
        if !crate::recipe::UNITS.contains(&u.as_str()) {
            return Response::error("unit is g, ml or unit", 400);
        }
    }
    for (name, v) in [("kcal", body.kcal_per100), ("protein", body.protein_per100), ("fat", body.fat_per100), ("carbs", body.carbs_per100), ("weight", body.weight_per_unit)] {
        if v.is_some_and(|x| !x.is_finite() || x < 0.0) {
            return Response::error(format!("{name} cannot be negative"), 400);
        }
    }
    if body.cost_per_basis.is_some_and(|c| c < 0) {
        return Response::error("cost cannot be negative", 400);
    }
    let rec = crate::hubstore::with_catalog(&place, move |cat| {
        let existing: Value =
            cat.supply(&id).and_then(|j| serde_json::from_str(&j).ok()).unwrap_or(json!({}));
        // Each field: the body's value, else what was there, else the default.
        let keep = |key: &str, given: Option<Value>, default: Value| given.or_else(|| existing.get(key).cloned()).unwrap_or(default);
        let opt = |key: &str, given: Option<Value>| given.or_else(|| existing.get(key).cloned()).unwrap_or(Value::Null);
        let rec = json!({
            "id": id,
            "name": keep("name", body.name.clone().map(Value::String), json!(id)),
            "unit": keep("unit", body.unit.clone().map(Value::String), json!("g")),
            "lowAt": keep("lowAt", body.low_at.map(|v| json!(v)), json!(0)),
            "kind": keep("kind", body.kind.clone().map(Value::String), json!(crate::recipe::KINDS[0])),
            "category": keep("category", body.category.clone().map(|c| json!(c.trim())), json!("")),
            "kcalPer100": opt("kcalPer100", body.kcal_per100.map(|v| json!(v))),
            "proteinPer100": opt("proteinPer100", body.protein_per100.map(|v| json!(v))),
            "fatPer100": opt("fatPer100", body.fat_per100.map(|v| json!(v))),
            "carbsPer100": opt("carbsPer100", body.carbs_per100.map(|v| json!(v))),
            "costPerBasis": opt("costPerBasis", body.cost_per_basis.map(|v| json!(v))),
            "weightPerUnit": opt("weightPerUnit", body.weight_per_unit.map(|v| json!(v))),
            "nutritionConfirmed": keep("nutritionConfirmed", body.nutrition_confirmed.map(|v| json!(v)), json!(false)),
            // Saving through the editor is an act of keeping: a retired supply
            // written again comes back to the list unless the body says otherwise.
            "active": json!(body.active.unwrap_or(true)),
        });
        cat.set_supply(&id, &rec.to_string());
        Ok(rec)
    })
    .await?;
    Response::from_json(&rec)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StockMoveIn {
    item: String,
    #[serde(default)]
    qty: Option<i64>,
    /// For a stocktake: what was actually counted.
    #[serde(default)]
    observed: Option<i64>,
    /// For waste: spoiled, dropped or unsold.
    #[serde(default)]
    reason: Option<String>,
}

/// `POST /api/owner/stock/:kind` — received, wasted or counted.
///
/// THE THREE A HUMAN CAUSES. Reserved, Consumed and Released are emitted by the
/// order lifecycle and are deliberately unreachable here: a hand-written
/// reservation has no order to settle it and would strand immediately.
pub async fn stock_move(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    use dowiz_hub::stock::{StockEvent, WasteReason};

    let body: StockMoveIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let Some(kind) = ctx.param("kind").cloned() else {
        return Response::error("which movement?", 400);
    };
    let item = body.item.trim().to_string();
    if item.is_empty() {
        return Response::error("which ingredient?", 400);
    }
    if crate::hubstore::load_catalog(&place).await?.catalog.supply(&item).is_none() {
        return Response::error("not found", 404);
    }
    let ev = match kind.as_str() {
        "received" => match body.qty {
            Some(qty) => StockEvent::Received { item, qty },
            None => return Response::error("how much?", 400),
        },
        "wasted" => match body.qty {
            Some(qty) => StockEvent::Wasted {
                item,
                qty,
                reason: body
                    .reason
                    .as_deref()
                    .and_then(WasteReason::from_str)
                    .unwrap_or(WasteReason::Spoiled),
            },
            None => return Response::error("how much?", 400),
        },
        "stocktake" => match body.observed {
            Some(observed) => StockEvent::Stocktake {
                item,
                observed,
                stocktake_id: format!("st_{}", now_ms()),
            },
            None => return Response::error("what was counted?", 400),
        },
        other => return Response::error(format!("no such movement: {other}"), 400),
    };
    let outcome = crate::hubstore::with_stock(&place, move |log| {
        log.append(&ev).map_err(|e| Error::RustError(e.to_string()))
    })
    .await;
    match outcome {
        Ok(()) => Response::from_json(&json!({ "ok": true })),
        // The ledger's refusals are the venue's business, not a server fault.
        Err(e) => Response::error(e.to_string(), 409),
    }
}

// ── where this venue delivers ───────────────────────────────────────────────

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
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
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
    let db = ctx.d1("DB")?;
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

// ── colours out of a photograph ─────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PixelsIn {
    /// Flat RGB triples in hex. The BROWSER decodes the image -- it already has
    /// a PNG and JPEG decoder and this Worker deliberately has neither.
    pixels: String,
}

/// `POST /api/owner/branding/extract` — suggest colours from an image.
///
/// SUGGESTS ONLY. Nothing is applied: the owner picks a swatch and posts it
/// back. An upload that silently repainted the storefront would be a change
/// nobody approved, made from a photograph.
pub async fn extract_branding(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: PixelsIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let db = ctx.d1("DB")?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let hex = body.pixels.trim();
    if hex.is_empty() || hex.len() % 6 != 0 {
        return Response::error("pixels must be whole rgb triples in hex", 400);
    }
    // 64x64 is what the client is asked to send; this allows a good deal more
    // and refuses a payload nobody meant to upload.
    if hex.len() > 6 * 65_536 {
        return Response::error("too many pixels; downsample first", 400);
    }
    let Some(bytes) = dowiz_hub::crypto::unhex(hex) else {
        return Response::error("pixels are not hex", 400);
    };
    let pixels: Vec<dowiz_hub::palette::Rgb> = bytes
        .chunks_exact(3)
        .map(|c| dowiz_hub::palette::Rgb::new(c[0], c[1], c[2]))
        .collect();
    let swatches = dowiz_hub::palette::dominant(&pixels, 6);
    if swatches.is_empty() {
        // Honest emptiness. Inventing a colour for a black-and-white menu would
        // present a guess as a finding.
        return Response::from_json(&json!({
            "swatches": [], "note": "no strong colours found in this image"
        }));
    }
    Response::from_json(&json!({
        "swatches": swatches.iter().map(|s| {
            let t = dowiz_hub::palette::Theme::from_seed(s.colour);
            json!({
                "hex": s.colour.hex(),
                "sharePct": (s.share_bp as f64 / 100.0 * 10.0).round() / 10.0,
                // Every suggestion arrives already contrast-checked, so an
                // owner cannot pick one that fails.
                "passes": t.contrast_report().into_iter().all(|(_, got, want)| got >= want),
            })
        }).collect::<Vec<_>>()
    }))
}

// ── speech ──────────────────────────────────────────────────────────────────
//
// THE CLASSIFIER IS DETERMINISTIC AND THE SERVICE SURVIVES IT BEING OFF.
// Ambiguity is rejected rather than guessed, and a consequential command is
// NEVER acted on from one utterance: the hub answers with a signed proposal and
// a read-back, and a person presses the button. Voice moves a hand, never a
// decision.
//
// The proposal is a token scoped to `voice:<verb>:<order>`, bound to the
// speaker AND their session, and short-lived. So a confirmation cannot be
// replayed, cannot be used by anyone else, and cannot be pointed at a different
// order than the one that was read back.

#[derive(Deserialize)]
#[serde(default)]
struct VoiceIn {
    transcript: String,
    /// The recogniser's own confidence, 0..1.
    confidence: f64,
    is_final: bool,
    lang: Option<String>,
    /// A token from a previous proposal. Its presence is what turns a
    /// suggestion into an action.
    confirm: Option<String>,
}

impl Default for VoiceIn {
    fn default() -> Self {
        VoiceIn {
            transcript: String::new(),
            confidence: 1.0,
            is_final: true,
            lang: None,
            confirm: None,
        }
    }
}

/// How long a read-back stays answerable. Long enough to hear it and say yes,
/// short enough that a phone left on a counter cannot confirm it an hour later.
const PROPOSAL_TTL_MS: i64 = 90_000;

/// `POST /api/voice`
pub async fn voice(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    use dowiz_hub::voice::{classify, Command, Speaker, Target};

    let body: VoiceIn = req.json().await.unwrap_or_default();
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;

    // WHICH VOCABULARY APPLIES IS DECIDED BY THE TOKEN, never by what the
    // caller says they are: a courier claiming to be an owner would otherwise
    // reach the owner's commands by typing a word.
    let (speaker, who, loc) =
        match crate::auth::authenticate(&req, &ctx.env, &db, now_ms()).await {
            Ok(crate::auth::Principal::Owner { user_id, active_location_id }) => (
                Speaker::Owner,
                user_id,
                active_location_id.unwrap_or_default(),
            ),
            Ok(crate::auth::Principal::Courier { courier_id, active_location_id, .. }) => {
                (Speaker::Courier, courier_id, active_location_id)
            }
            Ok(_) => return Response::error("forbidden role", 403),
            Err(e) => return e.into_response(),
        };
    let lang = body.lang.clone().unwrap_or_else(|| "uk".into());

    // A confirmation carries its own instruction; nothing is classified again.
    if let Some(tok) = &body.confirm {
        let claims = match dowiz_hub::token::verify(
            crate::auth::signing_key(&ctx.env).as_slice(),
            tok,
            now_ms(),
        ) {
            Ok(c) => c,
            Err(_) => {
                return Response::from_json(&json!({
                    "understood": false, "say": "та відповідь уже не дійсна"
                }))
            }
        };
        if claims.subject != who {
            return Response::from_json(&json!({
                "understood": false, "say": "це підтвердження не ваше"
            }));
        }
        let Some(rest) = claims.scope.strip_prefix("voice:") else {
            return Response::from_json(&json!({ "understood": false, "say": "не та відповідь" }));
        };
        let Some((verb, order)) = rest.split_once(':') else {
            return Response::from_json(&json!({ "understood": false, "say": "не та відповідь" }));
        };
        return Response::from_json(&json!({
            "understood": true, "needsConfirmation": false,
            // The ACTION IS NOT RUN HERE. The surface calls the same route the
            // button calls, so voice reaches exactly the checks a tap reaches
            // and cannot become a second way to move an order.
            "action": "do", "verb": verb, "orderId": order,
        }));
    }

    let cmd = classify(&body.transcript, body.confidence, body.is_final, speaker);

    // The orders this speaker may act on. An owner sees the venue's live queue;
    // a courier sees only their own run -- so a misheard number can never reach
    // somebody else's delivery.
    let loaded = crate::hubstore::load(&place).await?;
    let pool: Vec<Value> = loaded
        .hub
        .orders()
        .into_iter()
        .filter_map(|e| serde_json::from_str::<Value>(&e.order_json).ok())
        .filter(|o| {
            o.get("location_id").and_then(Value::as_str).map(|l| l == loc).unwrap_or(true)
        })
        .filter(|o| {
            let st = o.get("status").and_then(Value::as_str).unwrap_or("");
            let live = matches!(
                st,
                "PENDING" | "CONFIRMED" | "PREPARING" | "READY" | "IN_DELIVERY"
            );
            match speaker {
                Speaker::Owner => live,
                Speaker::Courier => {
                    live && o.get("courier_id").and_then(Value::as_str) == Some(who.as_str())
                }
            }
        })
        .collect();

    let resolve = |t: &Target| -> std::result::Result<String, &'static str> {
        let id = |o: &Value| o.get("id").and_then(Value::as_str).unwrap_or("").to_string();
        let at = |o: &Value| o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0);
        match t {
            Target::Newest => pool
                .iter()
                .max_by_key(|o| at(o))
                .map(id)
                .ok_or("зараз немає замовлень"),
            Target::Oldest => pool
                .iter()
                .min_by_key(|o| at(o))
                .map(id)
                .ok_or("зараз немає замовлень"),
            Target::Digits(d) => {
                let hits: Vec<String> =
                    pool.iter().filter(|o| id(o).ends_with(d.as_str())).map(id).collect();
                match hits.len() {
                    1 => Ok(hits[0].clone()),
                    0 => Err("такого номера серед відкритих немає"),
                    // AMBIGUITY IS REFUSED, not guessed. Two orders ending in
                    // the same digits is exactly when a guess moves the wrong
                    // one.
                    _ => Err("під цей номер підходить кілька — скажіть більше цифр"),
                }
            }
            Target::Unsaid => match pool.len() {
                1 => Ok(id(&pool[0])),
                0 => Err("зараз немає замовлень"),
                _ => Err("яке саме?"),
            },
        }
    };

    let propose = |verb: &str, order: &str| -> Option<String> {
        // `mint` returns the token itself here, not a Result: the signing key
        // is already in hand and there is nothing left to fail at.
        let now = now_ms();
        dowiz_hub::token::mint(
            crate::auth::signing_key(&ctx.env).as_slice(),
            &dowiz_hub::token::Claims {
                role: match speaker {
                    Speaker::Owner => dowiz_hub::token::Role::Owner,
                    Speaker::Courier => dowiz_hub::token::Role::Courier,
                },
                subject: who.clone(),
                session: String::new(),
                scope: format!("voice:{verb}:{order}"),
                issued_ms: now,
                expires_ms: now + PROPOSAL_TTL_MS,
            },
        )
        .into()
    };

    let out = match &cmd {
        Command::Unclear(why) => json!({
            "understood": false, "say": why, "heard": body.transcript
        }),
        Command::Status => {
            let waiting = pool
                .iter()
                .filter(|o| o.get("status").and_then(Value::as_str) == Some("PENDING"))
                .count();
            // READ-ONLY, so it runs at once. Making somebody confirm a question
            // is what turns voice into a form.
            json!({ "understood": true, "needsConfirmation": false, "action": "status",
                    "open": pool.len(), "waiting": waiting })
        }
        Command::Ask(q) => json!({
            "understood": true, "needsConfirmation": false, "action": "ask", "question": q
        }),
        Command::Shift { open } => {
            let verb = if *open { "shift_open" } else { "shift_close" };
            match propose(verb, "-") {
                Some(tok) => json!({ "understood": true, "needsConfirmation": true,
                                     "readback": cmd.readback(&lang), "token": tok }),
                None => json!({ "understood": false, "say": "не вдалося підготувати дію" }),
            }
        }
        Command::Order { verb, target } => match resolve(target) {
            Err(why) => json!({ "understood": false, "say": why, "heard": body.transcript }),
            Ok(order) => match propose(verb, &order) {
                Some(tok) => json!({ "understood": true, "needsConfirmation": true,
                                     "readback": cmd.readback(&lang), "token": tok,
                                     "orderId": order }),
                None => json!({ "understood": false, "say": "не вдалося підготувати дію" }),
            },
        },
        Command::Pickup { target } | Command::Deliver { target } => {
            let verb = if matches!(cmd, Command::Pickup { .. }) { "pickup" } else { "deliver" };
            match resolve(target) {
                Err(why) => json!({ "understood": false, "say": why, "heard": body.transcript }),
                Ok(order) => match propose(verb, &order) {
                    Some(tok) => json!({ "understood": true, "needsConfirmation": true,
                                         "readback": cmd.readback(&lang), "token": tok,
                                         "orderId": order }),
                    None => json!({ "understood": false, "say": "не вдалося підготувати дію" }),
                },
            }
        }
    };
    Response::from_json(&out)
}

/// `GET /api/public/rates` — what one unit of the venue's currency is worth.
///
/// DISPLAY ONLY, AND THE DISTINCTION IS THE WHOLE DESIGN. An order is priced,
/// totalled, charged and refunded in the VENUE'S currency; the kernel's money
/// law is exact integer arithmetic in one currency and a second one inside it
/// would be a rounding error with a customer's name on it. What a customer
/// switching to EUR gets is a READING of a price that is still in lek -- so the
/// figure that is authoritative and the figure that is convenient are never the
/// same number, and the surface says which is which.
///
/// INTEGER PARTS PER MILLION, not a float. The rate arrives as `0.0109` and is
/// stored and shipped as `10900`, so every consumer converts with
/// `amount * ppm / 10_000` into minor units and no surface has to agree with
/// another about rounding. MANIFESTO C2 keeps floats out of anything replayed;
/// this is not replayed, and it costs nothing to hold the line anyway.
///
/// CACHED FOR AN HOUR AT THE EDGE. A reference rate moves in fractions of a
/// percent over a day and a menu is read thousands of times, so fetching per
/// request would add a third-party round trip to the most-requested path in the
/// product for a number that had not changed.
///
/// IT NEVER FAILS THE PAGE. If the upstream is unreachable the answer is the
/// identity rate and `stale: true`, because a storefront that cannot render a
/// price because a currency API is down is worse than one that shows lek.
pub async fn rates(req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    let base = req
        .url()
        .ok()
        .and_then(|u| u.query_pairs().find(|(k, _)| k == "base").map(|(_, v)| v.to_string()))
        .unwrap_or_else(|| "ALL".to_string())
        .to_ascii_uppercase();
    // A currency code is three letters. Anything else is not one, and it would
    // otherwise be pasted into a third-party URL.
    if base.len() != 3 || !base.bytes().all(|b| b.is_ascii_alphabetic()) {
        return Response::error("base must be a three-letter currency code", 400);
    }

    let key = format!("https://rates.dowiz/{base}");
    let cache = Cache::default();
    if let Some(hit) = cache.get(&key, false).await? {
        return Ok(hit);
    }

    let identity = |stale: bool| {
        json!({
            "base": base,
            "ppm": { base.clone(): 1_000_000 },
            "decimals": { "ALL": 0, "EUR": 2, "USD": 2 },
            "stale": stale,
        })
    };

    let fetched = Fetch::Url(
        format!("https://open.er-api.com/v6/latest/{base}")
            .parse()
            .map_err(|e| Error::RustError(format!("rate url: {e}")))?,
    )
    .send()
    .await;

    let body = match fetched {
        Ok(mut r) if r.status_code() == 200 => r.json::<Value>().await.ok(),
        _ => None,
    };

    let mut out = identity(true);
    if let Some(v) = body {
        if v.get("result").and_then(Value::as_str) == Some("success") {
            let mut ppm = serde_json::Map::new();
            // Only the three this product renders. A map of 160 currencies is
            // 160 numbers nothing reads, on the hottest path in the service.
            for code in ["ALL", "EUR", "USD"] {
                if let Some(r) = v.get("rates").and_then(|m| m.get(code)).and_then(Value::as_f64) {
                    // The ONE float in this path, and it ends here: the wire
                    // format is a decimal and the ledger is integers, so the
                    // conversion happens once, at the boundary, and is rounded
                    // rather than truncated.
                    ppm.insert(code.to_string(), json!((r * 1_000_000.0).round() as i64));
                }
            }
            if !ppm.is_empty() {
                out = json!({
                    "base": base,
                    "ppm": ppm,
                    "decimals": { "ALL": 0, "EUR": 2, "USD": 2 },
                    "asOf": v.get("time_last_update_utc").cloned().unwrap_or(Value::Null),
                    "stale": false,
                });
            }
        }
    }

    let mut res = Response::from_json(&out)?;
    res.headers_mut().set("cache-control", "public, max-age=3600")?;
    let to_cache = res.cloned()?;
    cache.put(&key, to_cache).await?;
    Ok(res)
}


/// `POST /api/owner/supplies/:id/retire` — off the list, ledger kept. A dish
/// whose recipe still names it keeps reserving it, which is the honest
/// outcome: the kitchen still uses it, the owner just stopped tracking it.
pub async fn retire_supply(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct In {
        #[allow(dead_code)]
        location_id: Option<String>,
    }
    let _body: In = req.json().await.unwrap_or(In { location_id: None });
    let Some(id) = ctx.param("id").cloned() else { return Response::error("missing supply id", 400) };
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    if let Err(r) = owner_and_venue(&req, &ctx, &db).await {
        return Ok(r);
    }
    let done = crate::hubstore::with_catalog(&place, move |cat| {
        let Some(j) = cat.supply(&id) else { return Ok(false) };
        let mut v: Value = serde_json::from_str(&j).unwrap_or(json!({}));
        v["active"] = json!(false);
        cat.set_supply(&id, &v.to_string());
        Ok(true)
    })
    .await?;
    if !done {
        return Response::error("unknown supply", 404);
    }
    Response::from_json(&json!({ "ok": true }))
}
