//! Living-memory persistence — content-addressed store (M4 / W4-1).
//!
//! The kernel's HARD INVARIANT is pure-`std`, NO network deps. The DEFAULT
//! living-memory store is therefore native, std-only, and content-addressed:
//! an [`InMemoryStore`] keyed in a `BTreeMap` so iteration order is
//! deterministic and a [`snapshot_root`](MemoryStore::snapshot_root) over all
//! entries yields a reproducible content hash for tamper-evidence / merge
//! ordering.
//!
//! Postgres (`pgrust`) is allowed ONLY as an OPTIONAL adapter behind a
//! NON-default feature flag. We do NOT add the pgrust dependency now — the
//! trait boundary + a feature-gated stub ship so the SQL path is opt-in later
//! (see [`PgStore`]).

use std::collections::BTreeMap;
use std::sync::Mutex;

/// Deterministic content-addressable living-memory contract.
///
/// Implemented natively by [`InMemoryStore`] (std-only, the default) and,
/// behind the `pgrust` feature, by [`PgStore`] (SQL adapter stub — OFF by
/// default). The kernel never reaches over the network to satisfy this trait;
/// the default path is entirely in-process.
pub trait MemoryStore {
    /// Store `value` under `key`. Overwrites any prior value for the key.
    fn put(&self, key: &str, value: &[u8]) -> Result<(), String>;
    /// Fetch a clone of the bytes stored under `key`, if present.
    fn get(&self, key: &str) -> Option<Vec<u8>>;
    /// All keys currently present, in sorted (deterministic) order.
    fn keys(&self) -> Vec<String>;
    /// A deterministic content hash over ALL entries (a "snapshot root").
    ///
    /// Two stores holding the same key→value mapping yield the same root, and
    /// any change to any entry changes the root. Used for tamper-evidence and
    /// content-addressed merge ordering. Stable across runs/platforms.
    fn snapshot_root(&self) -> String;
}

/// Native std-only default living-memory store (content-addressed).
///
/// Backed by a `BTreeMap` behind a `Mutex` so [`keys`](MemoryStore::keys) is
/// always returned in deterministic sorted order and
/// [`snapshot_root`](MemoryStore::snapshot_root) is reproducible. No network,
/// no SQL, no new deps — satisfies the kernel's pure-`std` red line.
pub struct InMemoryStore {
    map: Mutex<BTreeMap<String, Vec<u8>>>,
}

impl InMemoryStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self {
            map: Mutex::new(BTreeMap::new()),
        }
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryStore for InMemoryStore {
    fn put(&self, key: &str, value: &[u8]) -> Result<(), String> {
        self.map
            .lock()
            .map_err(|e| e.to_string())?
            .insert(key.to_string(), value.to_vec());
        Ok(())
    }

    fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.map.lock().ok()?.get(key).cloned()
    }

    fn keys(&self) -> Vec<String> {
        self.map
            .lock()
            .ok()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }

    fn snapshot_root(&self) -> String {
        let m = match self.map.lock() {
            Ok(m) => m,
            Err(_) => return "lock_poisoned".to_string(),
        };
        // FNV-1a 64-bit over frame-delimited (key, value) pairs. Each entry is
        // wrapped in `len || bytes` frames so keys/values can never bleed into
        // one another and the fold is canonical regardless of insertion order.
        let mut h: u64 = FNV_OFFSET;
        for (k, v) in m.iter() {
            h = fnv1a(h, &(k.len() as u64).to_le_bytes());
            h = fnv1a(h, k.as_bytes());
            h = fnv1a(h, &(v.len() as u64).to_le_bytes());
            h = fnv1a(h, v);
        }
        format!("{:016x}", h)
    }
}

/// FNV-1a 64-bit offset basis.
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
/// FNV-1a 64-bit prime.
const FNV_PRIME: u64 = 0x100000001b3;

/// Single-pass FNV-1a 64-bit fold. std-only, no deps.
fn fnv1a(mut h: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

/// Optional Postgres (`pgrust`) adapter boundary — OFF by default.
///
/// This type is compiled ONLY under the `pgrust` feature. The kernel's hard
/// invariant is pure-`std` with NO network deps, so the default living-memory
/// store is the native [`InMemoryStore`]. `pgrust` is the opt-in SQL adapter:
/// it ships HERE as a trait-boundary stub so the SQL persistence path can be
/// written against the [`MemoryStore`] contract today without pulling the
/// Postgres dependency into the default (or any non-`pgrust`) build.
///
/// It intentionally does NOT implement the Postgres wire protocol — it only
/// satisfies the [`MemoryStore`] trait with a `todo!()`-free compile stub that
/// returns `Err("pgrust adapter not built")`. The real adapter (SQL ON via
/// this feature, OFF by default) is filled in behind this flag later.
#[cfg(feature = "pgrust")]
pub struct PgStore;

#[cfg(feature = "pgrust")]
impl MemoryStore for PgStore {
    fn put(&self, _key: &str, _value: &[u8]) -> Result<(), String> {
        Err("pgrust adapter not built".to_string())
    }

    fn get(&self, _key: &str) -> Option<Vec<u8>> {
        None
    }

    fn keys(&self) -> Vec<String> {
        Vec::new()
    }

    fn snapshot_root(&self) -> String {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_store_put_get_roundtrip() {
        let s = InMemoryStore::new();
        assert!(s.get("a").is_none(), "absent key must return None");
        s.put("a", b"hello").unwrap();
        assert_eq!(s.get("a").unwrap(), b"hello".to_vec());
        // Overwrite must replace, not append.
        s.put("a", b"world").unwrap();
        assert_eq!(s.get("a").unwrap(), b"world".to_vec());
    }

    #[test]
    fn memory_store_snapshot_root_changes_on_put() {
        let s = InMemoryStore::new();
        let empty = s.snapshot_root();
        s.put("k", b"v").unwrap();
        let after_one = s.snapshot_root();
        assert_ne!(
            empty, after_one,
            "snapshot root must change after the first put"
        );
        s.put("k2", b"v2").unwrap();
        assert_ne!(
            after_one,
            s.snapshot_root(),
            "snapshot root must change after a second put"
        );
    }

    #[test]
    fn memory_store_deterministic() {
        let mut a = InMemoryStore::new();
        let mut b = InMemoryStore::new();
        // Different insertion order must NOT affect the content root.
        a.put("x", b"1").unwrap();
        a.put("y", b"2").unwrap();
        b.put("y", b"2").unwrap();
        b.put("x", b"1").unwrap();
        assert_eq!(
            a.snapshot_root(),
            b.snapshot_root(),
            "same content ⇒ same root regardless of insertion order"
        );
        // A differing value must yield a differing root.
        let mut c = InMemoryStore::new();
        c.put("x", b"1").unwrap();
        c.put("y", b"99").unwrap();
        assert_ne!(a.snapshot_root(), c.snapshot_root());
    }

    #[test]
    fn memory_store_keys_sorted() {
        let s = InMemoryStore::new();
        s.put("c", b"3").unwrap();
        s.put("a", b"1").unwrap();
        s.put("b", b"2").unwrap();
        assert_eq!(
            s.keys(),
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            "keys must be returned in deterministic sorted order"
        );
    }
}
