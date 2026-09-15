//! The tenant's hub, served from its own machine.
//!
//! P67's shape: one restaurant, one hub, owner-claimed. That decision changes
//! three things about the code below, and each is a SIMPLIFICATION rather than a
//! port of the same idea to a new place.
//!
//!  * The store is a FILE. bebop's format was always file-shaped; on a Worker it
//!    had to live as a blob inside something else. Here it is what it is.
//!  * ONE process is ONE writer. The generation guard and its bounded replay
//!    existed because two Workers could read the same image; a mutex around the
//!    store is the whole of that problem here.
//!  * There is one tenant, so nothing is scoped by location at the boundary. A
//!    hub that holds one restaurant cannot leak another's data because it does
//!    not have any.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::extract::{Path as AxPath, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::Mutex;

use dowiz_hub::catalog::Catalog;
use dowiz_hub::{EventKind, Hub};
use dowiz_kernel::json_api;

/// Where the hub keeps itself. Two files, because a bebop store has one root and
/// the log's layout and the KV layout cannot share it.
pub struct HubPaths {
    pub log: PathBuf,
    pub catalog: PathBuf,
}

impl HubPaths {
    pub fn under(dir: &Path) -> Self {
        HubPaths {
            log: dir.join("orders.store"),
            catalog: dir.join("catalog.store"),
        }
    }
}

/// The hub's state. The mutex is the single-writer guarantee: on one machine the
/// only way two writes race is if this process lets them.
pub struct HubState {
    paths: HubPaths,
    write_lock: Mutex<()>,
}

pub type Shared = Arc<HubState>;

impl HubState {
    pub fn open(dir: &Path) -> std::io::Result<Shared> {
        std::fs::create_dir_all(dir)?;
        let paths = HubPaths::under(dir);
        // A hub that has never run creates its own empty images rather than
        // failing on first request. It is a new restaurant, not an error.
        if !paths.log.exists() {
            let hub = Hub::create().map_err(io_err)?;
            std::fs::write(&paths.log, hub.to_bytes())?;
        }
        if !paths.catalog.exists() {
            let mut cat = Catalog::create().map_err(io_err)?;
            let bytes = cat.to_bytes().map_err(io_err)?;
            std::fs::write(&paths.catalog, bytes)?;
        }
        Ok(Arc::new(HubState { paths, write_lock: Mutex::new(()) }))
    }

    fn read_log(&self) -> Result<Hub, HubHttpError> {
        let bytes = std::fs::read(&self.paths.log).map_err(|e| HubHttpError::Io(e.to_string()))?;
        Hub::load(&bytes).map_err(|_| HubHttpError::Corrupt("order log"))
    }

    fn read_catalog(&self) -> Result<Catalog, HubHttpError> {
        let bytes =
            std::fs::read(&self.paths.catalog).map_err(|e| HubHttpError::Io(e.to_string()))?;
        Catalog::load(&bytes).map_err(|_| HubHttpError::Corrupt("catalogue"))
    }

    /// Read, mutate, write the order log under the single-writer lock.
    ///
    /// The write is atomic at the filesystem level: a temporary file is renamed
    /// over the real one, so a crash mid-write leaves the PREVIOUS image intact
    /// rather than a half-written one. bebop's own commit orders three fsyncs for
    /// the same reason; rename gives it in one step when the whole image is
    /// rewritten anyway.
    async fn with_log<F, T>(&self, f: F) -> Result<T, HubHttpError>
    where
        F: FnOnce(&mut Hub) -> Result<T, HubHttpError>,
    {
        let _guard = self.write_lock.lock().await;
        let mut hub = self.read_log()?;
        let out = f(&mut hub)?;
        atomic_write(&self.paths.log, &hub.to_bytes())?;
        Ok(out)
    }

    async fn with_catalog<F, T>(&self, f: F) -> Result<T, HubHttpError>
    where
        F: FnOnce(&mut Catalog) -> Result<T, HubHttpError>,
    {
        let _guard = self.write_lock.lock().await;
        let mut cat = self.read_catalog()?;
        let out = f(&mut cat)?;
        let bytes = cat.to_bytes().map_err(|_| HubHttpError::Corrupt("catalogue"))?;
        atomic_write(&self.paths.catalog, &bytes)?;
        Ok(out)
    }
}

fn io_err<E: std::fmt::Debug>(e: E) -> std::io::Error {
    std::io::Error::other(format!("{e:?}"))
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), HubHttpError> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes).map_err(|e| HubHttpError::Io(e.to_string()))?;
    std::fs::rename(&tmp, path).map_err(|e| HubHttpError::Io(e.to_string()))?;
    Ok(())
}

// ── errors ───────────────────────────────────────────────────────────────────

pub enum HubHttpError {
    NotFound(&'static str),
    /// The kernel refused a transition. The caller's mistake, not the server's.
    Refused(String),
    Invalid(String),
    Conflict(String),
    /// An unreadable store. Fail LOUD: serving an empty hub because the image
    /// would not parse presents "no orders" as a healthy answer.
    Corrupt(&'static str),
    Io(String),
}

impl IntoResponse for HubHttpError {
    fn into_response(self) -> Response {
        let (code, msg) = match self {
            HubHttpError::NotFound(w) => (StatusCode::NOT_FOUND, w.to_string()),
            HubHttpError::Refused(m) => (StatusCode::CONFLICT, m),
            HubHttpError::Invalid(m) => (StatusCode::BAD_REQUEST, m),
            HubHttpError::Conflict(m) => (StatusCode::CONFLICT, m),
            HubHttpError::Corrupt(w) => (
                StatusCode::SERVICE_UNAVAILABLE,
                format!("{w} is unreadable; refusing to answer from a store that will not parse"),
            ),
            HubHttpError::Io(m) => (StatusCode::SERVICE_UNAVAILABLE, m),
        };
        (code, Json(json!({ "error": msg }))).into_response()
    }
}

// ── storefront ───────────────────────────────────────────────────────────────

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
}

#[derive(Deserialize)]
pub struct PlaceIn {
    pub items: Vec<LineIn>,
    pub contact: ContactIn,
    pub fulfilment: FulfilmentIn,
    #[serde(default)]
    pub payment: Option<String>,
}

pub async fn menu(State(st): State<Shared>, _slug: Option<AxPath<String>>) -> Result<Json<Value>, HubHttpError> {
    let cat = st.read_catalog()?;
    let Some(loc_json) = cat.location() else {
        return Err(HubHttpError::NotFound("this hub has no venue yet"));
    };
    let loc: Value = serde_json::from_str(&loc_json)
        .map_err(|_| HubHttpError::Corrupt("catalogue venue"))?;

    let mut cat_meta: Vec<(String, String, i64)> = cat
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
    cat_meta.sort_by_key(|(_, _, s)| *s);

    let products: Vec<(String, Value)> = cat
        .products()
        .into_iter()
        .filter_map(|(id, j)| serde_json::from_str::<Value>(&j).ok().map(|v| (id, v)))
        .collect();

    let mut cats = Vec::new();
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
                        "imageUrl": p.get("imageUrl").cloned().unwrap_or(Value::Null)
                    }),
                )
            })
            .collect();
        items.sort_by_key(|(s, _)| *s);
        if items.is_empty() {
            continue;
        }
        cats.push(json!({
            "id": cid, "name": cname, "sortOrder": csort,
            "products": items.into_iter().map(|(_, p)| p).collect::<Vec<_>>()
        }));
    }

    let paused = loc.get("delivery_paused").and_then(|x| x.as_i64()).unwrap_or(0) == 1;
    let status = if paused {
        "closed"
    } else {
        loc.get("status").and_then(|x| x.as_str()).unwrap_or("closed")
    };

    Ok(Json(json!({
        "location": {
            "id": loc.get("id").cloned().unwrap_or(Value::Null),
            "name": loc.get("name").cloned().unwrap_or(json!("")),
            "slug": loc.get("slug").cloned().unwrap_or(json!("")),
            "phone": loc.get("phone").cloned().unwrap_or(json!("")),
            "address": loc.get("address").cloned().unwrap_or(Value::Null),
            "status": status,
            "closesAt": loc.get("closes_at").cloned().unwrap_or(Value::Null),
            "deliveryEta": loc.get("delivery_eta").cloned().unwrap_or(json!("30-45")),
            "deliveryFee": loc.get("delivery_fee").cloned().unwrap_or(json!(0)),
            "freeDeliveryThreshold": loc.get("free_delivery_threshold").cloned().unwrap_or(Value::Null),
            "minOrder": loc.get("min_order").cloned().unwrap_or(json!(0)),
            "currencyCode": loc.get("currency_code").cloned().unwrap_or(json!("ALL")),
            "menuVersion": loc.get("menu_version").cloned().unwrap_or(json!(1)),
            "supportedLocales": loc.get("supported_locales").cloned().unwrap_or(json!(["sq"])),
            "defaultLocale": loc.get("default_locale").cloned().unwrap_or(json!("sq"))
        },
        "categories": cats
    })))
}

pub async fn place(
    State(st): State<Shared>,
    _slug: Option<AxPath<String>>,
    Json(body): Json<PlaceIn>,
) -> Result<Json<Value>, HubHttpError> {
    if body.items.is_empty() {
        return Err(HubHttpError::Invalid("empty order".into()));
    }
    if body.contact.phone.chars().filter(char::is_ascii_digit).count() < 8 {
        return Err(HubHttpError::Invalid("invalid phone".into()));
    }
    if body.fulfilment.kind == "delivery"
        && body.fulfilment.address.as_ref().map_or(true, |a| a.line.trim().is_empty())
    {
        return Err(HubHttpError::Invalid("delivery address required".into()));
    }

    let cat = st.read_catalog()?;
    let Some(loc_json) = cat.location() else {
        return Err(HubHttpError::NotFound("this hub has no venue yet"));
    };
    let loc: Value =
        serde_json::from_str(&loc_json).map_err(|_| HubHttpError::Corrupt("catalogue venue"))?;
    if loc.get("delivery_paused").and_then(|x| x.as_i64()).unwrap_or(0) == 1
        || loc.get("status").and_then(|x| x.as_str()) == Some("closed")
    {
        return Err(HubHttpError::Conflict("venue is closed".into()));
    }

    // THE MONEY RULE, unchanged by the move: every price is re-derived from the
    // catalogue and the browser's unit_price is discarded.
    let mut lines = Vec::with_capacity(body.items.len());
    let mut subtotal: i64 = 0;
    for it in &body.items {
        if !(1..=99).contains(&it.quantity) {
            return Err(HubHttpError::Invalid("invalid quantity".into()));
        }
        let Some(pj) = cat.product(&it.product_id) else {
            return Err(HubHttpError::Invalid(format!("unknown product: {}", it.product_id)));
        };
        let p: Value =
            serde_json::from_str(&pj).map_err(|_| HubHttpError::Corrupt("catalogue product"))?;
        if !p.get("available").and_then(|x| x.as_bool()).unwrap_or(false) {
            return Err(HubHttpError::Conflict(format!("unavailable: {}", it.product_id)));
        }
        let price = p.get("price").and_then(|x| x.as_i64()).unwrap_or(-1);
        if price < 0 {
            return Err(HubHttpError::Conflict(format!("no price: {}", it.product_id)));
        }
        subtotal += price * it.quantity;
        lines.push(json!({
            "product_id": it.product_id, "modifier_ids": it.modifier_ids,
            "quantity": it.quantity, "unit_price": price
        }));
    }
    let min_order = loc.get("min_order").and_then(|x| x.as_i64()).unwrap_or(0);
    if subtotal < min_order {
        return Err(HubHttpError::Conflict("below minimum order".into()));
    }

    let fee = match loc.get("free_delivery_threshold").and_then(|x| x.as_i64()) {
        Some(th) if subtotal >= th => 0,
        _ if body.fulfilment.kind == "pickup" => 0,
        _ => loc.get("delivery_fee").and_then(|x| x.as_i64()).unwrap_or(0),
    };

    let id = new_order_id();
    let created_at_ms = now_ms();
    let order_json = json_api::place_order_at(
        id.clone(),
        None,
        &serde_json::to_string(&lines).unwrap_or_else(|_| "[]".into()),
        created_at_ms,
        Some("storefront".into()),
    )
    .map_err(HubHttpError::Refused)?;

    let mut envelope: Value =
        serde_json::from_str(&order_json).map_err(|_| HubHttpError::Corrupt("kernel order"))?;
    envelope["delivery_fee"] = json!(fee);
    envelope["total"] = json!(subtotal + fee);
    envelope["contact"] = json!({ "name": body.contact.name, "phone": body.contact.phone });
    envelope["fulfilment"] = json!({
        "kind": body.fulfilment.kind,
        "address": body.fulfilment.address.as_ref().map(|a| json!({ "line": a.line, "note": a.note })),
        "fee": fee
    });
    envelope["payment"] = json!(body.payment.unwrap_or_else(|| "cash".into()));

    let stored = serde_json::to_string(&envelope).unwrap_or(order_json);
    let ev_id = id.clone();
    let ev_body = stored.clone();
    st.with_log(move |hub| {
        hub.append(EventKind::Placed, &ev_id, &ev_body, created_at_ms as u64, [0u8; 32])
            .map_err(|e| HubHttpError::Io(format!("{e:?}")))
    })
    .await?;

    Ok(Json(envelope))
}

pub async fn order(
    State(st): State<Shared>,
    AxPath(id): AxPath<String>,
) -> Result<Json<Value>, HubHttpError> {
    let hub = st.read_log()?;
    let raw = hub.order(&id).map_err(|_| HubHttpError::NotFound("order"))?;
    Ok(Json(serde_json::from_str(&raw).unwrap_or(json!({}))))
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// A time-ordered, collision-resistant id. Sorting by id sorts by arrival, which
/// the counter this replaces also did -- but this one does not restart at zero in
/// a second process, which is the property that mattered.
fn new_order_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let ms = now_ms() as u64;
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    // The random tail is what makes two hubs, or a restarted one, not collide.
    let r = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0);
    format!("ord_{ms:013}_{n:04x}{r:08x}")
}

#[derive(Deserialize)]
pub struct AdvanceIn {
    pub next_status: String,
}

/// Advance an order. The KERNEL decides whether the edge is legal; this handler
/// records its answer and nothing else. There is no ordered list of statuses in
/// this file, which is the whole point: a second list would be a second
/// authority, and the two would eventually disagree.
pub async fn advance(
    State(st): State<Shared>,
    AxPath(id): AxPath<String>,
    Json(body): Json<AdvanceIn>,
) -> Result<Json<Value>, HubHttpError> {
    let next = body.next_status;
    let out = st
        .with_log(move |hub| {
            let current = hub.order(&id).map_err(|_| HubHttpError::NotFound("order"))?;
            let updated =
                json_api::apply_event_logic(&current, &next).map_err(HubHttpError::Refused)?;

            // The kernel owns items, status, subtotal and the ledger. Delivery,
            // contact and payment ride alongside and MUST survive the
            // transition: an address lost on a status change is a failure that
            // only surfaces at the customer's door.
            let mut merged: Value = serde_json::from_str(&updated)
                .map_err(|_| HubHttpError::Corrupt("kernel order"))?;
            let old: Value = serde_json::from_str(&current).unwrap_or(json!({}));
            for k in [
                "contact", "fulfilment", "payment", "delivery_fee", "total", "courier_id",
                "rejection_reason", "payment_status",
            ] {
                if let Some(v) = old.get(k) {
                    merged[k] = v.clone();
                }
            }
            let body = serde_json::to_string(&merged).unwrap_or(updated);
            hub.append(EventKind::Advanced, &id, &body, now_ms() as u64, [0u8; 32])
                .map_err(|e| HubHttpError::Io(format!("{e:?}")))?;
            Ok(merged)
        })
        .await?;
    Ok(Json(out))
}

pub fn routes(state: Shared) -> Router {
    Router::new()
        // The paths the storefront already calls. `{slug}` is accepted and
        // ignored: one hub is one venue, so the slug identifies nothing here --
        // it is kept so the same client works against a hub and against a
        // multi-tenant deployment without a second code path.
        .route("/api/public/locations/{slug}/menu", get(menu))
        .route("/api/public/locations/{slug}/orders", post(place))
        .route("/api/menu", get(menu))
        .route("/api/order/{id}", get(order))
        .route("/api/order/{id}/advance", post(advance))
        .with_state(state)
}

/// Seed a hub's catalogue from a bundle file.
///
/// This is how a hub gets its menu, and it is deliberately a separate step from
/// serving: a hub that has not been seeded answers "this hub has no venue yet"
/// rather than an empty menu, because an empty menu looks like a restaurant with
/// nothing to sell instead of one that has not been set up.
pub fn seed_catalog(dir: &Path, bundle: &Path) -> std::io::Result<(usize, usize)> {
    #[derive(Deserialize)]
    struct Bundle {
        location: Value,
        #[serde(default)]
        categories: Vec<Value>,
        #[serde(default)]
        products: Vec<Value>,
    }
    let raw = std::fs::read_to_string(bundle)?;
    let b: Bundle = serde_json::from_str(&raw).map_err(io_err)?;

    std::fs::create_dir_all(dir)?;
    let paths = HubPaths::under(dir);
    let mut cat = if paths.catalog.exists() {
        Catalog::load(&std::fs::read(&paths.catalog)?).map_err(io_err)?
    } else {
        Catalog::create().map_err(io_err)?
    };

    cat.set_location(&serde_json::to_string(&b.location).map_err(io_err)?);
    let mut nc = 0;
    for c in &b.categories {
        let Some(id) = c.get("id").and_then(|x| x.as_str()) else { continue };
        cat.set_category(id, &serde_json::to_string(c).map_err(io_err)?);
        nc += 1;
    }
    let mut np = 0;
    for p in &b.products {
        let Some(id) = p.get("id").and_then(|x| x.as_str()) else { continue };
        cat.set_product(id, &serde_json::to_string(p).map_err(io_err)?);
        np += 1;
    }
    let bytes = cat.to_bytes().map_err(io_err)?;
    std::fs::write(&paths.catalog, bytes)?;
    Ok((nc, np))
}
