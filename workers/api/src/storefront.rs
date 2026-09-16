//! The public storefront: read the menu, place an order.
//!
//! THE MONEY RULE, and it is the reason this module exists rather than the
//! client posting a total: every line's price is RE-DERIVED from `products` here
//! before the kernel ever sees the order. Whatever `unit_price` the browser sent
//! is discarded. A client that inflates a discount, or deflates a price, changes
//! nothing -- which is the same guarantee `place_order_priced` gives natively,
//! reached through the JSON boundary.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use worker::*;

use crate::auth;
use dowiz_kernel::json_api;

#[derive(Deserialize)]
pub struct LineIn {
    pub product_id: String,
    #[serde(default)]
    pub modifier_ids: Vec<String>,
    pub quantity: i64,
}

#[derive(Deserialize)]
pub struct ContactIn {
    #[serde(default)]
    pub name: Option<String>,
    pub phone: String,
}

#[derive(Deserialize)]
pub struct AddressIn {
    pub line: String,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Deserialize)]
pub struct FulfilmentIn {
    pub kind: String,
    #[serde(default)]
    pub address: Option<AddressIn>,
    /// A pickup has no address to carry the customer's note on, so it rides
    /// here. Without this field the note was accepted and silently discarded.
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Deserialize)]
pub struct PlaceIn {
    pub items: Vec<LineIn>,
    pub contact: ContactIn,
    pub fulfilment: FulfilmentIn,
    #[serde(default)]
    pub payment: Option<String>,
    #[serde(default)]
    pub locale: Option<String>,
    /// A promo code as the customer typed it. Normalised and re-checked here;
    /// whatever the storefront showed as a preview is advisory.
    #[serde(default)]
    pub promo: Option<String>,
    /// A tip, in minor units. THE COURIER'S -- carried as its own field the
    /// whole way so it never lands in the venue's takings.
    #[serde(default)]
    pub tip: Option<i64>,
}

#[derive(Serialize)]
struct LocationOut {
    id: String,
    name: String,
    slug: String,
    phone: String,
    address: Option<String>,
    status: String,
    #[serde(rename = "closesAt")]
    closes_at: Option<String>,
    #[serde(rename = "deliveryEta")]
    delivery_eta: String,
    #[serde(rename = "deliveryFee")]
    delivery_fee: i64,
    #[serde(rename = "freeDeliveryThreshold")]
    free_delivery_threshold: Option<i64>,
    #[serde(rename = "minOrder")]
    min_order: i64,
    #[serde(rename = "currencyCode")]
    currency_code: String,
    #[serde(rename = "menuVersion")]
    menu_version: i64,
    #[serde(rename = "supportedLocales")]
    supported_locales: Value,
    #[serde(rename = "defaultLocale")]
    default_locale: String,
}

#[derive(Deserialize)]
struct LocRow {
    id: String,
    name: String,
    slug: String,
    phone: String,
    address: Option<String>,
    status: String,
    closes_at: Option<String>,
    delivery_eta: String,
    delivery_fee: i64,
    free_delivery_threshold: Option<i64>,
    min_order: i64,
    currency_code: String,
    menu_version: i64,
    supported_locales: String,
    default_locale: String,
    delivery_paused: i64,
}

#[derive(Deserialize)]
struct ProdRow {
    id: String,
    category_id: Option<String>,
    category_name: Option<String>,
    category_sort: Option<i64>,
    name: String,
    description: Option<String>,
    price: i64,
    available: i64,
    unavailable_note: Option<String>,
    image_url: Option<String>,
    sort_order: i64,
}

/// `GET /api/public/locations/:slug/menu`
pub async fn menu(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let Some(slug) = ctx.param("slug").cloned() else {
        return Response::error("missing slug", 400);
    };

    // ── SERVED FROM THE EDGE WHERE IT CAN BE ──
    //
    // This is the single most-requested thing in the product: every customer,
    // every page load, every language switch. It already declared
    // `max-age=30`, but a Worker's response is NOT edge-cached unless the
    // Worker puts it there -- so the header was a promise nothing kept, and
    // every visitor paid two D1 reads and a full catalogue fold.
    //
    // Thirty seconds is the window the header already claimed, and it is the
    // right one: a menu changes when an owner edits it, and a dish going off
    // sale reaching a customer half a minute late is a cost the venue can
    // absorb. Anything derived from the CLOCK -- whether the venue is open --
    // moves in minutes, not seconds, so thirty is inside its resolution too.
    let cache = Cache::default();
    let key = req.url()?.to_string();
    if let Some(hit) = cache.get(&key, false).await? {
        return Ok(hit);
    }
    let db = ctx.d1("DB")?;
    let loaded = crate::hubstore::load_catalog(&db).await?;

    let Some(loc_json) = loaded.catalog.location() else {
        return Response::error("not found", 404);
    };
    let loc: LocRow = serde_json::from_str(&loc_json)
        .map_err(|e| Error::RustError(format!("catalogue location unreadable: {e}")))?;
    if loc.slug != slug {
        return Response::error("not found", 404);
    }

    // The same record, untyped, for the fields that arrived after `LocRow` was
    // written. Growing the struct for each one means a venue saved by an older
    // hub fails to deserialise entirely; reading them off the Value means a
    // missing field is a missing field.
    let raw: Value = serde_json::from_str(&loc_json).unwrap_or(json!({}));

    // ── THE STATUS IS DERIVED, not read ──
    //
    // A paused venue is closed however its flag reads: the owner needs a way to
    // stop the queue without rewriting opening hours. And a SCHEDULE can only
    // ever close -- never open -- which is what lets the activation gate live on
    // the manual flag alone.
    let sched = raw
        .get("hours")
        .map(|h| dowiz_hub::hours::from_json(&h.to_string()))
        .unwrap_or_default();
    // Durrës is UTC+2. A Worker has no timezone database and needs none: one
    // venue sits in one place, and its offset is one configured number.
    let tz: i64 = raw.get("tz_offset_minutes").and_then(Value::as_i64).unwrap_or(120);
    let now_ms = Date::now().as_millis() as i64;
    let (weekday, minute) = dowiz_hub::hours::local_now(now_ms, tz);
    let scheduled_open = sched.is_empty() || sched.is_open_at(weekday, minute);
    let next_open = sched.next_open(weekday, minute);
    let paused = loc.delivery_paused == 1;
    let status = if paused || loc.status == "closed" || !scheduled_open {
        "closed".to_string()
    } else {
        loc.status.clone()
    };

    // Category order comes from the catalogue, and products are grouped into it.
    // Keys are sorted by the KV layout, so the order is stable across reads
    // rather than incidentally whatever the store returned.
    let mut cat_meta: Vec<(String, String, i64)> = loaded
        .catalog
        .categories()
        .into_iter()
        .filter_map(|(id, j)| {
            let v: Value = serde_json::from_str(&j).ok()?;
            Some((
                id,
                v.get("name").and_then(|x| x.as_str()).unwrap_or("—").to_string(),
                v.get("sortOrder").and_then(|x| x.as_i64()).unwrap_or(0),
            ))
        })
        .collect();
    cat_meta.sort_by_key(|(_, _, sort)| *sort);

    let products: Vec<(String, Value)> = loaded
        .catalog
        .products()
        .into_iter()
        .filter_map(|(id, j)| serde_json::from_str::<Value>(&j).ok().map(|v| (id, v)))
        .collect();

    let mut cats: Vec<Value> = Vec::new();
    for (cid, cname, csort) in &cat_meta {
        let mut items: Vec<(i64, Value)> = products
            .iter()
            .filter(|(_, p)| p.get("categoryId").and_then(|x| x.as_str()) == Some(cid.as_str()))
            .map(|(id, p)| {
                (
                    p.get("sortOrder").and_then(|x| x.as_i64()).unwrap_or(0),
                    json!({
                        "id": id,
                        "name": p.get("name").cloned().unwrap_or(json!("")),
                        "description": p.get("description").cloned().unwrap_or(Value::Null),
                        "price": p.get("price").cloned().unwrap_or(json!(0)),
                        "available": p.get("available").and_then(|x| x.as_bool()).unwrap_or(true),
                        "unavailableNote": p.get("unavailableNote").cloned().unwrap_or(Value::Null),
                        "imageUrl": p.get("imageUrl").cloned().unwrap_or(Value::Null),
                        // ── the four fields the storefront reads and this
                        // payload did not send ──
                        //
                        // Their absence was not cosmetic. `allergens` missing
                        // means every dish renders as "not declared" -- the
                        // loudest state -- even for one the venue declared
                        // clear. `modifierGroups` missing means a dish with
                        // choices is sold without them. `sizeCm` missing means
                        // the AR button never appears.
                        //
                        // `allergens` is passed through as STORED, including
                        // its absence: null and [] are different claims and
                        // flattening them here would undo the whole design.
                        "allergens": p.get("allergens").cloned().unwrap_or(Value::Null),
                        "modifierGroups": p.get("modifierGroups").cloned().unwrap_or(Value::Null),
                        "sizeCm": p.get("sizeCm").cloned().unwrap_or(Value::Null),
                        "sortOrder": p.get("sortOrder").cloned().unwrap_or(json!(0))
                    }),
                )
            })
            .collect();
        items.sort_by_key(|(sort, _)| *sort);
        if items.is_empty() {
            continue;
        }
        cats.push(json!({
            "id": cid, "name": cname, "sortOrder": csort,
            "products": items.into_iter().map(|(_, p)| p).collect::<Vec<_>>()
        }));
    }

    let mut location = serde_json::to_value(LocationOut {
        id: loc.id,
        name: loc.name,
        slug: loc.slug,
        phone: loc.phone,
        address: loc.address,
        status,
        closes_at: loc.closes_at,
        delivery_eta: loc.delivery_eta,
        delivery_fee: loc.delivery_fee,
        free_delivery_threshold: loc.free_delivery_threshold,
        min_order: loc.min_order,
        currency_code: loc.currency_code,
        menu_version: loc.menu_version,
        supported_locales: serde_json::from_str(&loc.supported_locales)
            .unwrap_or_else(|_| json!(["sq"])),
        default_locale: loc.default_locale,
    })
    .unwrap_or(json!({}));

    // ── the fields the storefront reads and this payload did not send ──
    //
    // Each absence had a visible consequence. No `theme` meant the venue's own
    // colours never reached its own storefront -- the branding editor wrote to a
    // field nothing read. No `pickup` meant the collection choice could not be
    // offered even where the hub accepted it. No `nextOpen` meant a closed venue
    // said "closed" instead of "opens at eleven", and a customer told only that
    // a place is shut goes somewhere else.
    location["theme"] = raw.get("theme").cloned().unwrap_or(Value::Null);
    location["pickup"] = json!(raw.get("pickup").and_then(Value::as_bool).unwrap_or(false));
    location["hasDeliveryZones"] = json!(raw.get("delivery_zones").is_some());
    location["nextOpen"] = next_open
        .map(|(d, m)| json!({ "weekday": d, "minute": m }))
        .unwrap_or(Value::Null);
    // Which KIND of closed, so the storefront can say which.
    location["closedReason"] = if paused {
        json!("paused")
    } else if loc.status == "closed" {
        json!("manual")
    } else if !scheduled_open {
        json!("hours")
    } else {
        Value::Null
    };
    // WHICH FEATURES THIS VENUE HAS ON. Sent with the menu rather than fetched
    // separately: a control that appears a moment after the page does is worse
    // than one that was never there, and a second request to decide what to
    // render is a second chance to render the wrong thing.
    let settings = crate::hubstore::load_settings(&db).await?.settings;
    let mut features = serde_json::Map::new();
    for (f, on) in dowiz_hub::features::all(&settings) {
        if f.surface != "storefront" {
            continue;
        }
        // The prefix is an internal namespace; the client asks for `tips`, not
        // `feature.tips`.
        features.insert(f.key.trim_start_matches("feature.").to_string(), json!(on));
    }
    location["features"] = Value::Object(features);

    location["telegramBot"] = ctx
        .env
        .secret("TELEGRAM_BOT_USERNAME")
        .map(|v| Value::from(v.to_string()))
        .unwrap_or(Value::Null);

    let out = json!({
        "location": location,
        "categories": cats,
        // The PUBLISHABLE key only. It is designed to be public -- it can create
        // a payment method and nothing else -- and the browser needs it to mount
        // the Payment Element. Absent when the card rail is off, so the storefront
        // can hide the option rather than offer one that cannot complete.
        "stripePublishableKey": ctx.env.secret("STRIPE_PUBLISHABLE_KEY")
            .map(|v| Value::from(v.to_string())).unwrap_or(Value::Null)
    });
    let mut res = Response::from_json(&out)?;
    // The menu is public and changes rarely; the version field is what a client
    // uses to notice it moved.
    res.headers_mut().set("cache-control", "public, max-age=30, stale-while-revalidate=300")?;
    // Stored AFTER the response is built and cloned, so the store cannot fail
    // the request: a cache that breaks a page is worse than no cache.
    if let Ok(copy) = res.cloned() {
        let _ = cache.put(&key, copy).await;
    }
    Ok(res)
}

/// `POST /api/public/locations/:slug/orders`
pub async fn place(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let Some(slug) = ctx.param("slug").cloned() else {
        return Response::error("missing slug", 400);
    };
    let body: PlaceIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    if body.items.is_empty() {
        return Response::error("empty order", 400);
    }
    if body.contact.phone.chars().filter(|c| c.is_ascii_digit()).count() < 8 {
        return Response::error("invalid phone", 400);
    }
    if body.fulfilment.kind == "delivery"
        && body.fulfilment.address.as_ref().map_or(true, |a| a.line.trim().is_empty())
    {
        return Response::error("delivery address required", 400);
    }

    let db = ctx.d1("DB")?;
    let loaded = crate::hubstore::load_catalog(&db).await?;
    let Some(loc_json) = loaded.catalog.location() else {
        return Response::error("not found", 404);
    };
    let loc: LocRow = serde_json::from_str(&loc_json)
        .map_err(|e| Error::RustError(format!("catalogue location unreadable: {e}")))?;
    if loc.slug != slug {
        return Response::error("not found", 404);
    }
    if loc.delivery_paused == 1 || loc.status == "closed" {
        return Response::error("venue is closed", 409);
    }

    // ── re-derive every price from the catalogue ──
    // The rule does not change with the store: whatever unit_price the browser
    // sent is discarded, and an unknown product fails CLOSED rather than being
    // priced at zero.
    let mut lines = Vec::with_capacity(body.items.len());
    let mut subtotal: i64 = 0;
    for it in &body.items {
        if it.quantity < 1 || it.quantity > 99 {
            return Response::error("invalid quantity", 400);
        }
        let Some(pj) = loaded.catalog.product(&it.product_id) else {
            return Response::error(format!("unknown product: {}", it.product_id), 400);
        };
        let p: Value = serde_json::from_str(&pj)
            .map_err(|e| Error::RustError(format!("catalogue product unreadable: {e}")))?;
        if !p.get("available").and_then(|x| x.as_bool()).unwrap_or(false) {
            return Response::error(format!("unavailable: {}", it.product_id), 409);
        }
        let price = p.get("price").and_then(|x| x.as_i64()).unwrap_or(-1);
        if price < 0 {
            return Response::error(format!("product has no price: {}", it.product_id), 409);
        }
        subtotal += price * it.quantity;
        lines.push(json!({
            "product_id": it.product_id, "modifier_ids": it.modifier_ids,
            "quantity": it.quantity, "unit_price": price   // trusted, from the catalogue
        }));
    }
    if subtotal < loc.min_order {
        return Response::error("below minimum order", 409);
    }

    let fee = match loc.free_delivery_threshold {
        Some(th) if subtotal >= th => 0,
        _ if body.fulfilment.kind == "pickup" => 0,
        _ => loc.delivery_fee,
    };

    let Some(id) = crate::edge_id() else {
        return Response::error("no platform CSPRNG for order id", 500);
    };
    let created_at_ms = Date::now().as_millis() as i64;

    let order_json = match json_api::place_order_at(
        id.clone(),
        None,
        &serde_json::to_string(&lines).unwrap_or_else(|_| "[]".into()),
        created_at_ms,
        Some("storefront".into()),
    ) {
        Ok(j) => j,
        Err(e) => return Response::error(e, 400),
    };

    // The kernel owns items and subtotal. Delivery, contact and fulfilment are
    // carried alongside until the aggregate's new fields reach this boundary.
    let mut envelope: Value = serde_json::from_str(&order_json)
        .map_err(|e| Error::RustError(format!("kernel order json unreadable: {e}")))?;
    // ── the tip ──
    //
    // Bounded on both sides. Zero or less is not a tip; the ceiling is the
    // order or 10000, whichever is more, because somebody meaning 200 and
    // typing 20000 would otherwise hand over a month's pay and find out at the
    // card rail.
    let tip = match body.tip.unwrap_or(0) {
        0 => 0,
        t if t < 0 => return Response::error("a tip cannot be negative", 400),
        t if t > subtotal.max(10_000) => {
            return Response::error("that tip is larger than the order", 400)
        }
        t => t,
    };

    // ── the promo code ──
    //
    // Decided AFTER the basket is priced and against the subtotal the hub
    // derived, never a number from the browser. The use-count is folded from
    // the orders themselves, so there is no counter that can disagree with
    // them; a rejected or cancelled order gives its use back, because the venue
    // never took the money.
    // The code is LOOKED UP here, before the order exists, so an unknown one
    // costs the customer a refusal and nothing else. Whether it still APPLIES is
    // decided in the append below, where the use-count cannot move underneath
    // the answer.
    let promo = match body.promo.as_deref().map(dowiz_hub::promo::normalise) {
        None => None,
        Some(code) if code.is_empty() => None,
        Some(code) => {
            match loaded.catalog.promo(&code).as_deref().and_then(dowiz_hub::promo::Promo::parse) {
                Some(p) => Some(p),
                None => return Response::error(dowiz_hub::promo::Refusal::Unknown.as_str(), 400),
            }
        }
    };

    // The fee was decided BEFORE the discount and on the undiscounted subtotal:
    // the other order lets a code quietly ADD a delivery charge by pushing the
    // basket back under the free-delivery threshold, and a customer who applied
    // a saving and watched the total go up is right to distrust the number.
    let total = subtotal + fee + tip;
    envelope["delivery_fee"] = json!(fee);
    envelope["tip"] = json!(tip);
    envelope["total"] = json!(total);
    envelope["location_id"] = json!(loc.id);
    envelope["contact"] = json!({ "name": body.contact.name, "phone": body.contact.phone });
    envelope["fulfilment"] = json!({
        "kind": body.fulfilment.kind,
        "note": body.fulfilment.note.as_deref().map(str::trim).filter(|n| !n.is_empty())
            .map(|n| json!(n)).unwrap_or(Value::Null),
        "address": body.fulfilment.address.as_ref().map(|a| json!({ "line": a.line, "note": a.note })),
        "fee": fee
    });
    let payment_kind = body.payment.clone().unwrap_or_else(|| "cash".into());
    envelope["payment"] = json!(payment_kind);

    let phone_hash = auth::sha256_hex(&body.contact.phone);
    // ── INGREDIENTS ARE RESERVED BEFORE THE ORDER EXISTS ──
    //
    // §4's fail-closed gate: if the kitchen cannot make it, the customer is
    // told now rather than phoned in twenty minutes. The reservation is all or
    // nothing across the whole basket, so a third line that is short does not
    // leave the first two held by an order that was never placed.
    //
    // A venue that has not modelled its ingredients reserves nothing and this
    // is a no-op -- stock control that must be complete before anything can be
    // sold is stock control nobody switches on.
    let bom_lines: Vec<(String, i64)> = body
        .items
        .iter()
        .filter_map(|it| Some((loaded.catalog.product(&it.product_id)?, it.quantity)))
        .collect();
    let reservations = dowiz_hub::stock::reservations_for(&id, &bom_lines);
    if !reservations.is_empty() {
        let evs = reservations.clone();
        let held = crate::hubstore::with_stock(&db, move |log| {
            log.append_all(&evs).map_err(|e| Error::RustError(e.to_string()))
        })
        .await;
        if let Err(e) = held {
            // The customer is told WHICH ingredient: "something is unavailable"
            // sends them hunting through a basket.
            return Response::error(format!("{e}"), 409);
        }
    }

    // THE DISCOUNT IS DECIDED BESIDE THE APPEND THAT MAKES IT REAL. Counting the
    // uses first and appending after would let two customers spend the last use
    // of the same code at once -- rare at one restaurant, and exactly the kind
    // of rare that only ever shows up as an unexplained loss.
    let seq = created_at_ms as u64;
    let ev_id = id.clone();
    let now_for_promo = Date::now().as_millis() as i64;
    let stored = crate::hubstore::with_hub(&db, move |hub| {
        // CLONED per attempt, not moved: `with_hub` retries when it loses the
        // generation guard, so the closure runs more than once and must not
        // consume what it patches.
        let mut envelope = envelope.clone();
        if let Some(p) = &promo {
            let used = crate::hubstore::promo_uses(hub, &p.code);
            let cut = p
                .redeem(subtotal, now_for_promo, used)
                .map_err(|r| Error::RustError(format!("promo: {}", r.as_str())))?;
            envelope["discount"] = json!(cut);
            envelope["promo"] = json!({ "code": p.code, "discount": cut });
            envelope["total"] = json!(subtotal - cut + fee + tip);
        }
        let stored = serde_json::to_string(&envelope).unwrap_or_else(|_| order_json.clone());
        hub.append(dowiz_hub::EventKind::Placed, &ev_id, &stored, seq, [0u8; 32])
            .map_err(|e| Error::RustError(format!("hub append failed: {e:?}")))?;
        Ok(stored)
    })
    .await;

    // THE ORDER DID NOT SURVIVE; ITS INGREDIENTS MUST NOT STAY HELD. A stranded
    // reservation makes a kitchen believe it is out of something it has, and
    // the failure is loud rather than silent because nothing else will notice.
    let stored = match stored {
        Ok(v) => v,
        Err(e) => {
            if !reservations.is_empty() {
                let oid = id.clone();
                let released = crate::hubstore::with_stock(&db, move |log| {
                    let led = log.ledger().map_err(|e| Error::RustError(e.to_string()))?;
                    let rel = dowiz_hub::stock::settle(&led, &oid, false);
                    if rel.is_empty() {
                        return Ok(());
                    }
                    log.append_all(&rel).map_err(|e| Error::RustError(e.to_string()))
                })
                .await;
                if let Err(re) = released {
                    console_error!("stock: could NOT release {id} after a failed placement: {re}");
                }
            }
            return Err(e);
        }
    };

    // The customer row is keyed by a HASH of the phone, never the phone itself,
    // so the table can be joined without holding the number in the clear.
    let cust_id = crate::edge_id().unwrap_or_else(|| format!("cust_{created_at_ms}"));
    let _ = db
        .prepare(
            "INSERT INTO customers (id,location_id,phone_hash,name,created_at_ms) VALUES (?1,?2,?3,?4,?5) \
             ON CONFLICT(location_id,phone_hash) DO UPDATE SET name = COALESCE(excluded.name, customers.name)",
        )
        .bind(&[cust_id.into(), loc.id.clone().into(), phone_hash.into(),
                body.contact.name.clone().unwrap_or_default().into(),
                worker::wasm_bindgen::JsValue::from_f64(created_at_ms as f64)])?
        .run()
        .await;

    // ── THE CUSTOMER'S KEY TO THEIR OWN ORDER ──
    //
    // Minted once, here, and returned exactly once. It is scoped to THIS order
    // and nothing else, so it cannot be walked to a neighbour's, and it carries
    // no phone and no name -- the claim shape refuses to hold them.
    //
    // Without it `/api/order/:id` had nothing to check and was public: anyone
    // who knew an id could read the name, the phone and the address off it.
    // Capability-by-obscurity, on a live deployment.
    //
    // Seven days, because that is how long somebody might reasonably come back
    // and ask what they ordered.
    let now = Date::now().as_millis() as i64;
    let customer_token = auth::sign(
        &ctx.env,
        &auth::Claims::Customer {
            // No account exists, so the subject is the order. There is
            // deliberately no customer registry.
            sub: id.clone(),
            order_id: id.clone(),
            location_id: loc.id.clone(),
            iat: now,
            exp: now + auth::CUSTOMER_TTL_MS,
        },
    )
    .ok();

    // A card order needs an intent before the browser can collect anything. The
    // ORDER ID is the idempotency key, so a retry -- a flaky connection, a double
    // tap, a replay after a lost generation guard -- returns the SAME intent
    // rather than charging twice.
    let mut out: Value = serde_json::from_str(&stored).unwrap_or(json!({}));
    // Returned once and never again: the hub keeps no copy, so a customer who
    // loses the link has lost it, which is the same guarantee the native
    // adapter gives.
    if let Some(tok) = customer_token {
        out["access_token"] = json!(tok);
    }
    if payment_kind == "card" {
        match crate::stripe::create_intent(&ctx.env, &id, total, &loc.currency_code).await {
            Ok((intent_id, client_secret)) => {
                out["payment_intent"] = json!(intent_id);
                out["client_secret"] = json!(client_secret);
            }
            // The order is already in the log and must not be lost because the
            // card rail is down. It comes back marked so the surface can offer
            // cash instead of pretending the order failed.
            Err(e) => {
                out["payment_error"] = json!(match e {
                    crate::stripe::PayError::NotConfigured => "card payments are not configured",
                    _ => "the card provider could not be reached",
                });
            }
        }
    }

    let mut res = Response::from_json(&out)?;
    res.headers_mut().set("content-type", "application/json; charset=utf-8")?;
    Ok(res)
}
