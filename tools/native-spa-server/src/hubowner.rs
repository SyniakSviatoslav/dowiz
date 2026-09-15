//! The owner's surface, against the hub's own log and catalogue.
//!
//! These paths existed in the front-end before the hub did. They used to reach
//! the cap-gated API, which answers 401 forever behind an empty AnchorRoster,
//! while the orders they wanted lived here. The admin pane is unchanged; what
//! changed is that there is now something behind it.
//!
//! THE KERNEL STILL DECIDES. An action names a destination — the kernel's own
//! `OwnerOrderAction::target` says which — and `apply_event_logic` decides
//! whether that edge is legal. There is no ordered list of statuses in this
//! file and no map from action to status: both live in the kernel, and a copy
//! here would be a second authority that eventually disagrees.
//!
//! `location_id` ARRIVES IN EVERY BODY and is ignored. One hub is one venue, so
//! it identifies nothing; it is accepted so the same front-end works against a
//! hub and against a multi-tenant deployment without a build flag.

use axum::extract::{Path as AxPath, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use dowiz_core::order_machine::OrderStatus;
use dowiz_core::ports::owner_surface::OwnerOrderAction;
use dowiz_hub::import;
use dowiz_hub::palette::{self, Rgb, Theme};
use dowiz_hub::EventKind;
use dowiz_kernel::json_api;

use crate::hub::{now_ms, HubHttpError, Shared};
use crate::hubauth::OwnerCaller;

/// Statuses an order passes through while it is still the venue's problem.
/// Derived from the kernel's own terminal check rather than listed here.
fn is_live(status: &str) -> bool {
    OrderStatus::from_str(status).is_some_and(|s| {
        !matches!(
            s,
            OrderStatus::Delivered
                | OrderStatus::PickedUp
                | OrderStatus::Rejected
                | OrderStatus::Cancelled
        )
    })
}

/// `GET /api/owner/orders` — the queue, newest first.
pub async fn orders(
    State(st): State<Shared>,
    _who: OwnerCaller,
) -> Result<Json<Value>, HubHttpError> {
    let hub = st.read_log()?;
    let mut out: Vec<Value> = hub
        .orders()
        .into_iter()
        .filter_map(|e| serde_json::from_str::<Value>(&e.order_json).ok())
        .collect();
    // Newest first: the owner is looking at what just came in, not at history.
    out.sort_by(|a, b| {
        b.get("created_at_ms")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .cmp(&a.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0))
    });
    Ok(Json(json!({ "orders": out })))
}

#[derive(Deserialize)]
pub struct ActionIn {
    pub action: String,
    #[serde(default)]
    pub reason: Option<String>,
    /// Accepted and ignored — see the module header.
    #[serde(default, rename = "location_id")]
    pub _location_id: Option<String>,
}

/// `POST /api/owner/orders/{id}/action`.
pub async fn order_action(
    State(st): State<Shared>,
    who: OwnerCaller,
    AxPath(id): AxPath<String>,
    Json(body): Json<ActionIn>,
) -> Result<Json<Value>, HubHttpError> {
    let action = OwnerOrderAction::from_verb(&body.action)
        .ok_or_else(|| HubHttpError::Invalid(format!("unknown action {:?}", body.action)))?;
    // A rejection the customer is shown must say why. The kernel makes `reason`
    // part of the signed bytes for exactly this reason; here it is at least
    // required rather than optional.
    if action == OwnerOrderAction::Reject
        && body.reason.as_deref().map_or(true, |r| r.trim().is_empty())
    {
        return Err(HubHttpError::Invalid("a rejection must carry a reason".into()));
    }

    let reason = body.reason.clone();
    let actor = who.0.person.id.clone();
    let notify_id = id.clone();
    let out = st
        .with_log(move |hub| {
            let current = hub.order(&id).map_err(|_| HubHttpError::NotFound("order"))?;
            let cur: Value = serde_json::from_str(&current).unwrap_or(json!({}));
            let from = cur
                .get("status")
                .and_then(Value::as_str)
                .and_then(OrderStatus::from_str)
                .ok_or(HubHttpError::Corrupt("order status"))?;
            // The KERNEL names the destination. This handler does not know the
            // order of statuses and must not learn it.
            let target = action
                .target(from)
                .ok_or_else(|| HubHttpError::Refused(format!(
                    "{} cannot apply to an order that is {}",
                    action.verb(),
                    from.as_str()
                )))?;

            let updated = json_api::apply_event_logic(&current, target.as_str())
                .map_err(HubHttpError::Refused)?;
            let mut merged: Value = serde_json::from_str(&updated)
                .map_err(|_| HubHttpError::Corrupt("kernel order"))?;
            crate::hub::carry_over(&cur, &mut merged);
            if let Some(r) = reason {
                merged["rejection_reason"] = json!(r);
            }
            // Who did it. An order that changed state with nobody's name on it
            // is unanswerable the next morning.
            merged["last_actor"] = json!(actor);
            let body = serde_json::to_string(&merged).unwrap_or(updated);
            hub.append(EventKind::Advanced, &id, &body, now_ms() as u64, [0u8; 32])
                .map_err(|e| HubHttpError::Io(format!("{e:?}")))?;
            Ok(merged)
        })
        .await?;

    st.notify_advanced(&notify_id, &out);
    Ok(Json(out))
}

/// `GET /api/owner/dashboard` — the four numbers on the top of the pane.
///
/// Computed from the log on every call rather than kept as counters. At one
/// venue the log is small and a derived number cannot drift from the events;
/// a maintained counter can, and does, exactly when something crashes midway.
pub async fn dashboard(
    State(st): State<Shared>,
    _who: OwnerCaller,
) -> Result<Json<Value>, HubHttpError> {
    let hub = st.read_log()?;
    let orders = hub.orders();

    let day_start = start_of_day_ms(now_ms());
    let (mut today, mut pending, mut active, mut revenue) = (0i64, 0i64, 0i64, 0i64);
    for ev in &orders {
        let Ok(o) = serde_json::from_str::<Value>(&ev.order_json) else { continue };
        let status = o.get("status").and_then(Value::as_str).unwrap_or("");
        let created = o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0);
        if status == OrderStatus::Pending.as_str() {
            pending += 1;
        }
        if is_live(status) {
            active += 1;
        }
        if created >= day_start {
            today += 1;
            // Revenue counts orders that were not refused. A rejected order is
            // not money the venue took, and counting it would overstate the day
            // every time the kitchen turned something down.
            if !matches!(status, "REJECTED" | "CANCELLED") {
                revenue += o.get("total").and_then(Value::as_i64).unwrap_or(0);
            }
        }
    }
    Ok(Json(json!({
        "todayOrders": today,
        "pending": pending,
        "active": active,
        "todayRevenue": revenue
    })))
}

/// Midnight, local to the venue.
///
/// `TZ_OFFSET_MINUTES` because "today" is the venue's day, not UTC's. Durrës is
/// UTC+1/+2, so a UTC day boundary would reset the owner's takings at one or two
/// in the morning — during service on a Saturday.
fn start_of_day_ms(now: i64) -> i64 {
    let offset_min: i64 = std::env::var("TZ_OFFSET_MINUTES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(120);
    let day = 24 * 60 * 60 * 1000;
    let local = now + offset_min * 60 * 1000;
    local - local.rem_euclid(day) - offset_min * 60 * 1000
}

#[derive(Deserialize)]
pub struct ProductIn {
    #[serde(default)]
    pub available: Option<bool>,
    #[serde(default)]
    pub unavailable_note: Option<String>,
    #[serde(default)]
    pub price: Option<i64>,
    #[serde(default, rename = "location_id")]
    pub _location_id: Option<String>,
}

/// `POST /api/owner/products/{id}` — stop-list a dish or change its price.
pub async fn update_product(
    State(st): State<Shared>,
    _who: OwnerCaller,
    AxPath(id): AxPath<String>,
    Json(body): Json<ProductIn>,
) -> Result<Json<Value>, HubHttpError> {
    if let Some(p) = body.price {
        // Integer minor units, and a negative price is not a discount, it is a
        // typo that would make the kernel's ledger owe the customer money.
        if p < 0 {
            return Err(HubHttpError::Invalid("price cannot be negative".into()));
        }
    }
    st.with_catalog(move |cat| {
        let raw = cat.product(&id).ok_or(HubHttpError::NotFound("product"))?;
        let mut p: Value =
            serde_json::from_str(&raw).map_err(|_| HubHttpError::Corrupt("catalogue product"))?;
        if let Some(a) = body.available {
            p["available"] = json!(a);
            // The note only makes sense while the dish is off. Leaving a stale
            // "none today" on a dish that is back is a lie the customer reads.
            if a {
                p["unavailableNote"] = Value::Null;
            }
        }
        if let Some(n) = body.unavailable_note {
            p["unavailableNote"] = if n.trim().is_empty() { Value::Null } else { json!(n) };
        }
        if let Some(price) = body.price {
            p["price"] = json!(price);
        }
        cat.set_product(&id, &serde_json::to_string(&p).unwrap_or(raw));
        Ok(Json(p))
    })
    .await
}

#[derive(Deserialize)]
pub struct LocationIn {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub delivery_paused: Option<bool>,
    #[serde(default, rename = "location_id")]
    pub _location_id: Option<String>,
}

/// `POST /api/owner/location` — open or close the venue.
pub async fn update_location(
    State(st): State<Shared>,
    _who: OwnerCaller,
    Json(body): Json<LocationIn>,
) -> Result<Json<Value>, HubHttpError> {
    if let Some(s) = &body.status {
        if !matches!(s.as_str(), "open" | "closed") {
            return Err(HubHttpError::Invalid(format!("unknown status {s:?}")));
        }
    }
    st.with_catalog(move |cat| {
        let raw = cat.location().ok_or(HubHttpError::NotFound("venue"))?;
        let mut loc: Value =
            serde_json::from_str(&raw).map_err(|_| HubHttpError::Corrupt("catalogue venue"))?;
        if let Some(s) = body.status {
            loc["status"] = json!(s);
        }
        if let Some(p) = body.delivery_paused {
            loc["delivery_paused"] = json!(if p { 1 } else { 0 });
        }
        cat.set_location(&serde_json::to_string(&loc).unwrap_or(raw));
        Ok(Json(loc))
    })
    .await
}

/// `GET /api/owner/couriers` — who can be assigned, and who is on shift.
pub async fn couriers(
    State(st): State<Shared>,
    _who: OwnerCaller,
) -> Result<Json<Value>, HubHttpError> {
    let roster = st.read_roster()?;
    let shifts = st.shifts().await;
    let list: Vec<Value> = roster
        .couriers()
        .into_iter()
        .map(|c| {
            json!({
                "id": c.id,
                "name": c.name,
                "active": c.active,
                "onShift": shifts.contains(&c.id),
            })
        })
        .collect();
    Ok(Json(json!({ "couriers": list })))
}

#[derive(Deserialize)]
pub struct AssignIn {
    pub courier_id: String,
}

/// `POST /api/owner/orders/{id}/assign` — hand an order to a courier.
///
/// The gap this closes: until now nothing could put a `courier_id` on an order,
/// so the courier app's task list was empty by construction no matter who was
/// logged into it.
pub async fn assign(
    State(st): State<Shared>,
    _who: OwnerCaller,
    AxPath(id): AxPath<String>,
    Json(body): Json<AssignIn>,
) -> Result<Json<Value>, HubHttpError> {
    let roster = st.read_roster()?;
    match roster.person(&body.courier_id) {
        Some(p) if p.role == dowiz_hub::token::Role::Courier && p.active => {}
        // Assigning to someone who is not an active courier would produce an
        // order that no app will ever show, which looks exactly like a lost
        // order to the person waiting for it.
        _ => return Err(HubHttpError::Invalid("not an active courier".into())),
    }

    let courier = body.courier_id.clone();
    st.with_log(move |hub| {
        let current = hub.order(&id).map_err(|_| HubHttpError::NotFound("order"))?;
        let mut o: Value =
            serde_json::from_str(&current).map_err(|_| HubHttpError::Corrupt("order"))?;
        let status = o.get("status").and_then(Value::as_str).unwrap_or("");
        if !is_live(status) {
            return Err(HubHttpError::Refused(format!(
                "order is {status} and no longer needs a courier"
            )));
        }
        o["courier_id"] = json!(courier);
        let body = serde_json::to_string(&o).unwrap_or(current);
        hub.append(EventKind::Advanced, &id, &body, now_ms() as u64, [0u8; 32])
            .map_err(|e| HubHttpError::Io(format!("{e:?}")))?;
        Ok(Json(o))
    })
    .await
}

#[derive(Deserialize, Default)]
pub struct ImportQuery {
    /// Parse and report, change nothing. The DEFAULT, because an import that
    /// applies on the first click is one the owner cannot inspect first, and a
    /// menu is the thing customers buy from.
    #[serde(default)]
    pub apply: Option<bool>,
    /// Also stop-list dishes the file does not mention.
    ///
    /// NEVER deletes. A deleted product leaves nothing for a past order to
    /// refer to, and the KV layout has no delete anyway; stop-listing takes the
    /// dish off the storefront while keeping it recoverable by re-importing.
    #[serde(default)]
    pub retire_missing: Option<bool>,
}

/// `POST /api/owner/menu/import` — the body is the file itself.
///
/// RAW BODY, not multipart. A spreadsheet export is a text file; wrapping it in
/// multipart would add a parser for no gain. The admin pane reads the file with
/// `FileReader` and posts its text.
pub async fn import_menu(
    State(st): State<Shared>,
    _who: OwnerCaller,
    Query(q): Query<ImportQuery>,
    text: String,
) -> Result<Json<Value>, HubHttpError> {
    if text.trim().is_empty() {
        return Err(HubHttpError::Invalid("the file is empty".into()));
    }
    // A generous ceiling that still refuses a file nobody meant to send. A
    // thousand-dish menu is well under this.
    if text.len() > 4 * 1024 * 1024 {
        return Err(HubHttpError::Invalid("file too large".into()));
    }

    let draft = import::from_csv(&text);
    let apply = q.apply.unwrap_or(false);
    let retire = q.retire_missing.unwrap_or(false);

    // What is on the menu now but not in the file. Reported either way, so the
    // owner can see the consequence before choosing to act on it.
    let existing: Vec<(String, String)> = st
        .read_catalog()?
        .products()
        .into_iter()
        .filter_map(|(id, j)| {
            let v: Value = serde_json::from_str(&j).ok()?;
            Some((id, v.get("name")?.as_str()?.to_string()))
        })
        .collect();
    let missing: Vec<Value> = existing
        .iter()
        .filter(|(id, _)| !draft.products.iter().any(|p| &p.id == id))
        .map(|(id, name)| json!({ "id": id, "name": name }))
        .collect();

    let summary = json!({
        "applied": apply,
        "categories": draft.categories.len(),
        "products": draft.products.len(),
        "warnings": draft.warnings,
        "notInFile": missing,
        "retired": if apply && retire { missing.len() } else { 0 },
    });

    if !apply {
        // The preview carries the parsed menu so the owner can read the prices
        // back before committing to them.
        let mut out = summary;
        out["draft"] = serde_json::from_str(&draft.as_json()).unwrap_or(Value::Null);
        return Ok(Json(out));
    }

    // REFUSE to apply a file that produced nothing. Applying an empty draft
    // would wipe a working menu because of a wrong separator or a missing
    // header -- the exact failure the parser warns about.
    if draft.products.is_empty() {
        return Err(HubHttpError::Invalid(format!(
            "nothing to import: {}",
            draft.warnings.join("; ")
        )));
    }

    st.with_catalog(move |cat| {
        for c in &draft.categories {
            cat.set_category(
                &c.id,
                &json!({ "id": c.id, "name": c.name, "sortOrder": c.sort_order }).to_string(),
            );
        }
        for p in &draft.products {
            // An EXISTING dish keeps its image. The file has no image column,
            // and re-importing a price list must not blank every photo the
            // owner uploaded.
            let image = cat
                .product(&p.id)
                .and_then(|j| serde_json::from_str::<Value>(&j).ok())
                .and_then(|v| v.get("imageUrl").cloned())
                .unwrap_or(Value::Null);
            cat.set_product(
                &p.id,
                &json!({
                    "id": p.id, "categoryId": p.category_id, "name": p.name,
                    "description": p.description, "price": p.price,
                    "available": p.available, "sortOrder": p.sort_order,
                    "imageUrl": image
                })
                .to_string(),
            );
        }
        if retire {
            for m in &missing {
                let Some(id) = m.get("id").and_then(Value::as_str) else { continue };
                let Some(raw) = cat.product(id) else { continue };
                let Ok(mut v) = serde_json::from_str::<Value>(&raw) else { continue };
                v["available"] = json!(false);
                v["unavailableNote"] = json!("not on the current menu");
                cat.set_product(id, &v.to_string());
            }
        }
        Ok(())
    })
    .await?;

    Ok(Json(summary))
}

#[derive(Deserialize)]
pub struct PixelsIn {
    /// The image as flat hex, two characters per channel, no separators.
    ///
    /// The ADMIN PANE decodes the image. There is no PNG or JPEG decoder on
    /// this side -- they are outside the zero-dep allowlist, and the browser
    /// already has excellent ones. It draws the file to a canvas, downsamples,
    /// and sends the pixels; the server does the colour mathematics and, more
    /// importantly, the contrast enforcement that a client could otherwise skip.
    pub pixels: String,
}

fn parse_pixels(hex: &str) -> Result<Vec<Rgb>, HubHttpError> {
    let hex = hex.trim();
    if hex.len() % 6 != 0 || hex.is_empty() {
        return Err(HubHttpError::Invalid("pixels must be whole rgb triples in hex".into()));
    }
    // 64x64 is what the client is asked to send; this allows a good deal more
    // and refuses a payload nobody meant to upload.
    if hex.len() > 6 * 65_536 {
        return Err(HubHttpError::Invalid("too many pixels; downsample first".into()));
    }
    let bytes = dowiz_hub::crypto::unhex(hex)
        .ok_or_else(|| HubHttpError::Invalid("pixels are not hex".into()))?;
    Ok(bytes.chunks_exact(3).map(|c| Rgb::new(c[0], c[1], c[2])).collect())
}

fn theme_json(t: &Theme) -> Value {
    json!({
        "light": t.as_css_tokens(),
        "dark": t.as_dark_css_tokens(),
        "primary": t.primary.hex(),
        "primaryAdjustedPct": t.primary_adjusted_pct,
        "contrast": t.contrast_report()
            .into_iter()
            .map(|(what, got, want)| json!({
                "pair": what,
                // Two decimals: a contrast ratio is read, not computed with.
                "ratio": (got * 100.0).round() / 100.0,
                "required": want,
                "passes": got >= want
            }))
            .collect::<Vec<_>>()
    })
}

/// `POST /api/owner/branding/extract` — suggest colours from an image.
///
/// SUGGESTS ONLY. Nothing is applied: the owner picks one of the swatches and
/// posts it back. An upload that silently repainted the storefront would be a
/// change no one approved, made from a photograph.
pub async fn extract_branding(
    _who: OwnerCaller,
    Json(body): Json<PixelsIn>,
) -> Result<Json<Value>, HubHttpError> {
    let pixels = parse_pixels(&body.pixels)?;
    let swatches = palette::dominant(&pixels, 6);
    if swatches.is_empty() {
        // Honest emptiness. Inventing a colour for a black-and-white menu would
        // present a guess as a finding.
        return Ok(Json(json!({
            "swatches": [],
            "note": "no strong colours found in this image"
        })));
    }
    let out: Vec<Value> = swatches
        .iter()
        .map(|s| {
            json!({
                "hex": s.colour.hex(),
                "sharePct": (s.share_bp as f64 / 100.0 * 10.0).round() / 10.0,
                "theme": theme_json(&Theme::from_seed(s.colour))
            })
        })
        .collect();
    Ok(Json(json!({ "swatches": out })))
}

#[derive(Deserialize)]
pub struct BrandingIn {
    /// The seed colour, as the owner chose it.
    pub primary: String,
}

/// `POST /api/owner/branding` — adopt a theme derived from one colour.
///
/// The SEED is stored alongside the derived tokens. Keeping it means the theme
/// can be re-derived if the derivation itself improves, without asking the owner
/// to pick their colour again.
pub async fn set_branding(
    State(st): State<Shared>,
    _who: OwnerCaller,
    Json(body): Json<BrandingIn>,
) -> Result<Json<Value>, HubHttpError> {
    let seed = Rgb::from_hex(&body.primary)
        .ok_or_else(|| HubHttpError::Invalid(format!("{:?} is not a colour", body.primary)))?;
    let theme = Theme::from_seed(seed);
    let payload = theme_json(&theme);

    // The derivation already walks the colour until every pair passes; this is
    // the belt to that braces. A theme that fails here is a bug in the
    // derivation, and shipping it to a storefront would make text unreadable
    // for every customer of this venue.
    if let Some(bad) = theme.contrast_report().into_iter().find(|(_, got, want)| got < want) {
        return Err(HubHttpError::Refused(format!(
            "derived theme fails WCAG: {} is {:.2}:1, needs {}:1",
            bad.0, bad.1, bad.2
        )));
    }

    let stored = json!({ "seed": seed.hex(), "light": payload["light"], "dark": payload["dark"] });
    st.with_catalog(move |cat| {
        let raw = cat.location().ok_or(HubHttpError::NotFound("venue"))?;
        let mut loc: Value =
            serde_json::from_str(&raw).map_err(|_| HubHttpError::Corrupt("catalogue venue"))?;
        loc["theme"] = stored;
        cat.set_location(&serde_json::to_string(&loc).unwrap_or(raw));
        Ok(())
    })
    .await?;
    Ok(Json(payload))
}

pub fn routes(state: Shared) -> Router {
    Router::new()
        .route("/api/owner/orders", get(orders))
        .route("/api/owner/orders/{id}/action", post(order_action))
        .route("/api/owner/orders/{id}/assign", post(assign))
        .route("/api/owner/dashboard", get(dashboard))
        .route("/api/owner/products/{id}", post(update_product))
        .route("/api/owner/location", post(update_location))
        .route("/api/owner/couriers", get(couriers))
        .route("/api/owner/menu/import", post(import_menu))
        .route("/api/owner/branding/extract", post(extract_branding))
        .route("/api/owner/branding", post(set_branding))
        .with_state(state)
}
