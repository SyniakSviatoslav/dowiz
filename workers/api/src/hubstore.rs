//! Where this hub's image is parked, and the single place that knows.
//!
//! The DATA MODEL is bebop: an append-only event log, folded to get an order's
//! state. This module only answers "where do those bytes live", and it is the
//! one function to change when they move.
//!
//! Today the image sits in a single row of the `hub_image` table. That is SQL as
//! a BYTE CONTAINER, not as a data model -- nothing queries inside it, there are
//! no columns describing an order, and the whole row is read and written whole.
//! The destination the architecture wants is a Durable Object for single-writer
//! serialisation with the image in object storage, and that is a change to
//! `load`/`save` and nothing else.
//!
//! THE RACE, named rather than ignored. Two Workers reading the same image and
//! writing back would lose one of the two appends. `save` therefore writes with a
//! generation guard: the row moves only if it is still the generation that was
//! read. A caller that loses the guard re-reads and retries, which is correct for
//! an append-only log because the retry replays onto the newer image rather than
//! over it. A Durable Object removes the race entirely by serialising the
//! writers; until then the guard is what keeps an order from vanishing.

use dowiz_hub::catalog::Catalog;
use dowiz_hub::Hub;
use worker::wasm_bindgen::JsValue;
use worker::*;

/// Two images, one hub. A bebop store has ONE root, and the event log's root and
/// the KV root are different layouts -- they cannot share an image. The split
/// also matches their lifecycles: the log is append-only and grows, the
/// catalogue is small and rewritten whole.
const IMAGE_LOG: &str = "log";
const IMAGE_CATALOG: &str = "catalog";
/// The venue's own configuration, and the drafts the assistant writes.
///
/// A FOURTH AND FIFTH IMAGE rather than more keys in the catalogue, because
/// their lifecycles differ: the catalogue is read on every storefront request
/// and must stay small, while settings are read only by the owner and posts
/// grow with every draft. Sharing one image would make every menu read carry
/// both.
const IMAGE_SETTINGS: &str = "settings";
const IMAGE_POSTS: &str = "posts";
/// The ingredient ledger. Append-only like the order log and for the same
/// reason: a stock level is a FOLD over what happened to the shelf, not a
/// number somebody edits. The refusal when a basket cannot be made is computed
/// from this, so a second mutable count would be a second answer to "can the
/// kitchen make it".
const IMAGE_STOCK: &str = "stock";

pub struct Loaded {
    pub hub: Hub,
    /// The generation this image was read at. Passed back to `save`.
    pub generation: i64,
}

pub struct LoadedCatalog {
    pub catalog: Catalog,
    pub generation: i64,
}

/// D1 refuses any single value over one million bytes.
///
/// THE LIMIT IS NOT NEGOTIABLE AND THE IMAGES ARE BIGGER. A fresh order log is
/// 4 MiB of arena, so `save` failed on the FIRST order ever placed through this
/// Worker -- as a bare 500, with the order lost. The catalogue was the same
/// story at 1 MiB, which is how a 52-dish menu could not be seeded.
///
/// So an image is stored in CHUNKS, in the same table, under `<id>#<n>`. 900_000
/// leaves room for the row's other columns without arithmetic nobody will
/// re-check. Chunk zero keeps the original id, so an image small enough to fit
/// in one row is stored exactly as it was before this change.
const CHUNK: usize = 900_000;

/// Read every chunk of an image back into one buffer.
async fn load_bytes(db: &D1Database, id: &str) -> Result<Option<(Vec<u8>, i64)>> {
    #[derive(serde::Deserialize)]
    struct Row {
        image: Vec<u8>,
        generation: i64,
    }
    let first: Option<Row> = db
        .prepare("SELECT image, generation FROM hub_image WHERE id = ?1")
        .bind(&[id.into()])?
        .first(None)
        .await?;
    let Some(first) = first else { return Ok(None) };
    let generation = first.generation;
    let mut out = first.image;
    // Chunks are read until one is missing rather than by a stored count: a
    // count is a second fact that can disagree with the rows, and the rows are
    // the ones that decide whether the image loads.
    for n in 1.. {
        let key = format!("{id}#{n}");
        let row: Option<Row> = db
            .prepare("SELECT image, generation FROM hub_image WHERE id = ?1")
            .bind(&[key.into()])?
            .first(None)
            .await?;
        match row {
            Some(r) => out.extend_from_slice(&r.image),
            None => break,
        }
    }
    Ok(Some((out, generation)))
}

/// Read the hub image, creating a fresh one the first time.
pub async fn load(db: &D1Database) -> Result<Loaded> {
    struct Row {
        image: Vec<u8>,
        generation: i64,
    }
    let row: Option<Row> =
        load_bytes(db, IMAGE_LOG).await?.map(|(image, generation)| Row { image, generation });

    match row {
        Some(r) => {
            let hub = Hub::load(&r.image)
                // A corrupt image must not be silently replaced with an empty
                // one: that would present a hub with no orders as a healthy hub.
                .map_err(|_| Error::RustError("hub image is unreadable".into()))?;
            Ok(Loaded { hub, generation: r.generation })
        }
        None => {
            // BORN SMALL, GROWN AS NEEDED. `Hub::create` allocates 4 MiB, which
            // on a Worker means every single order read five D1 chunks and
            // wrote five back -- and the third order of the day exceeded the
            // isolate's resource limit with error 1102. The log grows itself
            // when an append does not fit (doubling, chain copied verbatim), so
            // starting at 64 KiB costs a handful of doublings over a hub's life
            // and keeps the common case one row.
            let hub = Hub::create_sized(64 * 1024)
                .map_err(|_| Error::RustError("cannot create hub image".into()))?;
            Ok(Loaded { hub, generation: 0 })
        }
    }
}

/// Write one image back, but only if nobody else has since. Returns `false` when
/// the guard rejected the write, which means "re-read and replay", not "failed".
async fn save_image(db: &D1Database, id: &str, bytes: Vec<u8>, generation: i64) -> Result<bool> {
    let next = generation + 1;

    // ── the tail chunks ──
    //
    // Written BEFORE the head, and the head carries the generation guard. If the
    // write is interrupted between the two, the head still points at the old
    // generation, so the image that loads is the old one plus some unreferenced
    // tail bytes -- wrong-but-stale rather than half-new. The other order would
    // publish a head whose tail had not arrived yet.
    //
    // Chunks the new image does not need are removed after the head lands, not
    // before: an image that shrank must not lose its tail while the head still
    // describes the longer one.
    let head_len = bytes.len().min(CHUNK);
    let mut n = 1usize;
    let mut at = head_len;
    while at < bytes.len() {
        let end = (at + CHUNK).min(bytes.len());
        let key = format!("{id}#{n}");
        db.prepare(
            "INSERT INTO hub_image (id, image, generation, updated_at_ms) VALUES (?1,?2,?3,?4) \
             ON CONFLICT(id) DO UPDATE SET image = excluded.image, \
             generation = excluded.generation, updated_at_ms = excluded.updated_at_ms",
        )
        .bind(&[
            key.into(),
            bytes_to_js(&bytes[at..end]),
            JsValue::from_f64(next as f64),
            JsValue::from_f64(Date::now().as_millis() as f64),
        ])?
        .run()
        .await?;
        at = end;
        n += 1;
    }
    let chunks_written = n;
    let bytes = bytes[..head_len].to_vec();

    let res = if generation == 0 {
        db.prepare(
            "INSERT INTO hub_image (id, image, generation, updated_at_ms) VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT(id) DO NOTHING",
        )
        .bind(&[
            id.into(),
            bytes_to_js(&bytes),
            JsValue::from_f64(next as f64),
            JsValue::from_f64(Date::now().as_millis() as f64),
        ])?
        .run()
        .await?
    } else {
        db.prepare(
            "UPDATE hub_image SET image = ?2, generation = ?3, updated_at_ms = ?4 \
             WHERE id = ?1 AND generation = ?5",
        )
        .bind(&[
            id.into(),
            bytes_to_js(&bytes),
            JsValue::from_f64(next as f64),
            JsValue::from_f64(Date::now().as_millis() as f64),
            JsValue::from_f64(generation as f64),
        ])?
        .run()
        .await?
    };
    let landed = res.meta()?.and_then(|m| m.changes).unwrap_or(0) > 0;
    if landed {
        // Now that the head describes the shorter image, the chunks past its end
        // are unreachable and can go.
        for extra in chunks_written..chunks_written + 8 {
            let key = format!("{id}#{extra}");
            let _ = db
                .prepare("DELETE FROM hub_image WHERE id = ?1")
                .bind(&[key.into()])?
                .run()
                .await;
        }
    }
    Ok(landed)
}

pub async fn save(db: &D1Database, loaded: &Loaded) -> Result<bool> {
    save_image(db, IMAGE_LOG, loaded.hub.to_bytes(), loaded.generation).await
}

/// Read the catalogue image, creating an empty one the first time.
pub async fn load_catalog(db: &D1Database) -> Result<LoadedCatalog> {
    struct Row {
        image: Vec<u8>,
        generation: i64,
    }
    let row: Option<Row> =
        load_bytes(db, IMAGE_CATALOG).await?.map(|(image, generation)| Row { image, generation });
    match row {
        Some(r) => {
            let catalog = Catalog::load(&r.image)
                .map_err(|_| Error::RustError("catalogue image is unreadable".into()))?;
            Ok(LoadedCatalog { catalog, generation: r.generation })
        }
        None => {
            let catalog =
                Catalog::create().map_err(|_| Error::RustError("cannot create catalogue".into()))?;
            Ok(LoadedCatalog { catalog, generation: 0 })
        }
    }
}

pub struct LoadedSettings {
    pub settings: dowiz_hub::settings::Settings,
    pub generation: i64,
}

pub async fn load_settings(db: &D1Database) -> Result<LoadedSettings> {
    match load_bytes(db, IMAGE_SETTINGS).await? {
        Some((image, generation)) => {
            let settings = dowiz_hub::settings::Settings::load(&image)
                .map_err(|_| Error::RustError("settings image is unreadable".into()))?;
            Ok(LoadedSettings { settings, generation })
        }
        None => Ok(LoadedSettings {
            settings: dowiz_hub::settings::Settings::create()
                .map_err(|_| Error::RustError("cannot create settings".into()))?,
            generation: 0,
        }),
    }
}

pub async fn with_settings<F, T>(db: &D1Database, mut f: F) -> Result<T>
where
    F: FnMut(&mut dowiz_hub::settings::Settings) -> Result<T>,
{
    for _ in 0..5 {
        let mut loaded = load_settings(db).await?;
        let out = f(&mut loaded.settings)?;
        let bytes = loaded
            .settings
            .to_bytes()
            .map_err(|e| Error::RustError(format!("settings serialise failed: {e:?}")))?;
        if save_image(db, IMAGE_SETTINGS, bytes, loaded.generation).await? {
            return Ok(out);
        }
    }
    Err(Error::RustError("settings image is contended".into()))
}

pub struct LoadedPosts {
    pub posts: dowiz_hub::post::Posts,
    pub generation: i64,
}

pub async fn load_posts(db: &D1Database) -> Result<LoadedPosts> {
    match load_bytes(db, IMAGE_POSTS).await? {
        Some((image, generation)) => {
            let posts = dowiz_hub::post::Posts::load(&image)
                .map_err(|_| Error::RustError("posts image is unreadable".into()))?;
            Ok(LoadedPosts { posts, generation })
        }
        None => Ok(LoadedPosts {
            posts: dowiz_hub::post::Posts::create()
                .map_err(|_| Error::RustError("cannot create posts".into()))?,
            generation: 0,
        }),
    }
}

pub async fn with_posts<F, T>(db: &D1Database, mut f: F) -> Result<T>
where
    F: FnMut(&mut dowiz_hub::post::Posts) -> Result<T>,
{
    for _ in 0..5 {
        let mut loaded = load_posts(db).await?;
        let out = f(&mut loaded.posts)?;
        let bytes = loaded
            .posts
            .to_bytes()
            .map_err(|e| Error::RustError(format!("posts serialise failed: {e:?}")))?;
        if save_image(db, IMAGE_POSTS, bytes, loaded.generation).await? {
            return Ok(out);
        }
    }
    Err(Error::RustError("posts image is contended".into()))
}

pub struct LoadedStock {
    pub stock: dowiz_hub::stock::StockLog,
    pub generation: i64,
}

pub async fn load_stock(db: &D1Database) -> Result<LoadedStock> {
    match load_bytes(db, IMAGE_STOCK).await? {
        Some((image, generation)) => {
            let stock = dowiz_hub::stock::StockLog::load(&image)
                .map_err(|_| Error::RustError("stock image is unreadable".into()))?;
            Ok(LoadedStock { stock, generation })
        }
        None => Ok(LoadedStock {
            // Born small and grown by the log itself, for the reason the order
            // log is: 8 MiB of arena would be nine D1 chunks read and written
            // on every single order.
            stock: dowiz_hub::stock::StockLog::create_sized(64 * 1024)
                .map_err(|_| Error::RustError("cannot create stock image".into()))?,
            generation: 0,
        }),
    }
}

pub async fn with_stock<F, T>(db: &D1Database, mut f: F) -> Result<T>
where
    F: FnMut(&mut dowiz_hub::stock::StockLog) -> Result<T>,
{
    for _ in 0..5 {
        let mut loaded = load_stock(db).await?;
        let out = f(&mut loaded.stock)?;
        if save_image(db, IMAGE_STOCK, loaded.stock.to_bytes(), loaded.generation).await? {
            return Ok(out);
        }
    }
    Err(Error::RustError("stock image is contended".into()))
}

/// Read, mutate, write the catalogue under the same generation guard.
pub async fn with_catalog<F, T>(db: &D1Database, mut f: F) -> Result<T>
where
    F: FnMut(&mut Catalog) -> Result<T>,
{
    for _ in 0..5 {
        let mut loaded = load_catalog(db).await?;
        let out = f(&mut loaded.catalog)?;
        let bytes = loaded
            .catalog
            .to_bytes()
            .map_err(|e| Error::RustError(format!("catalogue serialise failed: {e:?}")))?;
        if save_image(db, IMAGE_CATALOG, bytes, loaded.generation).await? {
            return Ok(out);
        }
    }
    Err(Error::RustError(
        "catalogue image is contended; five attempts lost the generation guard".into(),
    ))
}

fn bytes_to_js(b: &[u8]) -> JsValue {
    // D1 stores a Uint8Array as a BLOB.
    worker::js_sys::Uint8Array::from(b).into()
}

/// Read, mutate, write — retrying when the generation guard rejects the write.
///
/// Bounded at five attempts: an append-only log makes a replay safe, but an
/// unbounded retry would turn a hot hub into a livelock rather than an error.
/// How many times a promo code has been redeemed, folded from the orders.
///
/// No counter is stored, for the reason the analytics give: a tally kept beside
/// the orders is a second number that can disagree with them, and when they
/// disagree it is always the tally that is wrong. A rejected or cancelled order
/// gives its use back -- the venue never took the money, so holding a use
/// against the customer would charge them for a refusal.
pub fn promo_uses(hub: &Hub, code: &str) -> i64 {
    hub.orders()
        .iter()
        .filter(|ev| {
            let Ok(o) = serde_json::from_str::<serde_json::Value>(&ev.order_json) else {
                return false;
            };
            let st = o.get("status").and_then(|s| s.as_str());
            if matches!(st, Some("REJECTED" | "CANCELLED")) {
                return false;
            }
            o.get("promo").and_then(|p| p.get("code")).and_then(|c| c.as_str()) == Some(code)
        })
        .count() as i64
}

/// Every field the HUB owns, carried across a kernel transition.
///
/// THE KERNEL RETURNS ITS OWN ORDER and knows nothing about delivery, contact,
/// discounts, tips or timestamps, so anything not on this list is ERASED by the
/// next status change.
///
/// It lives here because there were THREE copies of it -- one in the owner's
/// action, one in the courier's, one implied by the storefront -- and they had
/// drifted. The courier's copy had eight fields and was missing
/// `created_at_ms`, which made every delivery invisible to the earnings fold
/// (its "today" filter compares against a timestamp that had become zero), and
/// missing `tip`, which quietly deleted the courier's own money on pickup.
///
/// A rule with three copies is three rules. This is the one.
pub const HUB_OWNED: &[&str] = &[
    "location_id",
    "contact",
    "fulfilment",
    "payment",
    "payment_status",
    "delivery_fee",
    "courier_id",
    "created_at_ms",
    "rejection_reason",
    "cash_collected",
    "courier_note",
    "scheduled_for_ms",
    "tip",
    "discount",
    "promo",
    "feedback",
    "assigned_at_ms",
    "accepted_at_ms",
    "total",
];

/// Copy `HUB_OWNED` from the order as it was onto the order the kernel returned.
pub fn carry_over(old: &serde_json::Value, updated: &mut serde_json::Value) {
    for k in HUB_OWNED {
        if let Some(v) = old.get(*k) {
            updated[*k] = v.clone();
        }
    }
}

pub async fn with_hub<F, T>(db: &D1Database, mut f: F) -> Result<T>
where
    F: FnMut(&mut Hub) -> Result<T>,
{
    for _ in 0..5 {
        let mut loaded = load(db).await?;
        let out = f(&mut loaded.hub)?;
        if save(db, &loaded).await? {
            return Ok(out);
        }
    }
    Err(Error::RustError(
        "hub image is contended; five attempts lost the generation guard".into(),
    ))
}
