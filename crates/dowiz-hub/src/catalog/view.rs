//! THE CATALOGUE READ IN PLACE (W-ZC, R-BEBOPDB D.1 #1, 2026-10-06).
//!
//! `Catalog::load` copies the image into a `Store` and decodes all ~180 entries before
//! `product(id)` can answer -- the whole cost of every Durable Object handler that wanted
//! one dish (FRESH 368-787 us at 165 dishes, report F.3 W1). `CatalogView` answers the
//! same reads from the borrowed bytes through `bebop_store::kv::zc::KvIn`: a binary
//! search over the sorted keys and the one value. Writers keep `Catalog` (its `put` and
//! `compacted_bytes` need the entries in memory); nothing here writes.
//!
//! THE CRC. `open` is the full W-CRC check (the five objects, every slice, the key
//! order) and refuses what `Catalog::load` refuses, with the same `HubError`s. Its cost
//! is the crc of the value blob, because the format has no per-value crc. A holder that
//! keeps the bytes (the DO, per image generation) keeps `checked()` and `reopen`s with
//! it: O(1) and no crc (`workers/api/src/hubdo/catview.rs`).
//!
//! EVERY ANSWER IS `Catalog`'s, byte for byte -- the same UTF-8 strictness per method --
//! and `view::tests::the_view_answers_what_catalog_load_answers` walks every fixture
//! and catalogue shape in the tree to hold that.

use bebop_store::kv::zc::KvIn;
/// What `CatalogView::open` proved, for `reopen` (re-exported so a holder need not name bebop-store).
pub use bebop_store::kv::zc::Checked;
use bebop_store::verify::KvError;
use bebop_store::View;

use super::{Catalog, K_LOCATION, P_CATEGORY, P_PRODUCT, P_PROMO, P_SUPPLY};
use crate::HubError;

/// The catalogue's reads, answered by the decoded `Catalog` or by the in-place
/// `CatalogView` alike -- what a pure function takes when it only reads.
pub trait CatalogRead {
    fn location(&self) -> Option<String>;
    fn product(&self, id: &str) -> Option<String>;
    fn products(&self) -> Vec<(String, String)>;
    fn supply(&self, id: &str) -> Option<String>;
    fn supplies(&self) -> Vec<(String, String)>;
    fn promo(&self, code: &str) -> Option<String>;
    fn promos(&self) -> Vec<(String, String)>;
    fn categories(&self) -> Vec<(String, String)>;
    fn root(&self) -> String;
}

impl CatalogRead for Catalog {
    fn location(&self) -> Option<String> {
        Catalog::location(self)
    }
    fn product(&self, id: &str) -> Option<String> {
        Catalog::product(self, id)
    }
    fn products(&self) -> Vec<(String, String)> {
        Catalog::products(self)
    }
    fn supply(&self, id: &str) -> Option<String> {
        Catalog::supply(self, id)
    }
    fn supplies(&self) -> Vec<(String, String)> {
        Catalog::supplies(self)
    }
    fn promo(&self, code: &str) -> Option<String> {
        Catalog::promo(self, code)
    }
    fn promos(&self) -> Vec<(String, String)> {
        Catalog::promos(self)
    }
    fn categories(&self) -> Vec<(String, String)> {
        Catalog::categories(self)
    }
    fn root(&self) -> String {
        Catalog::root(self)
    }
}

/// A catalogue image, borrowed and read in place.
pub struct CatalogView<'a> {
    kv: KvIn<View<'a>>,
}

/// `kv_load`'s mapping: a changed byte keeps its name.
fn refusal(e: KvError) -> HubError {
    match e {
        KvError::NotKv => HubError::NotAHub,
        KvError::BadCrc(b) => HubError::BadCrc(b),
    }
}

impl<'a> CatalogView<'a> {
    /// The full check, once. Refuses exactly what `Catalog::load` refuses.
    pub fn open(bytes: &'a [u8]) -> Result<Self, HubError> {
        Ok(CatalogView { kv: KvIn::open(View::new(bytes)).map_err(refusal)? })
    }

    /// For bytes this holder already `open`ed: no crc, O(1). See the module.
    pub fn reopen(bytes: &'a [u8], checked: Checked) -> Result<Self, HubError> {
        Ok(CatalogView { kv: KvIn::reopen(View::new(bytes), checked).map_err(refusal)? })
    }

    /// What `open` learned, for `reopen`.
    pub fn checked(&self) -> Checked {
        self.kv.checked()
    }

    /// The value under `key` as UTF-8, `strict` as `Catalog` is per method. A slice the
    /// image does not hold is an absent value -- after `open` there is none; after a
    /// `reopen` on bytes that were not the ones checked it is still never a panic.
    fn text(&self, key: &str, strict: bool) -> Option<String> {
        let v = self.kv.get(key.as_bytes()).ok()??;
        if strict {
            std::str::from_utf8(&v).ok().map(str::to_string)
        } else {
            Some(String::from_utf8_lossy(&v).into_owned())
        }
    }

    /// `Catalog::entries_with_prefix`: the id after the prefix, values strictly UTF-8.
    fn with_prefix(&self, prefix: &str) -> Vec<(String, String)> {
        let Ok(range) = self.kv.prefix_range(prefix.as_bytes()) else { return Vec::new() };
        range
            .into_iter()
            .filter_map(|i| {
                let k = self.kv.key(i).ok()?;
                let k = String::from_utf8_lossy(&k);
                let v = self.kv.value(i).ok()?;
                Some((k.get(prefix.len()..)?.to_string(), std::str::from_utf8(&v).ok()?.to_string()))
            })
            .collect()
    }
}

impl CatalogRead for CatalogView<'_> {
    fn location(&self) -> Option<String> {
        self.text(K_LOCATION, true)
    }
    fn product(&self, id: &str) -> Option<String> {
        self.text(&format!("{P_PRODUCT}{id}"), true)
    }
    fn products(&self) -> Vec<(String, String)> {
        self.with_prefix(P_PRODUCT)
    }
    fn supply(&self, id: &str) -> Option<String> {
        self.text(&format!("{P_SUPPLY}{id}"), false)
    }
    fn supplies(&self) -> Vec<(String, String)> {
        self.with_prefix(P_SUPPLY)
    }
    fn promo(&self, code: &str) -> Option<String> {
        self.text(&format!("{P_PROMO}{code}"), false)
    }
    fn promos(&self) -> Vec<(String, String)> {
        self.with_prefix(P_PROMO)
    }
    fn categories(&self) -> Vec<(String, String)> {
        self.with_prefix(P_CATEGORY)
    }
    fn root(&self) -> String {
        format!("{:016x}", self.kv.snapshot_root_u64().unwrap_or(0))
    }
}

#[cfg(test)]
pub(crate) mod tests;
