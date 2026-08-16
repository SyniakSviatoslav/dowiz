//! Living-memory persistence — content-addressed store (M4 / W4-1): the pure half.
//!
//! The `no_std` core of the kernel's living-memory store: an [`InMemoryStore`]
//! keyed in a `BTreeMap` (behind a [`SpinLock`]) so iteration order is
//! deterministic and a [`snapshot_root`](MemoryStore::snapshot_root) over all
//! entries yields a reproducible content hash for tamper-evidence / merge
//! ordering.
//!
//! Zero `std` — `alloc` only (`BTreeMap`, `Vec`, `String`) plus the core
//! [`SpinLock`]. The std-only Postgres (`pgrust`) adapter — a REAL
//! `sqlx`-backed store — lives in the kernel shim
//! (`kernel/src/retrieval/memory_store.rs`), which re-exports this module and
//! adds `PgStore` behind the NON-default `pgrust` feature flag.

use crate::spinlock::SpinLock;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Deterministic content-addressable living-memory contract.
///
/// Implemented natively by [`InMemoryStore`] (pure, the default) and, behind
/// the `pgrust` feature in the kernel shim, by the SQL adapter (`PgStore`).
/// The default path is entirely in-process; no network, no SQL, no new deps.
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

/// Native pure default living-memory store (content-addressed).
///
/// Backed by a `BTreeMap` behind a [`SpinLock`] so [`keys`](MemoryStore::keys)
/// is always returned in deterministic sorted order and
/// [`snapshot_root`](MemoryStore::snapshot_root) is reproducible. No network,
/// no SQL, no new deps.
pub struct InMemoryStore {
    map: SpinLock<BTreeMap<String, Vec<u8>>>,
}

impl InMemoryStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self {
            map: SpinLock::new(BTreeMap::new()),
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
///
/// `pub` so the kernel shim's `PgStore` can fold the SAME basis over the same
/// `len || bytes` frames and produce roots comparable to [`InMemoryStore`].
pub const FNV_OFFSET: u64 = 0xcbf29ce484222325;
/// FNV-1a 64-bit prime.
const FNV_PRIME: u64 = 0x100000001b3;

/// Single-pass FNV-1a 64-bit fold. Pure, no deps.
///
/// `pub` so the kernel shim's `PgStore` reuses the identical fold for
/// cross-store-comparable [`snapshot_root`](MemoryStore::snapshot_root) roots.
pub fn fnv1a(mut h: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
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
        let a = InMemoryStore::new();
        let b = InMemoryStore::new();
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
        let c = InMemoryStore::new();
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
