//! The hub's catalogue: the venue and its menu, in a bebop KV store.
//!
//! A SECOND image beside the order log, and deliberately so. A bebop store has
//! ONE root, and the log's root and the KV root are different layouts — they
//! cannot share an image. The split is not a workaround: the two have opposite
//! lifecycles. The log is append-only and grows forever; the catalogue is small,
//! rewritten whole, and only ever holds the current menu.
//!
//! COST, stated up front. `commit` here is EAGER: it rewrites all four arrays and
//! the root on every write, which is O(n). At one restaurant — fifty products,
//! sixteen categories — that is a few kilobytes and nothing to defend. It would
//! be the wrong shape for a platform, and that is what the hub-per-tenant
//! architecture makes irrelevant rather than what it ignores.

use bebop_store::kv::Kv;
use bebop_store::Store;

use crate::HubError;

pub mod bom;

/// How large an image `projected` may build to measure a catalogue that does
/// NOT fit: four ceilings, so an import that would overflow is reported with
/// its number (up to 4000 per mille) instead of only "no".
const PROJECTION_BYTES: usize = 4 * DEFAULT_CATALOG_BYTES;

/// What saving the catalogue now would spend, and whether it can be saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Projection {
    /// Against the real ceiling, so `used_per_mille` passes 1000 when it overflows.
    pub usage: crate::Usage,
    /// `to_bytes` would succeed.
    pub fits: bool,
}

/// 1 MiB is generous for a menu: fifty products of JSON is tens of kilobytes.
pub const DEFAULT_CATALOG_BYTES: usize = 1024 * 1024;

const K_LOCATION: &str = "location";
const P_PRODUCT: &str = "product:";
const P_CATEGORY: &str = "category:";
/// An ingredient the kitchen holds: salmon, rice, a box of takeaway lids.
///
/// Kept in the CATALOGUE rather than the stock log because a supply's name and
/// unit are description, not history. The log holds what happened to it; this
/// holds what it is.
const P_SUPPLY: &str = "supply:";
/// A promo code. Stored under the NORMALISED code itself rather than a
/// generated id, so two promos sharing a code cannot exist: the second write
/// replaces the first instead of creating a pair the lookup would have to
/// choose between.
const P_PROMO: &str = "promo:";

pub struct Catalog {
    store: Store,
    kv: Kv,
}

impl Catalog {
    /// What this image has spent. See [`crate::Usage`].
    ///
    /// The catalogue is rewritten COMPACTED on every save, so the capacity in the
    /// image is whatever the doubling loop last picked and is not the limit.
    /// What refuses a write is `compacted_bytes_fit` running out of doublings
    /// at `DEFAULT_CATALOG_BYTES`, so that is the ceiling measured against.
    pub fn usage(&self) -> crate::Usage {
        crate::usage_of(&self.store, crate::ceiling_cells(DEFAULT_CATALOG_BYTES))
    }

    pub fn create() -> Result<Self, HubError> {
        let mut store = Store::create_bytes(DEFAULT_CATALOG_BYTES);
        Kv::init_bytes(&mut store)?;
        let kv = Kv::load(&store).ok_or(HubError::NotAHub)?;
        Ok(Catalog { store, kv })
    }

    pub fn load(bytes: &[u8]) -> Result<Self, HubError> {
        let store = Store::from_bytes(bytes);
        if store.pick().is_none() {
            return Err(HubError::NotAHub);
        }
        let kv = Kv::load(&store).ok_or(HubError::NotAHub)?;
        Ok(Catalog { store, kv })
    }

    /// Flush the in-memory entries into the image and hand back the bytes. The
    /// caller persists them; nothing here touches storage.
    /// THE IMAGE IS REWRITTEN WHOLE, not appended to.
    ///
    /// The store is append-only: every commit allocates a new generation and
    /// the old one is never reclaimed. For a KV that rewrites the same small
    /// map over and over, the arena is spent by the NUMBER OF WRITES rather
    /// than by the data -- measured at 313 empty commits before a fresh roster
    /// refused, while five hundred sessions in ONE commit fitted easily. A hub
    /// would therefore stop accepting logins after a few hundred of them.
    ///
    /// `compacted_bytes` commits the entries into a fresh image, so the file is
    /// as large as its content rather than as large as its history. See
    /// `Kv::compacted_bytes` for what that gives up (nothing anything here
    /// reads).
    pub fn to_bytes(&mut self) -> Result<Vec<u8>, HubError> {
        Ok(self.kv.compacted_bytes_fit(DEFAULT_CATALOG_BYTES)?)
    }

    /// What [`Self::to_bytes`] would spend, measured by compacting a copy, and
    /// whether it would succeed. Writes nothing: a DRY RUN asks this before an
    /// import, so "it would not fit" is said before Apply, not after it.
    /// Beyond four ceilings the answer is the store's own `ArenaFull` error.
    pub fn projected(&self) -> Result<Projection, HubError> {
        let ceiling = crate::ceiling_cells(DEFAULT_CATALOG_BYTES);
        let (bytes, fits) = match self.kv.compacted_bytes(DEFAULT_CATALOG_BYTES) {
            Ok(b) => (b, true),
            Err(e) if crate::e_is_full(&e) => (self.kv.compacted_bytes(PROJECTION_BYTES)?, false),
            Err(e) => return Err(e.into()),
        };
        Ok(Projection { usage: crate::usage_of(&Store::from_bytes(&bytes), ceiling), fits })
    }

    /// A fingerprint of the whole catalogue. Two hubs holding the same menu
    /// produce the same root, which is what makes a mirror checkable rather
    /// than assumed.
    pub fn root(&self) -> String {
        self.kv.snapshot_root()
    }

    pub fn set_location(&mut self, json: &str) {
        self.kv.put(K_LOCATION, json.as_bytes());
    }

    pub fn location(&self) -> Option<String> {
        self.kv.get(K_LOCATION).and_then(|v| String::from_utf8(v).ok())
    }

    pub fn set_product(&mut self, id: &str, json: &str) {
        self.kv.put(&format!("{P_PRODUCT}{id}"), json.as_bytes());
    }

    pub fn product(&self, id: &str) -> Option<String> {
        self.kv
            .get(&format!("{P_PRODUCT}{id}"))
            .and_then(|v| String::from_utf8(v).ok())
    }

    pub fn set_category(&mut self, id: &str, json: &str) {
        self.kv.put(&format!("{P_CATEGORY}{id}"), json.as_bytes());
    }

    /// Every product, in key order. Keys are kept sorted by the layout, so this
    /// is stable across runs rather than incidentally ordered.
    pub fn products(&self) -> Vec<(String, String)> {
        self.entries_with_prefix(P_PRODUCT)
    }

    pub fn set_supply(&mut self, id: &str, json: &str) {
        self.kv.put(&format!("{P_SUPPLY}{id}"), json.as_bytes());
    }

    pub fn supply(&self, id: &str) -> Option<String> {
        self.kv
            .get(&format!("{P_SUPPLY}{id}"))
            .map(|v| String::from_utf8_lossy(&v).into_owned())
    }

    pub fn supplies(&self) -> Vec<(String, String)> {
        self.entries_with_prefix(P_SUPPLY)
    }

    pub fn set_promo(&mut self, code: &str, json: &str) {
        self.kv.put(&format!("{P_PROMO}{code}"), json.as_bytes());
    }

    pub fn promo(&self, code: &str) -> Option<String> {
        self.kv
            .get(&format!("{P_PROMO}{code}"))
            .map(|v| String::from_utf8_lossy(&v).into_owned())
    }

    pub fn promos(&self) -> Vec<(String, String)> {
        self.entries_with_prefix(P_PROMO)
    }

    /// Removing a promo is a real delete, not a flag. A code the owner deleted
    /// must stop working; `active: false` is the separate, reversible thing.
    pub fn remove_promo(&mut self, code: &str) -> bool {
        self.kv.remove(&format!("{P_PROMO}{code}"))
    }

    /// Removing a dish is a real delete, for the same reason removing a promo
    /// is: a dish the venue took off the menu must stop being orderable, and
    /// `available: false` is the separate, reversible thing that says "not
    /// today". Used by the catalogue seed when it is asked to REPLACE rather
    /// than merge -- an import that only ever adds leaves the placeholder menu
    /// sitting beside the real one, which is how a venue ends up selling
    /// eighteen dishes it does not make.
    pub fn remove_product(&mut self, id: &str) -> bool {
        self.kv.remove(&format!("{P_PRODUCT}{id}"))
    }

    pub fn remove_category(&mut self, id: &str) -> bool {
        self.kv.remove(&format!("{P_CATEGORY}{id}"))
    }

    pub fn categories(&self) -> Vec<(String, String)> {
        self.entries_with_prefix(P_CATEGORY)
    }

    fn entries_with_prefix(&self, prefix: &str) -> Vec<(String, String)> {
        self.kv
            .keys()
            .into_iter()
            .filter(|k| k.starts_with(prefix))
            .filter_map(|k| {
                let id = k[prefix.len()..].to_string();
                let v = self.kv.get(&k)?;
                Some((id, String::from_utf8(v).ok()?))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
