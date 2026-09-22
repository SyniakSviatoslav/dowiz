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
    /// THE TELEPHONE NUMBER IS OPTIONAL, by operator decision (2026-09-17).
    ///
    /// It used to be mandatory and eight digits, so a customer who would not
    /// give a number could not order at all. The number is a courtesy to the
    /// courier, not a fact the order depends on: the order is identified by its
    /// own id, priced by the kernel and tracked by a link the customer already
    /// holds. `#[serde(default)]` also keeps an older client that omits the
    /// field entirely from failing to place an order.
    #[serde(default)]
    pub phone: String,
}

#[derive(Deserialize)]
pub struct AddressIn {
    pub line: String,
    #[serde(default)]
    pub note: Option<String>,
    /// The address as the customer filled it in, part by part: street, house,
    /// apartment, entrance, floor, and whether it is a private house. The
    /// `line` is what the courier reads; the parts are what the console can
    /// sort and the courier app can show as fields. Free-form on the wire,
    /// bounded here: five short strings and a flag, nothing else survives.
    #[serde(default)]
    pub parts: Option<Value>,
    /// The door on the map, in micro-degrees, when the customer placed a pin.
    /// The live estimate measures the courier's road to it; without them the
    /// road is unknown and the estimate says so.
    #[serde(default)]
    pub lat_udeg: Option<i64>,
    #[serde(default)]
    pub lon_udeg: Option<i64>,
}

/// The address parts a customer may give, and how long each may be.
const ADDRESS_PART_KEYS: [&str; 5] = ["street", "house", "apartment", "entrance", "floor"];
const ADDRESS_PART_MAX_CHARS: usize = 80;

/// Only the named keys, each a short trimmed string, plus the `private` flag.
fn clean_address_parts(v: &Value) -> Value {
    let mut out = serde_json::Map::new();
    if let Some(o) = v.as_object() {
        for k in ADDRESS_PART_KEYS {
            if let Some(t) = o.get(k).and_then(Value::as_str).map(str::trim) {
                if !t.is_empty() {
                    out.insert(k.into(), json!(t.chars().take(ADDRESS_PART_MAX_CHARS).collect::<String>()));
                }
            }
        }
        if let Some(p) = o.get("private").and_then(Value::as_bool) {
            out.insert("private".into(), json!(p));
        }
    }
    Value::Object(out)
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
    /// For `payment: "crypto"`: which of the venue's wallets the customer
    /// will pay into, by symbol (`USDT`, `BTC`). Resolved against the venue's
    /// own list; the address never comes from the browser.
    #[serde(default)]
    pub crypto_symbol: Option<String>,
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
    /// The venue's IANA zone, so a client that renders a local time renders the
    /// venue's and not the phone's. Also what the owner console's hours sheet
    /// shows, because a weekly schedule without its zone is a schedule in an
    /// unstated timezone -- which is the defect this field exists to end.
    tz: String,
}

#[derive(Deserialize)]
pub(crate) struct LocRow {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) slug: String,
    pub(crate) phone: String,
    address: Option<String>,
    pub(crate) status: String,
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

#[derive(Deserialize)]
#[allow(dead_code)]
struct I18nRow {
    entity_id: String,
    field: String,
    value: String,
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
    // `fresh` is the console's word: an owner who just saved a dish reads the
    // catalogue as it is, not as the edge remembers it. Neither read nor
    // written to the cache, so customers keep the thirty-second window.
    let fresh = req.url()?.query_pairs().any(|(k, _)| k == "fresh");
    if !fresh {
        if let Some(hit) = cache.get(&key, false).await? {
            return Ok(hit);
        }
    }
    let db = ctx.d1("DB")?;
    // The slug is not the id — see `Place::of_slug`.
    let place = crate::hubstore::Place::of_slug(&ctx, &slug).await?;
    let loaded = crate::hubstore::load_catalog(&place).await?;

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
    // THE OFFSET IS NOT A CONSTANT, which is what this used to assume: the line
    // here read "Durrës is UTC+2" and defaulted to 120 minutes all year.
    // Europe/Tirane is UTC+1 in winter, so from 25 October 2026 a venue whose
    // kitchen closes at 23:00 would have been reported closed from 22:00 -- and
    // one opening at 09:00 would have taken orders from 08:00.
    //
    // A Worker still needs no timezone database. It needs a NAME and the EU
    // rule, both of which are arithmetic in `dowiz_hub::tz`, and the name is
    // already on the record this handler has in hand -- no extra read.
    let zone = crate::hubstore::zone_of(Some(&raw));
    let now_ms = Date::now().as_millis() as i64;
    let (weekday, minute) = dowiz_hub::tz::local_weekday_minute(zone, now_ms);
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

    // ── THE MENU IS SERVED IN THE LANGUAGE THAT WAS ASKED FOR ──
    //
    // `?locale=` was accepted by the route and then IGNORED: the chrome around
    // the menu translated on every surface while the dishes stayed in the
    // venue's own language, which is what a customer reads as "the translation
    // does not work". `content_i18n` has held the translated strings since the
    // first catalogue migration and NOTHING HAS EVER READ IT -- the table was
    // created, documented, and never joined.
    //
    // Only a locale that DIFFERS from the venue's own costs a query, and a
    // missing translation falls back to the venue's string rather than to an
    // empty one: a dish with no Ukrainian name must still have a name.
    let want_locale = req
        .url()?
        .query_pairs()
        .find(|(k, _)| k == "locale")
        .map(|(_, v)| v.to_string())
        .unwrap_or_else(|| loc.default_locale.clone());

    let mut i18n: std::collections::HashMap<(String, String), String> =
        std::collections::HashMap::new();
    // What could not be translated and why. A failure here must not take the
    // menu down -- the venue's own words are still a menu -- but it must not
    // look like "no translations exist" either, which is exactly how 146
    // stored rows were served as Albanian for a day.
    let mut warnings: Vec<String> = Vec::new();
    if want_locale != loc.default_locale && !want_locale.is_empty() {
        // ONE PREFIX SCAN, AND NO IDS AT ALL.
        //
        // This was a chunked `WHERE entity_id IN (...)` against `content_i18n`,
        // and it carried two defects that were both the table's shape:
        //   * D1 refuses more than 100 bound values and a 165-dish catalogue
        //     plus its headings is 186 of them, so the ids had to be chunked --
        //     and before that was noticed, 187 ids in one statement failed
        //     SILENTLY and a venue served Albanian for a day.
        //   * `content_i18n` HAD NO VENUE COLUMN. Every row of every venue
        //     shared one table, keyed by an entity id and nothing else.
        //
        // In the venue's own image both stop existing. The image IS the venue,
        // so there is no venue to filter on; and the key carries the locale, so
        // the whole language is one sorted range. What is read is whatever the
        // catalogue actually has, which is also why the ids are no longer built
        // and passed at all.
        let want = format!("{want_locale}/");
        match crate::hubstore::load_table(
            &place,
            crate::hubstore::IMAGE_I18N,
            crate::hubstore::I18N_BYTES,
        )
        .await
        {
            Ok(l) => {
                for (key, value) in l.table.all(crate::hubstore::I18N_KIND) {
                    // `<locale>/<entity_type>/<entity_id>/<field>`
                    let Some(rest) = key.strip_prefix(&want) else { continue };
                    let mut parts = rest.splitn(3, '/');
                    let (_entity_type, id, field) =
                        match (parts.next(), parts.next(), parts.next()) {
                            (Some(a), Some(b), Some(c)) => (a, b, c),
                            _ => continue,
                        };
                    if !matches!(field, "name" | "description" | "ingredients") {
                        continue;
                    }
                    i18n.insert((id.to_string(), field.to_string()), value);
                }
            }
            // A failure here must not take the menu down -- the venue's own
            // words are still a menu -- but it must not look like "no
            // translations exist" either, which is exactly how 146 stored rows
            // were served as Albanian for a day.
            Err(e) => {
                console_error!("menu i18n {want_locale}: {e}");
                warnings.push(format!("translations unavailable: {e}"));
            }
        }
        // NOT MIGRATED YET? An empty image is indistinguishable from a venue
        // with no translations, and only one of those is worth a fallback. The
        // old table answers while it still has rows, and says so loudly every
        // time -- a silent fallback would work forever and the migration would
        // never be finished. Deleted with `migrate.rs`.
        if i18n.is_empty() {
            let mut ids: Vec<String> = products.iter().map(|(id, _)| id.clone()).collect();
            ids.extend(cat_meta.iter().map(|(id, _, _)| id.clone()));
            if let Some(old) = crate::migrate::i18n_fallback(&db, &want_locale, &ids).await {
                i18n = old;
            }
        }
    }
    let translated = |id: &str, field: &str, fallback: Value| -> Value {
        match i18n.get(&(id.to_string(), field.to_string())) {
            Some(v) => json!(v),
            None => fallback,
        }
    };
    // A list field: the stored value is a JSON array of strings. One that does
    // not parse as such falls back to the venue's own list rather than to a
    // one-element list holding the raw text.
    let translated_list = |id: &str, field: &str, fallback: Value| -> Value {
        match i18n
            .get(&(id.to_string(), field.to_string()))
            .and_then(|v| serde_json::from_str::<Value>(v).ok())
        {
            Some(Value::Array(a)) if a.iter().all(|x| x.is_string()) => Value::Array(a),
            _ => fallback,
        }
    };

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
                        "name": translated(id, "name",
                            p.get("name").cloned().unwrap_or(json!(""))),
                        "description": translated(id, "description",
                            p.get("description").cloned().unwrap_or(Value::Null)),
                        "price": p.get("price").cloned().unwrap_or(json!(0)),
                        "available": p.get("available").and_then(|x| x.as_bool()).unwrap_or(true),
                        "unavailableNote": p.get("unavailableNote").cloned().unwrap_or(Value::Null),
                        "imageUrl": p.get("imageUrl").cloned().unwrap_or(Value::Null),
                        // The grid's card, when the venue has uploaded one.
                        // `menu.js` turns the pair into a `srcset`, so a phone
                        // fetches about a tenth of the bytes for the same
                        // picture; absent, the full photograph is used exactly
                        // as before.
                        "imageUrlSmall": p.get("imageUrlSmall").cloned().unwrap_or(Value::Null),
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
                        // What the VENUE says this dish takes. Null means it
                        // has not said — which the estimate treats as the
                        // venue's default, never as instant.
                        "cookingMin": p.get("cookingMin").cloned().unwrap_or(Value::Null),
                        // ── What is in the dish ──
                        //
                        // The three questions a price cannot answer: what is in
                        // it, how much of it there is, what it does to the day.
                        // Every one of them is passed through AS STORED, null
                        // included: a dish whose protein the venue never
                        // declared must reach the screen as "not declared", and
                        // a zero here would reach it as "0 g" instead — the
                        // same distinction `allergens` is careful about, for
                        // the same reason.
                        // WHAT THE DISH IS, as the venue files it: salmon, tuna,
                        // vegetarian, hot. The customer's filter is built from
                        // these, so a venue that has declared none gets no
                        // filter rather than an empty one.
                        "tags": p.get("tags").cloned().unwrap_or(Value::Null),
                        "ingredients": translated_list(id, "ingredients",
                            p.get("ingredients").cloned().unwrap_or(Value::Null)),
                        "weightG": p.get("weightG").cloned().unwrap_or(Value::Null),
                        "nutrition": p.get("nutrition").cloned().unwrap_or(Value::Null),
                        "nutritionDerived": p.get("nutritionDerived").cloned().unwrap_or(Value::Null),
                        "taste": p.get("taste").cloned().unwrap_or(Value::Null),
                        "calories": p.get("calories").cloned().unwrap_or(Value::Null),
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
            // A category is a heading the customer reads, so it is translated
            // on exactly the same terms as the dishes under it.
            "id": cid, "name": translated(cid, "name", json!(cname)), "sortOrder": csort,
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
        // The name the venue is configured with, or the default this build
        // applies when it has none -- never an empty string, because a client
        // reading "" would have to invent a fallback and would invent a
        // different one from the server's.
        tz: raw
            .get("tz")
            .and_then(Value::as_str)
            .filter(|n| dowiz_hub::tz::zone(n).is_some())
            .unwrap_or(dowiz_hub::tz::DEFAULT_NAME)
            .to_string(),
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
    // The venue's own mark, for the same reason its own colours are here: a
    // storefront that carries the platform's name and not the venue's is a
    // storefront the customer does not recognise as the place they are
    // ordering from.
    location["logoUrl"] = raw.get("logo_url").cloned().unwrap_or(Value::Null);
    // What the venue puts around its mark (seal, motif, two supporting
    // colours), validated on the way in by `owner::clean_stage`.
    location["stage"] = raw.get("stage").cloned().unwrap_or(Value::Null);
    // Where the venue is, and what its Google listing says. Both are the
    // venue's own material about itself; the storefront draws a map from the
    // first and attributes the second to where it came from.
    location["lat"] = raw.get("lat").cloned().unwrap_or(Value::Null);
    location["lng"] = raw.get("lng").cloned().unwrap_or(Value::Null);
    location["google"] = raw.get("google").cloned().unwrap_or(Value::Null);
    // The WEEK, not just today. `status` and `nextOpen` answer "can I order
    // now"; a customer deciding whether to come on Sunday needs the table, and
    // the schedule has never left the Worker.
    location["hours"] = raw.get("hours").cloned().unwrap_or(Value::Null);
    location["pickup"] = json!(raw.get("pickup").and_then(Value::as_bool).unwrap_or(false));
    location["hasDeliveryZones"] = json!(raw.get("delivery_zones").is_some());
    location["nextOpen"] = next_open
        .map(|(d, m)| json!({ "weekday": d, "minute": m }))
        .unwrap_or(Value::Null);
    // The owner's OWN switches, for the console: `status` above is what a
    // customer sees (closed outside hours or when paused), not what was set.
    location["ownerStatus"] = json!(loc.status);
    location["deliveryPaused"] = json!(paused);
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
    let settings = crate::hubstore::load_settings(&place).await?.settings;
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
    // ── HOW THIS VENUE CAN BE PAID ──
    //
    // One block, decided here, so the storefront never offers a rail that
    // cannot complete. Cash is always on. Card, Apple Pay and Google Pay are
    // all the Stripe rail -- the wallets are the Payment Element's own tabs --
    // and exist exactly when the publishable key does. Crypto is the venue's
    // own wallets, declared by the owner (`payments.crypto` on the venue
    // record): a network, a symbol and an address each, and nothing is
    // invented for a venue that declared none.
    let stripe_on = ctx.env.secret("STRIPE_PUBLISHABLE_KEY").is_ok();
    let crypto = payment_wallets(&raw);
    location["payments"] = json!({
        "cash": true,
        "card": stripe_on,
        "applePay": stripe_on,
        "googlePay": stripe_on,
        "crypto": crypto,
    });

    let out = json!({
        "location": location,
        "categories": cats,
        // Empty when everything the menu needed was read. Anything here is a
        // degraded answer and says so, rather than a full one that is wrong.
        "warnings": warnings,
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
        if !fresh {
            let _ = cache.put(&key, copy).await;
        }
    }
    Ok(res)
}

/// The venue's crypto wallets, validated on the way OUT as well as in: only
/// entries with a network, a symbol and a non-empty address are ever shown.
pub(crate) fn payment_wallets(raw: &Value) -> Vec<Value> {
    raw.pointer("/payments/crypto")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter(|w| {
                    ["network", "symbol", "address"].iter().all(|k| {
                        w.get(k).and_then(Value::as_str).map_or(false, |s| !s.trim().is_empty())
                    })
                })
                .map(|w| {
                    json!({
                        "network": w["network"], "symbol": w["symbol"], "address": w["address"],
                        "note": w.get("note").cloned().unwrap_or(Value::Null)
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The most values one D1 statement may bind. Cloudflare's documented limit
/// is 100; a statement over it is refused at prepare time.
const D1_MAX_BINDS: usize = 100;

/// Every way an order can say it will be paid. Anything else is refused at
/// the boundary rather than stored as a word the kitchen has to interpret.
const PAYMENT_KINDS: [&str; 5] = ["cash", "card", "apple_pay", "google_pay", "crypto"];

/// `GET /manifest.webmanifest` -- the venue's storefront as an app.
///
/// One manifest per venue, written from the venue's own record: its name,
/// its paper as the splash and chrome colour, its mark as the icon. The mark's
/// real size is read from the PNG's own header (IHDR, bytes 16..24) so the
/// manifest declares what the file is, which is what a browser checks before
/// it offers to install. A venue with no mark gets no icon and the browser
/// says so; a placeholder would install a nameless square.
pub async fn manifest(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let slug = req
        .url()?
        .query_pairs()
        .find(|(k, _)| k == "s")
        .map(|(_, v)| v.to_string())
        .or_else(|| crate::hubstore::Place::slug_of_host(&req, &ctx));
    let Some(slug) = slug else {
        return Response::error("not found", 404);
    };
    let place = crate::hubstore::Place::of_slug(&ctx, &slug).await?;
    let loaded = crate::hubstore::load_catalog(&place).await?;
    let Some(loc_json) = loaded.catalog.location() else {
        return Response::error("not found", 404);
    };
    let raw: Value = serde_json::from_str(&loc_json).unwrap_or(json!({}));
    let name = raw.get("name").and_then(Value::as_str).unwrap_or("dowiz");
    let paper = raw
        .pointer("/theme/paper")
        .and_then(Value::as_str)
        .unwrap_or(MANIFEST_DEFAULT_PAPER);
    let lang = raw.get("default_locale").and_then(Value::as_str).unwrap_or("sq");
    let mut icons: Vec<Value> = Vec::new();
    if let Some(logo) = raw.get("logo_url").and_then(Value::as_str) {
        if let Some(key) = logo.strip_prefix("/media/") {
            let kv = ctx.kv("MEDIA")?;
            if let Some(bytes) = kv.get(key).bytes().await? {
                if let Some((w, h)) = png_size(&bytes) {
                    icons.push(json!({ "src": logo, "sizes": format!("{w}x{h}"), "type": "image/png", "purpose": "any maskable" }));
                }
            }
        }
    }
    let out = json!({
        "id": "/", "name": name, "short_name": short_name(name),
        "lang": lang, "dir": "ltr", "start_url": "/", "scope": "/",
        "display": "standalone", "display_override": ["standalone", "minimal-ui", "browser"],
        "orientation": "portrait", "background_color": paper, "theme_color": paper,
        "categories": ["food", "shopping"], "icons": icons,
    });
    let mut res = Response::from_json(&out)?;
    res.headers_mut().set("content-type", "application/manifest+json; charset=utf-8")?;
    res.headers_mut().set("cache-control", "public, max-age=300")?;
    Ok(res)
}

/// The paper a venue with no theme gets in its manifest: the shipped light paper.
const MANIFEST_DEFAULT_PAPER: &str = "#fbfaf8";
/// A launcher shows about this many characters under an icon. The short name
/// is the longest run of WHOLE words that fits ("Dubin", not "Dubin & Sush"),
/// and the first word cut only when even that is too long.
const MANIFEST_SHORT_NAME_CHARS: usize = 12;
fn short_name(name: &str) -> String {
    if name.chars().count() <= MANIFEST_SHORT_NAME_CHARS {
        return name.to_string();
    }
    let mut out = String::new();
    for word in name.split_whitespace() {
        let next = if out.is_empty() { word.to_string() } else { format!("{out} {word}") };
        if next.chars().count() > MANIFEST_SHORT_NAME_CHARS {
            break;
        }
        out = next;
    }
    // A trailing word with no letter in it ("&", "-") is not a name's end.
    while let Some(last) = out.split_whitespace().last() {
        if last.chars().any(char::is_alphanumeric) {
            break;
        }
        out = out[..out.len() - last.len()].trim_end().to_string();
    }
    if out.is_empty() {
        out = name.chars().take(MANIFEST_SHORT_NAME_CHARS).collect();
    }
    out
}
/// A PNG's size from its IHDR chunk, which every PNG starts with.
const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
const PNG_IHDR_WIDTH_AT: usize = 16;
fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < PNG_IHDR_WIDTH_AT + 8 || bytes[..8] != PNG_SIGNATURE {
        return None;
    }
    let w = u32::from_be_bytes(bytes[PNG_IHDR_WIDTH_AT..PNG_IHDR_WIDTH_AT + 4].try_into().ok()?);
    let h = u32::from_be_bytes(bytes[PNG_IHDR_WIDTH_AT + 4..PNG_IHDR_WIDTH_AT + 8].try_into().ok()?);
    (w > 0 && h > 0).then_some((w, h))
}

/// `POST /api/public/locations/:slug/orders`
pub async fn place(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let Some(slug) = ctx.param("slug").cloned() else {
        return Response::error("missing slug", 400);
    };
    // THE BODY IS READ AS TEXT FIRST, because the idempotency layer has to
    // fingerprint exactly what was sent. Parsing and re-serialising would make
    // "same key, different body" a question about serde's output rather than
    // about what the client did.
    let raw_body = match req.text().await {
        Ok(t) => t,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let idem_key = req.headers().get("idempotency-key").ok().flatten();
    let body: PlaceIn = match serde_json::from_str(&raw_body) {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    if body.items.is_empty() {
        return Response::error("empty order", 400);
    }
    // A number that IS given must still look like one -- a half-typed number is
    // worse than none, because the courier will try it. An EMPTY one is now
    // accepted: see `ContactIn::phone`.
    let phone = body.contact.phone.trim();
    if !phone.is_empty() && phone.chars().filter(|c| c.is_ascii_digit()).count() < 8 {
        return Response::error("invalid phone", 400);
    }
    if body.fulfilment.kind == "delivery"
        && body.fulfilment.address.as_ref().map_or(true, |a| a.line.trim().is_empty())
    {
        return Response::error("delivery address required", 400);
    }

    // The slug is not the id — see `Place::of_slug`.
    let place = crate::hubstore::Place::of_slug(&ctx, &slug).await?;
    let loaded = crate::hubstore::load_catalog(&place).await?;
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
    // ONE PRICER, in `services::ordering::pricing`, where each of these rules
    // has a test. It is the same call the promo preview makes, so the number a
    // customer is quoted and the number they are charged are computed once.
    //
    // WHAT IT DEFENDS, and each of these was a real defect: the options are
    // part of the price (a dish at 800 with "extra salmon +200", ordered twice,
    // was shown as 2000 and charged at 1600 — the venue paid the difference on
    // every order with a paid option); an option id belonging to no group on
    // the dish, or a required group left empty, was accepted silently; a
    // negative delta must not make a line pay the customer; and an unknown,
    // unavailable or unpriced dish fails CLOSED rather than at zero.
    let basket = match crate::services::ordering::pricing::price_basket(
        |id| loaded.catalog.product(id),
        body.items.iter().map(|it| crate::services::ordering::pricing::Want {
            product_id: &it.product_id,
            modifier_ids: &it.modifier_ids,
            quantity: it.quantity,
        }),
    ) {
        Ok(b) => b,
        Err(r) => return Response::error(r.text(), r.status()),
    };
    let subtotal = basket.subtotal;
    // The dish names, by product id, to be written back onto the kernel's
    // items below — see where they are applied for why they cannot be put
    // on the line here.
    //
    // THE DISH'S NAME TRAVELS WITH THE LINE, and it never did. Every surface
    // that shows an order line writes `i.name || i.product_id` — the console's
    // queue, the order sheet, the CSV an owner exports — and no line carried a
    // `name`, so a kitchen read `1x item-05` on every ticket this product ever
    // printed. A SNAPSHOT, deliberately: the catalogue is not versioned per
    // order, so resolving the name later would rename a dish on orders placed
    // before the rename and lose it entirely on one that has been deleted.
    let names: std::collections::BTreeMap<String, String> = basket
        .lines
        .iter()
        .map(|l| (l.product_id.clone(), l.name.clone()))
        .collect();
    // The same lines as the owner will read them, for the bell.
    let told: Vec<crate::notify::LineOut> = basket
        .lines
        .iter()
        .map(|l| crate::notify::LineOut {
            name: l.name.clone(),
            quantity: l.quantity,
            unit_price: l.unit_price,
        })
        .collect();
    let lines: Vec<Value> = basket
        .lines
        .iter()
        .map(|l| {
            json!({
                "product_id": l.product_id, "modifier_ids": l.modifier_ids,
                "quantity": l.quantity, "unit_price": l.unit_price   // base + options, from the catalogue
            })
        })
        .collect();
    if subtotal < loc.min_order {
        return Response::error("below minimum order", 409);
    }

    let fee = match loc.free_delivery_threshold {
        Some(th) if subtotal >= th => 0,
        _ if body.fulfilment.kind == "pickup" => 0,
        _ => loc.delivery_fee,
    };

    // ── IS THIS A RETRY? ──
    //
    // ASKED HERE, after the basket has been validated against the catalogue and
    // before the id is minted. Earlier and a malformed retry would be recorded
    // as an answer; later and the second order already exists, which is the
    // whole defect. The id below comes from the CSPRNG per attempt, so without
    // this a customer on a weak connection whose response was lost gets a
    // second order, a second reservation and a second ticket -- and Stripe's
    // idempotency key, derived from that id, cannot protect any of them.
    let created_at_ms = Date::now().as_millis() as i64;
    let idem_print = crate::idempotency::fingerprint(&raw_body);
    // The principal is the CONTACT this basket names, so one person's retry
    // cannot replay into another's order even under a guessed key.
    let idem_who = crate::auth::sha256_hex(phone);
    let idem = crate::idempotency::begin(
        &place,
        idem_key,
        &idem_who,
        "storefront.place",
        &raw_body,
        created_at_ms,
    )
    .await;
    let idem_key = match idem {
        crate::idempotency::Decision::NoKey => None,
        crate::idempotency::Decision::Proceed { key } => Some(key),
        crate::idempotency::Decision::Refuse(r) => return Ok(r),
        crate::idempotency::Decision::Replay { status, body } => {
            // THE FIRST CALL'S WHOLE ANSWER, not a marker. It carries the order
            // id, the customer's token and the payment intent; a 204 here would
            // leave the client with an order it cannot open.
            let mut res = Response::ok(body)?.with_status(status);
            res.headers_mut().set("content-type", "application/json")?;
            res.headers_mut().set("idempotent-replay", "true")?;
            return Ok(res);
        }
    };

    let Some(id) = crate::edge_id() else {
        return Response::error("no platform CSPRNG for order id", 500);
    };

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
    //
    // THE DISH'S NAME IS ONE OF THOSE FIELDS, and putting it on the line before
    // the kernel sees it does nothing: `place_order_at` parses each line into
    // the kernel's four-field `OrderItem` and re-emits it, so anything else is
    // dropped before it is ever stored. That is why the first attempt at this
    // wrote a `name` that never appeared anywhere.
    //
    // Every surface writes `i.name || i.product_id` — the console's queue, the
    // order sheet, the CSV an owner exports — and no line has ever carried a
    // name, so a kitchen ticket has always read `1x item-05`. It is a SNAPSHOT
    // on purpose: the catalogue is not versioned per order, so resolving the
    // name later would rename a dish on orders placed before the rename and
    // lose it entirely on one that has been deleted.
    let mut envelope: Value = serde_json::from_str(&order_json)
        .map_err(|e| Error::RustError(format!("kernel order json unreadable: {e}")))?;
    if let Some(items) = envelope.get_mut("items").and_then(Value::as_array_mut) {
        for line in items.iter_mut() {
            let Some(pid) = line.get("product_id").and_then(Value::as_str).map(str::to_string) else {
                continue;
            };
            if let Some(n) = names.get(&pid) {
                line["name"] = json!(n);
            }
        }
    }
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
        "address": body.fulfilment.address.as_ref().map(|a| json!({
            "line": a.line, "note": a.note,
            "parts": a.parts.as_ref().map(clean_address_parts).unwrap_or(Value::Null),
            "lat_udeg": a.lat_udeg, "lon_udeg": a.lon_udeg
        })),
        "fee": fee
    });
    let payment_kind = body.payment.clone().unwrap_or_else(|| "cash".into());
    if !PAYMENT_KINDS.contains(&payment_kind.as_str()) {
        return Response::error("unknown payment method", 400);
    }
    // A rail the venue does not have is refused BEFORE the order exists, so a
    // crypto order at a venue with no wallet never sits in the queue waiting
    // for money that has nowhere to go.
    let raw_loc: Value = serde_json::from_str(&loc_json).unwrap_or(json!({}));
    let wallets = payment_wallets(&raw_loc);
    let stripe_on = ctx.env.secret("STRIPE_PUBLISHABLE_KEY").is_ok();
    match payment_kind.as_str() {
        "crypto" if wallets.is_empty() => {
            return Response::error("this venue does not take crypto", 409)
        }
        "card" | "apple_pay" | "google_pay" if !stripe_on => {
            return Response::error("card payments are not configured", 409)
        }
        _ => {}
    }
    envelope["payment"] = json!(payment_kind);
    // Which wallet the customer chose to pay into, when there is a choice. The
    // symbol is enough: the address is looked up from the venue's own list, so
    // nothing a browser sent can redirect the money.
    if payment_kind == "crypto" {
        let want = body.crypto_symbol.as_deref().map(str::trim).unwrap_or("");
        let chosen = wallets
            .iter()
            .find(|w| w["symbol"].as_str() == Some(want))
            .or_else(|| wallets.first())
            .cloned()
            .unwrap_or(Value::Null);
        envelope["crypto"] = json!({ "wallet": chosen, "paid": false });
    }

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
        let held = crate::hubstore::with_stock(&place, move |log| {
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
    // THE ONE WRITER THAT STILL TAKES THE WHOLE IMAGE, and it is not an
    // oversight. Every other path appends through the object -- the Worker
    // sends an event and the object holds the log -- but a promotion's LAST
    // USE has to be counted and spent in the same breath as the append, or two
    // customers redeem it at once. `with_hub` is that breath: one read, one
    // decision, one write, guarded by the generation. A placement is once per
    // order, not once per poll, so what it costs is paid rarely; moving the
    // redemption into the object is phase 6's business, where the object gains
    // a command surface of its own.
    let stored = crate::hubstore::with_hub(&place, move |hub| {
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
                let released = crate::hubstore::with_stock(&place, move |log| {
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

    // WHAT THE CUSTOMER IS CHARGED IS WHAT THE ORDER SAYS, and those were two
    // different numbers. `total` above is `subtotal + fee + tip`, computed
    // BEFORE the promo: the discount is applied to `envelope["total"]` inside
    // the CAS closure, because redeeming a code has to happen in the same
    // breath as the write. The outer binding never learned about it, and it is
    // the one `create_intent` was handed -- so a card customer with a working
    // promo code saw 2700 on the screen, had 2700 stored against their order,
    // and was charged 3000. The webhook then recorded `amount_received` 3000
    // and marked it paid, so nothing downstream disagreed with anything.
    //
    // `stored` is the envelope as it was actually written, discount and all.
    // It is the only total with authority here, so it is the one that travels
    // to Stripe.
    let total = serde_json::from_str::<Value>(&stored)
        .ok()
        .and_then(|v| v.get("total").and_then(Value::as_i64))
        .unwrap_or(total);

    // The customer row is keyed by a HASH of the phone, never the phone itself,
    // so the table can be joined without holding the number in the clear.
    //
    // NO PHONE MEANS NO CUSTOMER ROW, and that is not a shortcut. The key is
    // `(location_id, phone_hash)`, and the hash of the empty string is a
    // CONSTANT: writing it would file every customer who declined to give a
    // number into ONE row per venue, each order overwriting the last one's
    // name. The order itself is complete without it — it carries its own
    // contact envelope — so the registry simply does not gain a row.
    if !phone.is_empty() {
    let cust_id = crate::edge_id().unwrap_or_else(|| format!("cust_{created_at_ms}"));
    // THE PHONE HASH IS THE KEY. The table had a surrogate id and a UNIQUE on
    // `(location_id, phone_hash)`; the venue is the image, so what is left of
    // that constraint is the hash, and it is the record's id. The upsert's
    // `COALESCE(excluded.name, customers.name)` is kept: a later order with no
    // name must not erase the name an earlier one gave.
    let who = phone_hash.clone();
    let given = body.contact.name.clone().unwrap_or_default();
    let _ = crate::hubstore::with_table(
        &place,
        crate::hubstore::IMAGE_PEOPLE,
        crate::hubstore::PEOPLE_BYTES,
        move |t| {
            let existing = t
                .get("cust", &who)
                .and_then(|j| serde_json::from_str::<Value>(&j).ok());
            let name = if given.trim().is_empty() {
                existing
                    .as_ref()
                    .and_then(|e| e.get("name").and_then(Value::as_str))
                    .unwrap_or("")
                    .to_string()
            } else {
                given.clone()
            };
            let created = existing
                .as_ref()
                .and_then(|e| e.get("created_at_ms").and_then(Value::as_i64))
                .unwrap_or(created_at_ms);
            let rec = json!({
                "id": cust_id, "phone_hash": who, "name": name, "created_at_ms": created,
            })
            .to_string();
            t.put("cust", &who, &rec, &[], &[])
                .map_err(|e| Error::RustError(format!("customer: {e}")))
        },
    )
    .await;
    }

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
    // The owner's phone, AFTER the log and BEFORE the card rail: a bell that
    // waits on Stripe is a bell that stays silent when Stripe is down.
    crate::notify::order_placed(&ctx.env, &place, &out, &told, &loc.currency_code, &loc.name).await;
    // Returned once and never again: the hub keeps no copy, so a customer who
    // loses the link has lost it, which is the same guarantee the native
    // adapter gives.
    if let Some(tok) = customer_token {
        out["access_token"] = json!(tok);
    }
    // Apple Pay and Google Pay ARE the card rail: the Payment Element shows
    // them as tabs on the same intent, and the intent is what a wallet pays.
    if matches!(payment_kind.as_str(), "card" | "apple_pay" | "google_pay") {
        // ── THE BREAKER, AND WHAT IT IS ACTUALLY FOR ──
        //
        // The fallback below already exists and already works: the order is in
        // the log and comes back marked so the surface can offer cash. What it
        // does not do is stop TRYING, so during a Stripe outage every customer
        // pays the full timeout before being offered that. This is what stops
        // the waiting, and it is the only thing it does.
        match crate::rail::admit(&place, crate::rail::Rail::Stripe, created_at_ms).await {
            crate::rail::Gate::Tripped { since_ms } => {
                console_error!("stripe rail open for {since_ms} ms; offering cash immediately");
                out["payment_error"] = json!("the card provider could not be reached");
            }
            crate::rail::Gate::Go => {
        match crate::stripe::create_intent(&ctx.env, &id, total, &loc.currency_code).await {
            Ok((intent_id, client_secret)) => {
                // ONE SUCCESS CLOSES IT. Requiring several would keep a
                // recovered provider shut out for no reason a customer would
                // accept.
                crate::rail::record(&place, crate::rail::Rail::Stripe, true, created_at_ms)
                    .await;
                out["payment_intent"] = json!(intent_id);
                out["client_secret"] = json!(client_secret);
            }
            // The order is already in the log and must not be lost because the
            // card rail is down. It comes back marked so the surface can offer
            // cash instead of pretending the order failed.
            Err(e) => {
                // A rail that is NOT CONFIGURED has not failed -- there is
                // nothing there to fail. Counting it would open the breaker on
                // every venue that takes only cash and then report them as
                // outages.
                if !matches!(e, crate::stripe::PayError::NotConfigured) {
                    crate::rail::record(
                        &place,
                        crate::rail::Rail::Stripe,
                        false,
                        created_at_ms,
                    )
                    .await;
                }
                out["payment_error"] = json!(match e {
                    crate::stripe::PayError::NotConfigured => "card payments are not configured",
                    _ => "the card provider could not be reached",
                });
            }
        }
            }
        }
    }

    // RECORDED AFTER THE ANSWER IS BUILT, and it cannot fail the call: the
    // order is already in the log and the customer already has their token.
    // Losing the record means a retry runs again, which is exactly the
    // behaviour without this layer -- never worse than that.
    if let Some(key) = idem_key {
        let body = out.to_string();
        crate::idempotency::finish(&place, &key, 200, &body, &idem_print, created_at_ms).await;
    }

    let mut res = Response::from_json(&out)?;
    res.headers_mut().set("content-type", "application/json; charset=utf-8")?;
    Ok(res)
}
