//! The order's time, as it stands NOW -- not the quote it was given.
//!
//! A quote at checkout answers "how long will this take" once, from the
//! basket and the distance. This module answers the question every screen
//! asks after that -- "how long is LEFT" -- from where the order actually is
//! in its life and where the courier actually is on the map:
//!
//! * before the venue confirms: the venue's usual time to answer, then the
//!   whole journey;
//! * while it cooks: the kitchen time that remains, measured from the moment
//!   cooking began, overlapped with the courier's ride to the venue;
//! * when it is ready: the courier's ride to the venue, if the courier is
//!   not there yet, then the handover, then the road to the door;
//! * on the road: the courier's last reported position to the door.
//!
//! The kernel still owns every minute: `dowiz_kernel::eta` turns metres into
//! minutes and baskets into kitchen time. This module only decides WHICH
//! legs are still ahead, and reads the clock and the map to do it -- which is
//! why it lives here and not there.
//!
//! Failures are quiet by design: an order without a position, a venue
//! without a location, a courier nobody has heard from in twenty minutes --
//! each drops its leg and says so in `known`, and the number still comes
//! back. A customer waiting is better served by "about 18 min, courier
//! position unknown" than by nothing.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::wasm_bindgen::JsValue;
use worker::*;

use dowiz_kernel::eta::{self, BasketItem, KitchenProfile};

/// A confirmation the venue has not given yet is expected within this long:
/// the console alerts at every new order, and a kitchen in service answers a
/// phone in about this time.
const CONFIRM_WAIT_MIN: u32 = 3;
/// A courier's position older than this is not a position; it is a memory.
pub const POSITION_FRESH_MS: i64 = 20 * 60 * 1000;
/// The far end of the range: a quarter more than the expected, plus a beat.
const RANGE_STRETCH_PCT: u32 = 25;
const RANGE_STRETCH_MIN: u32 = 2;
/// Kitchen time never reads zero while the food is not ready.
const MIN_LEFT_MIN: u32 = 1;
/// Micro-degrees per degree, the precision every coordinate here carries.
const UDEG: f64 = 1_000_000.0;

/// One courier's last known place on the map.
#[derive(Clone, Debug)]
pub struct CourierFix {
    pub courier_id: String,
    pub lat_udeg: i32,
    pub lon_udeg: i32,
    pub recorded_at_ms: i64,
}

#[derive(Deserialize)]
struct FixRow {
    courier_id: String,
    lat_udeg: f64,
    lon_udeg: f64,
    recorded_at_ms: f64,
}

/// The latest fresh position of every courier ON SHIFT at this venue. One
/// query for a whole queue: the console asks for fifty orders at once and
/// must not ask the map fifty times.
pub async fn fixes(db: &D1Database, location_id: &str, now_ms: i64) -> Vec<CourierFix> {
    let since = now_ms - POSITION_FRESH_MS;
    let _ = since;
    fixes_from_d1(db, location_id, now_ms).await
}

/// The object first, D1 second.
///
/// A courier on a socket sends their position into the OBJECT's memory, where
/// it costs no row and no request. A courier whose app is older, or whose
/// socket is down, still POSTs it into `courier_positions`, so both are read
/// and the object wins where it answers: it is the fresher of the two by
/// construction.
pub async fn fixes_at(
    place: &crate::hubstore::Place,
    location_id: &str,
    now_ms: i64,
) -> Vec<CourierFix> {
    let mut out = crate::hubstore::positions(place, now_ms).await.unwrap_or_default();
    let known: Vec<String> = out.iter().map(|f| f.courier_id.clone()).collect();
    for f in fixes_from_d1(&place.db, location_id, now_ms).await {
        if !known.contains(&f.courier_id) {
            out.push(f);
        }
    }
    out
}

async fn fixes_from_d1(db: &D1Database, location_id: &str, now_ms: i64) -> Vec<CourierFix> {
    let since = now_ms - POSITION_FRESH_MS;
    let stmt = db
        .prepare(
            "SELECT p.courier_id, p.lat_udeg, p.lon_udeg, p.recorded_at_ms \
             FROM courier_positions p \
             JOIN (SELECT courier_id, MAX(recorded_at_ms) AS m FROM courier_positions \
                   WHERE recorded_at_ms > ?2 GROUP BY courier_id) l \
               ON l.courier_id = p.courier_id AND l.m = p.recorded_at_ms \
             JOIN courier_shifts s ON s.courier_id = p.courier_id \
               AND s.location_id = ?1 AND s.ended_at_ms IS NULL",
        )
        .bind(&[location_id.into(), JsValue::from_f64(since as f64)]);
    let Ok(stmt) = stmt else { return Vec::new() };
    let rows = match stmt.all().await.and_then(|r| r.results::<FixRow>()) {
        Ok(r) => r,
        Err(e) => {
            console_error!("live_eta: courier positions unreadable: {e}");
            return Vec::new();
        }
    };
    rows.into_iter()
        .map(|r| CourierFix {
            courier_id: r.courier_id,
            lat_udeg: r.lat_udeg as i32,
            lon_udeg: r.lon_udeg as i32,
            recorded_at_ms: r.recorded_at_ms as i64,
        })
        .collect()
}

/// A coordinate off a record that may carry it as micro-degrees or as a float.
fn udeg(v: &Value, udeg_key: &str, float_key: &str) -> Option<i32> {
    if let Some(n) = v.get(udeg_key).and_then(Value::as_i64) {
        return Some(n as i32);
    }
    v.get(float_key).and_then(Value::as_f64).map(|f| (f * UDEG).round() as i32)
}

/// When the order entered `status`, from the stamps every transition leaves.
fn at(order: &Value, status: &str) -> Option<i64> {
    order.pointer(&format!("/at/{status}")).and_then(Value::as_i64)
}

/// Whole minutes elapsed since `ms`, never negative.
fn minutes_since(now_ms: i64, ms: i64) -> u32 {
    ((now_ms - ms).max(0) / 60_000) as u32
}

/// The estimate for one order, or `None` when the order is over.
///
/// `catalog_cooking` gives a product's cooking time; `fixes` is the venue's
/// fresh courier positions; `busy` names couriers already on a delivery, who
/// cannot be the one to come for this order.
pub fn estimate(
    order: &Value,
    loc: &Value,
    k: &KitchenProfile,
    catalog_cooking: &dyn Fn(&str) -> Option<u16>,
    fixes: &[CourierFix],
    busy: &[String],
    now_ms: i64,
) -> Option<Value> {
    let status = order.get("status").and_then(Value::as_str).unwrap_or("");
    if matches!(status, "DELIVERED" | "REJECTED" | "CANCELLED" | "PICKED_UP") {
        return None;
    }
    let pickup = order.pointer("/fulfilment/kind").and_then(Value::as_str) == Some("pickup");
    let created = order.get("created_at_ms").and_then(Value::as_i64).unwrap_or(now_ms);

    // ── the kitchen ──
    let items: Vec<BasketItem> = order
        .get("items")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|it| BasketItem {
                    cooking_min: it
                        .get("cookingMin")
                        .and_then(Value::as_u64)
                        .map(|n| n as u16)
                        .or_else(|| it.get("product_id").and_then(Value::as_str).and_then(catalog_cooking))
                        .unwrap_or(0),
                    quantity: it.get("quantity").and_then(Value::as_u64).unwrap_or(1) as u16,
                })
                .collect()
        })
        .unwrap_or_default();
    let prep = eta::prep_minutes(&items, k).unwrap_or(k.default_cooking_min as u32);
    let cooking_since = at(order, "PREPARING").or_else(|| at(order, "CONFIRMED"));
    let (confirm_wait, prep_left) = match status {
        "PENDING" => (CONFIRM_WAIT_MIN, prep),
        "CONFIRMED" | "PREPARING" => {
            let gone = cooking_since.map(|t| minutes_since(now_ms, t)).unwrap_or(0);
            (0, prep.saturating_sub(gone).max(MIN_LEFT_MIN))
        }
        _ => (0, 0),
    };

    // ── the road ──
    let venue = (udeg(loc, "latUdeg", "lat"), udeg(loc, "lonUdeg", "lng"));
    let door = (
        order.pointer("/fulfilment/address/lat_udeg").and_then(Value::as_i64).map(|n| n as i32),
        order.pointer("/fulfilment/address/lon_udeg").and_then(Value::as_i64).map(|n| n as i32),
    );
    // NO DOOR, NO LIVE ESTIMATE. A delivery placed without a map pin has no
    // road leg to compute; an estimate that silently dropped it read "18–24"
    // against a quoted "35–45", and the storefront preferred it. The quote
    // made at checkout stays the number until a courier's fix says otherwise.
    if !pickup && door.0.is_none() {
        return None;
    }
    let mut known = json!({ "venue": venue.0.is_some(), "door": door.0.is_some(), "courier": false });
    let assigned = order.get("courier_id").and_then(Value::as_str);
    let courier_fix = match assigned {
        Some(id) => fixes.iter().find(|f| f.courier_id == id).cloned(),
        None => {
            // The nearest courier who is on shift, fresh, and not on a run.
            let mut free: Vec<&CourierFix> = fixes.iter().filter(|f| !busy.iter().any(|b| b == &f.courier_id)).collect();
            if let (Some(vlat), Some(vlon)) = venue {
                free.sort_by_key(|f| eta::straight_line_m(vlat, vlon, f.lat_udeg, f.lon_udeg));
            }
            free.first().cloned().cloned()
        }
    };
    known["courier"] = json!(courier_fix.is_some());

    let mut to_venue = 0u32;
    let mut to_door = 0u32;
    // Every arm below sets it; the compiler holds that promise.
    let overhead: u32;
    if pickup {
        overhead = k.pickup_min as u32;
    } else {
        let venue_to_door = match (venue, door) {
            ((Some(vlat), Some(vlon)), (Some(dlat), Some(dlon))) => {
                eta::travel_minutes(eta::straight_line_m(vlat, vlon, dlat, dlon), k).unwrap_or(0)
            }
            _ => 0,
        };
        match status {
            "IN_DELIVERY" => {
                // From where the courier is to the door; failing a fix, the road
                // from the venue less the time already on it.
                to_door = match (&courier_fix, door) {
                    (Some(f), (Some(dlat), Some(dlon))) => {
                        eta::travel_minutes(eta::straight_line_m(f.lat_udeg, f.lon_udeg, dlat, dlon), k).unwrap_or(0)
                    }
                    _ => {
                        let gone = at(order, "IN_DELIVERY").map(|t| minutes_since(now_ms, t)).unwrap_or(0);
                        venue_to_door.saturating_sub(gone)
                    }
                }
                .max(MIN_LEFT_MIN);
                overhead = k.handover_min as u32;
            }
            _ => {
                to_venue = match (&courier_fix, venue) {
                    (Some(f), (Some(vlat), Some(vlon))) => {
                        eta::travel_minutes(eta::straight_line_m(f.lat_udeg, f.lon_udeg, vlat, vlon), k).unwrap_or(0)
                    }
                    _ => 0,
                };
                to_door = venue_to_door;
                overhead = k.pickup_min as u32 + k.handover_min as u32;
            }
        }
    }
    // The courier's ride to the venue overlaps the cooking: whichever ends
    // later is when the food leaves.
    let wait_for_food = if matches!(status, "IN_DELIVERY") { 0 } else { prep_left.max(to_venue) };
    let expected = confirm_wait + wait_for_food + to_door + overhead;
    let high = expected + expected * RANGE_STRETCH_PCT / 100 + RANGE_STRETCH_MIN;
    Some(json!({
        "minMin": expected,
        "maxMin": high,
        "range": format!("{expected}–{high}"),
        "readyAtMs": now_ms + (confirm_wait + wait_for_food) as i64 * 60_000,
        "arriveAtMs": now_ms + expected as i64 * 60_000,
        "parts": {
            "confirmMin": confirm_wait, "prepLeftMin": prep_left, "courierToVenueMin": to_venue,
            "toDoorMin": to_door, "overheadMin": overhead,
        },
        "known": known,
        "courierId": courier_fix.as_ref().map(|f| f.courier_id.clone()),
        "courierAt": courier_fix.as_ref().map(|f| json!({ "latUdeg": f.lat_udeg, "lonUdeg": f.lon_udeg, "recordedAtMs": f.recorded_at_ms })),
        "sinceCreatedMin": minutes_since(now_ms, created),
        "asOfMs": now_ms,
    }))
}

/// Attach a live estimate to every order in `orders`, in place, with one
/// read of the map. Orders that are over get none.
pub async fn attach_all(
    db: &D1Database,
    place: &crate::hubstore::Place,
    loaded: &crate::hubstore::LoadedCatalog,
    orders: &mut [Value],
    now_ms: i64,
) {
    let Some(loc_json) = loaded.catalog.location() else { return };
    let loc: Value = serde_json::from_str(&loc_json).unwrap_or(json!({}));
    let k = crate::eta::profile_of(&loc);
    let fixes = fixes_at(place, &place.venue, now_ms).await;
    let busy: Vec<String> = orders
        .iter()
        .filter(|o| o.get("status").and_then(Value::as_str) == Some("IN_DELIVERY"))
        .filter_map(|o| o.get("courier_id").and_then(Value::as_str).map(String::from))
        .collect();
    let cooking = |id: &str| -> Option<u16> {
        loaded
            .catalog
            .product(id)
            .and_then(|pj| serde_json::from_str::<Value>(&pj).ok())
            .and_then(|p| p.get("cookingMin").and_then(Value::as_u64))
            .map(|n| n as u16)
    };
    for o in orders.iter_mut() {
        if let Some(e) = estimate(o, &loc, &k, &cooking, &fixes, &busy, now_ms) { o["eta"] = e; }
    }
}

/// The estimate for ONE order, for the customer's own read of it.
pub async fn attach_one(
    db: &D1Database,
    place: &crate::hubstore::Place,
    order: &mut Value,
    now_ms: i64,
) {
    let Ok(loaded) = crate::hubstore::load_catalog(place).await else { return };
    // Whether a courier is busy is read from the venue's other live orders --
    // from the object's PROJECTION, which is a folded list of orders rather
    // than the log. This function used to load and fold the whole venue
    // history, on top of the load the caller had already done to find this
    // order; then it shared the caller's image; now neither of them reads an
    // image at all.
    let Ok(others) = crate::hubstore::orders(place).await else { return };
    let busy: Vec<String> = others
        .into_iter()
        .filter_map(|e| serde_json::from_str::<Value>(&e.order_json).ok())
        .filter(|o| o.get("status").and_then(Value::as_str) == Some("IN_DELIVERY"))
        .filter_map(|o| o.get("courier_id").and_then(Value::as_str).map(String::from))
        .collect();
    let Some(loc_json) = loaded.catalog.location() else { return };
    let loc: Value = serde_json::from_str(&loc_json).unwrap_or(json!({}));
    let k = crate::eta::profile_of(&loc);
    let fixes = fixes_at(place, &place.venue, now_ms).await;
    let cooking = |id: &str| -> Option<u16> {
        loaded
            .catalog
            .product(id)
            .and_then(|pj| serde_json::from_str::<Value>(&pj).ok())
            .and_then(|p| p.get("cookingMin").and_then(Value::as_u64))
            .map(|n| n as u16)
    };
    if let Some(mut e) = estimate(order, &loc, &k, &cooking, &fixes, &busy, now_ms) {
        // THE CUSTOMER SEES THE COURIER ONLY WHILE THE COURIER CARRIES THEIR
        // ORDER. Before pickup the fix is a person's whereabouts on someone
        // else's run; the console keeps it, the storefront does not get it.
        if order.get("status").and_then(Value::as_str) != Some("IN_DELIVERY") {
            if let Some(m) = e.as_object_mut() {
                m.remove("courierAt");
                m.remove("courierId");
            }
        }
        order["eta"] = e;
    }
}

/// Every status transition leaves its time on the order, so the estimate can
/// measure from the moment cooking began rather than guess.
pub fn stamp(order: &mut Value, status: &str, now_ms: i64) {
    if !order.get("at").is_some_and(Value::is_object) {
        order["at"] = json!({});
    }
    order["at"][status] = json!(now_ms);
}
