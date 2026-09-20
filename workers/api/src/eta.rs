//! Delivery estimate — the transport for `dowiz_kernel::eta`.
//!
//! **This module decides nothing.** How long a basket takes, how a queue is
//! spread across stations, how far a journey is — every one of those is the
//! kernel's answer, computed in integer minutes with no clock and no float.
//!
//! What is here is the venue's own settings and the state it already has: the
//! dishes' cooking times from the catalogue, the orders already on the pass
//! from the hub, and the customer's coordinates from the request.
//!
//! # Why the estimate moved out of a string
//!
//! A venue used to publish one range and every screen printed it: the same
//! answer for a coffee two streets away and a banquet across town with six
//! orders ahead. This endpoint answers the question that was actually being
//! asked — how long will MY order take, from HERE, right now.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use dowiz_kernel::eta::{self, BasketItem, KitchenProfile, QueuedOrder};

#[derive(Deserialize)]
struct Line {
    #[serde(default, rename = "cookingMin")]
    cooking_min: Option<u16>,
    /// HOW MANY, when the caller says. `null` means the same as absent.
    ///
    /// This was a bare `u16` with a serde default, and a serde default applies
    /// only to a MISSING key -- an explicit `null` was a type error, so the
    /// whole request came back `400 bad request: invalid type: unit value,
    /// expected u16`. A delivery estimate is the one answer that should degrade
    /// rather than refuse: the customer loses the minutes, not the basket. Any
    /// client that stringifies an unset quantity sends that null, and no client
    /// should have to know that this field distinguishes the two.
    #[serde(default)]
    quantity: Option<u16>,
    /// Optional: the product id, so the venue's own cooking time is used when
    /// the caller does not carry one.
    #[serde(default)]
    id: Option<String>,
}

#[derive(Deserialize)]
struct EtaBody {
    items: Vec<Line>,
    /// Straight to metres when the caller has a routed distance. Preferred over
    /// coordinates: a road is longer than a line and the kernel does not pretend
    /// otherwise.
    #[serde(default, rename = "distanceM")]
    distance_m: Option<u32>,
    #[serde(default, rename = "latUdeg")]
    lat_udeg: Option<i32>,
    #[serde(default, rename = "lonUdeg")]
    lon_udeg: Option<i32>,
    #[serde(default)]
    pickup: bool,
}

/// Read a venue's kitchen settings off its location record, falling back to the
/// kernel's defaults field by field — so a venue that has set only its courier
/// speed keeps every other default.
pub(crate) fn profile_of(loc: &Value) -> KitchenProfile {
    let d = KitchenProfile::default_profile();
    let u16f = |k: &str, fallback: u16| {
        loc.get(k).and_then(|v| v.as_u64()).map(|n| n as u16).unwrap_or(fallback)
    };
    KitchenProfile {
        stations: u16f("kitchenStations", d.stations).max(1),
        default_cooking_min: u16f("defaultCookingMin", d.default_cooking_min),
        per_extra_portion_min: u16f("perExtraPortionMin", d.per_extra_portion_min),
        pickup_min: u16f("pickupMin", d.pickup_min),
        handover_min: u16f("handoverMin", d.handover_min),
        courier_speed_m_per_min: u16f("courierSpeedMPerMin", d.courier_speed_m_per_min).max(1),
        spread_pct: u16f("etaSpreadPct", d.spread_pct),
    }
}

/// `POST /api/public/locations/:slug/eta`
pub async fn quote(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let Some(slug) = ctx.param("slug").cloned() else {
        return Response::error("missing slug", 400);
    };
    let body: EtaBody = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request: {e}"), 400),
    };
    if body.items.is_empty() {
        return Response::error("an empty basket has no estimate", 400);
    }

    let place = crate::hubstore::Place::of_slug(&ctx, &slug).await?;
    let loaded = crate::hubstore::load_catalog(&place).await?;
    let Some(loc_json) = loaded.catalog.location() else {
        return Response::error("not found", 404);
    };
    let loc: Value = serde_json::from_str(&loc_json).unwrap_or(json!({}));
    let k = profile_of(&loc);

    // Each line's cooking time: the caller's, else the venue's own for that
    // dish, else zero — which the kernel reads as "the venue has not said".
    let mut items = Vec::with_capacity(body.items.len());
    for line in &body.items {
        let from_catalogue = line.id.as_deref().and_then(|id| {
            loaded
                .catalog
                .product(id)
                .and_then(|pj| serde_json::from_str::<Value>(&pj).ok())
                .and_then(|p| p.get("cookingMin").and_then(|v| v.as_u64()))
                .map(|n| n as u16)
        });
        items.push(BasketItem {
            cooking_min: line.cooking_min.or(from_catalogue).unwrap_or(0),
            quantity: line.quantity.unwrap_or(1),
        });
    }

    // ── THE QUEUE ──
    //
    // Orders the hub has accepted and that are still IN THE KITCHEN. `READY`
    // and `IN_DELIVERY` have left it and hold nobody up, which is why they are
    // not counted: a courier on the road is not occupying a station.
    //
    // Each queued order's remaining preparation is the same sum the kernel
    // computes for a new basket — its own dishes' cooking times. When an order
    // carries no items the venue's default stands in, and the response says how
    // many orders were counted so a venue can see the queue it is being quoted
    // against.
    let hub = crate::hubstore::load(&place).await?;
    let mut ahead: Vec<QueuedOrder> = Vec::new();
    for e in crate::hubstore::orders_state(&hub.hub) {
        let Ok(v) = serde_json::from_str::<Value>(&e.order_json) else { continue };
        if v.get("location_id").and_then(|x| x.as_str()) != Some(place.venue.as_str()) {
            continue;
        }
        match v.get("status").and_then(|x| x.as_str()).unwrap_or("") {
            "CONFIRMED" | "PREPARING" => {}
            _ => continue,
        }
        let lines: Vec<BasketItem> = v
            .get("items")
            .and_then(|x| x.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|it| BasketItem {
                        cooking_min: it
                            .get("cookingMin")
                            .and_then(|n| n.as_u64())
                            .unwrap_or(0) as u16,
                        quantity: it.get("quantity").and_then(|n| n.as_u64()).unwrap_or(1) as u16,
                    })
                    .collect()
            })
            .unwrap_or_default();
        let remaining = if lines.is_empty() {
            k.default_cooking_min as u32
        } else {
            eta::prep_minutes(&lines, &k).unwrap_or(k.default_cooking_min as u32)
        };
        ahead.push(QueuedOrder {
            remaining_min: remaining.min(u16::MAX as u32) as u16,
        });
    }

    let distance_m = match (body.distance_m, body.lat_udeg, body.lon_udeg) {
        (Some(m), _, _) => m,
        (None, Some(lat), Some(lon)) => {
            let vlat = loc.get("latUdeg").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let vlon = loc.get("lonUdeg").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            if vlat == 0 && vlon == 0 {
                // The venue has no coordinates, so there is no distance to
                // compute. Saying so beats quoting a journey of zero.
                return Response::error(
                    "this venue has not set its own location, so a distance cannot be measured",
                    409,
                );
            }
            eta::straight_line_m(vlat, vlon, lat, lon)
        }
        _ => 0,
    };

    let answer = if body.pickup {
        eta::estimate_pickup(&items, &ahead, &k)
    } else {
        eta::estimate(&items, &ahead, distance_m, &k)
    };

    match answer {
        Ok(e) => Response::from_json(&json!({
            "range": e.range(),
            "lowMin": e.low_min,
            "highMin": e.high_min,
            // The parts, so a venue can see WHY an estimate moved rather than
            // only that it did.
            "parts": {
                "prepMin": e.prep_min,
                "queueMin": e.queue_min,
                "travelMin": e.travel_min,
                "overheadMin": e.overhead_min,
            },
            "distanceM": distance_m,
            "ordersAhead": ahead.len(),
        })),
        Err(err) => Response::error(err.message(), 422),
    }
}
