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

fn stub(env: &Env) -> Result<Stub> {
    env.durable_object("HUB")?.id_from_name(PLATFORM)?.get_stub()
}

/// Read one platform image, creating an empty one the first time.
///
/// A VENUE'S FIRST READ CANNOT ADOPT ANYTHING. `hubstore::do_image` has a
/// legacy fallback that seeds an object from the old `hub_image` table, scoped
/// to one named venue because unscoped it served one tenant's data from
/// another's hostname. There is no such path here: the platform images have no
/// legacy, and an absent image is an empty one.
pub async fn load(env: &Env, image: &str) -> Result<Loaded> {
    let stub = stub(env)?;
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
        let table = Table::load(&bytes, ceiling(image))
            // A CORRUPT IMAGE IS NOT AN EMPTY ONE. Reading it as empty is the
            // `.ok().flatten()` defect that would have orphaned a venue's whole
            // order log: a failure converted into an absence.
            .map_err(|_| Error::RustError(format!("platform image {image} is unreadable")))?;
        return Ok(Loaded { table, generation });
    }
    let table = Table::create(ceiling(image))
        .map_err(|_| Error::RustError(format!("cannot create platform image {image}")))?;
    Ok(Loaded { table, generation: 0 })
}

/// Write one platform image back, under the generation it was read at.
///
/// `false` means somebody else wrote first. The object serialises its own
/// calls, so this can only happen when two Workers read the same generation
/// before either wrote -- and the caller's answer is to re-read and re-decide,
/// never to force. `compare_and_swap` in the sense the hub blueprint means it.
pub async fn save(env: &Env, image: &str, loaded: &mut Loaded) -> Result<bool> {
    let bytes = loaded
        .table
        .to_bytes()
        .map_err(|e| Error::RustError(format!("platform image {image} will not serialise: {e:?}")))?;
    let stub = stub(env)?;
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
/// write the caller believes happened. Four attempts is enough for a
/// serialisation point that only two Workers ever reach at once, and the fifth
/// failure says so out loud.
pub async fn with<F, T>(env: &Env, image: &str, mut f: F) -> Result<T>
where
    F: FnMut(&mut Table) -> Result<T>,
{
    for attempt in 0..4 {
        let mut loaded = load(env, image).await?;
        let out = f(&mut loaded.table)?;
        if save(env, image, &mut loaded).await? {
            return Ok(out);
        }
        let _ = attempt;
    }
    Err(Error::RustError(format!(
        "platform image {image}: four writers won the guard in a row"
    )))
}
