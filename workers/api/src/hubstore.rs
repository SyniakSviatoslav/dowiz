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

/// WHERE a hub's images live, decided WITHOUT TOUCHING ANYTHING.
///
/// The Durable Object is addressed by venue, so the venue has to be known
/// before the image can be asked for — and the venue used to come from a
/// database query, which would have put the read behind the very round trip
/// `owner_beside` just took it out from behind.
///
/// It does not have to. An owner's token already carries `active_location_id`,
/// and a storefront request names its venue in the URL. Both are readable with
/// no I/O at all.
///
/// A TOKEN CLAIM IS A HINT, NEVER AN AUTHORITY. It only picks which object to
/// ask; whether the caller may see what comes back is still re-derived from the
/// membership table on every request, exactly as before. A forged claim
/// therefore buys an attacker a read of an object they will then be refused —
/// and since `owner_beside` verifies the token's signature before it starts,
/// even that needs this Worker's signing key.
pub struct Place {
    /// The one venue whose images may still be seeded from the legacy
    /// `hub_image` table. `None` means no venue may. See `do_image`.
    pub legacy_venue: Option<String>,
    pub db: D1Database,
    pub ns: ObjectNamespace,
    pub venue: String,
}

/// The fallback name, for the paths that legitimately have neither a token nor
/// a slug — `bootstrap`, which is what CREATES the first venue. Named rather
/// than defaulted silently, so a route that lands here by accident is greppable.
pub const UNNAMED_VENUE: &str = "hub";

impl Place {
    /// Pure. No query, no fetch — see the type's header for why that matters.
    ///
    /// THE NAME IS ALWAYS THE LOCATION ID, never the slug, and the difference is
    /// not cosmetic: this venue's id is `dubin-durres` and its slug is
    /// `dubin-sushi`. Taking whichever was to hand would have given the
    /// storefront one object and the owner console another, and the two would
    /// have drifted apart one order at a time with nothing reporting it. The
    /// slug routes resolve theirs with `of_slug` instead.
    pub fn of(req: &Request, ctx: &RouteContext<()>, venue: Option<&str>) -> Result<Self> {
        let venue = venue
            .map(|v| v.to_string())
            .or_else(|| claimed_venue(req, ctx))
            .unwrap_or_else(|| UNNAMED_VENUE.to_string());
        Ok(Place { db: ctx.d1("DB")?, ns: ctx.durable_object("HUB")?, venue, legacy_venue: legacy_venue(ctx) })
    }

    /// For a caller who can name no venue at all.
    ///
    /// AN ORDER'S OWN LINK CARRIES NO TOKEN until the customer one is minted,
    /// and `/api/order/:id` is reached with neither a slug nor a claim. Falling
    /// back to the placeholder name sent it to an empty object, and the answer
    /// came back 404 — the order was not missing, we were asking the wrong hub.
    /// The live check caught it in the first minute; the caution is that a
    /// wrong-hub read looks exactly like an absent record.
    ///
    /// WHEN THERE IS MORE THAN ONE VENUE THIS IS GENUINELY UNANSWERABLE, and
    /// 404 becomes the right answer rather than a bug: an anonymous request that
    /// names no venue has not said enough to be given an order. `LIMIT 1` is
    /// honest only while there is one, which is why it reads the table rather
    /// than assuming, and why it is written here where the assumption is
    /// visible.
    ///
    /// THAT DAY HAS ARRIVED AND THE HOST IS THE ANSWER. A second venue makes
    /// `LIMIT 1` pick one of them by rowid, which is not a fallback but a
    /// coin toss that reads an unrelated restaurant's orders. Now that a client
    /// hub is `sushi-durres.dowiz.org`, an anonymous request DOES name its
    /// venue -- in the Host header -- so it is asked before the guess, and the
    /// guess survives only for the single-venue workers.dev deployment where it
    /// was true to begin with.
    pub async fn of_any(req: &Request, ctx: &RouteContext<()>) -> Result<Self> {
        if let Some(venue) = claimed_venue(req, ctx) {
            return Ok(Place { db: ctx.d1("DB")?, ns: ctx.durable_object("HUB")?, venue, legacy_venue: legacy_venue(ctx) });
        }
        if let Some(slug) = Self::slug_of_host(req, ctx) {
            return Self::of_slug(ctx, &slug).await;
        }
        // ── AND WHERE IT IS NOT TRUE, IT FAILS CLOSED ──
        //
        // The guess below is `SELECT id FROM locations LIMIT 1` with no
        // ORDER BY: whatever SQLite hands back. It is reached on the apex, on
        // `www.`, and on the `*.workers.dev` URL -- which is exactly the URL an
        // owner is shown when they open the console there and copy a webhook
        // address out of it. On a platform with two venues that guess files one
        // venue's customer messages under another venue's id, and hands a
        // validly-signed Stripe event to a hub that has never heard of the
        // order.
        //
        // So it is allowed only while it is TRUE: one venue, one answer. With
        // more than one the request is refused, because a wrong tenant is worse
        // than no answer, and the caller is told which header would have
        // settled it.
        #[derive(serde::Deserialize)]
        struct Row {
            id: String,
        }
        let db = ctx.d1("DB")?;
        let rows: Vec<Row> =
            db.prepare("SELECT id FROM locations LIMIT 2").all().await?.results()?;
        let venue = match rows.len() {
            0 => UNNAMED_VENUE.to_string(),
            1 => rows.into_iter().next().map(|r| r.id).unwrap_or_else(|| UNNAMED_VENUE.to_string()),
            _ => {
                return Err(Error::RustError(
                    "this platform has more than one venue and this request named none: \
                     use the venue's own host, or a token that carries its id"
                        .into(),
                ))
            }
        };
        Ok(Place { db, ns: ctx.durable_object("HUB")?, venue, legacy_venue: legacy_venue(ctx) })
    }

    /// The venue a public URL names, by its slug.
    ///
    /// ONE QUERY, and a bridge rather than a fixture: it reads the `locations`
    /// table, which is the next thing to move out of SQL. It is here because a
    /// customer's request carries no token and therefore no id, and guessing
    /// that the slug IS the id is exactly the drift described above.
    pub async fn of_slug(ctx: &RouteContext<()>, slug: &str) -> Result<Self> {
        #[derive(serde::Deserialize)]
        struct Row {
            id: String,
        }
        let db = ctx.d1("DB")?;
        let row: Option<Row> = db
            .prepare("SELECT id FROM locations WHERE slug = ?1 LIMIT 1")
            .bind(&[slug.into()])?
            .first(None)
            .await?;
        let venue = row.map(|r| r.id).unwrap_or_else(|| UNNAMED_VENUE.to_string());
        Ok(Place { db, ns: ctx.durable_object("HUB")?, venue, legacy_venue: legacy_venue(ctx) })
    }

    /// The venue an AUTHORISED location id names.
    ///
    /// WHY THIS EXISTS. Several owner writers authorise against
    /// `body.location_id` -- `owner_at` checks the membership for exactly that
    /// venue -- and then build their `Place` with `of_any`, which resolves the
    /// venue from the TOKEN's claim. For an owner of one venue the two agree;
    /// for an owner of two they can differ, and then the check passes for
    /// venue B while the write lands in venue A. Nothing announces it: both
    /// are the owner's own venues, so no permission was crossed, and the
    /// catalogue simply changes in the wrong restaurant.
    ///
    /// The id is one this code has already checked. It is not a slug and it is
    /// not read from a URL.
    pub fn of_authorised(ctx: &RouteContext<()>, location_id: &str) -> Result<Self> {
        Ok(Place {
            db: ctx.d1("DB")?,
            ns: ctx.durable_object("HUB")?,
            venue: location_id.to_string(),
            legacy_venue: legacy_venue(ctx),
        })
    }

    /// The venue a request's Host header names, by slug.
    ///
    /// ONE CLIENT, ONE SUBDOMAIN: `sushi-durres.dowiz.org` is that venue's hub,
    /// and the apex is the platform. Before this, a storefront named its venue
    /// with `?s=<slug>` -- a query parameter a customer can edit, that makes
    /// every venue share one origin, and that reads like a debug handle on a
    /// link a restaurant prints on a receipt.
    ///
    /// ONLY A SUBDOMAIN OF THE PLATFORM DOMAIN COUNTS, and that restriction is
    /// load-bearing rather than tidy. Taking "the first label" of any host would
    /// read `dowiz-api.sviatoslavsyniak.workers.dev` as a venue called
    /// `dowiz-api` and send every request on the workers.dev URL to a hub that
    /// does not exist. So the host must END with the platform domain, and the
    /// part in front of it must be a single label.
    ///
    /// `www` and the apex are the platform itself, never a venue.
    pub fn slug_of_host(req: &Request, ctx: &RouteContext<()>) -> Option<String> {
        let platform = ctx
            .var("PLATFORM_HOST")
            .map(|v| v.to_string())
            .unwrap_or_else(|_| "dowiz.org".to_string());
        let host = req.headers().get("host").ok().flatten()?;
        // A Host may carry a port, and it is not part of the name.
        let host = host.split(':').next()?.to_ascii_lowercase();
        if host == platform || host == format!("www.{platform}") {
            return None;
        }
        let sub = host.strip_suffix(&format!(".{platform}"))?;
        if sub.is_empty() || sub.contains('.') || sub == "www" {
            return None;
        }
        Some(sub.to_string())
    }

    /// Is this request addressed to the platform itself rather than to a venue?
    pub fn is_platform_host(req: &Request, ctx: &RouteContext<()>) -> bool {
        let platform = ctx
            .var("PLATFORM_HOST")
            .map(|v| v.to_string())
            .unwrap_or_else(|_| "dowiz.org".to_string());
        match req.headers().get("host").ok().flatten() {
            Some(h) => {
                let h = h.split(':').next().unwrap_or("").to_ascii_lowercase();
                h == platform || h == format!("www.{platform}")
            }
            None => false,
        }
    }

    pub(crate) fn stub(&self) -> Result<Stub> {
        self.ns.id_from_name(&self.venue)?.get_stub()
    }
}

/// The one venue allowed to adopt an image from the legacy `hub_image` table.
///
/// Absent by default, and absent means nobody. See `do_image` for why the safe
/// direction is "no venue" rather than "the first row".
fn legacy_venue(ctx: &RouteContext<()>) -> Option<String> {
    ctx.var("LEGACY_VENUE").ok().map(|v| v.to_string()).filter(|v| !v.is_empty())
}

/// The venue named by the caller's own token, if the token is genuine.
///
/// The signature IS checked here, because an unverified claim would let anyone
/// choose which venue's object this Worker wakes up and reads.
fn claimed_venue(req: &Request, ctx: &RouteContext<()>) -> Option<String> {
    let token = crate::auth::bearer(req).ok()?;
    match crate::auth::verify(&ctx.env, &token, Date::now().as_millis() as i64).ok()? {
        crate::auth::Claims::Owner { active_location_id, .. } => active_location_id,
        crate::auth::Claims::Courier { active_location_id, .. } => Some(active_location_id),
        crate::auth::Claims::Customer { location_id, .. } => Some(location_id),
    }
}

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

/// D1 HANDS A BLOB TO JAVASCRIPT AS AN ARRAY OF NUMBERS — one JS value per byte.
///
/// THIS WAS THE 503. The hub log had grown to 524,288 bytes, so every request
/// that read it asked wasm-bindgen to walk half a million JsValues across the
/// JS/WASM boundary and allocate a handle for each. Cloudflare answered `503
/// error 1102`, "Worker exceeded resource limits", for every route that touches
/// an image, while `/healthz`, which touches none, kept answering 200 in 200 ms
/// — which is what made it look like an outage rather than a cost.
///
/// `hex()` makes it ONE string per column. SQLite has no base64 and hex doubles
/// the wire bytes; that trade is not close, because the expensive thing here is
/// the crossing, not the byte.
///
/// READ IN SLICES because a single D1 value may not exceed one million bytes —
/// the same limit `CHUNK` exists for — and `hex()` of a whole chunk would be
/// 1.8 MB of it. `substr` on a BLOB counts BYTES, and the slicing happens
/// inside SQLite, so the oversized value is never built in the first place.
const SLICE: usize = 250_000;
const SLICES: usize = 4;
/// If `CHUNK` is ever raised past what the slices cover, the tail of every
/// chunk would be silently dropped — a corrupt store that reads as a bebop
/// parse failure a long way from here. The build stops instead.
const _: () = assert!(SLICE * SLICES >= CHUNK);

/// Hex back to bytes, appended to `out`.
///
/// Returns false rather than guessing on anything that is not hex: a store that
/// half-decodes is worse than one that refuses, because the refusal names the
/// image while a bad byte surfaces as an unreadable arena.
fn from_hex(s: &str, out: &mut Vec<u8>) -> bool {
    fn nibble(c: u8) -> Option<u8> {
        match c {
            b'0'..=b'9' => Some(c - b'0'),
            b'a'..=b'f' => Some(c - b'a' + 10),
            b'A'..=b'F' => Some(c - b'A' + 10),
            _ => None,
        }
    }
    let b = s.as_bytes();
    if b.len() % 2 != 0 {
        return false;
    }
    out.reserve(b.len() / 2);
    for pair in b.chunks_exact(2) {
        match (nibble(pair[0]), nibble(pair[1])) {
            (Some(hi), Some(lo)) => out.push((hi << 4) | lo),
            _ => return false,
        }
    }
    true
}

/// Read one or more images, WHOLE, IN ONE QUERY.
///
/// MEASURED, AND IT WAS THE WHOLE COST. The previous version fetched chunk
/// zero, then probed for chunk one, then chunk two, until a query came back
/// empty -- so a single-chunk image took TWO round trips and the second was
/// always a miss. A handler needing both the log and the catalogue therefore
/// paid four. At D1's ~150 ms from this Worker that is 600 ms of waiting, which
/// matched the measurements almost exactly: dashboard 580 ms of server time,
/// analytics 680, against a 180 ms network baseline.
///
/// One query now, whatever the chunk count. The rows still decide the truth --
/// there is no stored count that could disagree with them -- but they are all
/// asked for at once.
/// One image, from the venue's Durable Object — SEEDING IT FROM D1 the first
/// time and only the first time.
///
/// THE MIGRATION IS A READ, NOT A SCRIPT. A one-shot job that moved every image
/// would have a window in which the old store had been read and the new one not
/// yet written, and would need to be run exactly once against exactly the right
/// rows. Doing it on the first miss instead means the copy happens under the
/// object's own serialisation, for the venue being asked about, and a venue
/// nobody has opened yet is simply not migrated until somebody does.
///
/// D1 STAYS AUTHORITATIVE UNTIL IT IS EMPTY OF MEANING, which is what makes
/// this safe to deploy against a pilot's live order log: if this path is
/// reverted, every byte is still in `hub_image` where it was. The seed writes
/// at generation zero, so the object's first write lands at one and the guard
/// behaves from there exactly as the D1 guard did.
async fn do_image(place: &Place, id: &str) -> Result<Option<(Vec<u8>, i64)>> {
    let stub = place.stub()?;
    let mut res = stub.fetch_with_str(&format!("https://hub/img/{id}")).await?;
    if res.status_code() == 200 {
        let generation = res
            .headers()
            .get("x-generation")
            .ok()
            .flatten()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        // ONE BULK COPY. A response body crosses the boundary as bytes, which is
        // the whole difference from D1 handing a BLOB over as one JS value per
        // byte -- see `SLICE` for what that cost.
        return Ok(Some((res.bytes().await?, generation)));
    }
    // 204: the object has never seen this image.
    //
    // ONLY THE LEGACY VENUE MAY ADOPT THE D1 IMAGE, and this scoping is the
    // difference between a migration and a data leak. `hub_image` is keyed by
    // IMAGE ID ALONE -- `catalog`, `log`, `settings`, `stock` -- because it was
    // written when there was one venue and the venue therefore needed no name.
    // Unscoped, the rule "an object that has never held this image seeds itself
    // from D1" means EVERY VENUE CREATED FROM NOW ON adopts the first venue's
    // catalogue, order log and settings on its first read. It is not a
    // hypothetical: the second venue on this platform came up serving the
    // first's 52 dishes, and its storefront showed them to the public.
    //
    // So the fallback applies to exactly one venue, named in `LEGACY_VENUE`, and
    // when that is unset there is NO fallback at all. Fail-closed: a venue that
    // wrongly starts empty is a menu an owner re-enters, and a venue that
    // wrongly starts full is one tenant's data served from another's hostname.
    let legacy = place.legacy_venue.as_deref();
    if legacy != Some(place.venue.as_str()) {
        return Ok(None);
    }
    let Some((bytes, _)) = load_images_d1(&place.db, &[id]).await?.remove(id) else {
        return Ok(None);
    };
    let mut req = Request::new_with_init(
        &format!("https://hub/img/{id}"),
        RequestInit::new().with_method(Method::Put).with_body(Some(bytes.clone().into())),
    )?;
    req.headers_mut()?.set("x-generation", "0")?;
    let seeded = stub.fetch_with_request(req).await?;
    if seeded.status_code() == 409 {
        // Another request seeded it between our read and our write. Theirs is
        // the same bytes; take what the object now holds rather than fight.
        // A LOOP AND NOT A RECURSIVE CALL: an async fn that awaits itself needs
        // boxing, and a boxed future here would allocate on the path this whole
        // change exists to make cheap.
        let mut again = stub.fetch_with_str(&format!("https://hub/img/{id}")).await?;
        if again.status_code() == 200 {
            let generation = again
                .headers()
                .get("x-generation")
                .ok()
                .flatten()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            return Ok(Some((again.bytes().await?, generation)));
        }
        return Err(Error::RustError(format!(
            "image {id} was seeded by another request and is now unreadable"
        )));
    }
    Ok(Some((bytes, 1)))
}

/// Several images, each from the object. Kept as a map so the callers above are
/// unchanged from when this was one query.
async fn load_images(
    place: &Place,
    ids: &[&str],
) -> Result<std::collections::HashMap<String, (Vec<u8>, i64)>> {
    // CONCURRENTLY. They are separate keys in the same object and nothing here
    // depends on anything there; `load_both` exists precisely because paying
    // for them one after the other was the cost.
    let mut out = std::collections::HashMap::new();
    let fetched = futures_util::future::join_all(ids.iter().map(|id| do_image(place, id))).await;
    for (id, got) in ids.iter().zip(fetched) {
        if let Some(v) = got? {
            out.insert((*id).to_string(), v);
        }
    }
    Ok(out)
}

/// The D1 reader, kept for exactly one job: seeding an object that has never
/// held this image. Nothing else calls it, and when every venue has been opened
/// once it and the `hub_image` table can go.
async fn load_images_d1(
    db: &D1Database,
    ids: &[&str],
) -> Result<std::collections::HashMap<String, (Vec<u8>, i64)>> {
    /// The slices are SPELLED OUT rather than collected with `#[serde(flatten)]`
    /// into a map. Flatten needs `deserialize_any`, which serde-wasm-bindgen
    /// supports only partially, and a deserialiser that fails here fails for
    /// every image at once on a surface that cannot be tested from this box.
    /// Four named fields cannot do that, and the assert below is what keeps them
    /// honest with `SLICES`.
    #[derive(serde::Deserialize)]
    struct Row {
        id: String,
        generation: i64,
        h0: String,
        h1: String,
        h2: String,
        h3: String,
    }
    const _: () = assert!(SLICES == 4, "Row has exactly this many hN fields");
    // `id = ?n OR id LIKE ?n || '#%'` per image. Built rather than fixed
    // because the caller decides how many it needs, and a query per image is
    // the thing being removed.
    let mut wheres = Vec::new();
    let mut binds: Vec<JsValue> = Vec::new();
    for (i, id) in ids.iter().enumerate() {
        wheres.push(format!("id = ?{n} OR id LIKE ?{n} || '#%'", n = i + 1));
        binds.push((*id).into());
    }
    // `substr` on a BLOB counts BYTES and is 1-based, so slice k starts at
    // k*SLICE+1. Past the end it yields an empty blob, and `hex` of that is the
    // empty string -- so a short image simply has empty trailing slices and
    // needs no length column that could disagree with the bytes.
    let cols: String = (0..SLICES)
        .map(|k| {
            format!(
                ", ifnull(hex(substr(image, {start}, {SLICE})), '') AS h{k}",
                start = k * SLICE + 1
            )
        })
        .collect();
    let sql = format!(
        "SELECT id, generation{cols} FROM hub_image WHERE {}",
        wheres.join(" OR ")
    );
    let rows: Vec<Row> = db.prepare(&sql).bind(&binds)?.all().await?.results()?;

    // DECODED AS EACH ROW IS CONSUMED, so a row's hex is freed before the next
    // row's is touched. Holding all of them and decoding afterwards would put
    // every image's hex -- twice the bytes of every image, by definition -- in
    // the isolate at once, which is the cost this whole change exists to avoid
    // paying. `into_iter` on the array moves each slice out and drops it at the
    // end of its own iteration.
    let mut by_base: std::collections::HashMap<String, Vec<(usize, Vec<u8>, i64)>> =
        std::collections::HashMap::new();
    for r in rows {
        let Some((base, n)) = ids.iter().find_map(|base| {
            if r.id == **base {
                Some(((*base).to_string(), 0usize))
            } else {
                r.id.strip_prefix(*base)
                    .and_then(|rest| rest.strip_prefix('#'))
                    .and_then(|n| n.parse::<usize>().ok())
                    .map(|n| ((*base).to_string(), n))
            }
        }) else {
            continue;
        };

        let hex_len = r.h0.len() + r.h1.len() + r.h2.len() + r.h3.len();
        let mut bytes = Vec::with_capacity(hex_len / 2);
        // IN SLICE ORDER. Out of order the image reassembles with its bytes
        // transposed, which reads as a corrupt arena rather than as a bug here.
        for (k, hex) in [r.h0, r.h1, r.h2, r.h3].into_iter().enumerate() {
            if !from_hex(&hex, &mut bytes) {
                return Err(Error::RustError(format!(
                    "image {base} chunk {} slice {k} is not hex",
                    r.id
                )));
            }
        }
        by_base.entry(base).or_default().push((n, bytes, r.generation));
    }

    // SORTED IN RUST, NOT IN SQL. `ORDER BY id` is a string sort, and a string
    // sort puts "log#10" before "log#2" -- so an image that ever reached ten
    // chunks would be reassembled with its bytes in the wrong order, which
    // reads as a corrupt store rather than as a sorting bug.
    let mut out: std::collections::HashMap<String, (Vec<u8>, i64)> =
        std::collections::HashMap::new();
    for base in ids {
        let Some(mut parts) = by_base.remove(*base) else { continue };
        parts.sort_by_key(|(n, _, _)| *n);
        // A tail with no head is not an image: chunk zero carries the
        // generation the guard is checked against, and assembling from chunk
        // one would silently drop the first 900 KB.
        if parts[0].0 != 0 {
            continue;
        }
        let total: usize = parts.iter().map(|(_, b, _)| b.len()).sum();
        // The FIRST chunk's buffer becomes the image, rather than a fresh one
        // it is copied into. Every image in this store is a single chunk today,
        // and the copy would be the largest allocation in the request.
        let mut it = parts.into_iter();
        let (_, mut buf, generation) = it.next().expect("checked non-empty above");
        buf.reserve_exact(total - buf.len());
        for (_, bytes, _) in it {
            buf.extend_from_slice(&bytes);
        }
        out.insert((*base).to_string(), (buf, generation));
    }
    Ok(out)
}

/// The log AND the catalogue, in one round trip.
///
/// Eleven handlers need both -- the analytics, the customer list, the promo
/// list, the storefront's order path -- and each was loading them separately,
/// which after the fix above is still two queries where one will do.
pub async fn load_both(place: &Place) -> Result<(Loaded, LoadedCatalog)> {
    let mut images = load_images(place, &[IMAGE_LOG, IMAGE_CATALOG]).await?;
    let hub = match images.remove(IMAGE_LOG) {
        Some((bytes, generation)) => Loaded {
            hub: Hub::load(&bytes)
                .map_err(|_| Error::RustError("hub image is unreadable".into()))?,
            generation,
        },
        None => Loaded {
            hub: Hub::create_sized(64 * 1024)
                .map_err(|_| Error::RustError("cannot create hub image".into()))?,
            generation: 0,
        },
    };
    let catalog = match images.remove(IMAGE_CATALOG) {
        Some((bytes, generation)) => LoadedCatalog {
            catalog: Catalog::load(&bytes)
                .map_err(|_| Error::RustError("catalogue image is unreadable".into()))?,
            generation,
        },
        None => LoadedCatalog {
            catalog: Catalog::create()
                .map_err(|_| Error::RustError("cannot create catalogue".into()))?,
            generation: 0,
        },
    };
    Ok((hub, catalog))
}

/// One image, by name.
async fn load_bytes(place: &Place, id: &str) -> Result<Option<(Vec<u8>, i64)>> {
    Ok(load_images(place, &[id]).await?.remove(id))
}

/// Read the hub image, creating a fresh one the first time.
pub async fn load(place: &Place) -> Result<Loaded> {
    struct Row {
        image: Vec<u8>,
        generation: i64,
    }
    let row: Option<Row> =
        load_bytes(place, IMAGE_LOG).await?.map(|(image, generation)| Row { image, generation });

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
/// Write an image to the venue's Durable Object.
///
/// The generation guard is unchanged in MEANING and now nearly unneeded in
/// fact: a Durable Object serialises its own requests, so the interleaving the
/// guard was written for cannot happen inside one. It is still sent, still
/// checked, and still answers 409, because the caller's retry loop speaks that
/// contract and because a guard costs one comparison.
///
/// THE D1 WRITER IS GONE, AND NOW SO IS ITS CODE. A previous version of this
/// comment said `save_image_d1` was "kept so a venue whose object has never
/// been woken can be seeded from the old store on first read" -- but nothing
/// called it, and what actually performs that seeding is the READ path
/// (`load_images` still falls back to `hub_image`, and `do_image` decides
/// which venue may adopt a legacy image). A function kept for a reason it does
/// not serve is the shape this repository has a rule about.
async fn save_image(place: &Place, id: &str, bytes: Vec<u8>, generation: i64) -> Result<bool> {
    let stub = place.stub()?;
    let mut req = Request::new_with_init(
        &format!("https://hub/img/{id}"),
        RequestInit::new().with_method(Method::Put).with_body(Some(bytes.into())),
    )?;
    req.headers_mut()?.set("x-generation", &generation.to_string())?;
    let res = stub.fetch_with_request(req).await?;
    match res.status_code() {
        200 => Ok(true),
        // Someone else moved it. The caller re-reads and replays, which is
        // correct for an append-only log: the retry lands on the newer image
        // rather than over it.
        409 => Ok(false),
        other => Err(Error::RustError(format!("hub object refused image {id}: {other}"))),
    }
}

pub async fn save(place: &Place, loaded: &Loaded) -> Result<bool> {
    // TRIMMED. The object stores the cells the arena actually uses; the tail of
    // zeros is re-created on load from the capacity in the superblock. A hub is
    // created at 64 KiB and doubles, so most of what a full image carries is
    // nothing, and it crossed the Worker-to-object hop on every single write.
    save_image(place, IMAGE_LOG, loaded.hub.to_bytes_trimmed(), loaded.generation).await
}

/// Read the catalogue image, creating an empty one the first time.
pub async fn load_catalog(place: &Place) -> Result<LoadedCatalog> {
    struct Row {
        image: Vec<u8>,
        generation: i64,
    }
    let row: Option<Row> =
        load_bytes(place, IMAGE_CATALOG).await?.map(|(image, generation)| Row { image, generation });
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

pub async fn load_settings(place: &Place) -> Result<LoadedSettings> {
    match load_bytes(place, IMAGE_SETTINGS).await? {
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

pub async fn with_settings<F, T>(place: &Place, mut f: F) -> Result<T>
where
    F: FnMut(&mut dowiz_hub::settings::Settings) -> Result<T>,
{
    for _ in 0..5 {
        let mut loaded = load_settings(place).await?;
        let out = f(&mut loaded.settings)?;
        let bytes = loaded
            .settings
            .to_bytes()
            .map_err(|e| Error::RustError(format!("settings serialise failed: {e:?}")))?;
        if save_image(place, IMAGE_SETTINGS, bytes, loaded.generation).await? {
            return Ok(out);
        }
    }
    Err(Error::RustError("settings image is contended".into()))
}

pub struct LoadedPosts {
    pub posts: dowiz_hub::post::Posts,
    pub generation: i64,
}

pub async fn load_posts(place: &Place) -> Result<LoadedPosts> {
    match load_bytes(place, IMAGE_POSTS).await? {
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

pub async fn with_posts<F, T>(place: &Place, mut f: F) -> Result<T>
where
    F: FnMut(&mut dowiz_hub::post::Posts) -> Result<T>,
{
    for _ in 0..5 {
        let mut loaded = load_posts(place).await?;
        let out = f(&mut loaded.posts)?;
        let bytes = loaded
            .posts
            .to_bytes()
            .map_err(|e| Error::RustError(format!("posts serialise failed: {e:?}")))?;
        if save_image(place, IMAGE_POSTS, bytes, loaded.generation).await? {
            return Ok(out);
        }
    }
    Err(Error::RustError("posts image is contended".into()))
}

pub struct LoadedStock {
    pub stock: dowiz_hub::stock::StockLog,
    pub generation: i64,
}

pub async fn load_stock(place: &Place) -> Result<LoadedStock> {
    match load_bytes(place, IMAGE_STOCK).await? {
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

pub async fn with_stock<F, T>(place: &Place, mut f: F) -> Result<T>
where
    F: FnMut(&mut dowiz_hub::stock::StockLog) -> Result<T>,
{
    for _ in 0..5 {
        let mut loaded = load_stock(place).await?;
        let before = loaded.stock.len();
        let out = f(&mut loaded.stock)?;
        if loaded.stock.len() == before {
            return Ok(out);
        }
        if save_image(place, IMAGE_STOCK, loaded.stock.to_bytes_trimmed(), loaded.generation)
            .await?
        {
            return Ok(out);
        }
    }
    Err(Error::RustError("stock image is contended".into()))
}

/// Read, mutate, write the catalogue under the same generation guard.
pub async fn with_catalog<F, T>(place: &Place, mut f: F) -> Result<T>
where
    F: FnMut(&mut Catalog) -> Result<T>,
{
    for _ in 0..5 {
        let mut loaded = load_catalog(place).await?;
        let out = f(&mut loaded.catalog)?;
        let bytes = loaded
            .catalog
            .to_bytes()
            .map_err(|e| Error::RustError(format!("catalogue serialise failed: {e:?}")))?;
        if save_image(place, IMAGE_CATALOG, bytes, loaded.generation).await? {
            return Ok(out);
        }
    }
    Err(Error::RustError(
        "catalogue image is contended; five attempts lost the generation guard".into(),
    ))
}

/// Write a BRAND-NEW catalogue for a venue, replacing whatever the object holds.
///
/// FRESH, NOT READ-MODIFY-WRITE, and for a new venue those are different in a
/// way that matters. `with_catalog` loads what is there and edits it -- correct
/// for an owner changing a price, wrong for a venue being born, because
/// "what is there" for a never-used object was once ANOTHER TENANT'S CATALOGUE
/// (see `do_image`). Building the image from nothing means a new hub cannot
/// inherit a menu under any bug in the read path, and it is also the repair for
/// a venue that already did.
/// Give a venue being BORN a complete set of empty images.
///
/// EVERY IMAGE, NOT JUST THE CATALOGUE, and that is what the second venue on
/// this platform taught. A Durable Object is addressed by the venue id, so a
/// slug that was ever used before reaches the SAME object -- and before
/// `do_image` was scoped, that object had already adopted the first venue's
/// images on its first read. Seeding only the catalogue left the new hub with
/// its own menu and somebody else's SETTINGS: the other venue's AI endpoint and
/// feature flags, read back through the new owner's console.
///
/// ONLY FOR A VENUE BEING CREATED. `create_hub` refuses a slug that is already
/// a hub, so this runs exactly once per venue and never against a hub that has
/// traded. It is deliberately not public beyond that caller's need.
pub async fn seed_fresh_hub(place: &Place, location_json: &str) -> Result<()> {
    seed_catalog(place, location_json).await?;

    // The four that hold no venue identity: empty is the whole content.
    let settings = dowiz_hub::settings::Settings::create()
        .and_then(|mut s| s.to_bytes())
        .map_err(|e| Error::RustError(format!("cannot create settings: {e:?}")))?;
    let posts = dowiz_hub::post::Posts::create()
        .and_then(|mut p| p.to_bytes())
        .map_err(|e| Error::RustError(format!("cannot create posts: {e:?}")))?;
    let stock = dowiz_hub::stock::StockLog::create_sized(64 * 1024)
        .map(|s| s.to_bytes_trimmed())
        .map_err(|e| Error::RustError(format!("cannot create stock: {e:?}")))?;
    let log = Hub::create_sized(64 * 1024)
        .map(|h| h.to_bytes_trimmed())
        .map_err(|e| Error::RustError(format!("cannot create hub log: {e:?}")))?;

    for (id, bytes) in [
        (IMAGE_SETTINGS, settings),
        (IMAGE_POSTS, posts),
        (IMAGE_STOCK, stock),
        (IMAGE_LOG, log),
    ] {
        // Read the generation the object is at, then overwrite. A fresh object
        // is at 0 and a polluted one is not, so this repairs as well as seeds.
        let generation = match load_bytes(place, id).await? {
            Some((_, g)) => g,
            None => 0,
        };
        if !save_image(place, id, bytes, generation).await? {
            return Err(Error::RustError(format!(
                "could not seed the '{id}' image: the generation moved while a venue was being created"
            )));
        }
    }
    Ok(())
}

pub async fn seed_catalog(place: &Place, location_json: &str) -> Result<()> {
    for _ in 0..5 {
        // The generation only, so the write is guarded; the CONTENTS are
        // discarded on purpose.
        let generation = load_catalog(place).await?.generation;
        let mut catalog =
            Catalog::create().map_err(|_| Error::RustError("cannot create catalogue".into()))?;
        catalog.set_location(location_json);
        let bytes = catalog
            .to_bytes()
            .map_err(|e| Error::RustError(format!("catalogue serialise failed: {e:?}")))?;
        if save_image(place, IMAGE_CATALOG, bytes, generation).await? {
            return Ok(());
        }
    }
    Err(Error::RustError(
        "catalogue image is contended; five attempts lost the generation guard".into(),
    ))
}

/// Read, mutate, write — retrying when the generation guard rejects the write.
///
/// Bounded at five attempts: an append-only log makes a replay safe, but an
/// unbounded retry would turn a hot hub into a livelock rather than an error.
/// ── THE PROJECTION PATH ──
///
/// What a reader asks the object for. `load()` still exists and still hands
/// back the whole image, because export, import, health and the catalogue
/// writers need the bytes; these three do not, and they are the hot ones.
///
/// ONE OBJECT CALL, ONE ANSWER. `orders()` used to mean: fetch every chunk of
/// the image, reassemble it, parse it into a store, fold every event the venue
/// has ever written, and keep the handful that matter. All of that still
/// happens -- inside the object, once per generation, cached -- and what
/// crosses the hop is the answer.

/// Every order, newest first, folded.
pub async fn orders(place: &Place) -> Result<Vec<crate::hubdo::OrderView>> {
    Ok(orders_at(place).await?.1)
}

/// The same, with the GENERATION the list was folded from.
///
/// THE TWO COME FROM ONE ANSWER, and that is the whole point of this function
/// existing. The orders route used to fetch the list and then ask a second
/// time for the generation; an append landing between the two made a client
/// store a list at generation G labelled G+1, and its next `?since=G+1` would
/// be answered "nothing changed" about the event at G+1 -- which the client
/// then never sees, because its copy is already past it. The object sets
/// `x-generation` on the same response; this reads it there.
pub async fn orders_at(place: &Place) -> Result<(i64, Vec<crate::hubdo::OrderView>)> {
    let stub = place.stub()?;
    let req = Request::new("https://hub/fold/orders", Method::Get)?;
    let mut res = stub.fetch_with_request(req).await?;
    if res.status_code() != 200 {
        return Err(Error::RustError(format!("hub object refused a projection: {}", res.status_code())));
    }
    let generation =
        res.headers().get("x-generation").ok().flatten().and_then(|v| v.parse().ok()).unwrap_or(0);
    Ok((generation, res.json().await?))
}

/// One order's folded state, or `None` if this hub never saw it.
pub async fn order(place: &Place, order_id: &str) -> Result<Option<String>> {
    let stub = place.stub()?;
    let req = Request::new(
        &format!("https://hub/fold/order?id={}", crate::mcp::enc(order_id)),
        Method::Get,
    )?;
    let mut res = stub.fetch_with_request(req).await?;
    match res.status_code() {
        200 => {
            let view: crate::hubdo::OrderView = res.json().await?;
            Ok(Some(view.order_json))
        }
        404 => Ok(None),
        other => Err(Error::RustError(format!("hub object refused an order: {other}"))),
    }
}

/// What changed since a generation the caller already has.
///
/// `Ok(None)` means the object cannot say -- a cold object, or an absence
/// longer than its window -- and the caller should read the list. It is not an
/// error and must not be logged as one: it is the ordinary answer after a
/// venue has been quiet long enough for its object to go to sleep.
pub async fn changes_since(
    place: &Place,
    since: i64,
) -> Result<(i64, Option<Vec<crate::hubdo::Change>>)> {
    let stub = place.stub()?;
    let req = Request::new(&format!("https://hub/fold/changes?since={since}"), Method::Get)?;
    let mut res = stub.fetch_with_request(req).await?;
    if res.status_code() != 200 {
        return Err(Error::RustError(format!("hub object refused a catch-up: {}", res.status_code())));
    }
    #[derive(serde::Deserialize)]
    struct Out {
        generation: i64,
        full: bool,
        changes: Vec<crate::hubdo::Change>,
    }
    let out: Out = res.json().await?;
    Ok((out.generation, if out.full { None } else { Some(out.changes) }))
}

/// The generation the object's log is at, without fetching the log.
///
/// The append path needs it to guard its write, and a HEAD-shaped question is
/// the cheapest thing this object answers: the projection is memoised, so on a
/// warm object this touches no storage at all.
pub async fn log_generation(place: &Place) -> Result<i64> {
    let stub = place.stub()?;
    let req = Request::new("https://hub/fold/generation", Method::Get)?;
    let res = stub.fetch_with_request(req).await?;
    Ok(res.headers().get("x-generation").ok().flatten().and_then(|v| v.parse().ok()).unwrap_or(0))
}

/// Append one event that does not depend on what the log already says.
///
/// A placement, an audit record: the payload is already decided, so there is
/// nothing to read and nothing to re-decide. The generation guard is still
/// carried -- it is what tells "somebody else wrote" from "it failed" -- and a
/// lost guard simply asks the object for the new generation and writes again.
pub async fn append_blind(
    place: &Place,
    kind: dowiz_hub::EventKind,
    subject: &str,
    payload: &str,
) -> Result<i64> {
    for _ in 0..5 {
        let generation = log_generation(place).await?;
        let body = serde_json::json!({
            "kind": kind as u8,
            "order_id": subject,
            "payload": payload,
            "clock": Date::now().as_millis(),
        });
        let stub = place.stub()?;
        let mut write = Request::new_with_init(
            "https://hub/fold/append",
            RequestInit::new()
                .with_method(Method::Post)
                .with_body(Some(JsValue::from_str(&body.to_string()))),
        )?;
        write.headers_mut()?.set("x-generation", &generation.to_string())?;
        write.headers_mut()?.set("content-type", "application/json")?;
        let mut res = stub.fetch_with_request(write).await?;
        match res.status_code() {
            200 => {
                let v: serde_json::Value = res.json().await.unwrap_or(serde_json::json!({}));
                return Ok(v.get("generation").and_then(serde_json::Value::as_i64).unwrap_or(0));
            }
            409 => continue,
            other => {
                return Err(Error::RustError(format!("hub object refused an append: {other}")))
            }
        }
    }
    Err(Error::RustError("hub log is contended; five attempts lost the generation guard".into()))
}

/// Append one event, retrying if another writer moved the log first.
///
/// `read` is given the CURRENT state of the order (`None` when the hub has
/// never seen it) and returns the event to write: its kind and its payload.
/// Returning `Ok(None)` means "nothing to record", which is not a failure --
/// the Stripe webhook replaying a payment already recorded takes that branch.
///
/// THE RETRY IS THE SAME CONTRACT `with_hub` had. A Durable Object serialises
/// its own requests, so the guard fires only if a second Worker appended
/// between this reader's question and this writer's answer; then the decision
/// is made again against the newer state, which is correct for a log whose
/// events are deltas.
pub async fn append_for<F>(
    place: &Place,
    order_id: &str,
    mut decide: F,
) -> Result<Option<serde_json::Value>>
where
    F: FnMut(Option<String>) -> Result<Option<(dowiz_hub::EventKind, String, serde_json::Value)>>,
{
    for _ in 0..5 {
        let stub = place.stub()?;
        let req = Request::new(
            &format!("https://hub/fold/order?id={}", crate::mcp::enc(order_id)),
            Method::Get,
        )?;
        let mut res = stub.fetch_with_request(req).await?;
        let generation: i64 =
            res.headers().get("x-generation").ok().flatten().and_then(|v| v.parse().ok()).unwrap_or(0);
        let current = match res.status_code() {
            200 => Some(res.json::<crate::hubdo::OrderView>().await?.order_json),
            404 => None,
            other => {
                return Err(Error::RustError(format!("hub object refused an order: {other}")))
            }
        };
        let Some((kind, payload, out)) = decide(current)? else { return Ok(None) };
        let body = serde_json::json!({
            "kind": kind as u8,
            "order_id": order_id,
            "payload": payload,
            "clock": Date::now().as_millis(),
        });
        let mut write = Request::new_with_init(
            "https://hub/fold/append",
            RequestInit::new()
                .with_method(Method::Post)
                .with_body(Some(JsValue::from_str(&body.to_string()))),
        )?;
        write.headers_mut()?.set("x-generation", &generation.to_string())?;
        write.headers_mut()?.set("content-type", "application/json")?;
        let res = stub.fetch_with_request(write).await?;
        match res.status_code() {
            200 => return Ok(Some(out)),
            // Someone else appended first: ask again and decide again.
            409 => continue,
            other => {
                return Err(Error::RustError(format!("hub object refused an append: {other}")))
            }
        }
    }
    Err(Error::RustError("hub log is contended; five attempts lost the generation guard".into()))
}

/// ── ROTATION: A BOUNDED HOT LOG WITH COLD HISTORY ──
///
/// How long a finished order stays on the hot path. Thirty days is what an
/// owner reaches for -- last month's numbers -- and everything older is read
/// from an archive, which is a different question and a different route.
pub const HOT_KEEP_MS: i64 = 30 * 24 * 60 * 60 * 1000;

/// Statuses that mean the order is still happening. These stay whatever their
/// age: an order that has been PENDING for forty days is a problem, and
/// archiving it would be hiding one.
const LIVE: &[&str] = &["PENDING", "CONFIRMED", "PREPARING", "READY", "IN_DELIVERY"];

/// Where a venue's archives are listed. Written by the rotation, read by the
/// history route and the backup; kept in settings rather than derived by
/// listing the object's keys, because a Durable Object cannot be asked what it
/// holds.
const ARCHIVES_KEY: &str = "log.archives";

/// The archives this hub has, oldest first.
pub fn archives_of(settings: &dowiz_hub::settings::Settings) -> Vec<String> {
    settings
        .get(ARCHIVES_KEY)
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// Move everything finished and older than `HOT_KEEP_MS` out of the hot log.
///
/// THE ARCHIVE IS WRITTEN FIRST AND THE HOT LOG SECOND, and if the second
/// write fails the venue has one extra copy of its history rather than none of
/// it. Re-running lands on the same archive id, which the object refuses to
/// overwrite -- an archive is written once by construction.
pub async fn rotate(place: &Place, now_ms: i64) -> Result<serde_json::Value> {
    let loaded = load(place).await?;
    let generation = loaded.generation;
    let before = loaded.hub.len();

    let mut keep: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut moved = 0usize;
    for e in orders_state(&loaded.hub) {
        let v: serde_json::Value = serde_json::from_str(&e.order_json).unwrap_or_default();
        let status = v.get("status").and_then(serde_json::Value::as_str).unwrap_or("");
        let fresh = (e.seq as i64) > now_ms - HOT_KEEP_MS;
        if LIVE.contains(&status) || fresh {
            keep.insert(e.order_id);
        } else {
            moved += 1;
        }
    }
    if moved == 0 {
        return Ok(serde_json::json!({ "rotated": false, "events": before, "reason": "nothing is old enough" }));
    }

    let mut hub = loaded.hub;
    let archived = hub
        .rotate(|id| keep.contains(id))
        .map_err(|e| Error::RustError(format!("rotation failed: {e:?}")))?;
    let archive_id = format!("{IMAGE_LOG}@{generation}");
    let archive_bytes = archived.len();

    // ── THE ARCHIVE IS WRITTEN FIRST, AND IT IS CHECKED ──
    //
    // Generation 0 means "this id has never been written". A refusal is not
    // automatically our own retry: a venue restored from a backup starts its
    // object's generation at 1 again while the RESTORED archives carry the old
    // object's high numbers, so a later rotation can compute an `archive_id`
    // that already names somebody else's bytes. Truncating the hot log against
    // that would put the rotated events nowhere, silently.
    //
    // So a refusal is verified: the archive already there must hold the same
    // events we were about to write. If it does, this is the retry the comment
    // assumed and the rotation carries on. If it does not, nothing is
    // truncated and the caller is told which id collided.
    let events_archived = before;
    let stored = save_image(place, &archive_id, archived, 0).await?;
    if !stored {
        let same = match load_bytes(place, &archive_id).await? {
            Some((bytes, _)) => Hub::load(&bytes).map(|h| h.len() == events_archived).unwrap_or(false),
            None => false,
        };
        if !same {
            return Err(Error::RustError(format!(
                "{archive_id} already holds a different archive; the hot log was left alone"
            )));
        }
    }
    // REGISTERED BEFORE THE HOT LOG IS CUT. If the cut fails, the archive is
    // still the venue's -- listed, backed up, readable -- rather than an
    // orphan nothing names.
    let listed = archive_id.clone();
    let _ = with_settings(place, move |s| {
        let mut all = archives_of(s);
        if !all.contains(&listed) {
            all.push(listed.clone());
        }
        s.set(ARCHIVES_KEY, &all.join(","));
        Ok(())
    })
    .await;
    if !save_image(place, IMAGE_LOG, hub.to_bytes_trimmed(), generation).await? {
        return Err(Error::RustError(
            "the log moved while it was being rotated; the archive is written and nothing was lost"
                .into(),
        ));
    }
    let listed = archive_id.clone();
    let _ = with_settings(place, move |s| {
        let mut all = archives_of(s);
        if !all.contains(&listed) {
            all.push(listed.clone());
        }
        s.set(ARCHIVES_KEY, &all.join(","));
        Ok(())
    })
    .await;

    Ok(serde_json::json!({
        "rotated": true,
        "archive": archive_id,
        "archiveWasNew": stored,
        "archiveBytes": archive_bytes,
        "ordersMoved": moved,
        "ordersKept": keep.len(),
        "eventsBefore": before,
        "eventsAfter": hub.len(),
    }))
}

/// Where the couriers are, from the object's memory.
///
/// Returns an empty list rather than an error when the object has hibernated
/// since the last fix: a position whose meaning expires in minutes is supposed
/// to disappear, and the caller falls back to the rows in D1.
pub async fn positions(place: &Place, now_ms: i64) -> Result<Vec<crate::live_eta::CourierFix>> {
    let stub = place.stub()?;
    let req = Request::new("https://hub/fold/positions", Method::Get)?;
    let mut res = stub.fetch_with_request(req).await?;
    if res.status_code() != 200 {
        return Ok(Vec::new());
    }
    let raw: std::collections::HashMap<String, crate::hubdo::Fix> = res.json().await?;
    Ok(raw
        .into_iter()
        .filter(|(_, f)| now_ms - f.at_ms < crate::live_eta::POSITION_FRESH_MS)
        .map(|(courier_id, f)| crate::live_eta::CourierFix {
            courier_id,
            lat_udeg: f.lat_e6 as i32,
            lon_udeg: f.lng_e6 as i32,
            recorded_at_ms: f.at_ms,
        })
        .collect())
}

/// Is this the name of an archive this module wrote?
///
/// CHECKED, NOT TRUSTED. The id reaches the object's storage, and it arrives
/// in a query parameter: an id that could name any image would let the history
/// route read the settings or the catalogue as if they were orders, and an id
/// with a path in it would be worth more than that. `log@` and digits, nothing
/// else -- and an empty generation is not digits.
pub fn is_archive_id(id: &str) -> bool {
    let Some(rest) = id.strip_prefix(IMAGE_LOG).and_then(|r| r.strip_prefix('@')) else {
        return false;
    };
    !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
}

/// One archive's orders, folded — the cold half of the history.
pub async fn archive_orders(place: &Place, archive_id: &str) -> Result<Option<Vec<crate::hubdo::OrderView>>> {
    if !is_archive_id(archive_id) {
        return Ok(None);
    }
    let Some((bytes, _)) = load_bytes(place, archive_id).await? else { return Ok(None) };
    let hub =
        Hub::load(&bytes).map_err(|_| Error::RustError("archive image is unreadable".into()))?;
    Ok(Some(orders_state(&hub).into_iter().map(crate::hubdo::OrderView::of_event).collect()))
}

/// The folded state of one order, or `None` if this hub never saw it.
///
/// USED BY THE OBJECT AND BY THE TESTS. Since phase 2 the Worker asks the
/// object for an order rather than folding one itself, so this is the
/// definition the object's `/fold/order` is checked against rather than a
/// path the Worker takes on a request.
#[cfg_attr(not(test), allow(dead_code))]
///
/// `Hub::order` returns the newest EVENT, which since phase 3 may be a delta.
/// Everything that wants the ORDER asks here, and for a log written before
/// deltas the two answers are identical -- a snapshot replaces the state, so
/// folding a history of snapshots yields the newest one.
pub fn order_state(hub: &Hub, order_id: &str) -> Option<String> {
    let history = hub.history(order_id);
    if history.is_empty() {
        return None;
    }
    let folded = crate::fold::fold(history.iter().map(|e| e.order_json.as_str()));
    if folded.is_null() {
        return None;
    }
    Some(folded.to_string())
}

/// Every order's folded state, newest order first — the shape `Hub::orders`
/// had, with `order_json` holding the FOLD rather than the last event.
///
/// ONE PASS. Folding each order by calling `order_state` in a loop would walk
/// the whole log once per order, which is the O(events × orders) shape phase 1
/// removed from `orders()` and must not come back through the fold. The events
/// are walked once, oldest first, and each order's state is built as they go.
pub fn orders_state(hub: &Hub) -> Vec<dowiz_hub::Event> {
    use std::collections::HashMap;
    let events = hub.events_oldest_first();
    // id → (index of its newest event, folded state, that newest event)
    let mut at: HashMap<String, usize> = HashMap::new();
    let mut state: HashMap<String, serde_json::Value> = HashMap::new();
    let mut newest: HashMap<String, dowiz_hub::Event> = HashMap::new();
    for (i, e) in events.into_iter().enumerate() {
        // NON-ORDER EVENTS ARE SKIPPED, exactly as `Hub::orders` skips them:
        // `Revealed` records an audit fact under a subject that is not an
        // order id, and counting it as a sale is how analytics lie.
        if !e.kind.is_order() {
            continue;
        }
        let entry = state.entry(e.order_id.clone()).or_insert(serde_json::Value::Null);
        *entry = crate::fold::fold_one(entry.take(), &e.order_json);
        at.insert(e.order_id.clone(), i);
        newest.insert(e.order_id.clone(), e);
    }
    let mut ids: Vec<(usize, String)> = at.into_iter().map(|(id, i)| (i, id)).collect();
    // Newest order first, which is the order every console renders in.
    ids.sort_by(|a, b| b.0.cmp(&a.0));
    ids.into_iter()
        .filter_map(|(_, id)| {
            let mut e = newest.remove(&id)?;
            let folded = state.remove(&id)?;
            if folded.is_null() {
                return None;
            }
            e.order_json = folded.to_string();
            Some(e)
        })
        .collect()
}

/// How many times a promo code has been redeemed, folded from the orders.
///
/// No counter is stored, for the reason the analytics give: a tally kept beside
/// the orders is a second number that can disagree with them, and when they
/// disagree it is always the tally that is wrong. A rejected or cancelled order
/// gives its use back -- the venue never took the money, so holding a use
/// against the customer would charge them for a refusal.
pub fn promo_uses(hub: &Hub, code: &str) -> i64 {
    promo_uses_in(&orders_state(hub).into_iter().map(crate::hubdo::OrderView::of_event).collect::<Vec<_>>(), code)
}

/// The same count over a PROJECTION, for a caller that already has one and
/// must not fetch the whole log to answer a discount.
pub fn promo_uses_in(listed: &[crate::hubdo::OrderView], code: &str) -> i64 {
    listed
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
    // When each status was entered: the live estimate measures from these.
    "at",
];

/// Copy `HUB_OWNED` from the order as it was onto the order the kernel returned.
pub fn carry_over(old: &serde_json::Value, updated: &mut serde_json::Value) {
    for k in HUB_OWNED {
        if let Some(v) = old.get(*k) {
            updated[*k] = v.clone();
        }
    }
}

pub async fn with_hub<F, T>(place: &Place, mut f: F) -> Result<T>
where
    F: FnMut(&mut Hub) -> Result<T>,
{
    for _ in 0..5 {
        let mut loaded = load(place).await?;
        let before = loaded.hub.len();
        let out = f(&mut loaded.hub)?;
        // NOTHING APPENDED, NOTHING WRITTEN. A read-check-then-maybe-append
        // (the Stripe webhook replaying an order already paid) used to rewrite
        // and re-chunk the whole image and bump the object's generation for
        // an image that had not changed. The EVENT COUNT is the signal, not
        // the store generation: `grow()` rebuilds the store from scratch and
        // its generation restarts, so two generations can be equal across a
        // real append while the count never is.
        if loaded.hub.len() == before {
            return Ok(out);
        }
        if save(place, &loaded).await? {
            return Ok(out);
        }
    }
    Err(Error::RustError(
        "hub image is contended; five attempts lost the generation guard".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AN ARCHIVE NAME IS A KEY INTO THE OBJECT'S STORAGE, and it arrives in a
    /// query parameter. Everything but `log@<digits>` has to be refused, or
    /// the history route is a way to read the settings image -- which holds
    /// this venue's tokens -- as if it were a list of orders.
    #[test]
    fn only_a_log_archive_name_is_accepted() {
        for good in ["log@0", "log@1", "log@1789000000000"] {
            assert!(is_archive_id(good), "{good} is an archive");
        }
        for bad in [
            "log", "log@", "settings", "catalog", "log@abc", "log@1x", "log@-1", "log@ 1",
            "log@1/../settings", "m:log@1", "c:log@1:0", "LOG@1", "log@1,log@2", "",
        ] {
            assert!(!is_archive_id(bad), "{bad:?} must not be read as an archive");
        }
    }

    /// THE PHASE-3 PROPERTY, AT THE LEVEL THE CONSOLE READS. A hub whose
    /// events are full envelopes and a hub whose events are deltas must
    /// produce the same list of orders -- same fields, same values, same
    /// order. Every image in production is the first kind; everything written
    /// from now on is the second.
    #[test]
    fn a_log_of_deltas_lists_the_same_orders_as_a_log_of_envelopes() {
        let envelope = |id: &str, status: &str, extra: &str| -> String {
            format!(
                r#"{{"id":"{id}","location_id":"v1","status":"{status}","total":2650,{extra}"items":[{{"product_id":"item-01","quantity":2}}],"contact":{{"name":"Ana","phone":"+355691234567"}}}}"#
            )
        };
        let mut full = Hub::create_sized(256 * 1024).unwrap();
        let mut deltas = Hub::create_sized(256 * 1024).unwrap();
        let mut clock = 1u64;
        for n in 0..3u64 {
            let id = format!("ord_{n}");
            let placed = envelope(&id, "PENDING", "");
            full.append(dowiz_hub::EventKind::Placed, &id, &placed, clock, [0u8; 32]).unwrap();
            deltas.append(dowiz_hub::EventKind::Placed, &id, &placed, clock, [0u8; 32]).unwrap();
            clock += 1;
            let mut state: serde_json::Value = serde_json::from_str(&placed).unwrap();
            for (status, stamp) in
                [("CONFIRMED", "confirmed_ms"), ("COOKING", "cooking_ms"), ("DELIVERED", "delivered_ms")]
            {
                let next = envelope(&id, status, &format!(r#""{stamp}":{clock},"#));
                let next_v: serde_json::Value = serde_json::from_str(&next).unwrap();
                full.append(dowiz_hub::EventKind::Advanced, &id, &next, clock, [0u8; 32]).unwrap();
                let d = crate::fold::delta(&state, &next_v).to_string();
                deltas.append(dowiz_hub::EventKind::Advanced, &id, &d, clock, [0u8; 32]).unwrap();
                state = next_v;
                clock += 1;
            }
        }

        let a = orders_state(&full);
        let b = orders_state(&deltas);
        assert_eq!(a.len(), 3);
        assert_eq!(a.len(), b.len(), "same number of orders");
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.order_id, y.order_id, "same order, same place in the list");
            let xv: serde_json::Value = serde_json::from_str(&x.order_json).unwrap();
            let yv: serde_json::Value = serde_json::from_str(&y.order_json).unwrap();
            assert_eq!(xv, yv, "same state for {}", x.order_id);
            assert_eq!(xv["status"], serde_json::json!("DELIVERED"));
            assert_eq!(xv["contact"]["name"], serde_json::json!("Ana"));
        }

        // And the point of the exercise, MEASURED rather than hoped for -- and
        // the measurement MOVED when the format underneath it did.
        //
        // Twelve events over three orders:
        //   v1 format: 2,619 cells of envelopes against 1,545 of deltas (41 % off)
        //   v2 format:   578 cells of envelopes against   444 of deltas (24 % off)
        //
        // The deltas did not get worse; the baseline got better. v2 packs eight
        // payload bytes into a cell, so the payload stopped being what an event
        // costs: what is left is the fixed header -- 12 cells, the store's
        // 2-cell object header and a 10-cell root -- which a delta and an
        // envelope pay alike. Against the original v1 envelope the two changes
        // together are 2,619 cells down to 444, 83 % off, and neither of them
        // gets there alone.
        let (d, f) = (deltas.usage().used_cells, full.usage().used_cells);
        println!("THREE ORDERS, TWELVE EVENTS: deltas {d} cells, envelopes {f} ({}%)", d * 100 / f);
        assert!(d < f, "a delta log must still be smaller: {d} against {f}");
        assert!(d * 10 < f * 9, "and by a tenth at least: {d} against {f}");
    }

    /// A courier taking an order writes a `Noted` event, which is an order
    /// event -- so its delta has to reach the list. Before the fold the
    /// assignment survived only because the NEXT full envelope happened to
    /// carry it; with deltas, dropping it would make the console show an
    /// unassigned order that a courier is already carrying.
    #[test]
    fn a_noted_assignment_reaches_the_folded_order() {
        let mut hub = Hub::create_sized(64 * 1024).unwrap();
        let placed = r#"{"id":"ord_1","location_id":"v1","status":"CONFIRMED"}"#;
        hub.append(dowiz_hub::EventKind::Placed, "ord_1", placed, 1, [0u8; 32]).unwrap();
        let before: serde_json::Value = serde_json::from_str(placed).unwrap();
        let mut after = before.clone();
        after["courier_id"] = serde_json::json!("cour_7");
        let d = crate::fold::delta(&before, &after).to_string();
        hub.append(dowiz_hub::EventKind::Noted, "ord_1", &d, 2, [0u8; 32]).unwrap();

        let listed = orders_state(&hub);
        assert_eq!(listed.len(), 1);
        let v: serde_json::Value = serde_json::from_str(&listed[0].order_json).unwrap();
        assert_eq!(v["courier_id"], serde_json::json!("cour_7"));
        assert_eq!(v["status"], serde_json::json!("CONFIRMED"), "the rest of the order survives");

        // And the single-order read agrees with the list.
        let one: serde_json::Value =
            serde_json::from_str(&order_state(&hub, "ord_1").unwrap()).unwrap();
        assert_eq!(one, v);
    }

    /// An audit record is not an order and must not become one -- not in the
    /// list, and not inside the order it names.
    #[test]
    fn an_audit_record_is_neither_an_order_nor_part_of_one() {
        let mut hub = Hub::create_sized(64 * 1024).unwrap();
        hub.append(
            dowiz_hub::EventKind::Placed,
            "ord_1",
            r#"{"id":"ord_1","status":"PENDING"}"#,
            1,
            [0u8; 32],
        )
        .unwrap();
        hub.append(
            dowiz_hub::EventKind::Revealed,
            "ord_1",
            r#"{"who":"owner_1","at_ms":2}"#,
            2,
            [0u8; 32],
        )
        .unwrap();
        let listed = orders_state(&hub);
        assert_eq!(listed.len(), 1);
        let v: serde_json::Value = serde_json::from_str(&listed[0].order_json).unwrap();
        assert!(v.get("who").is_none(), "the audit fact must not be in the order: {v}");
        let one: serde_json::Value =
            serde_json::from_str(&order_state(&hub, "ord_1").unwrap()).unwrap();
        assert!(one.get("who").is_none(), "{one}");
    }

    /// An order nobody placed has no state, rather than an empty one.
    #[test]
    fn an_unknown_order_has_no_state() {
        let hub = Hub::create_sized(64 * 1024).unwrap();
        assert!(order_state(&hub, "ord_missing").is_none());
        assert!(orders_state(&hub).is_empty());
    }

    /// SQLite's `hex()` emits UPPERCASE. Getting that wrong would decode every
    /// image to garbage while still returning `true`, which is the shape of
    /// failure this whole change exists to avoid.
    #[test]
    fn every_byte_survives_the_hex_round_trip() {
        let original: Vec<u8> = (0..=255u8).collect();
        let upper: String = original.iter().map(|b| format!("{b:02X}")).collect();
        let lower: String = original.iter().map(|b| format!("{b:02x}")).collect();

        for encoded in [&upper, &lower] {
            let mut out = Vec::new();
            assert!(from_hex(encoded, &mut out), "well-formed hex must decode");
            assert_eq!(out, original, "bytes must come back exactly");
        }
    }

    /// The reason `from_hex` returns a bool rather than skipping what it cannot
    /// read: a half-decoded image is a corrupt arena reported far from here.
    #[test]
    fn nothing_that_is_not_hex_is_guessed_at() {
        for bad in ["abc", "zz", "00ff0g", " 00", "00 ff"] {
            let mut out = Vec::new();
            assert!(!from_hex(bad, &mut out), "{bad:?} must be refused, not decoded");
        }
    }

    /// An image shorter than the slices has empty trailing ones, and an empty
    /// slice must contribute nothing -- not a zero byte, which would append
    /// padding to every image in the store.
    #[test]
    fn an_empty_slice_appends_nothing() {
        let mut out = vec![7u8, 8, 9];
        assert!(from_hex("", &mut out));
        assert_eq!(out, vec![7, 8, 9]);
    }

    /// THE ONE THAT MATTERS. Reassembles an image the way `load_images` does --
    /// slice by slice, in slice order -- from what SQLite's `substr`/`hex` pair
    /// would return for each, and checks the result against the original bytes.
    ///
    /// Sized to straddle a slice boundary AND end part-way through the next, so
    /// an off-by-one in the 1-based `substr` start or in the final short slice
    /// shows up as a difference rather than as a still-plausible image.
    #[test]
    fn a_multi_slice_image_reassembles_byte_for_byte() {
        let original: Vec<u8> = (0..SLICE + SLICE / 2).map(|i| (i % 251) as u8).collect();

        // What the query asks SQLite for, computed the same way the SQL is built.
        let slices: Vec<String> = (0..SLICES)
            .map(|k| {
                let start = k * SLICE;
                let end = (start + SLICE).min(original.len()).max(start.min(original.len()));
                original[start.min(original.len())..end]
                    .iter()
                    .map(|b| format!("{b:02X}"))
                    .collect()
            })
            .collect();
        assert_eq!(slices[0].len(), SLICE * 2, "slice 0 is full");
        assert_eq!(slices[1].len(), SLICE, "slice 1 is the remaining half");
        assert_eq!(slices[2], "", "nothing past the end");

        let mut rebuilt = Vec::new();
        for s in &slices {
            assert!(from_hex(s, &mut rebuilt));
        }
        assert_eq!(rebuilt, original, "the image must survive slicing");
    }

    /// The slices must cover a whole chunk. If `CHUNK` outgrows them the tail of
    /// every chunked image is silently dropped -- a `const` assert already stops
    /// the build, and this says out loud what it is protecting.
    #[test]
    fn the_slices_cover_a_whole_chunk() {
        assert!(SLICE * SLICES >= CHUNK, "{SLICE} x {SLICES} must cover {CHUNK}");
    }
}

/// Every image this venue has, for the venue to keep.
///
/// P68'S SOVEREIGN BACKUP, STARTED. `settings.rs` says the honest thing about
/// secrets at rest: what protects them is file mode on hardware the venue owns,
/// and real protection needs a key that lives somewhere else. The same is true
/// of the data itself. Today the only thing standing between Dubin & Sushi and
/// losing their entire history is Cloudflare's own thirty-day time travel —
/// which is a fine safety net and is not THEIRS. This is the copy they hold.
///
/// SELF-DESCRIBING AND SELF-CHECKING, because a backup nobody can verify is a
/// backup nobody can trust. The manifest names each image, its length and its
/// SHA-256, so a restore can refuse a corrupted file instead of feeding a
/// truncated arena to the kernel.
pub const IMAGES: &[&str] =
    &[IMAGE_LOG, IMAGE_CATALOG, IMAGE_SETTINGS, IMAGE_POSTS, IMAGE_STOCK];

/// Archives already copied off-site, so a nightly bundle carries each one ONCE.
const ARCHIVES_BACKED_KEY: &str = "log.archives.backed";

/// Which archives this venue has that have never been in a backup.
pub async fn archives_pending(place: &Place) -> Result<Vec<String>> {
    let settings = load_settings(place).await?.settings;
    let done: Vec<String> = settings
        .get(ARCHIVES_BACKED_KEY)
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    Ok(archives_of(&settings).into_iter().filter(|a| !done.contains(a)).collect())
}

/// Mark archives as copied. Called after a bundle lands, never before.
pub async fn archives_marked(place: &Place, ids: &[String]) -> Result<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let ids = ids.to_vec();
    with_settings(place, move |s| {
        let mut done: Vec<String> = s
            .get(ARCHIVES_BACKED_KEY)
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|x| !x.is_empty())
            .map(str::to_string)
            .collect();
        for id in &ids {
            if !done.contains(id) {
                done.push(id.clone());
            }
        }
        s.set(ARCHIVES_BACKED_KEY, &done.join(","));
        Ok(())
    })
    .await
}

pub async fn export(place: &Place) -> Result<serde_json::Value> {
    use sha2::{Digest, Sha256};
    let got = load_images(place, IMAGES).await?;
    let mut images = serde_json::Map::new();
    for id in IMAGES {
        let Some((bytes, generation)) = got.get(*id) else { continue };
        let digest: String =
            Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect();
        images.insert(
            (*id).to_string(),
            serde_json::json!({
                "generation": generation,
                "bytes": bytes.len(),
                "sha256": digest,
                // Base64 rather than hex: a backup is downloaded whole, so the
                // wire cost is paid once and a third smaller matters, unlike on
                // the read path where the crossing was the cost.
                "image": base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD, bytes),
            }),
        );
    }
    // THE ARCHIVES THAT HAVE NEVER BEEN COPIED, and only those.
    //
    // Rotation makes history into its own image, and an image nobody copies
    // off-site is history kept in exactly one place. Carrying EVERY archive in
    // EVERY nightly bundle would put it back where it started -- a file that
    // grows forever -- so each archive travels once and is marked by the
    // caller after the bundle lands.
    let pending = archives_pending(place).await.unwrap_or_default();
    let mut archives = serde_json::Map::new();
    if !pending.is_empty() {
        let ids: Vec<&str> = pending.iter().map(String::as_str).collect();
        let got = load_images(place, &ids).await?;
        for id in &pending {
            let Some((bytes, generation)) = got.get(id) else { continue };
            let digest: String =
                Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect();
            archives.insert(
                id.clone(),
                serde_json::json!({
                    "generation": generation,
                    "bytes": bytes.len(),
                    "sha256": digest,
                    "image": base64::Engine::encode(
                        &base64::engine::general_purpose::STANDARD, bytes),
                }),
            );
        }
    }

    Ok(serde_json::json!({
        "format": "dowiz-hub-backup/1",
        "venue": place.venue,
        "taken_at_ms": Date::now().as_millis() as i64,
        "images": images,
        "archives": archives,
    }))
}

/// Put a backup back — ONLY INTO A VENUE THAT HAS NONE.
///
/// THE REFUSAL IS THE FEATURE. A restore that overwrites a live hub is a
/// one-click way to erase a venue's entire history, and it would be reachable
/// by anything that could reach an owner's token. So this writes only where
/// generation is zero: disaster recovery into a fresh object, never a rollback
/// over something that exists. An operator who genuinely wants to roll back
/// deletes the object first, deliberately, which is a different act.
///
/// EVERY IMAGE IS CHECKED BEFORE ANY IMAGE IS WRITTEN. A bundle whose third
/// image is corrupt must not leave the first two in place and the rest missing.
pub async fn import(place: &Place, bundle: &serde_json::Value) -> Result<Vec<String>> {
    use sha2::{Digest, Sha256};
    if bundle.get("format").and_then(|v| v.as_str()) != Some("dowiz-hub-backup/1") {
        return Err(Error::RustError("not a dowiz hub backup".into()));
    }
    let Some(images) = bundle.get("images").and_then(|v| v.as_object()) else {
        return Err(Error::RustError("backup has no images".into()));
    };

    // A BUNDLE MAY CARRY ARCHIVES, and a restore that dropped them would put a
    // venue back with its live orders and no history -- which is the shape of
    // loss rotation is supposed to prevent. They are checked and written
    // exactly as the five fixed images are; only the name test differs,
    // because an archive's name carries the generation it was cut at.
    let archives = bundle.get("archives").and_then(|v| v.as_object());
    let named: Vec<(&String, &serde_json::Value)> =
        images.iter().chain(archives.into_iter().flatten()).collect();

    let mut staged: Vec<(String, Vec<u8>)> = Vec::new();
    for (id, entry) in named {
        if !IMAGES.contains(&id.as_str()) && !is_archive_id(id) {
            return Err(Error::RustError(format!("backup names an unknown image: {id}")));
        }
        let Some(b64) = entry.get("image").and_then(|v| v.as_str()) else {
            return Err(Error::RustError(format!("image {id} has no bytes")));
        };
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64)
            .map_err(|e| Error::RustError(format!("image {id} is not base64: {e}")))?;
        let want = entry.get("sha256").and_then(|v| v.as_str()).unwrap_or("");
        let got: String = Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
        if got != want {
            return Err(Error::RustError(format!(
                "image {id} does not match its digest; the backup is damaged"
            )));
        }
        staged.push((id.clone(), bytes));
    }

    // Now, and only now, that every image has been read and checked.
    let mut written = Vec::new();
    for (id, bytes) in staged {
        let existing = load_images(place, &[id.as_str()]).await?;
        if existing.get(&id).map(|(_, g)| *g).unwrap_or(0) != 0 {
            return Err(Error::RustError(format!(
                "{id} already exists in this venue; a restore never overwrites"
            )));
        }
        if !save_image(place, &id, bytes, 0).await? {
            return Err(Error::RustError(format!("{id} was written by someone else mid-restore")));
        }
        written.push(id);
    }
    Ok(written)
}
