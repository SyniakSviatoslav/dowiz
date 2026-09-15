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

use dowiz_core::order_machine::OrderStatus;
use dowiz_core::ports::notification::StatusMsg;
use dowiz_hub::catalog::Catalog;
use dowiz_hub::roster::Roster;
use dowiz_hub::settings::Settings;
use dowiz_hub::subs::Subs;
use dowiz_hub::zone::{self, Reach};
use dowiz_hub::{EventKind, Hub};
use dowiz_kernel::json_api;

use crate::notify::{esc, Telegram};

/// Where the hub keeps itself. Two files, because a bebop store has one root and
/// the log's layout and the KV layout cannot share it.
pub struct HubPaths {
    pub log: PathBuf,
    pub catalog: PathBuf,
    pub subs: PathBuf,
    pub roster: PathBuf,
    pub settings: PathBuf,
    /// Photographs. A DIRECTORY, not a store: see `dowiz_hub::media` for why
    /// blobs do not belong in a layout that rewrites itself on every write.
    pub media: PathBuf,
    pub posts: PathBuf,
    pub stock: PathBuf,
    /// The token signing key. A FILE and not a store, because it must be
    /// readable before any store is opened and must never travel with a backup
    /// of the data.
    pub key: PathBuf,
}

impl HubPaths {
    pub fn under(dir: &Path) -> Self {
        HubPaths {
            log: dir.join("orders.store"),
            catalog: dir.join("catalog.store"),
            subs: dir.join("subs.store"),
            roster: dir.join("roster.store"),
            settings: dir.join("settings.store"),
            media: dir.join("media"),
            posts: dir.join("posts.store"),
            stock: dir.join("stock.store"),
            key: dir.join("signing.key"),
        }
    }
}

/// The hub's state. The mutex is the single-writer guarantee: on one machine the
/// only way two writes race is if this process lets them.
pub struct HubState {
    paths: HubPaths,
    write_lock: Mutex<()>,
    /// `None` when no bot token is configured. A hub with no messenger is a
    /// working hub -- the web tracking page still shows every status -- so this
    /// is an absent channel, not a broken one.
    notify: Option<Arc<Telegram>>,
    /// Public origin, for the tracking link inside a message. Without it the
    /// message still says what happened; it just cannot offer a way back in.
    public_base: Option<String>,
    /// The word a staff member sends the bot to enrol their chat. Enrolment has
    /// to be gated on something: the bot's username is public, so without a
    /// shared secret anyone who finds it could subscribe to a restaurant's
    /// incoming orders, which carry the customer's name, phone and address.
    staff_code: Option<String>,
    /// The bot's @handle, for building `t.me` links. Separate from the token:
    /// the token must never leave this process, and the handle is public.
    bot_username: Option<String>,
    /// The HMAC key every bearer token is signed with. Held in memory for the
    /// process's life; rotating it invalidates every live token, which is the
    /// intended emergency behaviour.
    signing_key: Vec<u8>,
    /// Courier positions and shifts. In memory and gone on restart -- see
    /// `hubcourier`'s header for why that is the right shape for one and a
    /// stated compromise for the other.
    live: Mutex<crate::hubcourier::Live>,
    was_open: Mutex<Option<bool>>,
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
        if !paths.roster.exists() {
            let mut roster = Roster::create().map_err(io_err)?;
            let bytes = roster.to_bytes().map_err(io_err)?;
            std::fs::write(&paths.roster, bytes)?;
        }
        let signing_key = load_or_create_key(&paths.key)?;
        std::fs::create_dir_all(&paths.media)?;
        if !paths.stock.exists() {
            let log = dowiz_hub::stock::StockLog::create().map_err(io_err)?;
            std::fs::write(&paths.stock, log.to_bytes())?;
        }
        if !paths.posts.exists() {
            let mut ps = dowiz_hub::post::Posts::create().map_err(io_err)?;
            let bytes = ps.to_bytes().map_err(io_err)?;
            std::fs::write(&paths.posts, bytes)?;
        }
        if !paths.settings.exists() {
            let mut st = Settings::create().map_err(io_err)?;
            let bytes = st.to_bytes().map_err(io_err)?;
            std::fs::write(&paths.settings, bytes)?;
            // Settings hold provider tokens. Same mode as the signing key, and
            // for the same reason: on a VPS the file permission IS the
            // protection, since nothing here can be encrypted with a key that
            // does not live beside it.
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&paths.settings, std::fs::Permissions::from_mode(0o600))?;
            }
        }
        if !paths.subs.exists() {
            let mut subs = Subs::create().map_err(io_err)?;
            let bytes = subs.to_bytes().map_err(io_err)?;
            std::fs::write(&paths.subs, bytes)?;
        }
        Ok(Arc::new(HubState {
            paths,
            write_lock: Mutex::new(()),
            notify: Telegram::from_env().map(Arc::new),
            public_base: std::env::var("PUBLIC_BASE_URL").ok().filter(|v| !v.is_empty()),
            staff_code: std::env::var("TELEGRAM_STAFF_CODE").ok().filter(|v| !v.is_empty()),
            bot_username: std::env::var("TELEGRAM_BOT_USERNAME")
                .ok()
                .map(|v| v.trim_start_matches('@').to_string())
                .filter(|v| !v.is_empty()),
            signing_key,
            live: Mutex::new(Default::default()),
            // Whether the venue was open at the last check. In memory because
            // it exists only to spot the moment it flips, and a hub that just
            // restarted has no opinion about a transition it did not witness.
            was_open: Mutex::new(None),
        }))
    }

    pub(crate) fn read_log(&self) -> Result<Hub, HubHttpError> {
        let bytes = std::fs::read(&self.paths.log).map_err(|e| HubHttpError::Io(e.to_string()))?;
        Hub::load(&bytes).map_err(|_| HubHttpError::Corrupt("order log"))
    }

    pub(crate) fn read_catalog(&self) -> Result<Catalog, HubHttpError> {
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
    pub(crate) async fn with_log<F, T>(&self, f: F) -> Result<T, HubHttpError>
    where
        F: FnOnce(&mut Hub) -> Result<T, HubHttpError>,
    {
        let _guard = self.write_lock.lock().await;
        let mut hub = self.read_log()?;
        let out = f(&mut hub)?;
        atomic_write(&self.paths.log, &hub.to_bytes())?;
        Ok(out)
    }

    pub(crate) async fn with_catalog<F, T>(&self, f: F) -> Result<T, HubHttpError>
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

/// `Debug` is derived because these errors are LOGGED as well as returned: a
/// notification that could not read its roster must say why on stderr, and an
/// error type that cannot be printed turns that into silence.
#[derive(Debug)]
pub enum HubHttpError {
    NotFound(&'static str),
    /// No credential, or one that does not cover this thing.
    Unauthorized(&'static str),
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
            HubHttpError::Unauthorized(w) => (StatusCode::UNAUTHORIZED, w.to_string()),
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
    /// Micro-degrees, when the customer's browser supplied them.
    ///
    /// OPTIONAL, and the delivery-zone check is written around that: there is
    /// no geocoder here, so an address typed by someone who declined the
    /// location prompt has no position and cannot be checked. Those are
    /// accepted and flagged rather than refused.
    #[serde(default)]
    pub lat_udeg: Option<i64>,
    #[serde(default)]
    pub lon_udeg: Option<i64>,
}

#[derive(Deserialize)]
pub struct FulfilmentIn {
    pub kind: String,
    #[serde(default)]
    pub address: Option<AddressIn>,
    /// What the customer wants the venue to know. For a DELIVERY this rides on
    /// the address, where the courier reads it; a pickup has no address, so
    /// without this field the note was silently dropped -- the customer typed
    /// "I will be there at eight, under Ana" and nobody ever saw it.
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Deserialize)]
pub struct PlaceIn {
    pub items: Vec<LineIn>,
    /// When the customer wants it, in epoch milliseconds. Absent means now.
    #[serde(default)]
    pub scheduled_for_ms: Option<i64>,
    pub contact: ContactIn,
    pub fulfilment: FulfilmentIn,
    #[serde(default)]
    pub payment: Option<String>,
    /// A promo code as the customer typed it. Normalised and re-checked here;
    /// whatever the storefront showed as a preview is advisory.
    #[serde(default)]
    pub promo: Option<String>,
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
                        "imageUrl": p.get("imageUrl").cloned().unwrap_or(Value::Null),
                        // The real size, for the AR view. Absent means no AR
                        // button on that dish -- see the owner surface.
                        "sizeCm": p.get("sizeCm").cloned().unwrap_or(Value::Null),
                        // The choices a customer may make. Sent as data so the
                        // storefront renders the venue's own rules rather than
                        // a hardcoded set.
                        "modifierGroups": p.get("modifierGroups").cloned().unwrap_or(Value::Null),
                        "allergens": p.get("allergens").cloned().unwrap_or(Value::Null),
                        "tags": p.get("tags").cloned().unwrap_or(Value::Null)
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

    // ── open is DERIVED, and can only be narrowed by hand ──
    //
    // A manual flag alone means somebody has to remember at eleven at night.
    // The one they forget is the closing one, and the cost is orders arriving
    // at a dark kitchen. So: the schedule decides, a pause or a manual "closed"
    // can shut it early, and nothing can force it open outside its hours.
    //
    // A venue with NO schedule keeps working exactly as before, on the flag --
    // hours that must be configured before the venue can trade would be a
    // migration, not a feature.
    let paused = loc.get("delivery_paused").and_then(|x| x.as_i64()).unwrap_or(0) == 1;
    let manual = loc.get("status").and_then(|x| x.as_str()).unwrap_or("closed");
    let sched = loc
        .get("hours")
        .map(|h| dowiz_hub::hours::from_json(&h.to_string()))
        .unwrap_or_default();
    let tz: i64 = std::env::var("TZ_OFFSET_MINUTES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(120);
    let (weekday, minute) = dowiz_hub::hours::local_now(now_ms(), tz);
    let scheduled_open = sched.is_empty() || sched.is_open_at(weekday, minute);
    let next_open = sched.next_open(weekday, minute);

    let status = if paused || manual == "closed" || !scheduled_open {
        "closed"
    } else {
        manual
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
            "hours": loc.get("hours").cloned().unwrap_or(Value::Null),
            // WHEN, not just "no". A customer told only that a place is shut
            // goes somewhere else; one told it opens at eleven comes back.
            "nextOpen": next_open
                .map(|(d, m)| json!({ "weekday": d, "minute": m }))
                .unwrap_or(Value::Null),
            // Distinguishes "the owner shut it" from "it is outside hours", so
            // the storefront can say which.
            "closedReason": if paused { json!("paused") }
                else if manual == "closed" { json!("manual") }
                else if !scheduled_open { json!("hours") }
                else { Value::Null },
            "deliveryEta": loc.get("delivery_eta").cloned().unwrap_or(json!("30-45")),
            // Whether the customer may come and collect. The hub has accepted
            // pickup orders since the beginning; until now the storefront had
            // no way to offer one, so the capability was unreachable.
            "pickup": loc.get("pickup").and_then(Value::as_bool).unwrap_or(false),
            "deliveryFee": loc.get("delivery_fee").cloned().unwrap_or(json!(0)),
            "freeDeliveryThreshold": loc.get("free_delivery_threshold").cloned().unwrap_or(Value::Null),
            "minOrder": loc.get("min_order").cloned().unwrap_or(json!(0)),
            "currencyCode": loc.get("currency_code").cloned().unwrap_or(json!("ALL")),
            "menuVersion": loc.get("menu_version").cloned().unwrap_or(json!(1)),
            "supportedLocales": loc.get("supported_locales").cloned().unwrap_or(json!(["sq"])),
            "defaultLocale": loc.get("default_locale").cloned().unwrap_or(json!("sq")),
            // Whether a service area is configured at all. The storefront uses
            // this to decide whether asking for the customer's location is
            // worth the interruption -- a venue with no zones must not prompt.
            "hasDeliveryZones": !loc
                .get("delivery_zones")
                .map(|z| zone::from_json(&z.to_string()))
                .unwrap_or_default()
                .is_empty(),
            // The bot handle, so the storefront can offer a follow link. Absent
            // when no bot is configured, and the storefront then shows no
            // button -- rather than a link to a bot that does not exist.
            "telegramBot": st.bot_username.clone().map(Value::String).unwrap_or(Value::Null),
            // The venue's own colours, when they have set any. Absent means the
            // surfaces keep their shipped palette -- which is a complete,
            // contrast-checked theme in its own right, not a placeholder.
            "theme": loc.get("theme").cloned().unwrap_or(Value::Null)
        },
        "categories": cats
    })))
}

/// How many times a code has been redeemed, folded from the orders themselves.
///
/// No counter is stored. The reason is the one the analytics gave: a tally kept
/// beside the orders is a second number that can disagree with them, and when
/// they disagree it is always the tally that is wrong.
///
/// A rejected or cancelled order gives its use BACK. The venue never took the
/// money, so holding a use against the customer would charge them for an order
/// the kitchen refused.
pub(crate) fn promo_uses(hub: &Hub, code: &str) -> i64 {
    hub.orders()
        .iter()
        .filter(|ev| {
            let Ok(o) = serde_json::from_str::<Value>(&ev.order_json) else { return false };
            if matches!(o.get("status").and_then(Value::as_str), Some("REJECTED" | "CANCELLED")) {
                return false;
            }
            o.get("promo").and_then(|p| p.get("code")).and_then(Value::as_str) == Some(code)
        })
        .count() as i64
}

/// THE MONEY RULE, in one place: every price is re-derived from the catalogue
/// and the browser's `unit_price` is discarded.
///
/// Lifted out of `place` so the promo preview prices a basket the SAME way the
/// order does. A preview that ran its own arithmetic would eventually quote a
/// discount the order then refuses, and the customer would be right to call
/// that a bait.
fn price_lines(
    cat: &dowiz_hub::catalog::Catalog,
    items: &[LineIn],
) -> Result<(Vec<Value>, i64), HubHttpError> {
    let mut lines = Vec::with_capacity(items.len());
    let mut subtotal: i64 = 0;
    for it in items {
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
        // ── what the customer changed about the dish ──
        //
        // Validated and priced HERE. The basket arrives carrying option ids and
        // nothing else; a client that can name its own price will eventually be
        // asked to. A refusal names the group so the customer knows where to
        // look -- "choose 1 from Size" rather than "invalid order".
        let groups = dowiz_hub::modifiers::groups_of(&pj);
        let chosen = dowiz_hub::modifiers::price(&groups, &it.modifier_ids)
            .map_err(|e| HubHttpError::Invalid(format!("{}: {e}", p.get("name")
                .and_then(|n| n.as_str()).unwrap_or(&it.product_id))))?;
        let price = price.saturating_add(chosen.delta);
        if price < 0 {
            // A stack of negative deltas must not make a dish cost less than
            // nothing. Refusing beats handing the kernel a line that would owe
            // the customer money.
            return Err(HubHttpError::Invalid(format!(
                "those choices price {} below zero", it.product_id
            )));
        }

        subtotal += price * it.quantity;
        lines.push(json!({
            "product_id": it.product_id, "modifier_ids": it.modifier_ids,
            // The chosen options BY NAME, so the kitchen ticket reads in words
            // rather than in ids nobody can act on.
            "modifiers": chosen.chosen.iter()
                .map(|(id, name, delta)| json!({ "id": id, "name": name, "priceDelta": delta }))
                .collect::<Vec<_>>(),
            "quantity": it.quantity, "unit_price": price,
            // The name AS SOLD. Kept on the order rather than looked up later,
            // so renaming a dish -- or taking it off the menu -- does not
            // rewrite what a past order says was bought.
            "name": p.get("name").cloned().unwrap_or(Value::Null)
        }));
    }
    Ok((lines, subtotal))
}

#[derive(Deserialize)]
pub struct PromoCheckIn {
    pub code: String,
    pub items: Vec<LineIn>,
}

/// `POST /api/promo/check` — what would this code take off THIS basket?
///
/// The basket arrives as product ids and quantities, never as a subtotal: the
/// hub prices it with `price_lines`, the same function the order uses. A
/// preview that trusted a number from the browser would quote whatever the
/// browser asked for.
///
/// This is a PREVIEW and says so. It reads the use-count without the write
/// lock, so a code on its last use can be quoted here and refused at checkout.
/// The order is the authority; showing a stale preview costs a moment of
/// confusion, while taking this answer as binding would give the discount away
/// twice.
pub async fn promo_check(
    State(st): State<Shared>,
    Json(body): Json<PromoCheckIn>,
) -> Result<Json<Value>, HubHttpError> {
    use dowiz_hub::promo::{normalise, Promo, Refusal};

    let code = normalise(&body.code);
    let cat = st.read_catalog()?;
    let Some(p) = cat.promo(&code).as_deref().and_then(Promo::parse) else {
        return Err(HubHttpError::Invalid(Refusal::Unknown.as_str().into()));
    };
    let (_, subtotal) = price_lines(&cat, &body.items)?;
    let used = promo_uses(&st.read_log()?, &code);
    match p.redeem(subtotal, now_ms(), used) {
        Ok(cut) => Ok(Json(json!({
            "code": p.code, "discount": cut, "subtotal": subtotal, "total": subtotal - cut
        }))),
        Err(r) => Err(HubHttpError::Conflict(r.as_str().into())),
    }
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

    let (lines, subtotal) = price_lines(&cat, &body.items)?;
    // The code is looked up BEFORE the order is built, so an unknown code costs
    // the customer one refusal rather than a reserved basket. Whether it still
    // APPLIES is decided later, under the write lock, where the use-count
    // cannot move underneath the answer.
    let asked = body.promo.as_deref().map(dowiz_hub::promo::normalise).filter(|c| !c.is_empty());
    let promo = match &asked {
        None => None,
        Some(code) => match cat.promo(code).as_deref().and_then(dowiz_hub::promo::Promo::parse) {
            Some(p) => Some(p),
            None => {
                return Err(HubHttpError::Invalid(
                    dowiz_hub::promo::Refusal::Unknown.as_str().into(),
                ))
            }
        },
    };

    let min_order = loc.get("min_order").and_then(|x| x.as_i64()).unwrap_or(0);
    if subtotal < min_order {
        return Err(HubHttpError::Conflict("below minimum order".into()));
    }

    // THE FEE IS DECIDED BEFORE THE DISCOUNT, and on the undiscounted subtotal.
    // The other order would let a promo code quietly ADD a delivery fee by
    // pushing the basket back under the free-delivery threshold -- a customer
    // who applied a discount and watched the total go up would be right to
    // distrust the number. The venue gives away one delivery; it does not
    // spring a charge.
    let fee = match loc.get("free_delivery_threshold").and_then(|x| x.as_i64()) {
        Some(th) if subtotal >= th => 0,
        _ if body.fulfilment.kind == "pickup" => 0,
        _ => loc.get("delivery_fee").and_then(|x| x.as_i64()).unwrap_or(0),
    };

    // WILL THIS ACTUALLY BE DELIVERED? Checked here, before an order exists,
    // because the alternative is a courier sent forty minutes out of town and
    // every order behind it late. A refusal costs one sale; an accepted order
    // the venue cannot serve costs it the evening.
    let zones = loc
        .get("delivery_zones")
        .map(|z| zone::from_json(&z.to_string()))
        .unwrap_or_default();
    let point = body
        .fulfilment
        .address
        .as_ref()
        .and_then(|a| Some((a.lat_udeg?, a.lon_udeg?)));
    let reach = if body.fulfilment.kind == "delivery" {
        zone::reach(&zones, point)
    } else {
        // A pickup order is the customer's own journey; where they live is not
        // the venue's problem.
        Reach::Unrestricted
    };
    if let Reach::Outside { nearest_m } = reach {
        return Err(HubHttpError::Refused(format!(
            "outside the delivery area by about {} m",
            (nearest_m / 100) * 100
        )));
    }

    // A TIME THE VENUE CAN ACTUALLY HONOUR. Three bounds, each for a different
    // way this goes wrong:
    //   * in the past -- a clock skew or a stale form, and the kitchen would see
    //     an order that is already late the moment it arrives
    //   * inside the next few minutes -- indistinguishable from "now", and a
    //     scheduled order that is due immediately just confuses the queue
    //   * further out than a week -- a typo in a date field, and the order sits
    //     in the log for months looking live
    let created_at_ms = now_ms();
    let scheduled = match body.scheduled_for_ms {
        None => None,
        Some(t) => {
            const MIN_AHEAD_MS: i64 = 10 * 60 * 1000;
            const MAX_AHEAD_MS: i64 = 7 * 24 * 60 * 60 * 1000;
            if t < created_at_ms + MIN_AHEAD_MS {
                return Err(HubHttpError::Invalid(
                    "a scheduled order must be at least ten minutes ahead".into(),
                ));
            }
            if t > created_at_ms + MAX_AHEAD_MS {
                return Err(HubHttpError::Invalid("that is more than a week away".into()));
            }
            Some(t)
        }
    };

    let id = new_order_id();
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
    // The kernel returns its own lines, without the names. Put them back from
    // the lines we priced a moment ago.
    carry_item_names(&json!({ "items": lines }), &mut envelope);
    envelope["delivery_fee"] = json!(fee);
    envelope["total"] = json!(subtotal + fee);
    envelope["contact"] = json!({ "name": body.contact.name, "phone": body.contact.phone });
    envelope["fulfilment"] = json!({
        "kind": body.fulfilment.kind,
        "note": body.fulfilment.note.as_deref()
            .map(str::trim).filter(|n| !n.is_empty())
            .map(|n| json!(n)).unwrap_or(Value::Null),
        "address": body.fulfilment.address.as_ref().map(|a| json!({
            "line": a.line, "note": a.note,
            "lat_udeg": a.lat_udeg, "lon_udeg": a.lon_udeg
        })),
        "fee": fee
    });
    // An address the hub could NOT check is marked on the order, so the venue
    // sees it before dispatching rather than after. Silence here would present
    // an unverified address as a verified one.
    if reach == Reach::Unknown {
        envelope["delivery_area_unverified"] = json!(true);
    }
    envelope["payment"] = json!(body.payment.unwrap_or_else(|| "cash".into()));
    if let Some(t) = scheduled {
        envelope["scheduled_for_ms"] = json!(t);
    }

    // ── ingredients are reserved BEFORE the order exists ──
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
        .filter_map(|it| Some((cat.product(&it.product_id)?, it.quantity)))
        .collect();
    let reservations = dowiz_hub::stock::reservations_for(&id, &bom_lines);
    if !reservations.is_empty() {
        let evs = reservations.clone();
        st.with_stock(move |log| {
            log.append_all(&evs).map_err(|e| match e {
                dowiz_hub::stock::StockError::OutOfStock { item, .. } => {
                    // The customer is told WHICH ingredient, because "something
                    // is unavailable" sends them hunting through a basket.
                    HubHttpError::Conflict(format!("not enough {item} to make that right now"))
                }
                other => HubHttpError::Io(other.to_string()),
            })
        })
        .await?;
    }

    let ev_id = id.clone();
    // THE DISCOUNT IS DECIDED UNDER THE WRITE LOCK, beside the append that makes
    // it real. Counting the uses first and appending after would let two
    // customers redeem the last use of the same code in the same instant --
    // rare at one restaurant, and exactly the kind of rare that only shows up
    // as an unexplained loss.
    let placed = st
        .with_log(move |hub| {
            let mut envelope = envelope;
            if let Some(p) = promo {
                let used = promo_uses(hub, &p.code);
                let cut = p
                    .redeem(subtotal, created_at_ms, used)
                    .map_err(|r| HubHttpError::Conflict(r.as_str().into()))?;
                envelope["discount"] = json!(cut);
                envelope["promo"] = json!({ "code": p.code, "discount": cut });
                envelope["total"] = json!(subtotal - cut + fee);
            }
            let stored = serde_json::to_string(&envelope).unwrap_or(order_json);
            hub.append(EventKind::Placed, &ev_id, &stored, created_at_ms as u64, [0u8; 32])
                .map_err(|e| HubHttpError::Io(format!("{e:?}")))?;
            Ok(envelope)
        })
        .await;
    if placed.is_err() && !reservations.is_empty() {
        // The order did not survive; its ingredients must not stay held. A
        // failure here is loud rather than silent, because a stranded
        // reservation makes a kitchen believe it is out of something it has.
        let oid = id.clone();
        if let Err(e) = st
            .with_stock(move |log| {
                let led = log.ledger().map_err(|e| HubHttpError::Io(e.to_string()))?;
                let rel = dowiz_hub::stock::settle(&led, &oid, false);
                log.append_all(&rel).map_err(|e| HubHttpError::Io(e.to_string()))
            })
            .await
        {
            eprintln!("stock: could NOT release {id} after a failed placement: {e:?}");
        }
    }
    let mut envelope = placed?;

    // Only after the order is durable. Notifying first would let a crash between
    // the two produce a kitchen ticket for an order that does not exist.
    st.notify_placed(&envelope);

    // THE CUSTOMER'S KEY TO THEIR OWN ORDER, minted once, here, and returned
    // exactly once. It is scoped to this order and nothing else, so it cannot
    // be walked to a neighbour's; it is what the tracking page polls with and
    // what the browser keeps so "my orders" can exist without an account.
    //
    // Thirty days, because that is how long a person might reasonably come back
    // and ask what they ordered -- and because an order older than that is
    // history, not a live thing to watch.
    let now = now_ms();
    let customer_token = dowiz_hub::token::mint(
        st.signing_key(),
        &dowiz_hub::token::Claims {
            role: dowiz_hub::token::Role::Customer,
            // No account exists, so the subject is the order. There is
            // deliberately no customer registry: one would be a list of names,
            // phones and addresses that the venue does not need and cannot be
            // compelled to hand over if it is not there.
            subject: id.clone(),
            session: String::new(),
            scope: id.clone(),
            issued_ms: now,
            expires_ms: now + 30 * 24 * 60 * 60 * 1000,
        },
    );
    envelope["access_token"] = json!(customer_token);

    Ok(Json(envelope))
}

/// `GET /api/order/{id}` — one order, to whoever is entitled to it.
///
/// THIS USED TO BE PUBLIC, and that was capability-by-obscurity: an order id is
/// unguessable, so nobody could read a stranger's order in practice -- but the
/// order carries a name, a phone number and a home address, and "the URL is
/// hard to guess" is not an access rule. A link pasted into a chat, an id in a
/// server log, a screenshot of the tracking page: any of those handed the lot
/// over permanently.
///
/// Four parties may read an order, and no one else:
///   * the customer, holding the token minted when they placed it
///   * the venue owner
///   * the courier carrying it -- and only that courier
///   * anyone with the venue's staff Telegram binding, via the bot, which reads
///     the log directly and never comes through here
pub async fn order(
    State(st): State<Shared>,
    headers: axum::http::HeaderMap,
    AxPath(id): AxPath<String>,
) -> Result<Json<Value>, HubHttpError> {
    let hub = st.read_log()?;
    let raw = hub.order(&id).map_err(|_| HubHttpError::NotFound("order"))?;
    let envelope: Value = serde_json::from_str(&raw).unwrap_or(json!({}));

    let bearer = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|t| !t.is_empty());
    let Some(tok) = bearer else {
        return Err(HubHttpError::Unauthorized("this order needs the link you were given"));
    };
    let claims = dowiz_hub::token::verify(st.signing_key(), tok, now_ms())
        .map_err(|_| HubHttpError::Unauthorized("that link is no longer valid"))?;

    let allowed = match claims.role {
        // A customer token names exactly one order. It cannot be walked.
        dowiz_hub::token::Role::Customer => claims.scope == id,
        // Staff tokens must still be backed by a live session, or a logged-out
        // owner would keep reading orders until their token expired.
        dowiz_hub::token::Role::Owner => st
            .read_roster()?
            .session_owner(&claims.session)
            .as_deref()
            == Some(claims.subject.as_str()),
        dowiz_hub::token::Role::Courier => {
            let live = st.read_roster()?.session_owner(&claims.session).as_deref()
                == Some(claims.subject.as_str());
            // AND it has to be their run. A courier is not entitled to every
            // customer's address in the venue.
            live && envelope.get("courier_id").and_then(Value::as_str)
                == Some(claims.subject.as_str())
        }
        dowiz_hub::token::Role::Refresh => false,
    };
    if !allowed {
        return Err(HubHttpError::Unauthorized("that link is not for this order"));
    }
    Ok(Json(envelope))
}

#[derive(Deserialize)]
pub struct FeedbackIn {
    pub text: String,
}

/// `POST /api/order/{id}/feedback` — what the customer thought.
///
/// NO STARS. NO SCORE. NOT ON ANYONE.
///
/// The plan asked for a five-star rating here and flagged it against
/// NO-COURIER-SCORING, so this is the reconciliation rather than a shrug: a
/// number attached to an order is a number attached to whoever carried it the
/// moment anybody joins the two, and the join is one line of SQL nobody would
/// even notice writing. dowiz does not rank the people who work through it --
/// that is a D0 invariant, enforced in the kernel by omitting `Ord` from the
/// routing enums so a quality router is unrepresentable.
///
/// What survives is the part that was actually useful: a sentence, to the
/// venue, about one order. A kitchen can act on "the rice was cold"; it can do
/// nothing with a three.
///
/// Written ONCE. A comment the customer can rewrite is a comment the venue
/// cannot trust it read, and an endpoint that rewrites an order's fields on
/// demand is one a stranger with the link can use as an eraser.
pub async fn feedback(
    State(st): State<Shared>,
    headers: axum::http::HeaderMap,
    AxPath(id): AxPath<String>,
    Json(body): Json<FeedbackIn>,
) -> Result<Json<Value>, HubHttpError> {
    let text = body.text.trim().to_string();
    if text.is_empty() {
        return Err(HubHttpError::Invalid("say something, or say nothing".into()));
    }
    if text.chars().count() > 600 {
        return Err(HubHttpError::Invalid("that is longer than a note about an order".into()));
    }
    // The customer's own token for THIS order, and nothing else. An owner
    // leaving feedback on their own venue is not a thing worth supporting.
    let tok = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .ok_or(HubHttpError::Unauthorized("this order needs the link you were given"))?;
    let claims = dowiz_hub::token::verify(st.signing_key(), tok, now_ms())
        .map_err(|_| HubHttpError::Unauthorized("that link is no longer valid"))?;
    if claims.role != dowiz_hub::token::Role::Customer || claims.scope != id {
        return Err(HubHttpError::Unauthorized("that link is not for this order"));
    }

    let at = now_ms();
    st.with_log(move |hub| {
        let raw = hub.order(&id).map_err(|_| HubHttpError::NotFound("order"))?;
        let mut env: Value =
            serde_json::from_str(&raw).map_err(|_| HubHttpError::Corrupt("order"))?;
        if env.get("feedback").is_some() {
            return Err(HubHttpError::Conflict("you have already left a note".into()));
        }
        // Only once the order is over. Feedback on a pending order is a message
        // the kitchen needs NOW, and this is not a messaging channel -- saying
        // so beats letting it arrive somewhere nobody is watching.
        let status = env.get("status").and_then(Value::as_str).unwrap_or("");
        if !matches!(status, "DELIVERED" | "REJECTED" | "CANCELLED") {
            return Err(HubHttpError::Conflict(
                "this order is still running -- call the venue if something is wrong".into(),
            ));
        }
        env["feedback"] = json!({ "text": text, "at": at });
        let stored = serde_json::to_string(&env).unwrap_or(raw);
        hub.append(EventKind::Noted, &id, &stored, at as u64, [0u8; 32])
            .map_err(|e| HubHttpError::Io(format!("{e:?}")))?;
        Ok(Json(json!({ "ok": true })))
    })
    .await
}

/// Carry the hub's own fields across a kernel transition.
///
/// THE KERNEL OWNS items, status, subtotal and the ledger; it does not know
/// about addresses, phone numbers, couriers or what a dish is called, and it
/// returns an envelope without them. Everything the hub added has to be put
/// back, or it is lost the first time the order moves — an address that
/// disappears on "confirmed" only shows up at somebody's door.
///
/// ONE list, in one place. It was briefly three -- the public advance handler,
/// the owner's action and the courier's -- and three copies of a list of field
/// names is three chances to forget one. The one that gets forgotten is the one
/// nobody tests, which is why this is a function and not a convention.
pub(crate) fn carry_over(old: &Value, updated: &mut Value) {
    for k in [
        "contact",
        "fulfilment",
        "payment",
        "payment_status",
        "delivery_fee",
        "total",
        "courier_id",
        "created_at_ms",
        "rejection_reason",
        "cash_collected",
        "courier_note",
        "scheduled_for_ms",
        "last_actor",
        // THE DISCOUNT AND THE CODE THAT GAVE IT. Missing from this list until
        // now, and the consequence was not cosmetic: the use-count folds over
        // orders looking for `promo.code`, so the first status change erased
        // the evidence and a max-uses code became infinitely reusable. The
        // test that was supposed to catch it passed for the wrong reason --
        // it asserted the count went back to zero after a rejection, which it
        // did, because the field had been dropped rather than because the
        // rejection returned the use.
        "discount",
        "promo",
        // The customer's note. It is written after the order is over, so
        // nothing should follow it -- but "should" is how fields get lost.
        "feedback",
    ] {
        if let Some(v) = old.get(k) {
            updated[k] = v.clone();
        }
    }
    carry_item_names(old, updated);
}

/// Put the hub's own per-line fields back on the kernel's lines.
///
/// The kernel's order lines carry `product_id`, quantity and unit price — it has
/// no menu and no reason to. Everything the hub knows about a line and the
/// kernel does not is restored here: the dish's NAME, without which the kitchen
/// ticket reads "2x item-01", and the chosen MODIFIERS, without which it reads
/// "2x Sake Futomaki" for a customer who asked for no wasabi — a remake and an
/// apology.
///
/// ONE list, because this is the second field to go missing this way and there
/// will be a third. Matched BY product_id rather than by position, since the
/// kernel is free to reorder or merge lines and a positional match would put
/// one dish's choices on another's quantity.
fn carry_item_names(old: &Value, updated: &mut Value) {
    const LINE_FIELDS: [&str; 2] = ["name", "modifiers"];

    let Some(old_items) = old.get("items").and_then(Value::as_array) else { return };
    let Some(items) = updated.get_mut("items").and_then(Value::as_array_mut) else { return };
    for item in items {
        let Some(pid) = item.get("product_id").and_then(Value::as_str).map(str::to_string) else {
            continue;
        };
        let Some(src) = old_items
            .iter()
            .find(|o| o.get("product_id").and_then(Value::as_str) == Some(pid.as_str()))
        else {
            continue;
        };
        for f in LINE_FIELDS {
            if let Some(v) = src.get(f) {
                if !v.is_null() {
                    item[f] = v.clone();
                }
            }
        }
    }
}

// ── notification ─────────────────────────────────────────────────────────────

impl HubState {
    /// Read the subscription roster. Not under the write lock: readers do not
    /// need it, and holding it here would serialise every notification behind
    /// every order write.
    pub(crate) fn read_subs(&self) -> Result<Subs, HubHttpError> {
        let bytes = std::fs::read(&self.paths.subs).map_err(|e| HubHttpError::Io(e.to_string()))?;
        Subs::load(&bytes).map_err(|_| HubHttpError::Corrupt("subscriptions"))
    }

    async fn with_subs<F, T>(&self, f: F) -> Result<T, HubHttpError>
    where
        F: FnOnce(&mut Subs) -> Result<T, HubHttpError>,
    {
        let _guard = self.write_lock.lock().await;
        let mut subs = self.read_subs()?;
        let out = f(&mut subs)?;
        let bytes = subs.to_bytes().map_err(|e| HubHttpError::Io(format!("{e:?}")))?;
        atomic_write(&self.paths.subs, &bytes)?;
        Ok(out)
    }

    /// Bind a chat to an order, so its status changes reach that chat.
    pub async fn bind_order_chat(&self, order_id: &str, chat_id: &str) -> Result<(), HubHttpError> {
        // Refuse to bind an order that does not exist. Otherwise a stranger who
        // guesses at ids could park a chat on someone else's order and receive
        // its status changes.
        let hub = self.read_log()?;
        hub.order(order_id).map_err(|_| HubHttpError::NotFound("order"))?;
        let (o, c) = (order_id.to_string(), chat_id.to_string());
        self.with_subs(move |s| {
            s.bind_order(&o, &c);
            Ok(())
        })
        .await
    }

    /// Enrol a staff chat, if the caller knows the venue's enrolment word.
    ///
    /// Returns `Refused` rather than `NotFound` when the word is wrong, and the
    /// same when no word is configured at all -- a hub whose owner never set one
    /// must not be open to anyone who finds the bot.
    pub async fn bind_staff_chat(&self, chat_id: &str, code: &str) -> Result<(), HubHttpError> {
        match &self.staff_code {
            Some(expected) if constant_time_eq(expected.as_bytes(), code.as_bytes()) => {}
            _ => return Err(HubHttpError::Refused("wrong or unset enrolment code".into())),
        }
        let c = chat_id.to_string();
        self.with_subs(move |s| {
            s.bind_staff(&c);
            Ok(())
        })
        .await
    }

    pub async fn unbind_staff_chat(&self, chat_id: &str) -> Result<(), HubHttpError> {
        let c = chat_id.to_string();
        self.with_subs(move |s| {
            s.unbind_staff(&c);
            Ok(())
        })
        .await
    }

    /// Send, off the request path.
    ///
    /// A notification NEVER fails an order. The order is already durable in the
    /// log by the time this runs; making the customer's HTTP request wait on
    /// Telegram -- or worse, fail with it -- would trade a recorded order for a
    /// message. So this spawns, and a failure is logged with the reason and
    /// dropped.
    fn dispatch(&self, chats: Vec<String>, text: String) {
        let Some(tg) = self.notify.clone() else { return };
        if chats.is_empty() {
            return;
        }
        tokio::spawn(async move {
            for chat in chats {
                if let Err(e) = tg.send(&chat, &text).await {
                    // LOUD, per the repo's first law: a notification that
                    // silently did not go out is exactly the failure that looks
                    // like success. The chat id is printed; the token is not.
                    eprintln!("notify: send to chat {chat} FAILED: {e:?}");
                }
            }
        });
    }

    /// Tell the venue a new order arrived.
    pub(crate) fn notify_placed(&self, envelope: &Value) {
        if self.notify.is_none() {
            return;
        }
        let chats = match self.read_subs() {
            Ok(s) => s.staff(),
            Err(e) => {
                eprintln!("notify: cannot read subscriptions: {e:?}");
                return;
            }
        };
        self.dispatch(chats, staff_message(envelope));
    }

    /// Tell the customer their order moved.
    pub(crate) fn notify_advanced(&self, id: &str, envelope: &Value) {
        if self.notify.is_none() {
            return;
        }
        let chat = match self.read_subs() {
            Ok(s) => s.chat_for_order(id),
            Err(e) => {
                eprintln!("notify: cannot read subscriptions: {e:?}");
                return;
            }
        };
        // No binding is the normal case for a customer who never opened the
        // Telegram link. It is not an error and must not be logged as one.
        let Some(chat) = chat else { return };
        let Some(text) = customer_message(id, envelope, self.public_base.as_deref()) else {
            return;
        };
        self.dispatch(vec![chat], text);
    }
}

impl HubState {
    /// Interpret one message the venue's bot received, and answer it.
    ///
    /// This is where a chat becomes a subscription. The customer arrives through
    /// a `t.me/<bot>?start=<order_id>` deep link, so their very first message to
    /// the bot already carries the order they want to follow -- which is what
    /// makes the binding possible without ever asking them to type an id.
    ///
    /// Returns the reply to send, or `None` for a message that is not a command.
    /// Silence is the right answer to chatter: a bot that replies to everything
    /// trains people to mute it, and a muted bot delivers nothing.
    pub async fn handle_inbound_text(&self, chat_id: &str, text: &str) -> Option<String> {
        let text = text.trim();
        let (cmd, arg) = match text.split_once(char::is_whitespace) {
            Some((c, a)) => (c, a.trim()),
            None => (text, ""),
        };

        match cmd {
            // Telegram sends the deep-link payload as the argument to /start.
            "/start" if arg.starts_with("staff-") => {
                let code = &arg["staff-".len()..];
                match self.bind_staff_chat(chat_id, code).await {
                    Ok(()) => Some(
                        "✅ This chat will now receive new orders.\nSend /stop to leave.".into(),
                    ),
                    // Deliberately the same wording whether the code was wrong
                    // or none is configured: distinguishing them would tell an
                    // outsider whether a venue has enrolment switched on.
                    Err(_) => Some("That enrolment code is not valid.".into()),
                }
            }
            "/staff" if !arg.is_empty() => match self.bind_staff_chat(chat_id, arg).await {
                Ok(()) => Some("✅ This chat will now receive new orders.".into()),
                Err(_) => Some("That enrolment code is not valid.".into()),
            },
            "/start" if !arg.is_empty() => match self.bind_order_chat(arg, chat_id).await {
                Ok(()) => Some(format!(
                    "✅ You'll get updates for <code>{}</code> here.",
                    esc(arg)
                )),
                Err(HubHttpError::NotFound(_)) => Some("That order was not found.".into()),
                Err(e) => {
                    eprintln!("notify: bind for {arg} failed: {e:?}");
                    Some("Could not subscribe you just now. Please try again.".into())
                }
            },
            "/start" => Some(
                "Hi! Open a tracking link from your order to follow it here.".into(),
            ),
            "/stop" => {
                if let Err(e) = self.unbind_staff_chat(chat_id).await {
                    eprintln!("notify: unbind {chat_id} failed: {e:?}");
                    return Some("Could not unsubscribe just now.".into());
                }
                Some("You will no longer receive new orders here.".into())
            }
            "/status" if !arg.is_empty() => {
                let hub = self.read_log().ok()?;
                match hub.order(arg) {
                    Ok(raw) => {
                        let env: Value = serde_json::from_str(&raw).unwrap_or(json!({}));
                        customer_message(arg, &env, self.public_base.as_deref())
                            .or_else(|| Some("That order has no readable status.".into()))
                    }
                    Err(_) => Some("That order was not found.".into()),
                }
            }
            _ => None,
        }
    }

    /// Send a reply on the same chat. Separate from `handle_inbound_text` so the
    /// decision of what to say can be tested without a socket.
    pub fn reply(&self, chat_id: &str, text: String) {
        self.dispatch(vec![chat_id.to_string()], text);
    }
}

impl HubState {
    pub async fn set_position(&self, courier: &str, p: crate::hubcourier::Position) {
        self.live.lock().await.positions.insert(courier.to_string(), p);
    }

    pub async fn position(&self, courier: &str) -> Option<crate::hubcourier::Position> {
        self.live.lock().await.positions.get(courier).copied()
    }

    pub async fn set_shift(&self, courier: &str, open: bool) {
        let mut live = self.live.lock().await;
        if open {
            live.shifts.insert(courier.to_string(), true);
        } else {
            live.shifts.remove(courier);
            // Going off shift drops the last position too. Keeping it would
            // leave the owner looking at where someone was when they finished,
            // presented as where they are.
            live.positions.remove(courier);
        }
    }

    /// Who is on shift right now.
    pub async fn shifts(&self) -> Vec<String> {
        self.live.lock().await.shifts.keys().cloned().collect()
    }

    /// Add or replace a person on the roster.
    ///
    /// Adding an OWNER revokes every existing session for that id, so resetting
    /// a forgotten password actually locks out whoever was using the old one --
    /// which is the entire point of resetting it.
    pub async fn add_person(
        &self,
        id: &str,
        role: dowiz_hub::token::Role,
        name: &str,
        password: &str,
    ) -> std::io::Result<()> {
        let (id, name, password) = (id.to_string(), name.to_string(), password.to_string());
        self.with_roster(move |r| {
            r.revoke_all_for(&id);
            r.upsert_person(&id, role, &name, &password)
                .map_err(|e| HubHttpError::Io(format!("{e:?}")))
        })
        .await
        .map_err(|e| std::io::Error::other(format!("{e:?}")))
    }

    pub fn signing_key(&self) -> &[u8] {
        &self.signing_key
    }

    /// The venue's id, for the `user.locationId` the admin pane expects. Read
    /// from the catalogue rather than configured twice.
    pub fn location_id(&self) -> String {
        self.read_catalog()
            .ok()
            .and_then(|c| c.location())
            .and_then(|j| {
                serde_json::from_str::<Value>(&j)
                    .ok()?
                    .get("id")?
                    .as_str()
                    .map(str::to_string)
            })
            .unwrap_or_default()
    }

    /// Store an image and return the reference the catalogue keeps.
    ///
    /// Content-addressed, so storing the same photo twice writes nothing the
    /// second time -- the file is already there under the same name, with the
    /// same bytes, and rewriting it would only risk truncating a file a request
    /// is currently reading.
    pub fn put_media(&self, bytes: &[u8]) -> Result<dowiz_hub::media::Stored, HubHttpError> {
        let stored = dowiz_hub::media::prepare(bytes)
            .map_err(|e| HubHttpError::Invalid(e.to_string()))?;
        let path = self.paths.media.join(stored.filename());
        if !path.exists() {
            atomic_write(&path, bytes)?;
        }
        Ok(stored)
    }

    /// Read an image back by the name in its URL.
    pub fn get_media(&self, name: &str) -> Option<(Vec<u8>, dowiz_hub::media::Kind)> {
        // The name is validated by SHAPE before it touches the filesystem, so
        // no traversal can be expressed -- see `media::parse_name`.
        let (digest, kind) = dowiz_hub::media::parse_name(name)?;
        let path = self.paths.media.join(format!("{digest}.{}", kind.extension()));
        std::fs::read(path).ok().map(|b| (b, kind))
    }

    pub fn read_stock(&self) -> Result<dowiz_hub::stock::StockLog, HubHttpError> {
        let bytes = std::fs::read(&self.paths.stock).map_err(|e| HubHttpError::Io(e.to_string()))?;
        dowiz_hub::stock::StockLog::load(&bytes).map_err(|_| HubHttpError::Corrupt("stock log"))
    }

    pub async fn with_stock<F, T>(&self, f: F) -> Result<T, HubHttpError>
    where
        F: FnOnce(&mut dowiz_hub::stock::StockLog) -> Result<T, HubHttpError>,
    {
        let _guard = self.write_lock.lock().await;
        let mut log = self.read_stock()?;
        let out = f(&mut log)?;
        atomic_write(&self.paths.stock, &log.to_bytes())?;
        Ok(out)
    }

    pub fn read_posts(&self) -> Result<dowiz_hub::post::Posts, HubHttpError> {
        let bytes = std::fs::read(&self.paths.posts).map_err(|e| HubHttpError::Io(e.to_string()))?;
        dowiz_hub::post::Posts::load(&bytes).map_err(|_| HubHttpError::Corrupt("posts"))
    }

    pub async fn with_posts<F, T>(&self, f: F) -> Result<T, HubHttpError>
    where
        F: FnOnce(&mut dowiz_hub::post::Posts) -> Result<T, HubHttpError>,
    {
        let _guard = self.write_lock.lock().await;
        let mut ps = self.read_posts()?;
        let out = f(&mut ps)?;
        let bytes = ps.to_bytes().map_err(|e| HubHttpError::Io(format!("{e:?}")))?;
        atomic_write(&self.paths.posts, &bytes)?;
        Ok(out)
    }

    /// Was the venue closed when this was last looked at?
    ///
    /// `false` on the first call after a restart, so a hub coming up to an open
    /// venue does not announce a reopening that happened while it was down --
    /// or that never happened at all.
    pub fn was_closed(&self) -> bool {
        self.was_open.try_lock().map(|g| *g == Some(false)).unwrap_or(false)
    }

    pub async fn note_open_state(&self, open: bool) {
        *self.was_open.lock().await = Some(open);
    }

    pub fn read_settings(&self) -> Result<Settings, HubHttpError> {
        let bytes =
            std::fs::read(&self.paths.settings).map_err(|e| HubHttpError::Io(e.to_string()))?;
        Settings::load(&bytes).map_err(|_| HubHttpError::Corrupt("settings"))
    }

    pub async fn with_settings<F, T>(&self, f: F) -> Result<T, HubHttpError>
    where
        F: FnOnce(&mut Settings) -> Result<T, HubHttpError>,
    {
        let _guard = self.write_lock.lock().await;
        let mut st = self.read_settings()?;
        let out = f(&mut st)?;
        let bytes = st.to_bytes().map_err(|e| HubHttpError::Io(format!("{e:?}")))?;
        atomic_write(&self.paths.settings, &bytes)?;
        Ok(out)
    }

    pub fn read_roster(&self) -> Result<Roster, HubHttpError> {
        let bytes = std::fs::read(&self.paths.roster).map_err(|e| HubHttpError::Io(e.to_string()))?;
        let mut roster = Roster::load(&bytes).map_err(|_| HubHttpError::Corrupt("roster"))?;
        if let Some(n) = pbkdf2_override() {
            roster.set_iterations(n);
        }
        Ok(roster)
    }

    pub async fn with_roster<F, T>(&self, f: F) -> Result<T, HubHttpError>
    where
        F: FnOnce(&mut Roster) -> Result<T, HubHttpError>,
    {
        let _guard = self.write_lock.lock().await;
        let mut roster = self.read_roster()?;
        let out = f(&mut roster)?;
        let bytes = roster.to_bytes().map_err(|e| HubHttpError::Io(format!("{e:?}")))?;
        atomic_write(&self.paths.roster, &bytes)?;
        Ok(out)
    }
}

/// A lowered password-hashing cost, when one is configured.
///
/// EXISTS FOR TESTS, and says so out loud on every call rather than hiding in a
/// config file. An integration test cannot spend 600k iterations per login and
/// still be a test anyone runs; a PRODUCTION hub that sets this has weakened
/// every password on it, so it warns each time it is read instead of once at
/// startup, where the line would scroll away.
///
/// It cannot go below 1: a "KDF" with zero iterations is not a slow hash, it is
/// no hash.
fn pbkdf2_override() -> Option<u32> {
    let raw = std::env::var("HUB_PBKDF2_ITERATIONS").ok()?;
    let n: u32 = raw.trim().parse().ok()?;
    if n >= dowiz_hub::crypto::PBKDF2_ITERATIONS {
        return Some(n);
    }
    eprintln!(
        "[hub] WARNING: HUB_PBKDF2_ITERATIONS={n} is below the recommended {} -- \
         passwords on this hub are hashed more cheaply than they should be. \
         Unset it outside of tests.",
        dowiz_hub::crypto::PBKDF2_ITERATIONS
    );
    Some(n.max(1))
}

/// Read the hub's token signing key, creating one on first run.
///
/// GENERATED, not configured, so a hub that nobody set up is still safe rather
/// than signing with a default. `HUB_SIGNING_KEY` overrides it, for an operator
/// who wants the key in their own secret manager instead of on the disk.
///
/// Written 0600. It is the one file in the hub directory that must not travel
/// with a backup: holding it is enough to mint an owner token.
fn load_or_create_key(path: &Path) -> std::io::Result<Vec<u8>> {
    if let Ok(hexkey) = std::env::var("HUB_SIGNING_KEY") {
        let hexkey = hexkey.trim();
        if !hexkey.is_empty() {
            let key = dowiz_hub::crypto::unhex(hexkey).ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "HUB_SIGNING_KEY must be hex",
                )
            })?;
            if key.len() < 32 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "HUB_SIGNING_KEY must be at least 32 bytes",
                ));
            }
            return Ok(key);
        }
    }
    if path.exists() {
        let text = std::fs::read_to_string(path)?;
        return dowiz_hub::crypto::unhex(text.trim()).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "signing key file is not hex")
        });
    }
    let key = dowiz_hub::crypto::random_bytes(32)?;
    std::fs::write(path, dowiz_hub::crypto::hex(&key))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    eprintln!("[hub] minted a new token signing key at {}", path.display());
    Ok(key)
}

/// Compare two secrets without leaking their common prefix through timing.
///
/// The enrolment code is short and a bot can be messaged repeatedly, so a naive
/// `==` is a byte-at-a-time oracle. Length is allowed to differ observably --
/// that is not the part worth hiding.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b) {
        diff |= x ^ y;
    }
    diff == 0
}

/// What the kitchen needs to act, and nothing it does not.
///
/// This message DOES carry the customer's name, phone and address, unlike the
/// customer-facing one. That is not an inconsistency: the venue is already the
/// party fulfilling the order and has to have those to do it. The asymmetry is
/// the point -- the customer's own message carries no cart and no address,
/// because a phone on a table shows its notifications to whoever walks past.
pub fn staff_message(envelope: &Value) -> String {
    let id = envelope.get("id").and_then(Value::as_str).unwrap_or("?");
    let mut out = format!("🧾 <b>New order</b> <code>{}</code>", esc(id));

    if let Some(items) = envelope.get("items").and_then(Value::as_array) {
        for it in items {
            let qty = it.get("quantity").and_then(Value::as_i64).unwrap_or(1);
            let name = it
                .get("name")
                .and_then(Value::as_str)
                .or_else(|| it.get("product_id").and_then(Value::as_str))
                .unwrap_or("item");
            out.push_str(&format!("\n  {}× {}", qty, esc(name)));
            // The changes, under the dish, in words. A ticket that says
            // "2x Sake Futomaki" when the customer asked for no wasabi is a
            // remake and an apology.
            for m in it.get("modifiers").and_then(Value::as_array).into_iter().flatten() {
                if let Some(n) = m.get("name").and_then(Value::as_str) {
                    out.push_str(&format!("\n     · {}", esc(n)));
                }
            }
        }
    }
    if let Some(total) = envelope.get("total").and_then(Value::as_i64) {
        let cur = envelope
            .get("currency")
            .and_then(Value::as_str)
            .unwrap_or("ALL");
        out.push_str(&format!("\n<b>Total: {}</b>", money(total, cur)));
    }
    match envelope.get("fulfilment").and_then(|f| f.get("kind")).and_then(Value::as_str) {
        Some("delivery") => {
            let line = envelope
                .get("fulfilment")
                .and_then(|f| f.get("address"))
                .and_then(|a| a.get("line"))
                .and_then(Value::as_str)
                .unwrap_or("—");
            out.push_str(&format!("\n🛵 Delivery: {}", esc(line)));
            if let Some(note) = envelope
                .get("fulfilment")
                .and_then(|f| f.get("address"))
                .and_then(|a| a.get("note"))
                .and_then(Value::as_str)
                .filter(|n| !n.trim().is_empty())
            {
                out.push_str(&format!("\n  ↳ {}", esc(note)));
            }
        }
        _ => {
            out.push_str("\n🏠 Pickup");
            // The pickup note reaches the kitchen ticket, which is the only
            // place anybody would read it. It had nowhere to live until now.
            if let Some(note) = envelope
                .get("fulfilment")
                .and_then(|f| f.get("note"))
                .and_then(Value::as_str)
                .filter(|n| !n.trim().is_empty())
            {
                out.push_str(&format!("\n  ↳ {}", esc(note)));
            }
        }
    }
    if let Some(c) = envelope.get("contact") {
        let name = c.get("name").and_then(Value::as_str).unwrap_or("");
        let phone = c.get("phone").and_then(Value::as_str).unwrap_or("");
        out.push_str(&format!("\n☎️ {} {}", esc(name), esc(phone)));
    }
    if let Some(pay) = envelope.get("payment").and_then(Value::as_str) {
        out.push_str(&format!("\n💳 {}", esc(pay)));
    }
    out
}

/// What the customer is told.
///
/// The words come from the kernel's `StatusMsg`, not from this file. That is
/// deliberate: the advance handler already refuses to hold a second list of
/// statuses, and a notifier with its own copy of the status names would be
/// exactly that second list, one layer down.
///
/// `None` when the status is not one the kernel recognises -- better silent than
/// a message reading "your order is now ".
pub fn customer_message(id: &str, envelope: &Value, base: Option<&str>) -> Option<String> {
    let raw = envelope.get("status").and_then(Value::as_str)?;
    let status = OrderStatus::from_str(raw)?;
    let msg = StatusMsg::for_status(id, status);
    let mut out = format!("<b>{}</b>\n{}", esc(&msg.title), esc(&msg.body));
    if let Some(base) = base {
        out.push_str(&format!("\n\n{}/track?order={}", base.trim_end_matches('/'), esc(id)));
    }
    Some(out)
}

/// Minor units to a readable amount.
///
/// NO DECIMAL POINT, and that is not an oversight. The storefront, the admin
/// pane and the courier app all format these same integers with
/// `Intl.NumberFormat(..., { maximumFractionDigits: 0 })` -- the amount is
/// printed WHOLE, because the lek's minor unit is the lek. A `/ 100` here would
/// have put `30.00 ALL` on the kitchen's ticket for the order the customer was
/// shown as `3,000 ALL`: the same number, disagreeing with itself across two
/// screens, which is how a restaurant ends up arguing with a customer about a
/// price neither of them entered.
///
/// Integer arithmetic only -- MANIFESTO C2 bans floats on any path that touches
/// money, and "it is only for display" is how a float gets onto that path.
///
/// The CODE is printed, not a symbol, because the catalogue carries
/// `currencyCode` and a symbol table here would be a second currency authority.
fn money(minor: i64, currency: &str) -> String {
    let (sign, v) = if minor < 0 { ("-", minor.unsigned_abs()) } else { ("", minor as u64) };
    let digits = v.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            grouped.push('\u{202f}'); // narrow no-break space, as Intl uses
        }
        grouped.push(c);
    }
    format!("{sign}{grouped} {currency}")
}

// ── helpers ──────────────────────────────────────────────────────────────────

pub fn now_ms() -> i64 {
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
    let id_for_notify = id.clone();
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
            carry_over(&old, &mut merged);
            let body = serde_json::to_string(&merged).unwrap_or(updated);
            hub.append(EventKind::Advanced, &id, &body, now_ms() as u64, [0u8; 32])
                .map_err(|e| HubHttpError::Io(format!("{e:?}")))?;
            Ok(merged)
        })
        .await?;
    st.notify_advanced(&id_for_notify, &out);
    Ok(Json(out))
}

#[derive(Deserialize)]
pub struct SubscribeIn {
    pub chat_id: String,
}

#[derive(Deserialize)]
pub struct StaffSubscribeIn {
    pub chat_id: String,
    pub code: String,
}

/// Bind a chat to one order. Called by the messenger webhook after a customer
/// taps the tracking link, never by the storefront -- the browser does not know
/// a chat id and must not be able to assert one.
pub async fn subscribe_order(
    State(st): State<Shared>,
    AxPath(id): AxPath<String>,
    Json(body): Json<SubscribeIn>,
) -> Result<Json<Value>, HubHttpError> {
    st.bind_order_chat(&id, &body.chat_id).await?;
    Ok(Json(json!({ "subscribed": true, "order_id": id })))
}

pub async fn subscribe_staff(
    State(st): State<Shared>,
    Json(body): Json<StaffSubscribeIn>,
) -> Result<Json<Value>, HubHttpError> {
    st.bind_staff_chat(&body.chat_id, &body.code).await?;
    Ok(Json(json!({ "subscribed": true })))
}

pub async fn unsubscribe_staff(
    State(st): State<Shared>,
    Json(body): Json<SubscribeIn>,
) -> Result<Json<Value>, HubHttpError> {
    st.unbind_staff_chat(&body.chat_id).await?;
    Ok(Json(json!({ "subscribed": false })))
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
        .route("/api/promo/check", post(promo_check))
        .route("/api/order/{id}", get(order))
        .route("/api/order/{id}/advance", post(advance))
        .route("/api/order/{id}/subscribe", post(subscribe_order))
        .route("/api/order/{id}/feedback", post(feedback))
        .route("/api/hub/staff/subscribe", post(subscribe_staff))
        .route("/api/hub/staff/unsubscribe", post(unsubscribe_staff))
        .route("/media/{name}", get(media))
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

#[cfg(test)]
mod notify_tests {
    use super::*;

    fn tmpdir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("dowiz_hub_notify_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn sample_order() -> Value {
        json!({
            "id": "ord_0000000000001_0000abcd",
            "status": "PENDING",
            "items": [{ "product_id": "p1", "name": "Sushi set", "quantity": 2 }],
            "total": 3000,
            "payment": "cash",
            "contact": { "name": "Ana", "phone": "+355691234567" },
            "fulfilment": {
                "kind": "delivery",
                "address": { "line": "Rruga Taulantia 12", "note": "ring twice" }
            }
        })
    }

    /// The kitchen ticket must carry everything needed to cook and deliver.
    /// Each assertion here is a thing a courier would otherwise have to phone
    /// the restaurant to ask for.
    #[test]
    fn the_staff_ticket_carries_what_the_kitchen_needs() {
        let m = staff_message(&sample_order());
        assert!(m.contains("ord_0000000000001_0000abcd"), "the id: {m}");
        assert!(m.contains("2× Sushi set"), "quantity and item: {m}");
        assert!(m.contains("3\u{202f}000 ALL"), "the total, printed whole: {m}");
        assert!(m.contains("Rruga Taulantia 12"), "the address: {m}");
        assert!(m.contains("ring twice"), "the delivery note: {m}");
        assert!(m.contains("+355691234567"), "the phone: {m}");
        assert!(m.contains("cash"), "the payment method: {m}");
    }

    /// Money is integer minor units all the way to the screen. A float here
    /// would be the one place MANIFESTO C2 gets quietly violated "just for
    /// display".
    #[test]
    fn money_formats_without_floats() {
        // Whole units, grouped -- the same shape the storefront's
        // `Intl.NumberFormat(..., {maximumFractionDigits: 0})` produces. If this
        // ever grows a decimal point, the ticket and the customer's screen have
        // started disagreeing about the same order.
        assert_eq!(money(0, "ALL"), "0 ALL");
        assert_eq!(money(5, "ALL"), "5 ALL");
        assert_eq!(money(900, "ALL"), "900 ALL");
        assert_eq!(money(3000, "ALL"), "3\u{202f}000 ALL");
        assert_eq!(money(1_234_567, "ALL"), "1\u{202f}234\u{202f}567 ALL");
        assert_eq!(money(-250, "ALL"), "-250 ALL");
        assert_eq!(money(19999, "EUR"), "19\u{202f}999 EUR");
        // Neither end of the range may panic: the formatter runs on the
        // notification path, and a panic there takes the send task down with
        // it. `-i64::MIN` overflows, which is why the negation is unsigned.
        let _ = money(i64::MAX, "ALL");
        let _ = money(i64::MIN, "ALL");
    }

    /// A pickup order must not claim a delivery address it does not have.
    #[test]
    fn a_pickup_ticket_says_pickup() {
        let mut o = sample_order();
        o["fulfilment"] = json!({ "kind": "pickup" });
        let m = staff_message(&o);
        assert!(m.contains("Pickup"), "{m}");
        assert!(!m.contains("Rruga"), "no stale address: {m}");
    }

    /// The customer's message must NOT leak their own cart and address back at
    /// them on a lock screen. The venue's message carries those; this one does
    /// not, and that asymmetry is deliberate.
    #[test]
    fn the_customer_message_carries_no_pii_or_cart() {
        let mut o = sample_order();
        o["status"] = json!("CONFIRMED");
        let m = customer_message("ord_1", &o, Some("https://dubin.example")).expect("message");
        assert!(m.contains("Order confirmed"), "the kernel's own wording: {m}");
        assert!(!m.contains("Rruga Taulantia"), "no address: {m}");
        assert!(!m.contains("+355"), "no phone: {m}");
        assert!(!m.contains("Sushi"), "no cart: {m}");
        assert!(m.contains("https://dubin.example/track?order=ord_1"), "tracking link: {m}");
    }

    /// The status words come from the kernel. If this drifts, two lists of
    /// statuses exist and one of them is wrong.
    #[test]
    fn customer_wording_tracks_the_kernel() {
        for (status, expected) in [
            ("PENDING", "Order received"),
            ("PREPARING", "Preparing"),
            ("IN_DELIVERY", "On the way"),
            ("DELIVERED", "Delivered"),
        ] {
            let o = json!({ "status": status });
            let m = customer_message("ord_1", &o, None).unwrap_or_else(|| panic!("{status}"));
            let want = StatusMsg::for_status(
                "ord_1",
                OrderStatus::from_str(status).unwrap_or_else(|| panic!("{status} unknown")),
            );
            assert_eq!(want.title, expected, "the kernel's wording moved for {status}");
            assert!(m.contains(expected), "{status}: {m}");
        }
    }

    /// An unknown status produces NO message rather than a message with a hole
    /// in it. A customer reading "Your order is now" learns nothing and worries.
    #[test]
    fn an_unknown_status_sends_nothing() {
        assert!(customer_message("ord_1", &json!({ "status": "WAT" }), None).is_none());
        assert!(customer_message("ord_1", &json!({}), None).is_none());
    }

    #[test]
    fn constant_time_eq_is_still_correct() {
        assert!(constant_time_eq(b"secret", b"secret"));
        assert!(!constant_time_eq(b"secret", b"secreT"));
        assert!(!constant_time_eq(b"secret", b"secre"));
        assert!(constant_time_eq(b"", b""));
    }

    /// Enrolment must be refused when the venue never set a code. Otherwise a
    /// fresh hub broadcasts its customers' names, phones and addresses to
    /// whoever messages the bot first.
    #[tokio::test]
    async fn staff_enrolment_is_closed_by_default() {
        let dir = tmpdir("closed");
        let st = HubState::open(&dir).expect("open");
        assert!(st.staff_code.is_none(), "no code configured in this test");
        assert!(st.bind_staff_chat("99001", "").await.is_err());
        assert!(st.bind_staff_chat("99001", "anything").await.is_err());
        assert!(st.read_subs().expect("subs").staff().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Binding a chat to an order that does not exist must be refused, or a
    /// stranger guessing ids receives someone else's order updates.
    #[tokio::test]
    async fn an_unknown_order_cannot_be_subscribed_to() {
        let dir = tmpdir("unknown");
        let st = HubState::open(&dir).expect("open");
        match st.bind_order_chat("ord_nope", "99001").await {
            Err(HubHttpError::NotFound(_)) => {}
            other => panic!("expected NotFound, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The command surface, end to end against a real store: a placed order can
    /// be followed, a wrong code is refused, and /stop leaves.
    #[tokio::test]
    async fn the_command_surface_binds_and_unbinds() {
        let dir = tmpdir("commands");
        let mut st = HubState::open(&dir).expect("open");
        // A code, set the way a deployment would.
        Arc::get_mut(&mut st).expect("sole owner").staff_code = Some("letmein".into());

        // An order has to exist before it can be followed.
        let ev_id = "ord_test_0001".to_string();
        let ev_body = json!({ "id": &ev_id, "status": "PENDING" }).to_string();
        st.with_log(move |hub| {
            hub.append(EventKind::Placed, &ev_id, &ev_body, 1_700_000_000_000, [0u8; 32])
                .map_err(|e| HubHttpError::Io(format!("{e:?}")))
        })
        .await
        .expect("seed order");

        let r = st.handle_inbound_text("55501", "/start ord_test_0001").await.expect("reply");
        assert!(r.contains("ord_test_0001"), "{r}");
        assert_eq!(st.read_subs().unwrap().chat_for_order("ord_test_0001").as_deref(), Some("55501"));

        // A wrong code must not enrol, and must not say which half was wrong.
        let r = st.handle_inbound_text("99001", "/start staff-nope").await.expect("reply");
        assert!(r.contains("not valid"), "{r}");
        assert!(st.read_subs().unwrap().staff().is_empty());

        let r = st.handle_inbound_text("99001", "/start staff-letmein").await.expect("reply");
        assert!(r.contains("new orders"), "{r}");
        assert_eq!(st.read_subs().unwrap().staff(), vec!["99001"]);

        // /status answers from the log, not from a cache.
        let r = st.handle_inbound_text("55501", "/status ord_test_0001").await.expect("reply");
        assert!(r.contains("Order received"), "{r}");
        let r = st.handle_inbound_text("55501", "/status ord_missing").await.expect("reply");
        assert!(r.contains("not found"), "{r}");

        st.handle_inbound_text("99001", "/stop").await.expect("reply");
        assert!(st.read_subs().unwrap().staff().is_empty(), "/stop must actually stop");

        // Ordinary chatter gets no reply at all.
        assert!(st.handle_inbound_text("55501", "hello there").await.is_none());
        assert!(st.handle_inbound_text("55501", "two sushi please").await.is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// `GET /media/{name}` — serve a stored photograph.
///
/// PUBLIC, and it has to be: these are the pictures on a restaurant's menu.
/// What makes that safe is that the only thing addressable here is a sha256 of
/// bytes this hub itself stored, under a type its own sniffer decided.
///
/// `immutable` with a year's max-age, which content addressing earns honestly:
/// the bytes behind a digest cannot change, so a browser that has one never
/// needs to ask again. `nosniff` because the type came from the bytes and the
/// browser must not second-guess it.
pub async fn media(
    State(st): State<Shared>,
    AxPath(name): AxPath<String>,
) -> Response {
    match st.get_media(&name) {
        Some((bytes, kind)) => (
            [
                (axum::http::header::CONTENT_TYPE, kind.mime()),
                (axum::http::header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
                (axum::http::header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            ],
            bytes,
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "no such image").into_response(),
    }
}
