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
    pub fn to_bytes(&mut self) -> Result<Vec<u8>, HubError> {
        self.kv.commit_into_bytes(&mut self.store)?;
        Ok(self.store.to_bytes())
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
mod tests {
    use super::*;

    #[test]
    fn a_catalogue_round_trips_through_bytes() {
        let mut c = Catalog::create().unwrap();
        c.set_location(r#"{"name":"Dubin & Sushi","currency":"ALL"}"#);
        c.set_category("cat_chef", r#"{"name":"Chef's Picks"}"#);
        c.set_product("p1", r#"{"name":"Sake Futomaki","price":900}"#);
        c.set_product("p2", r#"{"name":"Ebi Futomaki","price":850}"#);
        let bytes = c.to_bytes().unwrap();

        let back = Catalog::load(&bytes).unwrap();
        assert!(back.location().unwrap().contains("Dubin"));
        assert_eq!(back.products().len(), 2);
        assert_eq!(back.categories().len(), 1);
        assert!(back.product("p1").unwrap().contains("900"));
    }

    #[test]
    fn products_and_categories_do_not_leak_into_each_other() {
        let mut c = Catalog::create().unwrap();
        c.set_product("x", "{}");
        c.set_category("x", "{}");
        let bytes = c.to_bytes().unwrap();
        let back = Catalog::load(&bytes).unwrap();
        // Same id, different namespaces: a prefix collision here would show one
        // as the other.
        assert_eq!(back.products().len(), 1);
        assert_eq!(back.categories().len(), 1);
        assert_eq!(back.products()[0].0, "x");
        assert_eq!(back.categories()[0].0, "x");
    }

    /// The root is a fingerprint of the CONTENT, so an identical menu on two
    /// hubs is checkably identical and a changed price is checkably different.
    #[test]
    fn the_root_follows_the_content() {
        let mut a = Catalog::create().unwrap();
        a.set_product("p1", r#"{"price":900}"#);
        let _ = a.to_bytes().unwrap();

        let mut b = Catalog::create().unwrap();
        b.set_product("p1", r#"{"price":900}"#);
        let _ = b.to_bytes().unwrap();
        assert_eq!(a.root(), b.root(), "same menu, same root");

        b.set_product("p1", r#"{"price":950}"#);
        let _ = b.to_bytes().unwrap();
        assert_ne!(a.root(), b.root(), "one changed price must change the root");
    }

    /// A deleted promo must be gone from the IMAGE, not just from the in-memory
    /// entries. The commit rewrites all four arrays, so a delete that only
    /// dropped the entry would still be readable after a reload.
    #[test]
    fn a_deleted_promo_does_not_come_back_after_a_reload() {
        let mut c = Catalog::create().unwrap();
        c.set_promo("SAVE10", r#"{"code":"SAVE10","kind":"percent","value":10}"#);
        c.set_promo("WELCOME", r#"{"code":"WELCOME","kind":"fixed","value":300}"#);
        let _ = c.to_bytes().unwrap();

        assert!(c.remove_promo("SAVE10"));
        assert!(!c.remove_promo("SAVE10"), "removing it twice is not a second delete");
        let bytes = c.to_bytes().unwrap();

        let back = Catalog::load(&bytes).unwrap();
        assert_eq!(back.promos().len(), 1);
        assert!(back.promo("SAVE10").is_none(), "the deleted code is readable after reload");
        assert!(back.promo("WELCOME").is_some());
    }

    #[test]
    fn load_refuses_a_non_store() {
        assert!(matches!(Catalog::load(&[0u8; 4096]), Err(HubError::NotAHub)));
    }
}
