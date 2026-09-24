//! The platform's own bebop images, in a Durable Object like every venue's.
//!
//! WHY THIS EXISTS. D1 is going. Twenty-nine relational tables hold what this
//! platform knows, and every one of them is either a VENUE'S fact -- which
//! belongs in that venue's object, where `id_from_name(location_id)` makes
//! tenancy structural rather than a column a query has to remember to filter on
//! -- or a PLATFORM fact: who the people are, which venues exist, which host
//! answers for which venue. Those are what this module holds.
//!
//! NO NEW DURABLE OBJECT CLASS, and that is deliberate rather than lazy. The
//! `HubImages` class already stores chunked images by name behind a generation
//! guard, and its own header says "NO SQL REACHES THIS MODULE". A second class
//! would be a second migration, a second storage API to keep honest, and a
//! second place the chunking limit has to be remembered. `__platform` is simply
//! a name no venue can have: a venue name is a location id, and an id starting
//! with two underscores is not one this platform mints.
//!
//! THE SERIALISATION POINT, stated before somebody discovers it. One object is
//! one writer, so every login on the platform is serialised through it. At two
//! venues that is free and it is also exactly what makes a uniqueness check on
//! an email address CORRECT -- nothing else runs between the read and the
//! write. When the number of PEOPLE makes it bind, the images that grow with
//! people shard by the first hex digit of the key's sha256; the registry never
//! shards, because it has to answer "which venue is this host" in one read.
//! That is named here and deliberately not built.

use worker::*;

use dowiz_hub::logimage::LogImage;
use dowiz_hub::table::Table;

/// The object every platform image lives in.
///
/// Two underscores: a location id is a slug and never starts with one, so this
/// name cannot collide with a venue's object however venues are created later.
pub const PLATFORM: &str = "__platform";

/// The images, and what each one holds. One `Table` per image, because a
/// transaction touches exactly one image -- two images in the same object are
/// two puts and two generations, and anything needing both is a saga that has
/// to say so.
pub const REGISTRY: &str = "registry";
pub const IDENTITY: &str = "identity";
pub const SESSIONS: &str = "sessions";
pub const COURIERS: &str = "couriers";
/// One record per address, updated in place. A Kv and NOT an append log: the
/// blueprint listed it as an EvLog and that was wrong -- "one row per address,
/// a second submit refreshes it" is an upsert, and folding an append log to
/// find the current state of a few hundred addresses would be work done to
/// reach a shape a keyed set already has.
pub const WAITLIST: &str = "waitlist";
/// THE ERASURE REGISTER (P3): one pseudonymous record per forgotten person
/// per venue, replayed after a restore so a backup cannot bring them back.
/// Its OWN image and not a kind in `registry`: the registry answers "which
/// venue is this host" in one read and must not grow with people.
pub const ERASURES: &str = "erasures";

/// The refusal point for each image, which is the only honest denominator for a
/// compacted image's usage -- see `bebop-ceiling-not-capacity`, where a healthy
/// venue read 829 per mille because the gauge used capacity instead.
///
/// Sized from what they hold rather than from a round number: the registry is
/// one record per venue, identity is one per person plus their memberships,
/// sessions turn over constantly and are swept nightly, couriers are a roster.
pub const REGISTRY_BYTES: usize = 512 * 1024;
pub const IDENTITY_BYTES: usize = 2 * 1024 * 1024;
pub const SESSIONS_BYTES: usize = 4 * 1024 * 1024;
pub const COURIERS_BYTES: usize = 1024 * 1024;
pub const WAITLIST_BYTES: usize = 1024 * 1024;
/// ~200 bytes a person plus 40 an order: thousands of erasures before it binds.
pub const ERASURES_BYTES: usize = 2 * 1024 * 1024;

/// Platform-level failures. An append log, because that is what a failure
/// record is: it arrives, it is read back newest first, and it is pruned.
pub const ERRORS: &str = "errors";

/// The ceiling for an image, by name. One place, so a caller cannot load an
/// image at one ceiling and save it at another -- which would make the doubling
/// loop refuse at a size the reader thought was fine.
pub fn ceiling(image: &str) -> usize {
    match image {
        REGISTRY => REGISTRY_BYTES,
        IDENTITY => IDENTITY_BYTES,
        SESSIONS => SESSIONS_BYTES,
        COURIERS => COURIERS_BYTES,
        WAITLIST => WAITLIST_BYTES,
        ERASURES => ERASURES_BYTES,
        // An unknown image is a programming error, not a runtime condition. A
        // generous default here would let a typo create a second image that
        // silently shadows the one that was meant.
        _ => 256 * 1024,
    }
}

/// A loaded platform image and the generation it was read at.
pub struct Loaded {
    pub table: Table,
    pub generation: i64,
}

/// The same, for an append-only image.
pub struct LoadedLog {
    pub log: LogImage,
    pub generation: i64,
}

/// Read one append-only image from an object, creating an empty one the first
/// time.
pub async fn load_log_at(stub: &Stub, image: &str) -> Result<LoadedLog> {
    let mut res = stub.fetch_with_str(&format!("https://hub/img/{image}")).await?;
    if res.status_code() == 200 {
        let generation = res
            .headers()
            .get("x-generation")
            .ok()
            .flatten()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let bytes = res.bytes().await?;
        let log = LogImage::load(&bytes)
            .map_err(|_| Error::RustError(format!("log image {image} is unreadable")))?;
        return Ok(LoadedLog { log, generation });
    }
    let log = LogImage::create()
        .map_err(|_| Error::RustError(format!("cannot create log image {image}")))?;
    Ok(LoadedLog { log, generation: 0 })
}

/// Write one append-only image back, under the generation it was read at.
pub async fn save_log_at(stub: &Stub, image: &str, loaded: &LoadedLog) -> Result<bool> {
    let mut req = Request::new_with_init(
        &format!("https://hub/img/{image}"),
        RequestInit::new().with_method(Method::Put).with_body(Some(loaded.log.to_bytes().into())),
    )?;
    req.headers_mut()?.set("x-generation", &loaded.generation.to_string())?;
    let res = stub.fetch_with_request(req).await?;
    Ok(res.status_code() != 409)
}

/// Read, append, write — with a bounded retry when the guard is lost.
pub async fn with_log_at<F, T>(stub: &Stub, image: &str, mut f: F) -> Result<T>
where
    F: FnMut(&mut LogImage) -> Result<T>,
{
    for _ in 0..4 {
        let mut loaded = load_log_at(stub, image).await?;
        let out = f(&mut loaded.log)?;
        if save_log_at(stub, image, &loaded).await? {
            return Ok(out);
        }
    }
    Err(Error::RustError(format!("log image {image}: four writers won the guard in a row")))
}

fn stub(env: &Env) -> Result<Stub> {
    env.durable_object("HUB")?.id_from_name(PLATFORM)?.get_stub()
}

// ── the primitives, over any object ────────────────────────────────────────
//
// A venue's images and the platform's are the same mechanism on the same class;
// only the object's name differs. These take the stub so `hubstore` can offer
// the venue side without a second copy of the generation guard -- which is the
// kind of duplication that ends with two guards behaving differently.

/// Read one table image from an object, creating an empty one the first time.
pub async fn load_at(stub: &Stub, image: &str, ceiling_bytes: usize) -> Result<Loaded> {
    let mut res = stub.fetch_with_str(&format!("https://hub/img/{image}")).await?;
    if res.status_code() == 200 {
        let generation = res
            .headers()
            .get("x-generation")
            .ok()
            .flatten()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let bytes = res.bytes().await?;
        let table = Table::load(&bytes, ceiling_bytes)
            // A CORRUPT IMAGE IS NOT AN EMPTY ONE. Reading it as empty is the
            // `.ok().flatten()` defect that would have orphaned a venue's whole
            // order log: a failure converted into an absence.
            .map_err(|_| Error::RustError(format!("image {image} is unreadable")))?;
        return Ok(Loaded { table, generation });
    }
    let table = Table::create(ceiling_bytes)
        .map_err(|_| Error::RustError(format!("cannot create image {image}")))?;
    Ok(Loaded { table, generation: 0 })
}

/// Write one table image back, under the generation it was read at.
pub async fn save_at(stub: &Stub, image: &str, loaded: &mut Loaded) -> Result<bool> {
    let bytes = loaded
        .table
        .to_bytes()
        .map_err(|e| Error::RustError(format!("image {image} will not serialise: {e:?}")))?;
    let mut req = Request::new_with_init(
        &format!("https://hub/img/{image}"),
        RequestInit::new().with_method(Method::Put).with_body(Some(bytes.into())),
    )?;
    req.headers_mut()?.set("x-generation", &loaded.generation.to_string())?;
    let res = stub.fetch_with_request(req).await?;
    Ok(res.status_code() != 409)
}

/// Read, change, write — with a bounded retry when the guard is lost.
///
/// THE RETRY IS BOUNDED AND THE REFUSAL IS TYPED. An unbounded retry on a
/// contended image is a request that never returns, and a silent give-up is a
/// write the caller believes happened. Four attempts is enough for an object
/// that serialises its own calls, and the fifth failure says so out loud
/// instead of pretending the write landed.
pub async fn with_at<F, T>(stub: &Stub, image: &str, ceiling_bytes: usize, mut f: F) -> Result<T>
where
    F: FnMut(&mut Table) -> Result<T>,
{
    for _ in 0..4 {
        let mut loaded = load_at(stub, image, ceiling_bytes).await?;
        let out = f(&mut loaded.table)?;
        if save_at(stub, image, &mut loaded).await? {
            return Ok(out);
        }
    }
    Err(Error::RustError(format!("image {image}: four writers won the guard in a row")))
}

/// Read one platform image, creating an empty one the first time.
///
/// A VENUE'S FIRST READ CANNOT ADOPT ANYTHING. `hubstore::do_image` has a
/// legacy fallback that seeds an object from the old `hub_image` table, scoped
/// to one named venue because unscoped it served one tenant's data from
/// another's hostname. There is no such path here: the platform images have no
/// legacy, and an absent image is an empty one.
pub async fn load(env: &Env, image: &str) -> Result<Loaded> {
    load_at(&stub(env)?, image, ceiling(image)).await
}

/// Read, change, write, on the platform object.
pub async fn with<F, T>(env: &Env, image: &str, f: F) -> Result<T>
where
    F: FnMut(&mut Table) -> Result<T>,
{
    with_at(&stub(env)?, image, ceiling(image), f).await
}
