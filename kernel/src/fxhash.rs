//! fxhash.rs — std host shim (pure `FxHasher`/`FxBuildHasher` live in
//! `dowiz_core::fxhash`; the std-only `HashMap`/`HashSet` aliases stay here).
//!
//! The no_std core ships the deterministic multiply-xor hasher; these aliases
//! bind it to `std::collections::HashMap`/`HashSet` for the *determinism* goal
//! (not the no_std goal — for no_std use `BTreeMap`/`BTreeSet` directly).

pub use dowiz_core::fxhash::*;

/// Convenience: a `HashMap`/`HashSet` type alias with the deterministic hasher.
pub type FxHashMap<K, V> = std::collections::HashMap<K, V, FxBuildHasher>;
pub type FxHashSet<K> = std::collections::HashSet<K, FxBuildHasher>;

/// Convenience constructor.
pub fn fx_hash_map<K: Eq + core::hash::Hash, V>() -> FxHashMap<K, V> {
    FxHashMap::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fx_hash_map_works() {
        let mut m: FxHashMap<&str, i32> = fx_hash_map();
        m.insert("a", 1);
        m.insert("b", 2);
        assert_eq!(m.get("a"), Some(&1));
        assert_eq!(m.len(), 2);
    }
}
