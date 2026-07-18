//! Generational-index slot arena — a `Copy`-handle, per-element arena whose stale
//! handles are a safe `None` (the ABA / stale-index bug is unrepresentable).
//!
//! Thin dowiz-flavored wrapper over [`thunderdome::Arena`] (a real, zero-runtime-
//! dependency generational-index crate: 8-byte keys, niche-packed 8-byte
//! `Option<Index>`). OFF by default — compiled ONLY under the `slot-arena` feature,
//! so the canonical order/money core pulls zero extra crates (same opt-in discipline
//! as `pq` / `gpu` / `pgrust`).
//!
//! # Why this exists (deep-dive §5 — operator override of the "no adoption yet" verdict)
//!
//! `docs/research/OPUS-PERF-ARENA-DEEPDIVE-2026-07-18.md` swept the tree six ways and
//! found **no current** code pattern that needs generational-index safety — its own
//! verdict was "(c) no adoption now". The operator **explicitly overrode** that verdict
//! after hearing it: land thunderdome now as forward-looking infrastructure, behind a
//! feature so it costs the default build nothing. This module is that integration surface.
//!
//! The first named trigger the deep-dive parked (§5.2) is **an incremental mesh/graph
//! index that deletes nodes while other structures still hold references to them**: a
//! plain `Vec` index into such a structure silently reads a recycled slot after a
//! removal (ABA); a generational handle instead becomes a safe `None`. Future blueprint
//! work builds against *this* API — never `thunderdome::*` directly — so the backing
//! crate can later be swapped (e.g. for the hand-rolled `SlotArena` sketched in the
//! deep-dive §3) without touching a single call site.
//!
//! # Why a wrapper, not a re-export
//!
//! Callers get a stable, dowiz-flavored surface ([`SlotArena`] + an opaque [`Handle`])
//! that mirrors the vocabulary of `arena.rs`. [`Handle`] is an opaque newtype — its
//! `thunderdome::Index` payload is private, so no call site can manufacture, unpack, or
//! depend on the crate's internals. That is what makes the backing crate swappable.
//!
//! # Soundness / degrade discipline (same posture as `arena.rs`)
//!
//! - **Degrade-closed on every fallible op.** [`get`](SlotArena::get),
//!   [`get_mut`](SlotArena::get_mut), and [`remove`](SlotArena::remove) return `Option`
//!   and NEVER panic on a stale, out-of-range, or already-removed handle — a bad handle
//!   is a `None`, exactly like `BumpArena::alloc_slice` returns `None` on exhaustion.
//! - **ABA defeated by construction.** On removal the slot's generation is bumped, so
//!   every outstanding copy of the handle is invalidated. When the freed slot is later
//!   recycled by an `insert`, the new handle carries the higher generation; the old
//!   handle still resolves to `None`. A recycled slot can never be silently read through
//!   a stale handle (documented wrap horizon: `NonZeroU32` generation ≈ 2³² reuses of
//!   that one slot — the same class as slotmap's 2³¹).
//! - **Memory edge (the deep-dive's citable win).** [`Handle`] is 8 bytes and
//!   `Option<Handle>` is *also* 8 bytes (niche-packed into thunderdome's `NonZeroU32`
//!   generation) — asserted in the tests, not assumed.
//! - **No `unsafe` in this wrapper.** The only `unsafe` is upstream in thunderdome's
//!   audited packing; this module adds none.
//!
//! # Orientation note (contrast with `arena.rs`)
//!
//! [`BumpArena`](crate::arena::BumpArena) is a *phase/region* allocator: bump-allocate a
//! family of same-shaped buffers, use them, free them ALL with one `O(1)` `reset`; it has
//! no per-element free. [`SlotArena`] is its **sibling**, not its replacement: a
//! *per-element* arena with stable `Copy` handles that survive individual
//! removal-and-reuse without the stale-index / ABA bug. Reach for `BumpArena` for
//! scratch that dies at end-of-pass; reach for `SlotArena` when elements come and go
//! individually and references to them must outlive some of them safely.

use thunderdome::{Arena, Index};

/// A `Copy` handle into a [`SlotArena`]. 8 bytes (`u32` slot + `NonZeroU32` generation);
/// `Option<Handle>` is also 8 bytes (niche-packed). Opaque: the backing
/// `thunderdome::Index` is private, so call sites cannot forge or unpack a handle — the
/// backing crate stays swappable.
///
/// A handle is valid ONLY while its generation matches the slot's current generation. A
/// handle to a since-removed (or removed-then-recycled) element resolves to `None` on
/// [`get`](SlotArena::get) / [`get_mut`](SlotArena::get_mut) and `false` on
/// [`contains`](SlotArena::contains) — the ABA / stale-index bug is unrepresentable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Handle(Index);

/// A generational slot arena. `O(1)` insert / get / remove; removed slots are recycled
/// via thunderdome's internal free-list, and generation counters make a dangling handle a
/// safe `None`, never a silent read of a recycled value.
///
/// Degrade-closed: every fallible op returns `Option`, never panics on a bad handle. The
/// backing store is a dense single `Vec<Slot<T>>`, so live iteration is cache-friendly.
#[derive(Debug)]
pub struct SlotArena<T>(Arena<T>);

impl<T> Default for SlotArena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> SlotArena<T> {
    /// An empty arena. Allocation-free until the first [`insert`](Self::insert).
    pub fn new() -> Self {
        SlotArena(Arena::new())
    }

    /// An empty arena pre-sized for `cap` elements — one up-front reserve, so the first
    /// `cap` inserts do not reallocate. Sizing telemetry, same spirit as
    /// `BumpArena::with_capacity`.
    pub fn with_capacity(cap: usize) -> Self {
        SlotArena(Arena::with_capacity(cap))
    }

    /// Number of live elements.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// `true` iff no live elements.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Current backing capacity (elements) — honest sizing telemetry, may exceed
    /// [`len`](Self::len) after removals recycle slots.
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    /// Insert `value`, returning a stable `Copy` [`Handle`]. Reuses a recycled slot if one
    /// is free (O(1), no allocation), else pushes a new slot. The handle carries the
    /// slot's CURRENT generation, so it stays valid exactly until that element is removed.
    pub fn insert(&mut self, value: T) -> Handle {
        Handle(self.0.insert(value))
    }

    /// `Some(&T)` iff `handle` names a live element (slot in range, occupied, generation
    /// matches). A handle to a removed/recycled slot is `None` — stale-index is a safe miss.
    pub fn get(&self, handle: Handle) -> Option<&T> {
        self.0.get(handle.0)
    }

    /// Mutable sibling of [`get`](Self::get). `None` for a stale/out-of-range handle.
    pub fn get_mut(&mut self, handle: Handle) -> Option<&mut T> {
        self.0.get_mut(handle.0)
    }

    /// `true` iff `handle` names a live element. Cheap membership test (no borrow held).
    pub fn contains(&self, handle: Handle) -> bool {
        self.0.contains(handle.0)
    }

    /// Remove the element named by `handle`, returning it. Bumps the slot's generation
    /// (invalidating every outstanding copy of the handle) and recycles the slot. A
    /// double-remove or a stale remove is a safe `None`, never a panic.
    pub fn remove(&mut self, handle: Handle) -> Option<T> {
        self.0.remove(handle.0)
    }

    /// Drop all elements and reset to empty (retains capacity for reuse). Every
    /// previously-issued handle is thereby invalidated.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Iterate `(Handle, &T)` over the live elements, in dense storage order — the walk a
    /// mesh/graph index uses to visit every live node. Order is unspecified but stable
    /// between mutations.
    pub fn iter(&self) -> impl Iterator<Item = (Handle, &T)> {
        self.0.iter().map(|(idx, v)| (Handle(idx), v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn insert_get_get_mut_remove_roundtrip() {
        let mut a: SlotArena<u32> = SlotArena::new();
        assert!(a.is_empty());
        let h = a.insert(42);
        assert_eq!(a.len(), 1);
        assert!(!a.is_empty());
        assert_eq!(a.get(h), Some(&42));
        assert!(a.contains(h));
        // Mutate in place through the handle.
        *a.get_mut(h).expect("live handle") = 99;
        assert_eq!(a.get(h), Some(&99));
        // Remove returns the value and empties the arena.
        assert_eq!(a.remove(h), Some(99));
        assert_eq!(a.len(), 0);
        assert!(a.is_empty());
    }

    #[test]
    fn stale_handle_is_rejected_after_removal() {
        let mut a: SlotArena<&'static str> = SlotArena::new();
        let h = a.insert("courier");
        assert!(a.contains(h));
        a.remove(h);
        // The core degrade-closed property: a handle to a removed element is a safe None,
        // NOT a panic and NOT a silent read.
        assert_eq!(a.get(h), None);
        assert_eq!(a.get_mut(h), None);
        assert!(!a.contains(h));
    }

    #[test]
    fn double_remove_is_a_safe_none() {
        let mut a: SlotArena<i64> = SlotArena::new();
        let h = a.insert(-7);
        assert_eq!(a.remove(h), Some(-7));
        // Removing the same handle again must not panic and must not resurrect anything.
        assert_eq!(a.remove(h), None);
        assert_eq!(a.len(), 0);
    }

    #[test]
    fn aba_defeated_across_removal_and_slot_reuse() {
        // The property the whole crate exists to provide: after a slot is removed and
        // then RECYCLED by a later insert, the OLD handle must still be rejected even
        // though a live value now occupies that slot.
        let mut a: SlotArena<u32> = SlotArena::new();
        let stale = a.insert(1);
        assert_eq!(a.remove(stale), Some(1));
        // This insert recycles the freed slot (thunderdome's free-list), but with a
        // bumped generation, so the fresh handle differs from the stale one.
        let fresh = a.insert(2);
        assert_ne!(stale, fresh, "recycled slot must carry a new generation");
        // ABA defeated: the stale handle does NOT read the recycled value.
        assert_eq!(a.get(stale), None);
        assert!(!a.contains(stale));
        // The fresh handle is the only valid view of the recycled slot.
        assert_eq!(a.get(fresh), Some(&2));
    }

    #[test]
    fn many_handles_stay_independently_valid() {
        let mut a: SlotArena<u32> = SlotArena::new();
        let handles: Vec<Handle> = (0..8).map(|i| a.insert(i)).collect();
        assert_eq!(a.len(), 8);
        // Remove the even-valued elements; odd ones must remain readable via their handles.
        for (i, &h) in handles.iter().enumerate() {
            if i % 2 == 0 {
                assert_eq!(a.remove(h), Some(i as u32));
            }
        }
        assert_eq!(a.len(), 4);
        for (i, &h) in handles.iter().enumerate() {
            if i % 2 == 0 {
                assert_eq!(a.get(h), None, "removed handle must be a safe None");
            } else {
                assert_eq!(a.get(h), Some(&(i as u32)), "surviving handle stays valid");
            }
        }
    }

    #[test]
    fn iter_visits_only_live_elements() {
        let mut a: SlotArena<u32> = SlotArena::new();
        let h0 = a.insert(10);
        let _h1 = a.insert(20);
        let h2 = a.insert(30);
        a.remove(h0);
        a.remove(h2);
        // Only the surviving element (20) is visited; its handle round-trips.
        let live: Vec<(Handle, u32)> = a.iter().map(|(h, &v)| (h, v)).collect();
        assert_eq!(live.len(), 1);
        assert_eq!(live[0].1, 20);
        assert_eq!(a.get(live[0].0), Some(&20));
    }

    #[test]
    fn clear_invalidates_all_handles() {
        let mut a: SlotArena<u32> = SlotArena::new();
        let h = a.insert(5);
        a.clear();
        assert!(a.is_empty());
        assert_eq!(a.get(h), None);
        assert!(!a.contains(h));
    }

    #[test]
    fn handle_and_option_handle_are_both_eight_bytes() {
        // The deep-dive's citable memory win: an 8-byte key AND a niche-packed 8-byte
        // Option<key>. Asserted, not assumed (deep-dive §3.2 / §2).
        assert_eq!(size_of::<Handle>(), 8, "Handle must be 8 bytes");
        assert_eq!(
            size_of::<Option<Handle>>(),
            8,
            "Option<Handle> must niche-pack to 8 bytes"
        );
    }
}
