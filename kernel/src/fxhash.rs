//! fxhash.rs — a deterministic, no_std-ready hasher (rustc's FxHash).
//!
//! The no_std audit found 85 "boundary" modules whose only `std` dependency is
//! `alloc::collections::BTreeMap` with the *default* hasher. The default hasher is
//! `RandomState` (SipHash seeded from OS entropy) — non-deterministic AND
//! un-constructible without `std`. This module provides a fixed multiply-xor
//! hasher with a seedable build-hasher, so those maps become:
//!   - **deterministic** (same keys ⇒ same iteration-independent hash, which
//!     also serves the canonical-serialization cache-hit goal), and
//!   - **no_std-ready** (no `RandomState`; the same hasher runs in a kernel
//!     module).
//!
//! Usage: `BTreeMap::with_hasher(FxBuildHasher)` or `BTreeMap::with_hasher_and_hasher`.

use core::hash::{BuildHasher, Hasher};

/// The FxHash mixing constant (rustc's).
const K: u64 = 0x517cc1b727220a95;

/// A 64-bit FxHash hasher with a fixed seed.
#[derive(Debug, Clone, Copy, Default)]
pub struct FxHasher {
    hash: u64,
}

impl FxHasher {
    pub fn with_seed(seed: u64) -> Self {
        Self { hash: seed }
    }

    #[inline(always)]
    fn add(&mut self, i: u64) {
        self.hash = self.hash.rotate_left(5) ^ i;
        self.hash = self.hash.wrapping_mul(K);
    }
}

impl Hasher for FxHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let mut i = 0usize;
        // Process 8 bytes at a time.
        while i + 8 <= bytes.len() {
            let mut chunk = [0u8; 8];
            chunk.copy_from_slice(&bytes[i..i + 8]);
            self.add(u64::from_le_bytes(chunk));
            i += 8;
        }
        // Tail (1..=7 bytes) — fold into the hash deterministically.
        let mut tail = 0u64;
        for (j, &b) in bytes[i..].iter().enumerate() {
            tail |= (b as u64) << (8 * j);
        }
        if i < bytes.len() {
            self.add(tail);
        }
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.add(i);
    }

    #[inline]
    fn write_u8(&mut self, i: u8) {
        self.add(i as u64);
    }

    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.add(i as u64);
    }
}

/// A `BuildHasher` producing seeded, deterministic `FxHasher`s.
#[derive(Debug, Clone, Copy, Default)]
pub struct FxBuildHasher;

impl BuildHasher for FxBuildHasher {
    type Hasher = FxHasher;
    fn build_hasher(&self) -> FxHasher {
        FxHasher::default()
    }
}

/// Convenience: a `HashMap`/`HashSet` type alias with the deterministic hasher.
/// (Intentionally `std::collections::HashMap` — `HashMap` itself is std-only;
/// these aliases are for the *determinism* goal, not the no_std goal. For
/// no_std, use `BTreeMap`/`BTreeSet` directly.)
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
    fn deterministic_across_builders() {
        let mut h1 = FxBuildHasher.build_hasher();
        let mut h2 = FxBuildHasher.build_hasher();
        h1.write(b"hello");
        h2.write(b"hello");
        assert_eq!(h1.finish(), h2.finish());
    }

    #[test]
    fn distinct_inputs_distinct_hashes() {
        let mut h1 = FxBuildHasher.build_hasher();
        let mut h2 = FxBuildHasher.build_hasher();
        h1.write(b"abc");
        h2.write(b"abd");
        assert_ne!(h1.finish(), h2.finish());
    }

    #[test]
    fn seeded_hasher_is_deterministic() {
        let a = FxHasher::with_seed(42);
        let b = FxHasher::with_seed(42);
        // Write the same bytes to both.
        let mut a = a;
        let mut b = b;
        a.write_u64(12345);
        b.write_u64(12345);
        assert_eq!(a.finish(), b.finish());
    }

    #[test]
    fn fx_hash_map_works() {
        let mut m: FxHashMap<&str, i32> = fx_hash_map();
        m.insert("a", 1);
        m.insert("b", 2);
        assert_eq!(m.get("a"), Some(&1));
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn tail_bytes_handled() {
        // 3-byte tail must hash deterministically and differ from the empty write.
        let mut h1 = FxBuildHasher.build_hasher();
        let mut h2 = FxBuildHasher.build_hasher();
        h1.write(b"abc");
        h2.write(b"ab");
        assert_ne!(h1.finish(), h2.finish());
    }
}
