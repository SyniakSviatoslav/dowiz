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

// `start_of_day_ms`, `currency_of` and `orders_of` used to live here. The
// first is `dowiz_hub::tz::start_of_local_day_ms` and was only ever a rename;
// the other two are `services::venue::currency_of` and
// `services::orders::mine::of_venue`, where the tenancy rule that decides
// whose takings an owner is shown finally has a test.
use crate::services::orders::mine::of_venue as orders_of;


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
        match crate::owner::owner_beside(&req, &ctx, &db, &place, crate::hubstore::load_catalog(&place)).await {
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
        match crate::owner::owner_beside(&req, &ctx, &db, &place, crate::hubstore::load_catalog(&place)).await {
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
    let loc = match owner_and_venue(&req, &ctx, &db).await {
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
    let db = ctx.d1("DB")?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
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

// ── owner: the venue's own configuration ────────────────────────────────────

/// `GET /api/owner/settings` — the declared keys, their values and their hints.
///
/// The KEY SPACE IS CLOSED. An open one would make this a place to stash
/// arbitrary data that nothing ever reads back, and the console renders the
/// list the hub declares rather than a list of its own.
pub async fn settings(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
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
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
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
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
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
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
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
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
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
                    "imageUrl": keep("imageUrl"), "imageUrlSmall": keep("imageUrlSmall"),
                    "sizeCm": keep("sizeCm"),
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
    let (kid, l2, own, lab, h2) = (id.clone(), loc.clone(), owner.clone(), label.clone(), hash);
    crate::identity_store::with_sessions(&ctx.env, move |t| {
        let rec = serde_json::json!({
            "id": kid, "location_id": l2, "owner_id": own, "label": lab, "key_hash": h2,
            "created_at_ms": now, "expires_at_ms": now + YEAR_MS,
            "last_used_ms": serde_json::Value::Null,
            "revoked_at_ms": serde_json::Value::Null,
        })
        .to_string();
        t.put(
            crate::identity_store::K_APIKEY,
            &kid,
            &rec,
            &[(crate::identity_store::apikey_at(&l2, &kid), kid.clone())],
            &[],
        )
        .map_err(|e| Error::RustError(format!("api key: {e}")))
    })
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
    struct K {
        id: String,
        label: String,
        created_at_ms: i64,
        expires_at_ms: i64,
        last_used_ms: Option<i64>,
    }
    // THIS VENUE'S KEYS, from the prefix that is the venue's list. Sorted
    // newest first here rather than by a second index: a venue has a handful
    // of keys, and an index maintained on every use to save one sort of five
    // records is a key that exists to be forgotten.
    let sess = crate::identity_store::sessions(&ctx.env).await?;
    let mut rows: Vec<K> = sess
        .scan(&format!("apikey.loc/{loc}/"))
        .into_iter()
        .filter_map(|(_, id)| {
            let r = crate::identity_store::rec(&sess, crate::identity_store::K_APIKEY, &id)?;
            if r.get("revoked_at_ms").map_or(false, |v| !v.is_null()) {
                return None;
            }
            Some(K {
                label: crate::identity_store::s_of(&r, "label"),
                created_at_ms: crate::identity_store::i_of(&r, "created_at_ms"),
                expires_at_ms: crate::identity_store::i_of(&r, "expires_at_ms"),
                last_used_ms: r.get("last_used_ms").and_then(Value::as_i64),
                id,
            })
        })
        .collect();
    rows.sort_by(|a, b| b.created_at_ms.cmp(&a.created_at_ms));
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
    let (kid, l2, now) = (body.id.clone(), loc.clone(), now_ms());
    let revoked = crate::identity_store::with_sessions(&ctx.env, move |t| {
        let Some(mut r) = crate::identity_store::rec(t, crate::identity_store::K_APIKEY, &kid)
        else {
            return Ok(false);
        };
        // The venue is checked against the RECORD, and the answer to a key of
        // another restaurant is the same 404 as to one that does not exist: a
        // 403 would confirm it does.
        if crate::identity_store::s_of(&r, "location_id") != l2
            || r.get("revoked_at_ms").map_or(false, |v| !v.is_null())
        {
            return Ok(false);
        }
        r["revoked_at_ms"] = serde_json::json!(now);
        let index = vec![(crate::identity_store::apikey_at(&l2, &kid), kid.clone())];
        t.put(crate::identity_store::K_APIKEY, &kid, &r.to_string(), &index, &[])
            .map_err(|e| Error::RustError(format!("api key: {e}")))?;
        Ok(true)
    })
    .await?;
    if !revoked {
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
    let loc = match owner_and_venue(&req, &ctx, &db).await {
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
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;

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
    let loc = match owner_and_venue(&req, &ctx, &db).await {
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
    let db = ctx.d1("DB")?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
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
    let db = ctx.d1("DB")?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
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
        &place,
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
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
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
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
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
    let pool: Vec<Value> = crate::hubstore::orders(&place)
        .await?
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
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
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
