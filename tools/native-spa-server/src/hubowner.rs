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
///
/// This SAID it was derived from the kernel's terminal check and then listed
/// the statuses by hand anyway -- without `CompensatedRefund`, so a refunded
/// order stayed on the venue's screen for ever.
fn is_live(status: &str) -> bool {
    !crate::ostatus::is_terminal(status)
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

    // ── the stock side of the same intent ──
    //
    // §4's lifecycle coupling: entry into Preparing emits Consumed; every
    // cancellation edge emits Released. Derived from what the LEDGER holds for
    // this order rather than recomputed from the basket -- if the recipe
    // changed between placing and cooking, releasing a recomputed quantity
    // would strand the difference forever.
    //
    // Settled AFTER the order moved, never before: the kernel owns whether the
    // transition is legal at all, and taking ingredients off the shelf for a
    // transition it then refuses would be a loss with no order behind it.
    let settle = match action {
        OwnerOrderAction::MarkPreparing => Some(true),
        OwnerOrderAction::Reject | OwnerOrderAction::Cancel => Some(false),
        _ => None,
    };
    if let Some(consume) = settle {
        let oid = notify_id.clone();
        if let Err(e) = st
            .with_stock(move |log| {
                let led = log.ledger().map_err(|e| HubHttpError::Io(e.to_string()))?;
                let evs = dowiz_hub::stock::settle(&led, &oid, consume);
                if evs.is_empty() {
                    return Ok(());
                }
                log.append_all(&evs).map_err(|e| HubHttpError::Io(e.to_string()))
            })
            .await
        {
            // LOUD, and it does not fail the transition. The order has already
            // moved and the customer has been told; refusing now would leave
            // the order and the ledger disagreeing in the other direction.
            eprintln!("stock: settling {notify_id} ({}) failed: {e:?}", action.verb());
        }
    }

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
            // The tip is the COURIER'S. Counting it here would put money in
            // the venue's takings that the venue never earned and will not
            // keep. `venue_took` is the one definition of both rules.
            revenue += crate::ostatus::venue_took(
                o.get("total").and_then(Value::as_i64).unwrap_or(0),
                o.get("tip").and_then(Value::as_i64).unwrap_or(0),
                status,
            );
        }
    }
    // ── READINESS: dishes on sale that nobody has declared ──
    //
    // The publish gate refuses NEW listings, and deliberately does not sweep
    // the menu: taking fifty dishes off sale the moment the field arrived would
    // close a working restaurant to fix its paperwork. What it does instead is
    // count them and say so, every time the owner opens the dashboard, until
    // the number is zero.
    let cat = st.read_catalog()?;
    let (mut undeclared, mut on_sale_undeclared) = (0i64, 0i64);
    for (_, pj) in cat.products() {
        if dowiz_hub::allergens::read(&pj).is_declared() {
            continue;
        }
        undeclared += 1;
        if serde_json::from_str::<Value>(&pj)
            .ok()
            .and_then(|p| p.get("available").and_then(Value::as_bool))
            .unwrap_or(false)
        {
            on_sale_undeclared += 1;
        }
    }

    Ok(json!({
        "todayOrders": today,
        "pending": pending,
        "active": active,
        "todayRevenue": revenue,
        "scheduled": scheduled,
        "undeclared": undeclared,
        "onSaleUndeclared": on_sale_undeclared
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
    /// The fourteen declarable allergens this dish contains.
    ///
    /// AN EMPTY ARRAY IS A CLAIM -- "none of the fourteen" -- and absent is not.
    /// `Option<Vec<_>>` keeps the two apart all the way from the wire: `None`
    /// means this request is not about allergens, `Some([])` means somebody
    /// has just declared the dish free of them.
    #[serde(default)]
    pub allergens: Option<Vec<String>>,
    /// The dish's real widest dimension, in centimetres.
    ///
    /// What makes the AR view answer a question rather than be a novelty: it is
    /// the number that tells a customer whether the "large" set is actually
    /// large. Optional, and a dish without one simply has no AR button -- a
    /// guessed size would answer the question WRONGLY, which is worse than not
    /// answering it.
    #[serde(default)]
    pub size_cm: Option<i64>,
    /// The choices a customer may make about this dish. Replaces the whole set
    /// rather than merging: a partial merge of nested groups is ambiguous
    /// (is a missing group deleted or untouched?) and the editor always has the
    /// full list in hand anyway.
    #[serde(default)]
    pub modifier_groups: Option<Value>,
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
    edit_product(&st, &id, body.available, body.unavailable_note, body.price, body.size_cm,
                 body.modifier_groups, body.allergens)
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
    edit_product(st, id, Some(available), note, None, None, None, None).await
}

async fn edit_product(
    st: &Shared,
    id: &str,
    available: Option<bool>,
    note: Option<String>,
    price: Option<i64>,
    size_cm: Option<i64>,
    modifier_groups: Option<Value>,
    allergens: Option<Vec<String>>,
) -> Result<Value, HubHttpError> {
    // Checked before the write, so a refused list never reaches the record.
    let allergens = match allergens {
        None => None,
        Some(raw) => Some(
            dowiz_hub::allergens::validate(&raw).map_err(HubHttpError::Invalid)?,
        ),
    };
    if let Some(p) = price {
        // Integer minor units, and a negative price is not a discount, it is a
        // typo that would make the kernel's ledger owe the customer money.
        if p < 0 {
            return Err(HubHttpError::Invalid("price cannot be negative".into()));
        }
    }
    if let Some(g) = &modifier_groups {
        // PARSED BACK before it is stored. A group the reader cannot see is a
        // rule the owner believes is enforced and is not -- the same failure
        // the delivery zones had, and the same fix. The commonest cause is a
        // group with no id, which silently borrowed its first option's until
        // that was fixed.
        let declared = g.as_array().map(|a| a.len()).unwrap_or(0);
        let readable =
            dowiz_hub::modifiers::groups_of(&json!({ "modifierGroups": g }).to_string()).len();
        if declared != readable {
            return Err(HubHttpError::Invalid(format!(
                "{} of {declared} option groups could not be read; each needs an id \
                 and at least one option that has an id",
                declared - readable
            )));
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
        if let Some(list) = &allergens {
            p["allergens"] = json!(list);
        }
        // ── THE PUBLISH GATE ──
        //
        // A dish nobody has declared cannot go on sale. Not a warning, not a
        // badge: a refusal, because a customer with an allergy reading a menu
        // cannot tell "we checked and it is clear" from "nobody filled this
        // in", and every surface renders both as no warning.
        //
        // Declaring costs one action and can say "none of the fourteen". The
        // gate is per DISH, so a venue is never blocked wholesale -- the dishes
        // that are declared keep selling while the rest are finished.
        if available == Some(true) {
            let decided = allergens.as_ref().map(|l| {
                if l.is_empty() {
                    dowiz_hub::allergens::Declaration::None
                } else {
                    dowiz_hub::allergens::Declaration::Contains(l.clone())
                }
            });
            let state = decided.unwrap_or_else(|| dowiz_hub::allergens::read(&raw));
            if !state.is_declared() {
                return Err(HubHttpError::Conflict(
                    "declare this dish's allergens before putting it on sale; \
                     'none of the fourteen' is a valid answer, an empty field is not"
                        .into(),
                ));
            }
        }
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
        if let Some(g) = modifier_groups {
            p["modifierGroups"] = g;
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
    /// The number a customer rings when the app cannot help them, and one of
    /// the two ways a venue satisfies the notifications leg of the activation
    /// gate. There was NO way to set this before: the storefront read the field
    /// and nothing wrote it, so every venue showed an empty phone unless it had
    /// been seeded with one.
    #[serde(default)]
    pub phone: Option<String>,
    /// Can a customer come and collect? The other half of the fulfilment leg,
    /// and likewise unreachable until now.
    #[serde(default)]
    pub pickup: Option<bool>,
    /// Seven arrays of `{open, close}`, in minutes since local midnight.
    /// An EMPTY array of arrays removes the schedule and returns the venue to
    /// the manual flag.
    #[serde(default)]
    pub hours: Option<Value>,
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
        // ── THE ACTIVATION GATE ──
        //
        // Opening is the moment a stranger can place an order, so it is the
        // moment all three legs have to hold. CLOSING is never gated: a venue
        // must always be able to stop taking orders, whatever state it is in.
        if s == "open" {
            let f = activation_facts(&st).await?;
            let missing = dowiz_hub::activation::missing(&f);
            if !missing.is_empty() {
                return Err(HubHttpError::Conflict(
                    missing.iter().map(|r| r.as_str()).collect::<Vec<_>>().join("; "),
                ));
            }
        }
    }
    if let Some(h) = &body.hours {
        // PARSED BACK before storing, like the zones and the option groups. A
        // schedule the reader cannot see would leave the venue on its manual
        // flag while the owner believed it was automatic -- and they would find
        // out by staying open all night.
        let declared: usize = h
            .as_array()
            .map(|days| days.iter().filter_map(|d| d.as_array()).map(|d| d.len()).sum())
            .unwrap_or(0);
        let sched = dowiz_hub::hours::from_json(&h.to_string());
        let readable: usize = sched.days.iter().map(|d| d.len()).sum();
        if declared != readable {
            return Err(HubHttpError::Invalid(format!(
                "{} of {declared} time windows could not be read; each needs `open` and \
                 `close` as minutes since midnight, 0-1440, and they must differ",
                declared - readable
            )));
        }
    }
    if let Some(ph) = &body.phone {
        // Empty CLEARS it -- a venue with no phone should be able to say so
        // rather than keep a number that no longer answers. Anything else has
        // to look like a number, or the activation gate would pass a venue
        // nobody can call.
        if !ph.trim().is_empty() && ph.chars().filter(char::is_ascii_digit).count() < 8 {
            return Err(HubHttpError::Invalid("that does not look like a phone number".into()));
        }
    }
    st.with_catalog(move |cat| {
        let raw = cat.location().ok_or(HubHttpError::NotFound("venue"))?;
        let mut loc: Value =
            serde_json::from_str(&raw).map_err(|_| HubHttpError::Corrupt("catalogue venue"))?;
        if let Some(s) = body.status {
            loc["status"] = json!(s);
        }
        if let Some(ph) = body.phone {
            loc["phone"] = if ph.trim().is_empty() { Value::Null } else { json!(ph.trim()) };
        }
        if let Some(p) = body.pickup {
            loc["pickup"] = json!(p);
        }
        if let Some(p) = body.delivery_paused {
            loc["delivery_paused"] = json!(if p { 1 } else { 0 });
        }
        if let Some(h) = body.hours {
            loc["hours"] = h;
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
    let now = now_ms();
    // Pending invites sit in the SAME list as the people. An owner asking "who
    // delivers for me" counts the person they invited yesterday among the
    // answer, and a separate panel for them is a panel nobody opens.
    let pending: Vec<Value> = roster
        .invites()
        .into_iter()
        .map(|i| {
            json!({
                "id": i.id, "name": i.name, "madeMs": i.made_ms, "untilMs": i.until_ms,
                "expired": now >= i.until_ms,
            })
        })
        .collect();
    Ok(Json(json!({ "couriers": list, "invites": pending })))
}

/// Gather what the activation gate needs to decide. Every lookup happens here;
/// `dowiz_hub::activation` gets plain facts and no I/O.
async fn activation_facts(st: &Shared) -> Result<dowiz_hub::activation::Facts, HubHttpError> {
    let cat = st.read_catalog()?;
    let loc: Value = cat
        .location()
        .and_then(|j| serde_json::from_str(&j).ok())
        .unwrap_or_else(|| json!({}));

    let sellable = cat
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

    let phone = loc.get("phone").and_then(Value::as_str).unwrap_or("");
    Ok(dowiz_hub::activation::Facts {
        sellable_dishes: sellable,
        telegram_chats: st.read_subs().map(|s| s.staff().len()).unwrap_or(0),
        // Eight digits, the same floor the storefront applies to a customer's
        // number. An empty string and a placeholder both have to fail here, or
        // the gate passes a venue nobody can call.
        has_venue_phone: phone.chars().filter(char::is_ascii_digit).count() >= 8,
        // A fee of zero is configured -- free delivery is a decision. What is
        // NOT configured is the absence of the key.
        delivery_configured: loc.get("delivery_fee").is_some()
            || loc.get("delivery_zones").is_some(),
        pickup_enabled: loc.get("pickup").and_then(Value::as_bool).unwrap_or(false),
    })
}

/// `GET /api/owner/activation` — what is still missing, and can we open.
///
/// Read freely and often: this is what the owner pane shows while they are
/// setting the venue up, so it has to be cheap and has to be the SAME answer
/// the gate will give when they press open.
pub async fn activation(
    State(st): State<Shared>,
    _who: OwnerCaller,
) -> Result<Json<Value>, HubHttpError> {
    let f = activation_facts(&st).await?;
    let missing = dowiz_hub::activation::missing(&f);
    Ok(Json(json!({
        "canOpen": missing.is_empty(),
        "missing": missing.iter()
            .map(|r| json!({ "key": r.key(), "why": r.as_str() }))
            .collect::<Vec<_>>(),
        "facts": {
            "sellableDishes": f.sellable_dishes,
            "telegramChats": f.telegram_chats,
            "hasVenuePhone": f.has_venue_phone,
            "deliveryConfigured": f.delivery_configured,
            "pickupEnabled": f.pickup_enabled,
        }
    })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InviteIn {
    /// The phone the courier will log in with. It IS the account id, so it is
    /// fixed at invite time rather than chosen later.
    pub phone: String,
    pub name: String,
}

/// `POST /api/owner/couriers/invite` — mint a code, shown ONCE.
///
/// The hub cannot show it again: it is stored hashed, exactly like a password,
/// because until it is claimed it opens an account. The owner reads it out or
/// sends it; if it is lost, they issue another, which replaces the first.
pub async fn invite_courier(
    State(st): State<Shared>,
    _who: OwnerCaller,
    Json(raw): Json<Value>,
) -> Result<Json<Value>, HubHttpError> {
    /// A week. Long enough for a courier who starts next Monday, short enough
    /// that a code found in an old message no longer opens anything.
    const TTL_MS: i64 = 7 * 24 * 60 * 60 * 1000;

    let body: InviteIn =
        serde_json::from_value(raw).map_err(|e| HubHttpError::Invalid(e.to_string()))?;
    let phone = body.phone.trim().to_string();
    if phone.chars().filter(char::is_ascii_digit).count() < 8 {
        return Err(HubHttpError::Invalid("that does not look like a phone number".into()));
    }
    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err(HubHttpError::Invalid("who is this code for?".into()));
    }
    if st.read_roster()?.person(&phone).is_some() {
        return Err(HubHttpError::Conflict("that phone already has an account".into()));
    }
    let code = dowiz_hub::roster::new_invite_code()
        .map_err(|_| HubHttpError::Io("no randomness available".into()))?;
    let (c, now) = (code.clone(), now_ms());
    st.with_roster(move |r| {
        r.create_invite(&phone, dowiz_hub::token::Role::Courier, &name, &c, now, TTL_MS)
            .map_err(|_| HubHttpError::Io("could not write the invite".into()))
    })
    .await?;
    Ok(Json(json!({ "code": code, "expiresMs": now + TTL_MS })))
}

/// `POST /api/owner/couriers/{id}/uninvite` — withdraw a pending code.
pub async fn uninvite_courier(
    State(st): State<Shared>,
    _who: OwnerCaller,
    AxPath(id): AxPath<String>,
) -> Result<Json<Value>, HubHttpError> {
    let gone = st.with_roster(move |r| Ok(r.revoke_invite(&id))).await?;
    if !gone {
        return Err(HubHttpError::NotFound("invite"));
    }
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourierActiveIn {
    pub active: bool,
}

/// `POST /api/owner/couriers/{id}/active` — a courier who has left.
///
/// Their record STAYS. An order they delivered still names them, and deleting
/// the person would leave that order pointing at nobody. What changes is that
/// they can no longer log in; every session they hold dies with it, because a
/// courier who has left must not keep a live app in their pocket.
pub async fn set_courier_active(
    State(st): State<Shared>,
    _who: OwnerCaller,
    AxPath(id): AxPath<String>,
    Json(body): Json<CourierActiveIn>,
) -> Result<Json<Value>, HubHttpError> {
    let active = body.active;
    let revoked = st
        .with_roster(move |r| {
            if !r.set_active(&id, active) {
                return Err(HubHttpError::NotFound("courier"));
            }
            Ok(if active { 0 } else { r.revoke_all_for(&id) })
        })
        .await?;
    Ok(Json(json!({ "ok": true, "active": active, "sessionsRevoked": revoked })))
}

/// `GET /api/owner/couriers/{id}` — one courier, folded from the orders.
///
/// Deliveries, cash held and shifts, and nothing that could be read as a score.
/// There is no rating here and there will not be one: NO-COURIER-SCORING is a
/// red line, and an average-minutes-per-delivery figure is a ranking with the
/// serial numbers filed off.
pub async fn courier_detail(
    State(st): State<Shared>,
    _who: OwnerCaller,
    AxPath(id): AxPath<String>,
) -> Result<Json<Value>, HubHttpError> {
    let roster = st.read_roster()?;
    let Some(p) = roster.person(&id) else {
        return Err(HubHttpError::NotFound("courier"));
    };
    let hub = st.read_log()?;
    let now = now_ms();
    let day = 24 * 60 * 60 * 1000;
    let month = start_of_day_ms(now) - 29 * day;

    let (mut delivered, mut in_flight, mut cash_held) = (0i64, 0i64, 0i64);
    let mut recent: Vec<Value> = Vec::new();
    for ev in hub.orders() {
        let Ok(o) = serde_json::from_str::<Value>(&ev.order_json) else { continue };
        if o.get("courier_id").and_then(Value::as_str) != Some(id.as_str()) {
            continue;
        }
        let status = o.get("status").and_then(Value::as_str).unwrap_or("");
        let at = o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0);
        match status {
            "DELIVERED" if at >= month => delivered += 1,
            "IN_DELIVERY" | "READY" => {
                in_flight += 1;
                if o.get("payment").and_then(Value::as_str) == Some("cash") {
                    cash_held += o.get("total").and_then(Value::as_i64).unwrap_or(0);
                }
            }
            _ => {}
        }
        if recent.len() < 20 {
            recent.push(json!({
                "id": o.get("id").cloned().unwrap_or(Value::Null),
                "status": status, "at": at,
                "total": o.get("total").cloned().unwrap_or(json!(0)),
            }));
        }
    }
    let currency = st
        .read_catalog()?
        .location()
        .and_then(|j| serde_json::from_str::<Value>(&j).ok())
        .and_then(|l| l.get("currency").and_then(Value::as_str).map(String::from))
        .unwrap_or_else(|| "ALL".into());

    Ok(Json(json!({
        "id": p.id, "name": p.name, "active": p.active,
        "onShift": st.shifts().await.contains(&p.id),
        "delivered30d": delivered, "inFlight": in_flight, "cashHeld": cash_held,
        "orders": recent, "currency": currency,
    })))
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
        // WHEN IT WAS OFFERED. An assignment nobody answers must not sit on one
        // courier's screen for the rest of the evening while the food goes
        // cold: after the offer window it returns to the pool. The moment is
        // recorded here because this is the only place that knows it.
        o["assigned_at_ms"] = json!(now_ms());
        o["accepted_at_ms"] = Value::Null;
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
            // A spreadsheet has no column for these either.
            let mods = existing
                .as_ref()
                .and_then(|v| v.get("modifierGroups").cloned())
                .unwrap_or(Value::Null);
            // AND ITS ALLERGENS. Blanking these would be the worst of the four:
            // a re-imported price list would make every declared dish
            // undeclared, the publish gate would then refuse to keep them on
            // sale, and a venue would find its whole menu stopped by an import
            // that looked like it only touched prices.
            let allergens = existing
                .as_ref()
                .and_then(|v| v.get("allergens").cloned())
                .unwrap_or(Value::Null);
            cat.set_product(
                &p.id,
                &json!({
                    "id": p.id, "categoryId": p.category_id, "name": p.name,
                    "description": p.description, "price": p.price,
                    "available": p.available, "sortOrder": p.sort_order,
                    "imageUrl": image, "sizeCm": size, "modifierGroups": mods,
                    "allergens": allergens
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
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrandingIn {
    /// The seed colour, as the owner chose it. Still the only REQUIRED field:
    /// a colour picked out of a photograph is one colour and must not have to
    /// invent four more.
    pub primary: String,
    #[serde(default)]
    pub ink: Option<String>,
    #[serde(default)]
    pub paper: Option<String>,
    #[serde(default)]
    pub type_pair: Option<String>,
    #[serde(default)]
    pub radius: Option<i64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresetIn {
    pub preset: String,
}

/// `GET /api/owner/branding` — what the venue has, and what it could have.
///
/// The presets and the type pairs come from the HUB rather than being written
/// into the pane, so the owner's console and the storefront can never disagree
/// about which pairs exist -- and a pair that is not on this list is not a pair
/// the storefront will render.
pub async fn branding(
    State(st): State<Shared>,
    _who: OwnerCaller,
) -> Result<Json<Value>, HubHttpError> {
    use dowiz_hub::brand::{Brand, PRESETS, RADIUS_MAX, TYPE_PAIRS};
    let stored = st
        .read_catalog()?
        .location()
        .and_then(|j| serde_json::from_str::<Value>(&j).ok())
        .and_then(|l| l.get("theme").cloned())
        .unwrap_or(Value::Null);
    let b = match &stored {
        Value::Null => Brand::shipped(),
        v => Brand::parse(&v.to_string()),
    };
    Ok(Json(json!({
        "brand": {
            "primary": b.accent.hex(), "ink": b.ink.hex(), "paper": b.paper.hex(),
            "typePair": b.type_pair, "radius": b.radius,
        },
        "presets": PRESETS.iter().map(|p| json!({
            "id": p.id, "label": p.label, "primary": p.accent,
            "ink": p.ink, "paper": p.paper, "typePair": p.type_pair, "radius": p.radius,
        })).collect::<Vec<_>>(),
        "typePairs": TYPE_PAIRS.iter().map(|t| json!({ "id": t.id, "label": t.label }))
            .collect::<Vec<_>>(),
        "radiusMax": RADIUS_MAX,
    })))
}

/// `POST /api/owner/branding/preset` — take a whole look at once.
pub async fn set_preset(
    State(st): State<Shared>,
    who: OwnerCaller,
    Json(body): Json<PresetIn>,
) -> Result<Json<Value>, HubHttpError> {
    let Some(b) = dowiz_hub::brand::preset(&body.preset) else {
        return Err(HubHttpError::Invalid(format!("{:?} is not a preset", body.preset)));
    };
    apply_brand(&st, who, b).await
}

/// `POST /api/owner/branding` — adopt a theme derived from one colour.
///
/// The SEED is stored alongside the derived tokens. Keeping it means the theme
/// can be re-derived if the derivation itself improves, without asking the owner
/// to pick their colour again.
pub async fn set_branding(
    State(st): State<Shared>,
    who: OwnerCaller,
    Json(body): Json<BrandingIn>,
) -> Result<Json<Value>, HubHttpError> {
    use dowiz_hub::brand::{type_pair, Brand, RADIUS_MAX};

    let colour = |what: &str, v: &str| {
        Rgb::from_hex(v).ok_or_else(|| HubHttpError::Invalid(format!("{what}: {v:?} is not a colour")))
    };
    let d = Brand::shipped();
    let b = Brand {
        accent: colour("accent", &body.primary)?,
        ink: match &body.ink {
            Some(v) => colour("ink", v)?,
            None => d.ink,
        },
        paper: match &body.paper {
            Some(v) => colour("paper", v)?,
            None => d.paper,
        },
        // AN UNKNOWN PAIR IS REFUSED, not defaulted. Silently substituting one
        // would leave the owner looking at a font they did not pick and no
        // reason why.
        type_pair: match &body.type_pair {
            Some(v) => {
                type_pair(v).ok_or_else(|| HubHttpError::Invalid(format!("{v:?} is not a type pair")))?.id
            }
            None => d.type_pair,
        },
        radius: match body.radius {
            Some(r) if !(0..=RADIUS_MAX).contains(&r) => {
                return Err(HubHttpError::Invalid(format!("a radius is 0 to {RADIUS_MAX} px")))
            }
            Some(r) => r,
            None => d.radius,
        },
    };
    apply_brand(&st, who, b).await
}

async fn apply_brand(
    st: &Shared,
    _who: OwnerCaller,
    b: dowiz_hub::brand::Brand,
) -> Result<Json<Value>, HubHttpError> {
    let seed = b.accent;
    let theme = b.theme();
    let mut payload = theme_json(&theme);
    payload["typePair"] = json!(b.type_pair);
    payload["radius"] = json!(b.radius);

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

    let stored = json!({
        "seed": seed.hex(),
        "ink": b.ink.hex(), "paper": b.paper.hex(),
        "typePair": b.type_pair, "radius": b.radius,
        "light": payload["light"], "dark": payload["dark"]
    });
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
        .route("/api/owner/activation", get(activation))
        .route("/api/owner/products/{id}", post(update_product))
        .route("/api/owner/location", post(update_location))
        .route("/api/owner/couriers", get(couriers))
        .route("/api/owner/couriers/invite", post(invite_courier))
        .route("/api/owner/couriers/{id}", get(courier_detail))
        .route("/api/owner/couriers/{id}/uninvite", post(uninvite_courier))
        .route("/api/owner/couriers/{id}/active", post(set_courier_active))
        .route("/api/owner/menu/import", post(import_menu))
        .route("/api/owner/branding/extract", post(extract_branding))
        .route("/api/owner/branding", get(branding))
        .route("/api/owner/branding", post(set_branding))
        .route("/api/owner/branding/preset", post(set_preset))
        .route("/api/owner/settings", get(settings))
        .route("/api/owner/settings", post(set_setting))
        .route("/api/owner/assist", post(owner_assist))
        .route("/api/owner/agent", post(owner_agent))
        .route("/api/owner/zones", post(set_zones))
        .route("/api/owner/products/{id}/image", post(set_product_image))
        .route("/api/owner/products/{id}/image/clear", post(clear_product_image))
        .route("/api/public/reach", get(public_reach))
        .route("/api/owner/stock", get(stock))
        .route("/api/owner/analytics", get(analytics))
        .route("/api/owner/customers", get(customers))
        .route("/api/owner/customers/reveals", get(reveals))
        .route("/api/owner/customers/{key}/reveal", post(reveal_customer))
        .route("/api/owner/promotions", get(promotions))
        .route("/api/owner/promotions", post(set_promotion))
        .route("/api/owner/promotions/{code}/delete", post(delete_promotion))
        .route("/api/owner/supplies", post(set_supply))
        .route("/api/owner/stock/{kind}", post(stock_move))
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
    // WHAT IS RUNNING OUT, computed here and handed over as fact. The model is
    // never asked to work out whether something is low -- it is told, the same
    // way it is told the order counts. A model doing arithmetic over a stock
    // ledger would eventually get one wrong, and the owner would order fish
    // they already have or not order fish they do not.
    let low: Vec<Value> = match st.read_stock().and_then(|l| {
        l.ledger().map_err(|e| HubHttpError::Io(e.to_string()))
    }) {
        Ok(led) => st
            .read_catalog()
            .map(|cat| {
                cat.supplies()
                    .into_iter()
                    .filter_map(|(id, j)| {
                        let v: Value = serde_json::from_str(&j).ok()?;
                        let avail = led.available(&id);
                        let low_at = v.get("lowAt").and_then(Value::as_i64).unwrap_or(0);
                        (low_at > 0 && avail <= low_at).then(|| {
                            json!({
                                "item": v.get("name").cloned().unwrap_or(json!(id)),
                                "available": avail,
                                "unit": v.get("unit").cloned().unwrap_or(json!("g")),
                                "threshold": low_at,
                            })
                        })
                    })
                    .collect()
            })
            .unwrap_or_default(),
        // A stock log that will not read must not take the whole assistant
        // down: the order questions are still answerable without it.
        Err(_) => Vec::new(),
    };

    Ok(json!({
        "now_iso_minutes_since_epoch": now / 60_000,
        "live_orders": live,
        "couriers_on_shift": shifts,
        "running_low": low,
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


/// `POST /api/owner/agent` — the venue's own agent, not just an answer.
///
/// THE DIFFERENCE FROM `/assist` IS THAT IT CHOOSES. `assist` is handed a fixed
/// set of facts and phrases them; this one decides each turn whether it knows
/// enough or needs to look something up — in the hub's graph, or on the web —
/// and says which. That is why the steps come back with the answer: an owner
/// who cannot see what it read cannot tell a wrong answer from a wrong lookup.
///
/// LOCAL IS THE POINT AND IS REPORTED EVERY TIME. `ai.endpoint` defaults to an
/// Ollama on this machine, and `local` in the response says whether it stayed
/// there. An agent that browses on the venue's behalf while quietly sending the
/// venue's orders to a hosted provider would be the worst of both.
pub async fn owner_agent(
    State(st): State<Shared>,
    _who: OwnerCaller,
    Json(body): Json<AskIn>,
) -> Result<Json<Value>, HubHttpError> {
    let question = body.question.trim().to_string();
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

    let facts = owner_facts(&st).await?;
    // REDACTED WHEN THE MODEL IS NOT ON THIS MACHINE, by the same rule the
    // assistant follows. The agent's extra reach makes the rule matter more,
    // not less.
    let sent = if assistant.local { facts } else { crate::ai::redact(&facts) };

    let hub = st.read_log()?;
    let catalog = st.read_catalog()?;
    let knowledge = crate::agent::HubKnowledge::new(&hub, &catalog);
    let run = crate::agent::run(&assistant, &knowledge, &question, &sent)
        .await
        .map_err(|e| HubHttpError::Io(e.to_string()))?;

    let steps: Vec<Value> = run
        .steps
        .iter()
        .map(|s| {
            let (tool, arg) = match &s.action {
                crate::agent::Action::Graph(q) => ("graph", q.clone()),
                crate::agent::Action::Web(u) => ("web", u.clone()),
                crate::agent::Action::Answer(_) => ("answer", String::new()),
            };
            json!({ "tool": tool, "arg": arg, "sawChars": s.observation.chars().count() })
        })
        .collect();

    Ok(Json(json!({
        "answer": run.answer,
        "steps": steps,
        // Said plainly rather than inferred from the step count: an answer
        // assembled because the turns ran out is a weaker answer.
        "exhausted": run.exhausted,
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



// ── customers ───────────────────────────────────────────────────────────────
//
// THERE IS STILL NO CUSTOMER REGISTRY. This is a FOLD over the orders, computed
// per request and stored nowhere, so the venue holds exactly what it held
// before: the orders people placed.
//
// The comment this replaces claimed the absence of a registry meant the data
// "cannot be compelled to be handed over if it is not there". That was
// overstated and worth correcting: the orders carry the name, the phone and the
// address already. What a registry would have added is not the data but the
// CONVENIENCE of it -- a ready-made list, sorted by value, one click from
// export. So the protection moves to where it can still do work: the list is
// redacted by default, un-redacting one customer is a deliberate act, and that
// act is written into the append-only log where it cannot be quietly removed.
//
// The key is the PHONE, because it is the one field a person reliably repeats.
// Names are typed differently every time and addresses change.

// THE MASK IS `dowiz_hub::redact`, AND IT USED TO BE HERE TOO. This file and
// `workers/api` each held a copy, character for character: the same rule, the
// same defect (a number of six digits or fewer came back with every digit
// visible behind a row of dots) and the same absence of tests. A rule with two
// implementations is a rule that gets fixed once.
use dowiz_hub::redact::{name as mask_name, phone as mask_phone};

/// A stable, non-reversible handle for a phone, used as the id in URLs and in
/// the audit log. The audit entry must not carry the number it is about, and a
/// URL that contains a customer's phone is a phone number in every proxy log
/// between here and the browser.
fn customer_key(st: &Shared, phone: &str) -> String {
    let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
    let mac = dowiz_hub::crypto::hmac_sha256(st.signing_key(), digits.as_bytes());
    dowiz_hub::crypto::hex(&mac[..8])
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CustomersQuery {
    #[serde(default)]
    pub sort: Option<String>,
}

/// `GET /api/owner/customers` — who orders here, without saying who they are.
pub async fn customers(
    State(st): State<Shared>,
    _who: OwnerCaller,
    Query(q): Query<CustomersQuery>,
) -> Result<Json<Value>, HubHttpError> {
    let hub = st.read_log()?;
    // (key, masked name, masked phone, orders, spent, last_at)
    let mut rows: Vec<(String, String, String, i64, i64, i64)> = Vec::new();
    for ev in hub.orders() {
        let Ok(o) = serde_json::from_str::<Value>(&ev.order_json) else { continue };
        let Some(phone) = o.get("contact").and_then(|c| c.get("phone")).and_then(Value::as_str)
        else {
            continue;
        };
        let name = o
            .get("contact")
            .and_then(|c| c.get("name"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let at = o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0);
        // A refused order is not money the venue took, so it does not count
        // towards what this customer is worth -- the same rule the takings use.
        // What they spent WITH THE VENUE. The tip went to the courier.
        let spent = crate::ostatus::venue_took(
            o.get("total").and_then(Value::as_i64).unwrap_or(0),
            o.get("tip").and_then(Value::as_i64).unwrap_or(0),
            o.get("status").and_then(Value::as_str).unwrap_or(""),
        );
        let key = customer_key(&st, phone);
        match rows.iter_mut().find(|r| r.0 == key) {
            Some(r) => {
                r.3 += 1;
                r.4 += spent;
                r.5 = r.5.max(at);
            }
            None => rows.push((
                key,
                mask_name(name),
                mask_phone(phone),
                1,
                spent,
                at,
            )),
        }
    }
    match q.sort.as_deref() {
        Some("spent") => rows.sort_by(|a, b| b.4.cmp(&a.4).then(b.5.cmp(&a.5))),
        Some("orders") => rows.sort_by(|a, b| b.3.cmp(&a.3).then(b.5.cmp(&a.5))),
        // Newest first by default: the question an owner asks at the end of a
        // shift is who has just been in, not who is worth the most.
        _ => rows.sort_by(|a, b| b.5.cmp(&a.5)),
    }
    let currency = st
        .read_catalog()?
        .location()
        .and_then(|j| serde_json::from_str::<Value>(&j).ok())
        .and_then(|l| l.get("currency").and_then(Value::as_str).map(String::from))
        .unwrap_or_else(|| "ALL".into());
    Ok(Json(json!({
        "customers": rows.iter().map(|r| json!({
            "key": r.0, "name": r.1, "phone": r.2,
            "orders": r.3, "spent": r.4, "lastAt": r.5,
        })).collect::<Vec<_>>(),
        "currency": currency,
    })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevealIn {
    /// Why. Not validated against a list, because the reasons are whatever a
    /// restaurant's evening throws up -- but it is REQUIRED, so the log says
    /// more than "somebody looked".
    pub reason: String,
}

/// `POST /api/owner/customers/{key}/reveal` — un-redact one person, on the record.
///
/// The audit entry is appended BEFORE the answer is returned. If the append
/// fails, nothing is revealed: an un-auditable reveal is the one thing this
/// route must not do, and returning the number first and logging afterwards
/// would make the log best-effort.
pub async fn reveal_customer(
    State(st): State<Shared>,
    who: OwnerCaller,
    AxPath(key): AxPath<String>,
    Json(body): Json<RevealIn>,
) -> Result<Json<Value>, HubHttpError> {
    let reason = body.reason.trim().to_string();
    if reason.len() < 3 {
        return Err(HubHttpError::Invalid("say why you are looking".into()));
    }
    let hub = st.read_log()?;
    let mut found: Option<(String, String, Vec<Value>)> = None;
    for ev in hub.orders() {
        let Ok(o) = serde_json::from_str::<Value>(&ev.order_json) else { continue };
        let Some(phone) = o.get("contact").and_then(|c| c.get("phone")).and_then(Value::as_str)
        else {
            continue;
        };
        if customer_key(&st, phone) != key {
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
        return Err(HubHttpError::NotFound("customer"));
    };

    let entry = json!({ "by": who.0.person.id, "at": now_ms(), "reason": reason }).to_string();
    let subject = format!("cust:{key}");
    let at = now_ms() as u64;
    st.with_log(move |log| {
        log.append(dowiz_hub::EventKind::Revealed, &subject, &entry, at, [0u8; 32])
            .map_err(|e| HubHttpError::Io(format!("{e:?}")))
    })
    .await?;

    Ok(Json(json!({ "name": name, "phone": phone, "orders": orders })))
}

/// `GET /api/owner/customers/reveals` — who has been looking.
///
/// Readable by the owner, which is the only role that can reveal. The point of
/// a log nobody reads is small; the point of one anybody with the pane can read
/// is that a staff member knows their lookups are visible.
pub async fn reveals(
    State(st): State<Shared>,
    _who: OwnerCaller,
) -> Result<Json<Value>, HubHttpError> {
    let hub = st.read_log()?;
    let out: Vec<Value> = hub
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
    Ok(Json(json!({ "reveals": out })))
}

// ── promo codes ─────────────────────────────────────────────────────────────

/// AN UNKNOWN FIELD IS A REFUSAL, not a shrug.
///
/// The default is to ignore what serde does not recognise, and for a promo that
/// default gives money away: a client that sends `until` instead of `untilMs`
/// gets a code with NO expiry, silently, forever. The cost of being strict is a
/// 400 with the field named; the cost of being lenient is an unbounded
/// discount nobody created on purpose. This was not a hypothesis -- the first
/// run of the tests below saved an "expired" code that was still live.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PromoIn {
    pub code: String,
    pub kind: String,
    pub value: i64,
    #[serde(default)]
    pub min_order: Option<i64>,
    #[serde(default)]
    pub from_ms: Option<i64>,
    #[serde(default)]
    pub until_ms: Option<i64>,
    #[serde(default)]
    pub max_uses: Option<i64>,
    #[serde(default)]
    pub active: Option<bool>,
}

/// `GET /api/owner/promotions` — every code, with its DERIVED status.
///
/// Status is computed, never stored: a stored one goes stale the moment the
/// clock passes the window, and the owner would be reading a label that no
/// longer describes what the code does.
pub async fn promotions(
    State(st): State<Shared>,
    _who: OwnerCaller,
) -> Result<Json<Value>, HubHttpError> {
    use dowiz_hub::promo::Promo;

    let cat = st.read_catalog()?;
    let hub = st.read_log()?;
    let now = now_ms();
    let mut rows: Vec<Value> = cat
        .promos()
        .into_iter()
        .filter_map(|(_, j)| Promo::parse(&j))
        .map(|p| {
            let used = crate::hub::promo_uses(&hub, &p.code);
            json!({
                "code": p.code, "kind": p.kind.as_str(), "value": p.value,
                "minOrder": p.min_order, "fromMs": p.from_ms, "untilMs": p.until_ms,
                "maxUses": p.max_uses, "active": p.active,
                "used": used, "status": p.status(now, used).as_str(),
            })
        })
        .collect();
    // Alphabetical, because a code is looked up by its name. Sorting by status
    // would move a row the moment a window closed, under the owner's cursor.
    rows.sort_by(|a, b| a["code"].as_str().cmp(&b["code"].as_str()));
    let currency = cat
        .location()
        .and_then(|j| serde_json::from_str::<Value>(&j).ok())
        .and_then(|l| l.get("currency").and_then(Value::as_str).map(String::from))
        .unwrap_or_else(|| "ALL".into());
    Ok(Json(json!({ "promotions": rows, "currency": currency })))
}

/// `POST /api/owner/promotions` — create or replace one.
///
/// Replace, not merge: a promo is six numbers read together, and a partial
/// update would let an owner change the percentage while a forgotten window
/// from last month silently keeps it expired.
pub async fn set_promotion(
    State(st): State<Shared>,
    _who: OwnerCaller,
    Json(raw): Json<Value>,
) -> Result<Json<Value>, HubHttpError> {
    use dowiz_hub::promo::{normalise, valid_code, valid_value, Kind, Promo};

    // Deserialised BY HAND rather than through the extractor, so a rejected
    // field comes back as a sentence naming it -- serde says "unknown field
    // `until`, expected one of ..." -- instead of the extractor's bare 422 with
    // an empty body, which tells the owner only that something was wrong.
    let body: PromoIn =
        serde_json::from_value(raw).map_err(|e| HubHttpError::Invalid(e.to_string()))?;

    let code = normalise(&body.code);
    if !valid_code(&code) {
        return Err(HubHttpError::Invalid(
            "a code is 3 to 16 letters or digits".into(),
        ));
    }
    let Some(kind) = Kind::parse(&body.kind) else {
        return Err(HubHttpError::Invalid("a code takes off a percent or a fixed amount".into()));
    };
    if !valid_value(kind, body.value) {
        return Err(HubHttpError::Invalid(match kind {
            Kind::Percent => "a percentage is between 1 and 100".into(),
            Kind::Fixed => "a fixed discount must be more than nothing".into(),
        }));
    }
    // A window that ends before it starts is a code that can never apply. It is
    // refused rather than saved, because the list would show it as Scheduled
    // forever and nobody would know why.
    if let (Some(f), Some(u)) = (body.from_ms, body.until_ms) {
        if u <= f {
            return Err(HubHttpError::Invalid("that window ends before it starts".into()));
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
    st.with_catalog(move |cat| {
        cat.set_promo(&code, &stored);
        Ok(())
    })
    .await?;
    Ok(Json(json!({ "ok": true, "code": p.code })))
}

/// `POST /api/owner/promotions/{code}/delete` — a real delete.
///
/// Distinct from the active switch on purpose. Switching off is reversible and
/// keeps the dates; deleting means the code stops working and the owner is
/// free to reuse the word.
pub async fn delete_promotion(
    State(st): State<Shared>,
    _who: OwnerCaller,
    AxPath(code): AxPath<String>,
) -> Result<Json<Value>, HubHttpError> {
    let code = dowiz_hub::promo::normalise(&code);
    let gone = st.with_catalog(move |cat| Ok(cat.remove_promo(&code))).await?;
    if !gone {
        return Err(HubHttpError::NotFound("promo code"));
    }
    Ok(Json(json!({ "ok": true })))
}

// ── supplies and stock ───────────────────────────────────────────────────────

/// `GET /api/owner/stock` — what is on the shelf, and what is running out.
pub async fn stock(
    State(st): State<Shared>,
    _who: OwnerCaller,
) -> Result<Json<Value>, HubHttpError> {
    let cat = st.read_catalog()?;
    let log = st.read_stock()?;
    let led = log.ledger().map_err(|e| HubHttpError::Io(e.to_string()))?;

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
                // The only derived flag, and it is derived on read rather than
                // stored: a "low" boolean in the store would go stale the moment
                // the threshold moved.
                "low": low_at > 0 && level.available() <= low_at,
            }))
        })
        .collect();

    // Reservations nobody will ever settle. Surfaced rather than swept: this is
    // stock the venue believes it owes to an order that ended.
    let stranded: Vec<Value> = led
        .stranded()
        .into_iter()
        .map(|(order, item, qty)| json!({ "order": order, "item": item, "qty": qty }))
        .collect();

    Ok(Json(json!({ "supplies": rows, "stranded": stranded })))
}

#[derive(Deserialize)]
pub struct SupplyIn {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    /// Base unit — grams, millilitres, pieces. Free text, because a venue knows
    /// what it counts in better than a fixed list does.
    #[serde(default)]
    pub unit: Option<String>,
    /// Tell me when available drops to this. Zero means never.
    #[serde(default)]
    pub low_at: Option<i64>,
}

/// `POST /api/owner/supplies` — add or edit an ingredient.
pub async fn set_supply(
    State(st): State<Shared>,
    _who: OwnerCaller,
    Json(body): Json<SupplyIn>,
) -> Result<Json<Value>, HubHttpError> {
    let id = body.id.trim().to_string();
    if id.is_empty() || id.len() > 64 {
        return Err(HubHttpError::Invalid("an ingredient needs a short id".into()));
    }
    if body.low_at.is_some_and(|v| v < 0) {
        return Err(HubHttpError::Invalid("a threshold cannot be negative".into()));
    }
    st.with_catalog(move |cat| {
        let existing: Value = cat
            .supply(&id)
            .and_then(|j| serde_json::from_str(&j).ok())
            .unwrap_or(json!({}));
        let rec = json!({
            "id": id,
            "name": body.name.clone()
                .map(Value::String)
                .or_else(|| existing.get("name").cloned())
                .unwrap_or(json!(id)),
            "unit": body.unit.clone()
                .map(Value::String)
                .or_else(|| existing.get("unit").cloned())
                .unwrap_or(json!("g")),
            "lowAt": body.low_at
                .map(|v| json!(v))
                .or_else(|| existing.get("lowAt").cloned())
                .unwrap_or(json!(0)),
        });
        cat.set_supply(&id, &rec.to_string());
        Ok(Json(rec))
    })
    .await
}

#[derive(Deserialize)]
pub struct StockMoveIn {
    pub item: String,
    #[serde(default)]
    pub qty: Option<i64>,
    /// For a stocktake: what was actually counted.
    #[serde(default)]
    pub observed: Option<i64>,
    /// For waste: spoiled, dropped or unsold.
    #[serde(default)]
    pub reason: Option<String>,
}

/// `POST /api/owner/stock/{kind}` — received, wasted or counted.
///
/// The three events a HUMAN causes. Reserved, Consumed and Released are
/// emitted by the order lifecycle and are deliberately NOT reachable here: a
/// hand-written reservation has no order to settle it and would strand
/// immediately.
pub async fn stock_move(
    State(st): State<Shared>,
    _who: OwnerCaller,
    AxPath(kind): AxPath<String>,
    Json(body): Json<StockMoveIn>,
) -> Result<Json<Value>, HubHttpError> {
    use dowiz_hub::stock::{StockEvent, WasteReason};

    let item = body.item.trim().to_string();
    if item.is_empty() {
        return Err(HubHttpError::Invalid("which ingredient?".into()));
    }
    if st.read_catalog()?.supply(&item).is_none() {
        return Err(HubHttpError::NotFound("ingredient"));
    }

    let ev = match kind.as_str() {
        "received" => StockEvent::Received {
            item,
            qty: body.qty.ok_or_else(|| HubHttpError::Invalid("how much?".into()))?,
        },
        "wasted" => StockEvent::Wasted {
            item,
            qty: body.qty.ok_or_else(|| HubHttpError::Invalid("how much?".into()))?,
            reason: body
                .reason
                .as_deref()
                .and_then(WasteReason::from_str)
                .unwrap_or(WasteReason::Spoiled),
        },
        "stocktake" => StockEvent::Stocktake {
            item,
            observed: body
                .observed
                .ok_or_else(|| HubHttpError::Invalid("what was counted?".into()))?,
            // The id ties a count to the person and moment that made it, so a
            // basis reset is attributable rather than anonymous.
            stocktake_id: format!("st_{}", now_ms()),
        },
        other => return Err(HubHttpError::Invalid(format!("no such movement: {other}"))),
    };

    st.with_stock(move |log| {
        log.append(&ev).map_err(|e| match e {
            dowiz_hub::stock::StockError::OutOfStock { .. }
            | dowiz_hub::stock::StockError::Linkage(_) => HubHttpError::Conflict(e.to_string()),
            other => HubHttpError::Invalid(other.to_string()),
        })
    })
    .await?;

    let led = st.read_stock()?.ledger().map_err(|e| HubHttpError::Io(e.to_string()))?;
    let lvl = led.level(&body.item);
    Ok(Json(json!({
        "item": body.item, "onHand": lvl.on_hand,
        "reserved": lvl.reserved, "available": lvl.available()
    })))
}

// ── analytics ────────────────────────────────────────────────────────────────

/// `GET /api/owner/analytics?days=7`
///
/// DERIVED ON READ from the order log, like the dashboard. There is no
/// analytics store and there should not be: the log already holds every fact,
/// and a second table of pre-aggregated numbers is a second thing that can
/// disagree with it. A venue's month is a few thousand orders; folding them is
/// microseconds.
///
/// §7.3 asks for revenue over time, top products, and an hour-of-day picture.
/// It also asks for a delivery geo-map and ingredient consumption; the first
/// needs coordinates most orders do not carry and the second is answerable from
/// the stock log rather than here, so neither is faked with a placeholder.
pub async fn analytics(
    State(st): State<Shared>,
    _who: OwnerCaller,
    Query(q): Query<AnalyticsQuery>,
) -> Result<Json<Value>, HubHttpError> {
    // 7 or 30, and nothing else: two windows an owner reasons about, rather
    // than an arbitrary number that invites a query nobody can interpret.
    let days = if q.days.unwrap_or(7) >= 30 { 30 } else { 7 };
    let now = now_ms();
    let day_ms = 24 * 60 * 60 * 1000;
    let from = start_of_day_ms(now) - (days as i64 - 1) * day_ms;

    let hub = st.read_log()?;
    let cat = st.read_catalog()?;

    let mut by_day: Vec<(i64, i64, i64)> = (0..days as i64)
        .map(|i| (from + i * day_ms, 0i64, 0i64))
        .collect();
    let mut by_hour = [0i64; 24];
    let mut products: Vec<(String, i64, i64)> = Vec::new();
    let (mut orders, mut revenue, mut rejected) = (0i64, 0i64, 0i64);
    let (mut delivery, mut pickup) = (0i64, 0i64);

    let tz: i64 = std::env::var("TZ_OFFSET_MINUTES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(120);

    for ev in hub.orders() {
        let Ok(o) = serde_json::from_str::<Value>(&ev.order_json) else { continue };
        let at = o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0);
        if at < from {
            continue;
        }
        let status = o.get("status").and_then(Value::as_str).unwrap_or("");
        let refused = !crate::ostatus::took_money(status);
        orders += 1;
        if refused {
            rejected += 1;
        }
        // Money the venue TOOK: a refused or refunded order is not revenue,
        // and the tip is the courier's money passing through.
        let total = crate::ostatus::venue_took(
            o.get("total").and_then(Value::as_i64).unwrap_or(0),
            o.get("tip").and_then(Value::as_i64).unwrap_or(0),
            status,
        );
        revenue += total;

        match o.get("fulfilment").and_then(|f| f.get("kind")).and_then(Value::as_str) {
            Some("pickup") => pickup += 1,
            _ => delivery += 1,
        }

        let idx = ((at - from) / day_ms).clamp(0, days as i64 - 1) as usize;
        by_day[idx].1 += 1;
        by_day[idx].2 += total;

        let (_, minute) = dowiz_hub::hours::local_now(at, tz);
        by_hour[(minute / 60).clamp(0, 23) as usize] += 1;

        if refused {
            continue;
        }
        for item in o.get("items").and_then(Value::as_array).into_iter().flatten() {
            let Some(pid) = item.get("product_id").and_then(Value::as_str) else { continue };
            let qty = item.get("quantity").and_then(Value::as_i64).unwrap_or(1);
            let line = item.get("unit_price").and_then(Value::as_i64).unwrap_or(0) * qty;
            match products.iter_mut().find(|(p, _, _)| p == pid) {
                Some((_, n, m)) => {
                    *n += qty;
                    *m += line;
                }
                None => products.push((pid.to_string(), qty, line)),
            }
        }
    }

    // By money, then by name, so a tie is stable and two reads agree.
    products.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
    let top: Vec<Value> = products
        .iter()
        .take(10)
        .map(|(pid, qty, money)| {
            let name = cat
                .product(pid)
                .and_then(|j| serde_json::from_str::<Value>(&j).ok())
                .and_then(|v| v.get("name").and_then(Value::as_str).map(str::to_string))
                .unwrap_or_else(|| pid.clone());
            json!({ "id": pid, "name": name, "quantity": qty, "revenue": money })
        })
        .collect();

    Ok(Json(json!({
        "days": days,
        "orders": orders,
        "revenue": revenue,
        "rejected": rejected,
        // The average is INTEGER division. A mean order value of 1732.6667
        // implies a precision the underlying integers do not have, and money
        // never becomes a float in this system.
        "averageOrder": if orders > rejected { revenue / (orders - rejected) } else { 0 },
        "delivery": delivery,
        "pickup": pickup,
        "byDay": by_day.iter()
            .map(|(at, n, m)| json!({ "at": at, "orders": n, "revenue": m }))
            .collect::<Vec<_>>(),
        "byHour": by_hour.to_vec(),
        "topProducts": top,
        "currency": "ALL",
    })))
}

#[derive(Deserialize, Default)]
pub struct AnalyticsQuery {
    #[serde(default)]
    pub days: Option<u32>,
}
