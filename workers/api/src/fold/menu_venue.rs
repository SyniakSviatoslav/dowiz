//! The storefront menu's two halves, as pure functions (R2, `fold::menu`).
//!
//! MOVED FROM `storefront::menu`, where they ran in the Worker per request.
//! The rules and their reasons are unchanged and are kept here in short; the
//! long history of each field is in that file's git log.

use serde::Serialize;
use serde_json::{json, Value};

use crate::storefront::LocRow;

#[derive(Serialize)]
struct LocationOut<'a> {
    id: &'a str,
    name: &'a str,
    slug: &'a str,
    phone: &'a str,
    address: &'a Option<String>,
    status: &'a str,
    #[serde(rename = "closesAt")]
    closes_at: &'a Option<String>,
    #[serde(rename = "deliveryEta")]
    delivery_eta: &'a str,
    #[serde(rename = "deliveryFee")]
    delivery_fee: i64,
    #[serde(rename = "freeDeliveryThreshold")]
    free_delivery_threshold: Option<i64>,
    #[serde(rename = "minOrder")]
    min_order: i64,
    #[serde(rename = "currencyCode")]
    currency_code: &'a str,
    #[serde(rename = "menuVersion")]
    menu_version: i64,
    #[serde(rename = "supportedLocales")]
    supported_locales: Value,
    #[serde(rename = "defaultLocale")]
    default_locale: &'a str,
    /// The venue's IANA zone, never "": a client would invent a different fallback.
    tz: String,
}

/// The `location` block, everything but the clock: rendered once per memo.
pub fn base(loc: &LocRow, raw: &Value, settings: &dowiz_hub::settings::Settings, rails: &super::menu::Rails) -> Value {
    let mut location = serde_json::to_value(LocationOut {
        id: &loc.id,
        name: &loc.name,
        slug: &loc.slug,
        phone: &loc.phone,
        address: &loc.address,
        status: &loc.status,
        closes_at: &loc.closes_at,
        delivery_eta: &loc.delivery_eta,
        delivery_fee: loc.delivery_fee,
        free_delivery_threshold: loc.free_delivery_threshold,
        min_order: loc.min_order,
        currency_code: &loc.currency_code,
        menu_version: loc.menu_version,
        supported_locales: serde_json::from_str(&loc.supported_locales).unwrap_or_else(|_| json!(["sq"])),
        default_locale: &loc.default_locale,
        tz: raw
            .get("tz")
            .and_then(Value::as_str)
            .filter(|n| dowiz_hub::tz::zone(n).is_some())
            .unwrap_or(dowiz_hub::tz::DEFAULT_NAME)
            .to_string(),
    })
    .unwrap_or(json!({}));
    // The venue's own material about itself, passed through as stored.
    for (out, key) in [("theme", "theme"), ("logoUrl", "logo_url"), ("stage", "stage"), ("lat", "lat"),
        ("lng", "lng"), ("google", "google"), ("hours", "hours")]
    {
        location[out] = raw.get(key).cloned().unwrap_or(Value::Null);
    }
    location["pickup"] = json!(raw.get("pickup").and_then(Value::as_bool).unwrap_or(false));
    location["hasDeliveryZones"] = json!(crate::services::venue::activation::has_delivery_zones(raw));
    location["deliveryZones"] = crate::services::venue::activation::delivery_zones(raw);
    // The owner's OWN switches: `status` is what a customer sees, not what was set.
    location["ownerStatus"] = json!(loc.status);
    location["deliveryPaused"] = json!(loc.delivery_paused == 1);
    // WHICH FEATURES THIS VENUE HAS ON, sent with the menu so a control never
    // appears a moment after the page does.
    let mut features = serde_json::Map::new();
    for (f, on) in dowiz_hub::features::all(settings) {
        if f.surface == "storefront" {
            features.insert(f.key.trim_start_matches("feature.").to_string(), json!(on));
        }
    }
    location["features"] = Value::Object(features);
    location["telegramBot"] = rails.telegram_bot.as_ref().map_or(Value::Null, |b| json!(b));
    // HOW THIS VENUE CAN BE PAID: cash always; card and the wallets exactly when
    // the Stripe key exists; crypto only the venue's own declared wallets.
    let stripe_on = rails.stripe_key.is_some();
    location["payments"] = json!({
        "cash": true, "card": stripe_on, "applePay": stripe_on, "googlePay": stripe_on,
        "crypto": crate::storefront::payment_wallets(raw),
    });
    location
}

/// The `location` block at `now_ms`. THE STATUS IS DERIVED, not read: a paused
/// venue is closed however its flag reads, and a schedule can only close.
pub fn at(base: &Value, loc: &LocRow, raw: &Value, now_ms: i64) -> Value {
    let sched = raw.get("hours").map(|h| dowiz_hub::hours::from_json(&h.to_string())).unwrap_or_default();
    let zone = crate::hubstore::zone_of(Some(raw));
    let (weekday, minute) = dowiz_hub::tz::local_weekday_minute(zone, now_ms);
    let scheduled_open = sched.is_empty() || sched.is_open_at(weekday, minute);
    let next_open = sched.next_open(weekday, minute);
    let paused = loc.delivery_paused == 1;
    let closed = paused || loc.status == "closed" || !scheduled_open;
    let mut location = base.clone();
    location["status"] = json!(if closed { "closed" } else { loc.status.as_str() });
    location["nextOpen"] = next_open.map(|(d, m)| json!({ "weekday": d, "minute": m })).unwrap_or(Value::Null);
    location["closedReason"] = if paused {
        json!("paused")
    } else if loc.status == "closed" {
        json!("manual")
    } else if !scheduled_open {
        json!("hours")
    } else {
        Value::Null
    };
    location
}

/// The `categories` and `warnings` arrays for one locale, as JSON. `own` is the
/// venue's own language, which costs no translation lookup; a missing
/// translation falls back to `second` (the customer's `ru -> en`), then to the
/// venue's string, never to an empty one.
pub fn render(
    cats: &[(String, String, i64)],
    products: &[(String, Value)],
    i18n: &Result<Vec<(String, String)>, String>,
    want: &str,
    second: Option<&str>,
    own: bool,
) -> (String, String) {
    let mut ranked: std::collections::HashMap<(String, String), (u8, String)> = std::collections::HashMap::new();
    let mut warnings: Vec<String> = Vec::new();
    if !own {
        match i18n {
            Ok(all) => {
                for (key, value) in all {
                    crate::storefront::i18n_keep(&mut ranked, key, value.clone(), want, second);
                }
            }
            // Not "no translations exist": the venue's words are still a menu, and it says so.
            Err(e) => warnings.push(format!("translations unavailable: {e}")),
        }
    }
    let words: std::collections::HashMap<(&str, &str), &str> =
        ranked.iter().map(|((id, f), (_, v))| ((id.as_str(), f.as_str()), v.as_str())).collect();
    let said = |id: &str, field: &str, fallback: Value| words.get(&(id, field)).map_or(fallback, |v| json!(v));
    let said_list = |id: &str, field: &str, fallback: Value| {
        match words.get(&(id, field)).and_then(|v| serde_json::from_str::<Value>(v).ok()) {
            Some(Value::Array(a)) if a.iter().all(Value::is_string) => Value::Array(a),
            _ => fallback,
        }
    };
    let field = |p: &Value, k: &str| p.get(k).cloned().unwrap_or(Value::Null);
    let mut out: Vec<Value> = Vec::new();
    for (cid, cname, csort) in cats {
        let mut items: Vec<(i64, Value)> = products
            .iter()
            .filter(|(_, p)| p.get("categoryId").and_then(Value::as_str) == Some(cid.as_str()))
            .map(|(id, p)| {
                let sort = p.get("sortOrder").and_then(Value::as_i64).unwrap_or(0);
                let mut item = json!({
                    "id": id,
                    "name": said(id, "name", p.get("name").cloned().unwrap_or(json!(""))),
                    "description": said(id, "description", field(p, "description")),
                    "price": p.get("price").cloned().unwrap_or(json!(0)),
                    "available": p.get("available").and_then(Value::as_bool).unwrap_or(true),
                    "ingredients": said_list(id, "ingredients", field(p, "ingredients")),
                    "sortOrder": p.get("sortOrder").cloned().unwrap_or(json!(0)),
                });
                // Passed through AS STORED, null included: null and [] are different claims.
                for k in ["unavailableNote", "imageUrl", "imageUrlSmall", "allergens", "modifierGroups", "sizeCm",
                    "cookingMin", "tags", "weightG", "nutrition", "nutritionDerived", "taste", "sense", "station", "calories"]
                {
                    item[k] = field(p, k);
                }
                (sort, item)
            })
            .collect();
        items.sort_by_key(|(sort, _)| *sort);
        if items.is_empty() {
            continue;
        }
        out.push(json!({
            "id": cid, "name": said(cid, "name", json!(cname)), "sortOrder": csort,
            "products": items.into_iter().map(|(_, p)| p).collect::<Vec<_>>()
        }));
    }
    (Value::Array(out).to_string(), json!(warnings).to_string())
}
