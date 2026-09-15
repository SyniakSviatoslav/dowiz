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
    apply_owner_action(&st, &who.0.person.id, &id, &body.action, body.reason).await.map(Json)
}

/// Move an order, as the owner.
///
/// THE ONE IMPLEMENTATION. The HTTP route above and the MCP tool both call this,
/// so an action taken from a chat client obeys exactly the rules an action taken
/// from the admin pane obeys, lands in the same log, and records the same actor.
/// A parallel path for the machine-facing surface would be a second set of
/// rules, and the less-tested one would be the one a model could reach.
pub async fn apply_owner_action(
    st: &Shared,
    actor: &str,
    id: &str,
    verb: &str,
    reason: Option<String>,
) -> Result<Value, HubHttpError> {
    let action = OwnerOrderAction::from_verb(verb)
        .ok_or_else(|| HubHttpError::Invalid(format!("unknown action {verb:?}")))?;
    // A rejection the customer is shown must say why. The kernel makes `reason`
    // part of the signed bytes for exactly this reason; here it is at least
    // required rather than optional.
    if action == OwnerOrderAction::Reject
        && reason.as_deref().map_or(true, |r| r.trim().is_empty())
    {
        return Err(HubHttpError::Invalid("a rejection must carry a reason".into()));
    }

    let (actor, id) = (actor.to_string(), id.to_string());
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
            let target = action.target(from).ok_or_else(|| {
                HubHttpError::Refused(format!(
                    "{} cannot apply to an order that is {}",
                    action.verb(),
                    from.as_str()
                ))
            })?;

            let updated = json_api::apply_event_logic(&current, target.as_str())
                .map_err(HubHttpError::Refused)?;
            let mut merged: Value = serde_json::from_str(&updated)
                .map_err(|_| HubHttpError::Corrupt("kernel order"))?;
            crate::hub::carry_over(&cur, &mut merged);
            if let Some(r) = reason {
                merged["rejection_reason"] = json!(r);
            }
            // Who did it. An order that changed state with nobody's name on it
            // is unanswerable the next morning -- and "the owner's MCP client"
            // is still the owner.
            merged["last_actor"] = json!(actor);
            let body = serde_json::to_string(&merged).unwrap_or(updated);
            hub.append(EventKind::Advanced, &id, &body, now_ms() as u64, [0u8; 32])
                .map_err(|e| HubHttpError::Io(format!("{e:?}")))?;
            Ok(merged)
        })
        .await?;

    st.notify_advanced(&notify_id, &out);
    Ok(out)
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
    dashboard_facts(&st).await.map(Json)
}

/// The four numbers, shared by the HTTP route and the MCP tool.
pub async fn dashboard_facts(st: &Shared) -> Result<Value, HubHttpError> {
    let hub = st.read_log()?;
    let orders = hub.orders();

    let day_start = start_of_day_ms(now_ms());
    let (mut today, mut pending, mut active, mut revenue) = (0i64, 0i64, 0i64, 0i64);
    let mut scheduled = 0i64;
    for ev in &orders {
        let Ok(o) = serde_json::from_str::<Value>(&ev.order_json) else { continue };
        let status = o.get("status").and_then(Value::as_str).unwrap_or("");
        let created = o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0);
        // A SCHEDULED order is pending, but it is not waiting on the kitchen
        // yet. Counting it in "waiting" would have the owner chasing an order
        // that is not due for three hours, and would make the number that
        // drives the alert sound permanently wrong.
        let due_later = o
            .get("scheduled_for_ms")
            .and_then(Value::as_i64)
            .is_some_and(|t| t > now_ms());
        if status == OrderStatus::Pending.as_str() && !due_later {
            pending += 1;
        }
        if due_later {
            scheduled += 1;
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
    Ok(json!({
        "todayOrders": today,
        "pending": pending,
        "active": active,
        "todayRevenue": revenue,
        "scheduled": scheduled
    }))
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
    /// The dish's real widest dimension, in centimetres.
    ///
    /// What makes the AR view answer a question rather than be a novelty: it is
    /// the number that tells a customer whether the "large" set is actually
    /// large. Optional, and a dish without one simply has no AR button -- a
    /// guessed size would answer the question WRONGLY, which is worse than not
    /// answering it.
    #[serde(default)]
    pub size_cm: Option<i64>,
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
    edit_product(&st, &id, body.available, body.unavailable_note, body.price, body.size_cm)
        .await
        .map(Json)
}

/// Take a dish off the menu or put it back, shared with the MCP tool.
pub async fn set_product_availability(
    st: &Shared,
    id: &str,
    available: bool,
    note: Option<String>,
) -> Result<Value, HubHttpError> {
    edit_product(st, id, Some(available), note, None, None).await
}

async fn edit_product(
    st: &Shared,
    id: &str,
    available: Option<bool>,
    note: Option<String>,
    price: Option<i64>,
    size_cm: Option<i64>,
) -> Result<Value, HubHttpError> {
    if let Some(p) = price {
        // Integer minor units, and a negative price is not a discount, it is a
        // typo that would make the kernel's ledger owe the customer money.
        if p < 0 {
            return Err(HubHttpError::Invalid("price cannot be negative".into()));
        }
    }
    if let Some(cm) = size_cm {
        // A plate is not two millimetres across and not two metres. Both bounds
        // are generous; both refuse a typo that would put a dish the size of a
        // table on someone's table.
        if !(3..=120).contains(&cm) {
            return Err(HubHttpError::Invalid(
                "a dish is between 3 and 120 cm across".into(),
            ));
        }
    }
    let id = id.to_string();
    st.with_catalog(move |cat| {
        let raw = cat.product(&id).ok_or(HubHttpError::NotFound("product"))?;
        let mut p: Value =
            serde_json::from_str(&raw).map_err(|_| HubHttpError::Corrupt("catalogue product"))?;
        if let Some(a) = available {
            p["available"] = json!(a);
            // The note only makes sense while the dish is off. Leaving a stale
            // "none today" on a dish that is back is a lie the customer reads.
            if a {
                p["unavailableNote"] = Value::Null;
            }
        }
        if let Some(n) = note {
            p["unavailableNote"] = if n.trim().is_empty() { Value::Null } else { json!(n) };
        }
        if let Some(price) = price {
            p["price"] = json!(price);
        }
        if let Some(cm) = size_cm {
            p["sizeCm"] = json!(cm);
        }
        cat.set_product(&id, &serde_json::to_string(&p).unwrap_or(raw));
        Ok(p)
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
            // An EXISTING dish keeps its image AND its measured size. The file
            // has neither column, and re-importing a price list must not blank
            // every photo the owner uploaded or every size they measured.
            let existing = cat
                .product(&p.id)
                .and_then(|j| serde_json::from_str::<Value>(&j).ok());
            let image = existing
                .as_ref()
                .and_then(|v| v.get("imageUrl").cloned())
                .unwrap_or(Value::Null);
            let size = existing
                .as_ref()
                .and_then(|v| v.get("sizeCm").cloned())
                .unwrap_or(Value::Null);
            cat.set_product(
                &p.id,
                &json!({
                    "id": p.id, "categoryId": p.category_id, "name": p.name,
                    "description": p.description, "price": p.price,
                    "available": p.available, "sortOrder": p.sort_order,
                    "imageUrl": image, "sizeCm": size
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
        .route("/api/owner/settings", get(settings))
        .route("/api/owner/settings", post(set_setting))
        .route("/api/owner/assist", post(owner_assist))
        .route("/api/owner/zones", post(set_zones))
        .route("/api/owner/products/{id}/image", post(set_product_image))
        .route("/api/owner/products/{id}/image/clear", post(clear_product_image))
        .route("/api/public/reach", get(public_reach))
        .with_state(state)
}

// ── settings and the assistant ───────────────────────────────────────────────

/// `GET /api/owner/settings` — what is configured, with secrets redacted.
///
/// Returns the DECLARATIONS alongside the values, so the settings pane is built
/// from the same list the hub consults. Two lists of settings drift, and the one
/// that drifts is the one the owner reads.
pub async fn settings(
    State(st): State<Shared>,
    _who: OwnerCaller,
) -> Result<Json<Value>, HubHttpError> {
    let s = st.read_settings()?;
    let values: Value = serde_json::from_str(&s.as_json()).unwrap_or(json!({}));
    let known: Vec<Value> = dowiz_hub::settings::KNOWN
        .iter()
        .map(|k| {
            json!({
                "key": k.key, "label": k.label, "hint": k.hint,
                "default": k.default,
                "secret": dowiz_hub::settings::is_secret(k.key)
            })
        })
        .collect();
    Ok(Json(json!({ "values": values, "known": known })))
}

#[derive(Deserialize)]
pub struct SettingIn {
    pub key: String,
    /// An empty value CLEARS the setting, which is how an owner removes a token
    /// they can no longer see.
    pub value: String,
}

/// `POST /api/owner/settings`.
pub async fn set_setting(
    State(st): State<Shared>,
    _who: OwnerCaller,
    Json(body): Json<SettingIn>,
) -> Result<Json<Value>, HubHttpError> {
    // Only declared keys. An open key space would let this become a place to
    // stash arbitrary data, and nothing would ever read it back.
    if !dowiz_hub::settings::KNOWN.iter().any(|k| k.key == body.key) {
        return Err(HubHttpError::Invalid(format!("unknown setting {:?}", body.key)));
    }
    if body.value.len() > 4096 {
        return Err(HubHttpError::Invalid("value too long".into()));
    }
    // An endpoint is checked HERE, when the owner can still fix it, rather than
    // at the first question when they are mid-rush.
    if body.key == "ai.endpoint" && !body.value.trim().is_empty() {
        let u = crate::httpc::parse_url(body.value.trim())
            .map_err(|e| HubHttpError::Invalid(format!("{e}")))?;
        if !u.tls && !crate::httpc::is_local(&u.host) {
            return Err(HubHttpError::Invalid(
                "plain http is only allowed to an address on this machine".into(),
            ));
        }
    }
    let (k, v) = (body.key.clone(), body.value.trim().to_string());
    st.with_settings(move |s| {
        if v.is_empty() {
            s.clear(&k);
        } else {
            s.set(&k, &v);
        }
        Ok(())
    })
    .await?;
    Ok(Json(json!({ "ok": true, "key": body.key })))
}

#[derive(Deserialize)]
pub struct AskIn {
    pub question: String,
}

/// Everything the assistant is allowed to know about the venue right now.
///
/// COMPUTED HERE, by the same code paths the dashboard uses, and handed to the
/// model as fact. The model phrases; it does not count. A model asked to total
/// a day's orders would eventually get one wrong, and the owner would have no
/// way to tell which.
async fn owner_facts(st: &Shared) -> Result<Value, HubHttpError> {
    let hub = st.read_log()?;
    let now = now_ms();
    let shifts = st.shifts().await;
    let mut live: Vec<Value> = Vec::new();
    for ev in hub.orders() {
        let Ok(o) = serde_json::from_str::<Value>(&ev.order_json) else { continue };
        let status = o.get("status").and_then(Value::as_str).unwrap_or("");
        if !is_live(status) {
            continue;
        }
        let created = o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(now);
        live.push(json!({
            "id": o.get("id").cloned().unwrap_or(Value::Null),
            "status": status,
            "total": o.get("total").cloned().unwrap_or(Value::Null),
            "waiting_minutes": (now - created) / 60_000,
            "fulfilment": o.get("fulfilment").and_then(|f| f.get("kind")).cloned().unwrap_or(Value::Null),
            "courier_id": o.get("courier_id").cloned().unwrap_or(Value::Null),
            "contact": o.get("contact").cloned().unwrap_or(Value::Null),
            "address": o.get("fulfilment").and_then(|f| f.get("address")).cloned().unwrap_or(Value::Null),
            "items": o.get("items").cloned().unwrap_or(Value::Null),
        }));
    }
    live.sort_by_key(|o| -o["waiting_minutes"].as_i64().unwrap_or(0));
    Ok(json!({
        "now_iso_minutes_since_epoch": now / 60_000,
        "live_orders": live,
        "couriers_on_shift": shifts,
        "currency": "ALL",
    }))
}

/// Ask, and answer.
///
/// Shared by both surfaces so the redaction rule -- full facts to a model on
/// this machine, PII withheld from anything else -- has ONE implementation. Two
/// would be two chances to leave it out of the second one.
pub(crate) async fn assist_public(
    st: &Shared,
    system: &'static str,
    facts: Value,
    question: &str,
) -> Result<Json<Value>, HubHttpError> {
    let question = question.trim();
    if question.is_empty() {
        return Err(HubHttpError::Invalid("no question".into()));
    }
    if question.chars().count() > 2000 {
        return Err(HubHttpError::Invalid("question too long".into()));
    }

    let settings = st.read_settings()?;
    let assistant = crate::ai::Assistant::from_settings(&settings).map_err(|e| match e {
        crate::ai::AiError::Disabled => HubHttpError::Refused(e.to_string()),
        other => HubHttpError::Invalid(other.to_string()),
    })?;

    let sent = if assistant.local { facts.clone() } else { crate::ai::redact(&facts) };
    let prompt = format!("FACTS:\n{sent}\n\nQUESTION:\n{question}");

    // A local model on modest hardware is slow; a minute is a long wait but a
    // shorter cap would make the local-first default unusable and push venues
    // to a hosted provider, which is the opposite of the intent.
    let answer = assistant
        .ask(system, &prompt, 60_000)
        .await
        .map_err(|e| HubHttpError::Io(e.to_string()))?;

    Ok(Json(json!({
        "answer": answer,
        // The owner is told, every time, whether their customers' details left
        // the machine. Burying this in a settings page would make it a thing
        // they configured once and forgot.
        "local": assistant.local,
        "contextRedacted": !assistant.local,
    })))
}

/// `POST /api/owner/assist`.
pub async fn owner_assist(
    State(st): State<Shared>,
    _who: OwnerCaller,
    Json(body): Json<AskIn>,
) -> Result<Json<Value>, HubHttpError> {
    let facts = owner_facts(&st).await?;
    assist_public(&st, crate::ai::SYSTEM_OWNER, facts, &body.question).await
}

#[derive(Deserialize)]
pub struct ZonesIn {
    /// The service area, as the owner drew it. An EMPTY list removes every
    /// restriction, which is how a venue turns the check off.
    pub zones: Vec<Value>,
}

/// `POST /api/owner/zones` — where this venue delivers.
pub async fn set_zones(
    State(st): State<Shared>,
    _who: OwnerCaller,
    Json(body): Json<ZonesIn>,
) -> Result<Json<Value>, HubHttpError> {
    let raw = serde_json::to_string(&body.zones).unwrap_or_else(|_| "[]".into());
    // PARSED BACK before it is stored. A zone the reader cannot understand is
    // treated as no zone at all -- which ACCEPTS every order -- so a
    // configuration that silently means nothing would quietly turn the check
    // off while the owner believed they had switched it on.
    let parsed = dowiz_hub::zone::from_json(&raw);
    if parsed.len() != body.zones.len() {
        return Err(HubHttpError::Invalid(format!(
            "{} of {} zones could not be read; a circle needs lat, lon and radius_m, \
             a polygon needs at least three points",
            body.zones.len() - parsed.len(),
            body.zones.len()
        )));
    }

    let zones = body.zones.clone();
    st.with_catalog(move |cat| {
        let raw = cat.location().ok_or(HubHttpError::NotFound("venue"))?;
        let mut loc: Value =
            serde_json::from_str(&raw).map_err(|_| HubHttpError::Corrupt("catalogue venue"))?;
        loc["delivery_zones"] = Value::Array(zones);
        cat.set_location(&serde_json::to_string(&loc).unwrap_or(raw));
        Ok(())
    })
    .await?;
    Ok(Json(json!({ "zones": parsed.len() })))
}

#[derive(Deserialize)]
pub struct ReachQuery {
    pub lat_udeg: i64,
    pub lon_udeg: i64,
}

/// `GET /api/public/reach?lat_udeg=&lon_udeg=` — can you be delivered to?
///
/// PUBLIC on purpose. A customer should learn this before filling a basket, not
/// at checkout after choosing thirty euros of food. It reveals only whether the
/// venue serves a point, which is information the venue wants advertised.
pub async fn public_reach(
    State(st): State<Shared>,
    Query(q): Query<ReachQuery>,
) -> Result<Json<Value>, HubHttpError> {
    let loc = st
        .read_catalog()?
        .location()
        .ok_or(HubHttpError::NotFound("venue"))?;
    let loc: Value = serde_json::from_str(&loc).map_err(|_| HubHttpError::Corrupt("venue"))?;
    let zones = loc
        .get("delivery_zones")
        .map(|z| dowiz_hub::zone::from_json(&z.to_string()))
        .unwrap_or_default();
    let out = match dowiz_hub::zone::reach(&zones, Some((q.lat_udeg, q.lon_udeg))) {
        dowiz_hub::zone::Reach::Inside => json!({ "deliverable": true }),
        dowiz_hub::zone::Reach::Unrestricted => json!({ "deliverable": true, "unrestricted": true }),
        dowiz_hub::zone::Reach::Unknown => json!({ "deliverable": true, "unverified": true }),
        dowiz_hub::zone::Reach::Outside { nearest_m } => json!({
            "deliverable": false,
            // Rounded to 100 m: a metre-exact distance implies a precision the
            // flat-earth approximation does not have, and invites an argument
            // about whether someone is 47 or 52 metres outside.
            "nearestMetres": (nearest_m / 100) * 100
        }),
    };
    Ok(Json(out))
}

// ── photographs ──────────────────────────────────────────────────────────────

/// `POST /api/owner/products/{id}/image` — the body is the image itself.
///
/// RAW BODY rather than multipart, for the same reason the menu import takes
/// raw CSV: the admin pane already has the bytes in hand after re-encoding them
/// on a canvas, and a multipart parser would be a dependency and an attack
/// surface added for nothing.
pub async fn set_product_image(
    State(st): State<Shared>,
    _who: OwnerCaller,
    AxPath(id): AxPath<String>,
    body: axum::body::Bytes,
) -> Result<Json<Value>, HubHttpError> {
    // The product must exist BEFORE a file is written, or a typo in an id
    // leaves an orphan blob nothing will ever reference or clean up.
    if st.read_catalog()?.product(&id).is_none() {
        return Err(HubHttpError::NotFound("product"));
    }
    let stored = st.put_media(&body)?;
    let url = stored.url();

    let (id2, url2) = (id.clone(), url.clone());
    st.with_catalog(move |cat| {
        let raw = cat.product(&id2).ok_or(HubHttpError::NotFound("product"))?;
        let mut p: Value =
            serde_json::from_str(&raw).map_err(|_| HubHttpError::Corrupt("catalogue product"))?;
        // The PREVIOUS image is not deleted. Another product may reference the
        // same bytes -- content addressing makes that likely, not rare -- and
        // an order placed an hour ago still names the dish it was sold as.
        // Reclaiming unreferenced blobs is a sweep, not a side effect of an
        // edit.
        p["imageUrl"] = json!(url2);
        cat.set_product(&id2, &serde_json::to_string(&p).unwrap_or(raw));
        Ok(())
    })
    .await?;

    Ok(Json(json!({ "imageUrl": url, "bytes": stored.bytes, "type": stored.kind.mime() })))
}

/// `DELETE`-shaped: `POST /api/owner/products/{id}/image/clear`.
pub async fn clear_product_image(
    State(st): State<Shared>,
    _who: OwnerCaller,
    AxPath(id): AxPath<String>,
) -> Result<Json<Value>, HubHttpError> {
    st.with_catalog(move |cat| {
        let raw = cat.product(&id).ok_or(HubHttpError::NotFound("product"))?;
        let mut p: Value =
            serde_json::from_str(&raw).map_err(|_| HubHttpError::Corrupt("catalogue product"))?;
        p["imageUrl"] = Value::Null;
        cat.set_product(&id, &serde_json::to_string(&p).unwrap_or(raw));
        Ok(Json(json!({ "imageUrl": Value::Null })))
    })
    .await
}
