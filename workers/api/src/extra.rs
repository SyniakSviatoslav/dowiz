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

// ── owner: the venue's own configuration ────────────────────────────────────

/// `GET /api/owner/settings` — the declared keys, their values and their hints.
///
/// The KEY SPACE IS CLOSED. An open one would make this a place to stash
/// arbitrary data that nothing ever reads back, and the console renders the
/// list the hub declares rather than a list of its own.
pub async fn settings(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
    };
    if let Err(r) = owner_at(&req, &ctx, &db, &loc).await {
        return Ok(r);
    }
    let loaded = crate::hubstore::load_settings(&db).await?;
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
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
    };
    if let Err(r) = owner_at(&req, &ctx, &db, &loc).await {
        return Ok(r);
    }
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
    crate::hubstore::with_settings(&db, move |s| {
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
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
    };
    if let Err(r) = owner_at(&req, &ctx, &db, &loc).await {
        return Ok(r);
    }
    let s = crate::hubstore::load_settings(&db).await?.settings;
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
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
    };
    if let Err(r) = owner_at(&req, &ctx, &db, &loc).await {
        return Ok(r);
    }
    // Only a DECLARED flag. An open key space would make this a way to write
    // arbitrary settings, and nothing would ever read them back.
    if dowiz_hub::features::get(&body.key).is_none() {
        return Response::error(format!("{:?} is not a feature", body.key), 400);
    }
    let (key, value) = (body.key.clone(), if body.on { "1" } else { "0" });
    crate::hubstore::with_settings(&db, move |s| {
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
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
    };
    if let Err(r) = owner_at(&req, &ctx, &db, &loc).await {
        return Ok(r);
    }
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

    let cat = crate::hubstore::load_catalog(&db).await?.catalog;
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
    crate::hubstore::with_catalog(&db, move |cat| {
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
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
    };
    if let Err(r) = owner_at(&req, &ctx, &db, &loc).await {
        return Ok(r);
    }
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
            "id": c.phone.clone().unwrap_or_else(|| c.id.clone()),
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
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
    };
    let owner = match owner_at(&req, &ctx, &db, &loc).await {
        Ok(id) => id,
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
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
    };
    if let Err(r) = owner_at(&req, &ctx, &db, &loc).await {
        return Ok(r);
    }
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
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
    };
    if let Err(r) = owner_at(&req, &ctx, &db, &loc).await {
        return Ok(r);
    }
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
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
    };
    if let Err(r) = owner_at(&req, &ctx, &db, &loc).await {
        return Ok(r);
    }
    let posts = crate::hubstore::load_posts(&db).await?.posts;
    let s = crate::hubstore::load_settings(&db).await?.settings;
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
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
    };
    if let Err(r) = owner_at(&req, &ctx, &db, &loc).await {
        return Ok(r);
    }
    let s = crate::hubstore::load_settings(&db).await?.settings;
    if !s.flag("social.enabled") {
        return Response::error("social drafting is switched off", 409);
    }

    let cat = crate::hubstore::load_catalog(&db).await?.catalog;
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
    let loaded = crate::hubstore::load(&db).await?;
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

    let posts_img = crate::hubstore::load_posts(&db).await?.posts;
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
        crate::hubstore::with_posts(&db, move |p| {
            p.set_catalogue_snapshot(&snap);
            Ok(())
        })
        .await?;
        return Response::from_json(&json!({ "drafted": 0, "note": "nothing new to say" }));
    }

    let mut written = Vec::new();
    for subject in &subjects {
        let prompt = post::prompt_for(subject, venue_name, &lang);
        let mut res = crate::assist::ask(&db, post::SYSTEM_POST, json!({}), &prompt).await?;
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
        crate::hubstore::with_posts(&db, move |posts| {
            posts.put(&stored);
            Ok(())
        })
        .await?;
        written.push(json!({ "id": p.id, "text": p.text, "about": p.subject_tag }));
    }
    let snap = snapshot.clone();
    crate::hubstore::with_posts(&db, move |p| {
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
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
    };
    if let Err(r) = owner_at(&req, &ctx, &db, &loc).await {
        return Ok(r);
    }
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing post", 400);
    };
    let posts = crate::hubstore::load_posts(&db).await?.posts;
    let Some(mut p) = posts.get(&id) else {
        return Response::error("not found", 404);
    };
    if p.state == PostState::Published {
        return Response::error("that post is already out", 409);
    }
    if let Some(t) = body.text.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        p.text = t.to_string();
    }

    let s = crate::hubstore::load_settings(&db).await?.settings;
    let channel = s.known("social.telegram.channel");
    let token = ctx.env.secret("TELEGRAM_BOT_TOKEN").map(|v| v.to_string()).ok();
    let (state, error) = match (token, channel.trim().is_empty()) {
        (None, _) => (PostState::Failed, Some("no Telegram bot is configured".to_string())),
        (_, true) => (PostState::Failed, Some("no channel is set".to_string())),
        (Some(token), false) => {
            let url = format!("https://api.telegram.org/bot{token}/sendMessage");
            let mut headers = Headers::new();
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
    p.state = state;
    p.error = error.clone().unwrap_or_default();
    p.decided_ms = now_ms();
    let stored = p.clone();
    crate::hubstore::with_posts(&db, move |ps| {
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
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
    };
    if let Err(r) = owner_at(&req, &ctx, &db, &loc).await {
        return Ok(r);
    }
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing post", 400);
    };
    let posts = crate::hubstore::load_posts(&db).await?.posts;
    let Some(mut p) = posts.get(&id) else {
        return Response::error("not found", 404);
    };
    if p.state == PostState::Published {
        return Response::error("that post is already out", 409);
    }
    p.state = PostState::Rejected;
    crate::hubstore::with_posts(&db, move |ps| {
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
pub async fn owner_assist(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: AskIn = match req.json().await {
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
    let loaded = crate::hubstore::load(&db).await?;
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
    let cat = crate::hubstore::load_catalog(&db).await?.catalog;
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
    let facts = json!({ "now_ms": now, "live_orders": live, "off_the_menu": off });
    crate::assist::ask(&db, crate::assist::SYSTEM_OWNER, facts, &body.question).await
}

/// `POST /api/courier/assist` — a question about this courier's own run.
pub async fn courier_assist(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: AskIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let db = ctx.d1("DB")?;
    let me = match crate::auth::authenticate(&req, &ctx.env, &db, now_ms()).await {
        Ok(crate::auth::Principal::Courier { courier_id, .. }) => courier_id,
        Ok(_) => return Response::error("forbidden role", 403),
        Err(e) => return e.into_response(),
    };
    let loaded = crate::hubstore::load(&db).await?;
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
    crate::assist::ask(&db, crate::assist::SYSTEM_COURIER, facts, &body.question).await
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
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
    };
    let owner = match owner_at(&req, &ctx, &db, &loc).await {
        Ok(id) => id,
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
    let hash = match crate::auth::hash_password(&secret) {
        Ok(h) => h,
        Err(e) => return e.into_response(),
    };
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
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
    };
    if let Err(r) = owner_at(&req, &ctx, &db, &loc).await {
        return Ok(r);
    }
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
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
    };
    if let Err(r) = owner_at(&req, &ctx, &db, &loc).await {
        return Ok(r);
    }
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
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
    };
    if let Err(r) = owner_at(&req, &ctx, &db, &loc).await {
        return Ok(r);
    }
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing product", 400);
    };
    // The product must exist BEFORE a blob is written, or a typo in an id
    // leaves an orphan nothing will ever reference or clean up.
    let cat = crate::hubstore::load_catalog(&db).await?.catalog;
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
    crate::hubstore::with_catalog(&db, move |cat| {
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

/// `POST /api/owner/products/:id/image/clear`
pub async fn clear_product_image(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
    };
    if let Err(r) = owner_at(&req, &ctx, &db, &loc).await {
        return Ok(r);
    }
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing product", 400);
    };
    // The blob STAYS. Clearing a dish's photo is not a statement about every
    // other dish that might share those bytes, nor about the orders that
    // already carry the URL.
    crate::hubstore::with_catalog(&db, move |cat| {
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
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
    };
    if let Err(r) = owner_at(&req, &ctx, &db, &loc).await {
        return Ok(r);
    }
    let cat = crate::hubstore::load_catalog(&db).await?.catalog;
    let log = crate::hubstore::load_stock(&db).await?.stock;
    let led = match log.ledger() {
        Ok(l) => l,
        Err(e) => return Response::error(e.to_string(), 500),
    };
    let rows: Vec<Value> = cat
        .supplies()
        .into_iter()
        .filter_map(|(id, j)| {
            let v: Value = serde_json::from_str(&j).ok()?;
            let level = led.level(&id);
            let low_at = v.get("lowAt").and_then(Value::as_i64).unwrap_or(0);
            Some(json!({
                "id": id,
                "name": v.get("name").cloned().unwrap_or(Value::Null),
                "unit": v.get("unit").cloned().unwrap_or(json!("g")),
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
}

/// `POST /api/owner/supplies` — add or edit an ingredient.
pub async fn set_supply(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: SupplyIn = match req.json().await {
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
    let id = body.id.trim().to_string();
    if id.is_empty() || id.len() > 64 {
        return Response::error("an ingredient needs a short id", 400);
    }
    if body.low_at.is_some_and(|v| v < 0) {
        return Response::error("a threshold cannot be negative", 400);
    }
    let rec = crate::hubstore::with_catalog(&db, move |cat| {
        let existing: Value =
            cat.supply(&id).and_then(|j| serde_json::from_str(&j).ok()).unwrap_or(json!({}));
        let rec = json!({
            "id": id,
            "name": body.name.clone().map(Value::String)
                .or_else(|| existing.get("name").cloned()).unwrap_or(json!(id)),
            "unit": body.unit.clone().map(Value::String)
                .or_else(|| existing.get("unit").cloned()).unwrap_or(json!("g")),
            "lowAt": body.low_at.map(|v| json!(v))
                .or_else(|| existing.get("lowAt").cloned()).unwrap_or(json!(0)),
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
    let Some(loc) = venue_of(&req, &db).await else {
        return Response::error("this hub has no venue yet", 404);
    };
    if let Err(r) = owner_at(&req, &ctx, &db, &loc).await {
        return Ok(r);
    }
    let Some(kind) = ctx.param("kind").cloned() else {
        return Response::error("which movement?", 400);
    };
    let item = body.item.trim().to_string();
    if item.is_empty() {
        return Response::error("which ingredient?", 400);
    }
    if crate::hubstore::load_catalog(&db).await?.catalog.supply(&item).is_none() {
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
    let outcome = crate::hubstore::with_stock(&db, move |log| {
        log.append(&ev).map_err(|e| Error::RustError(e.to_string()))
    })
    .await;
    match outcome {
        Ok(()) => Response::from_json(&json!({ "ok": true })),
        // The ledger's refusals are the venue's business, not a server fault.
        Err(e) => Response::error(e.to_string(), 409),
    }
}
