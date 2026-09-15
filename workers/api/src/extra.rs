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
use worker::*;

use crate::owner::{now_ms, owner_at, venue_of};

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
    // TWO IMAGES, one hub: the log and the catalogue have different roots and
    // cannot share one, so a route that reads both loads both.
    let loaded = crate::hubstore::load(&db).await?;
    let cat = crate::hubstore::load_catalog(&db).await?.catalog;
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
    // The customer's own token for THIS order and nothing else. The owner's is
    // refused too: this is the customer's voice and not the venue's.
    match crate::auth::authenticate(&req, &ctx.env, &db, now_ms()).await {
        Ok(crate::auth::Principal::Customer { order_id, .. }) if order_id == id => {}
        Ok(_) => return Response::error("that link is not for this order", 401),
        Err(_) => return Response::error("this order needs the link you were given", 401),
    }

    let at = now_ms();
    let oid = id.clone();
    let outcome = crate::hubstore::with_hub(&db, move |hub| {
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
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
    };
    if let Err(r) = owner_at(&req, &ctx, &db, &loc).await {
        return Ok(r);
    }
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
    let loaded = crate::hubstore::load(&db).await?;
    let cat = crate::hubstore::load_catalog(&db).await?.catalog;

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
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
    };
    if let Err(r) = owner_at(&req, &ctx, &db, &loc).await {
        return Ok(r);
    }
    let loaded = crate::hubstore::load(&db).await?;
    let cat = crate::hubstore::load_catalog(&db).await?.catalog;
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
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
    };
    if let Err(r) = owner_at(&req, &ctx, &db, &loc).await {
        return Ok(r);
    }
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
    crate::hubstore::with_catalog(&db, move |cat| {
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
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
    };
    if let Err(r) = owner_at(&req, &ctx, &db, &loc).await {
        return Ok(r);
    }
    let Some(code) = ctx.param("code").cloned() else {
        return Response::error("missing code", 400);
    };
    let code = dowiz_hub::promo::normalise(&code);
    let gone = crate::hubstore::with_catalog(&db, move |cat| Ok(cat.remove_promo(&code))).await?;
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
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
    };
    if let Err(r) = owner_at(&req, &ctx, &db, &loc).await {
        return Ok(r);
    }
    let loaded = crate::hubstore::load_catalog(&db).await?;
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
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
    };
    if let Err(r) = owner_at(&req, &ctx, &db, &loc).await {
        return Ok(r);
    }
    use dowiz_hub::brand::{Brand, PRESETS, RADIUS_MAX, TYPE_PAIRS};
    let loaded = crate::hubstore::load_catalog(&db).await?;
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

async fn store_brand(db: &D1Database, b: dowiz_hub::brand::Brand) -> Result<Response> {
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
    crate::hubstore::with_catalog(db, move |cat| {
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
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
    };
    if let Err(r) = owner_at(&req, &ctx, &db, &loc).await {
        return Ok(r);
    }
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
    store_brand(&db, Brand { accent, ink, paper, type_pair: pair, radius }).await
}

/// `POST /api/owner/branding/preset?location_id=` — a whole look at once.
pub async fn set_preset(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: PresetIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let db = ctx.d1("DB")?;
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
    };
    if let Err(r) = owner_at(&req, &ctx, &db, &loc).await {
        return Ok(r);
    }
    let Some(b) = dowiz_hub::brand::preset(&body.preset) else {
        return Response::error(format!("{:?} is not a preset", body.preset), 400);
    };
    store_brand(&db, b).await
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
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
    };
    if let Err(r) = owner_at(&req, &ctx, &db, &loc).await {
        return Ok(r);
    }
    let sort = req
        .url()
        .ok()
        .and_then(|u| u.query_pairs().find(|(k, _)| k == "sort").map(|(_, v)| v.to_string()));
    let secret = signing_secret(&ctx.env);
    let loaded = crate::hubstore::load(&db).await?;

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
    let cat = crate::hubstore::load_catalog(&db).await?.catalog;
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
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
    };
    let who = match owner_at(&req, &ctx, &db, &loc).await {
        Ok(id) => id,
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
    let loaded = crate::hubstore::load(&db).await?;
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
    crate::hubstore::with_hub(&db, move |hub| {
        hub.append(dowiz_hub::EventKind::Revealed, &subject, &entry, at as u64, [0u8; 32])
            .map_err(|e| Error::RustError(format!("{e:?}")))
    })
    .await?;

    Response::from_json(&json!({ "name": name, "phone": phone, "orders": orders }))
}

/// `GET /api/owner/customers/reveals?location_id=` — who has been looking.
pub async fn reveals(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
    };
    if let Err(r) = owner_at(&req, &ctx, &db, &loc).await {
        return Ok(r);
    }
    let loaded = crate::hubstore::load(&db).await?;
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
    let me = match crate::auth::authenticate(&req, &ctx.env, &db, now_ms()).await {
        Ok(crate::auth::Principal::Courier { courier_id, .. }) => courier_id,
        Ok(_) => return Response::error("forbidden role", 403),
        Err(e) => return e.into_response(),
    };
    let loaded = crate::hubstore::load(&db).await?;
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
