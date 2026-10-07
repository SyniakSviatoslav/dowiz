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

use std::collections::BTreeSet;

use bebop_store::kv::delta::{self, Appended, Op};
use bebop_store::kv::Kv;
use bebop_store::Store;

use crate::HubError;

pub mod bom;
/// The edit journal: every change, replayable to any moment (W-PITR).
pub mod edits;
/// The catalogue read in place, without decoding it (W-ZC).
pub mod view;
pub use view::{CatalogRead, CatalogView};

/// How large an image `projected` may build to measure a catalogue that does
/// NOT fit: two ceilings (20 MiB of Worker memory at worst), so an import that
/// would overflow is reported with its number (up to 2000 per mille) instead of only "no".
const PROJECTION_BYTES: usize = 2 * DEFAULT_CATALOG_BYTES;

/// What saving the catalogue now would spend, and whether it can be saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Projection {
    /// Against the real ceiling, so `used_per_mille` passes 1000 when it overflows.
    pub usage: crate::Usage,
    /// `to_bytes` would succeed.
    pub fits: bool,
}

/// The shared ceiling (10 MiB). 1 MiB was too small: dubin-sushi's 165 dishes with
/// supplies and recipes reached 844 per mille of it on 2026-09-25.
pub const DEFAULT_CATALOG_BYTES: usize = crate::CEILING_BYTES;

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
    /// W-DELTA: the keys changed since this catalogue was LOADED from bytes, so `to_bytes`
    /// can append them to `store` instead of rewriting it. `None` = created fresh: the
    /// first `to_bytes` compacts, as every write did before.
    dirty: Option<BTreeSet<String>>,
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
        let mut store = Store::create_bytes(DEFAULT_CATALOG_BYTES)?;
        Kv::init_bytes(&mut store)?;
        let kv = Kv::load(&store).ok_or(HubError::NotAHub)?;
        Ok(Catalog { store, kv, dirty: None })
    }

    pub fn load(bytes: &[u8]) -> Result<Self, HubError> {
        let store = Store::from_bytes(bytes);
        if store.pick().is_none() {
            return Err(HubError::NotAHub);
        }
        let kv = crate::kv_load(&store)?;
        Ok(Catalog { store, kv, dirty: Some(BTreeSet::new()) })
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
    ///
    /// W-DELTA (2026-10-06): a catalogue LOADED from bytes appends what changed -- one record
    /// per changed key and a new root (`bebop_store::kv::delta`) -- so the image changes only
    /// in its superblock page and at its tail. It compacts exactly as above when it was
    /// created fresh, when the chain or the dead cells pass the trigger, when the image is
    /// v1, or when the arena is full. The entries in memory are the truth either way.
    /// Nothing changed since the load: the image as it is (no new generation).
    pub fn to_bytes(&mut self) -> Result<Vec<u8>, HubError> {
        if let Some(dirty) = &self.dirty {
            if dirty.is_empty() {
                return Ok(self.store.to_bytes_trimmed());
            }
            let vals: Vec<(&String, Option<Vec<u8>>)> = dirty.iter().map(|k| (k, self.kv.get(k))).collect();
            let ops: Vec<Op> = vals
                .iter()
                .map(|(k, v)| match v {
                    Some(v) => Op::Put(k, v),
                    None => Op::Remove(k),
                })
                .collect();
            // ANY failure of the append falls back to compaction: the entries are the truth,
            // and compacting them is what every write did before this existed.
            if let Ok(Appended::Delta(_)) = delta::append_delta(&mut self.store, &ops) {
                self.dirty = Some(BTreeSet::new());
                return Ok(self.store.to_bytes_trimmed());
            }
        }
        self.compact()
    }

    /// Rewrite the image whole (v2, no chain), and carry on appending onto THAT. Public so
    /// an operator path can force it (a rollback to a build that predates v3 needs it).
    pub fn compact(&mut self) -> Result<Vec<u8>, HubError> {
        let bytes = self.kv.compacted_bytes_fit(DEFAULT_CATALOG_BYTES)?;
        self.store = Store::from_bytes(&bytes);
        self.dirty = Some(BTreeSet::new());
        Ok(bytes)
    }

    /// Record a changed key for the next `to_bytes`.
    fn touch(&mut self, key: String) {
        if let Some(d) = &mut self.dirty {
            d.insert(key);
        }
    }

    fn put(&mut self, key: String, json: &str) {
        self.kv.put(&key, json.as_bytes());
        self.touch(key);
    }

    fn del(&mut self, key: String) -> bool {
        let gone = self.kv.remove(&key);
        if gone {
            self.touch(key);
        }
        gone
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
        self.put(K_LOCATION.to_string(), json);
    }

    pub fn location(&self) -> Option<String> {
        self.kv.get(K_LOCATION).and_then(|v| String::from_utf8(v).ok())
    }

    pub fn set_product(&mut self, id: &str, json: &str) {
        self.put(format!("{P_PRODUCT}{id}"), json);
    }

    pub fn product(&self, id: &str) -> Option<String> {
        self.kv
            .get(&format!("{P_PRODUCT}{id}"))
            .and_then(|v| String::from_utf8(v).ok())
    }

    pub fn set_category(&mut self, id: &str, json: &str) {
        self.put(format!("{P_CATEGORY}{id}"), json);
    }

    /// Every product, in key order. Keys are kept sorted by the layout, so this
    /// is stable across runs rather than incidentally ordered.
    pub fn products(&self) -> Vec<(String, String)> {
        self.entries_with_prefix(P_PRODUCT)
    }

    pub fn set_supply(&mut self, id: &str, json: &str) {
        self.put(format!("{P_SUPPLY}{id}"), json);
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
        self.put(format!("{P_PROMO}{code}"), json);
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
        self.del(format!("{P_PROMO}{code}"))
    }

    /// Removing a supply is a real delete, used only by the owner's
    /// ingredients reset: `active: false` (retire) keeps the row and its id,
    /// and a supply re-created under the same id would inherit its old shelf.
    pub fn remove_supply(&mut self, id: &str) -> bool {
        self.del(format!("{P_SUPPLY}{id}"))
    }

    /// Removing a dish is a real delete, for the same reason removing a promo
    /// is: a dish the venue took off the menu must stop being orderable, and
    /// `available: false` is the separate, reversible thing that says "not
    /// today". Used by the catalogue seed when it is asked to REPLACE rather
    /// than merge -- an import that only ever adds leaves the placeholder menu
    /// sitting beside the real one, which is how a venue ends up selling
    /// eighteen dishes it does not make.
    pub fn remove_product(&mut self, id: &str) -> bool {
        self.del(format!("{P_PRODUCT}{id}"))
    }

    pub fn remove_category(&mut self, id: &str) -> bool {
        self.del(format!("{P_CATEGORY}{id}"))
    }

    pub fn categories(&self) -> Vec<(String, String)> {
        self.entries_with_prefix(P_CATEGORY)
    }

    /// One pass over the entries (W-ZC): this was `keys()` and then a linear `get` per
    /// key, O(n^2) for every `products()`.
    fn entries_with_prefix(&self, prefix: &str) -> Vec<(String, String)> {
        self.kv
            .entries
            .iter()
            .filter(|(k, _)| k.starts_with(prefix))
            .filter_map(|(k, v)| Some((k[prefix.len()..].to_string(), String::from_utf8(v.clone()).ok()?)))
            .collect()
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod delta_tests;
