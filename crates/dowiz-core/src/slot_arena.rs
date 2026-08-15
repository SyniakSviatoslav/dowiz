#![allow(unused)]
//! slot_arena.rs — Generational-index slot arena (no_std, zero-dep).
//!
//! A `Copy`-handle, per-element arena whose stale handles are a safe `None`
//! (the ABA / stale-index bug is unrepresentable). This is the hand-rolled
//! `SlotArena` sketched in `docs/research/OPUS-PERF-ARENA-DEEPDIVE-2026-07-18.md`
//! §3 — a pure `Vec<Slot<T>>` + `NonZeroU32` generation counter + free-list,
//! with **zero external dependencies** (no `thunderdome`).
//!
//! # Soundness / degrade discipline
//! - Degrade-closed on every fallible op: `get`/`get_mut`/`remove` return `Option`,
//!   never panic on a stale/out-of-range/removed handle.
//! - ABA defeated by construction: removal bumps the slot's generation, invalidating
//!   every outstanding handle; a recycled slot carries a higher generation, so the
//!   stale handle still resolves to `None` (wrap horizon ≈ 2³² reuses of a slot).
//! - `Handle` is 8 bytes; `Option<Handle>` is also 8 bytes (niche-packed into the
//!   `NonZeroU32` generation) — asserted, not assumed.

use core::num::NonZeroU32;
use alloc::vec::Vec;

/// A `Copy` handle into a [`SlotArena`]. 8 bytes (`u32` slot + `NonZeroU32`
/// generation); `Option<Handle>` is also 8 bytes (niche-packed). Opaque — fields
/// are private so call sites cannot forge or unpack a handle.
///
/// A handle is valid ONLY while its generation matches the slot's current
/// generation; a handle to a since-removed (or removed-then-recycled) element
/// resolves to `None` / `false` — the stale-index bug is unrepresentable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Handle {
    index: u32,
    generation: NonZeroU32,
}

/// A generational slot arena. O(1) insert / get / remove; removed slots are
/// recycled via a free-list, and generation counters make a dangling handle a
/// safe `None`, never a silent read of a recycled value.
///
/// Backing store is a dense `Vec<Slot<T>>`, so live iteration is cache-friendly.
#[derive(Debug)]
pub struct SlotArena<T> {
    slots: Vec<Slot<T>>,
    free: Vec<u32>,
    len: usize,
}

#[derive(Debug)]
struct Slot<T> {
    generation: NonZeroU32,
    value: Option<T>,
}

#[inline]
fn next_generation(g: NonZeroU32) -> NonZeroU32 {
    NonZeroU32::new(g.get().wrapping_add(1)).unwrap_or_else(|| NonZeroU32::new(1).unwrap())
}

impl<T> Default for SlotArena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> SlotArena<T> {
    /// An empty arena. Allocation-free until the first [`insert`](Self::insert).
    pub fn new() -> Self {
        SlotArena {
            slots: Vec::new(),
            free: Vec::new(),
            len: 0,
        }
    }

    /// An empty arena pre-sized for `cap` elements — one up-front reserve.
    pub fn with_capacity(cap: usize) -> Self {
        SlotArena {
            slots: Vec::with_capacity(cap),
            free: Vec::new(),
            len: 0,
        }
    }

    /// Number of live elements.
    pub fn len(&self) -> usize {
        self.len
    }

    /// `true` iff no live elements.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Current backing capacity (elements) — honest sizing telemetry, may exceed
    /// [`len`](Self::len) after removals recycle slots.
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    /// Insert `value`, returning a stable `Copy` [`Handle`]. Reuses a recycled slot
    /// if one is free (O(1), no allocation), else pushes a new slot.
    pub fn insert(&mut self, value: T) -> Handle {
        let index = if let Some(i) = self.free.pop() {
            let g = next_generation(self.slots[i as usize].generation);
            self.slots[i as usize].generation = g;
            self.slots[i as usize].value = Some(value);
            i
        } else {
            let index = self.slots.len() as u32;
            self.slots.push(Slot {
                generation: NonZeroU32::new(1).unwrap(),
                value: Some(value),
            });
            index
        };
        self.len += 1;
        Handle {
            index,
            generation: self.slots[index as usize].generation,
        }
    }

    /// `Some(&T)` iff `handle` names a live element.
    pub fn get(&self, handle: Handle) -> Option<&T> {
        let slot = self.slots.get(handle.index as usize)?;
        if slot.generation != handle.generation {
            return None;
        }
        slot.value.as_ref()
    }

    /// Mutable sibling of [`get`](Self::get).
    pub fn get_mut(&mut self, handle: Handle) -> Option<&mut T> {
        let slot = self.slots.get_mut(handle.index as usize)?;
        if slot.generation != handle.generation {
            return None;
        }
        slot.value.as_mut()
    }

    /// `true` iff `handle` names a live element.
    pub fn contains(&self, handle: Handle) -> bool {
        self.get(handle).is_some()
    }

    /// Remove the element named by `handle`, returning it. Bumps the slot's
    /// generation and recycles the slot. A double-remove / stale remove is a safe
    /// `None`, never a panic.
    pub fn remove(&mut self, handle: Handle) -> Option<T> {
        let slot = self.slots.get_mut(handle.index as usize)?;
        if slot.generation != handle.generation {
            return None;
        }
        let v = slot.value.take()?;
        slot.generation = next_generation(slot.generation);
        self.free.push(handle.index);
        self.len -= 1;
        Some(v)
    }

    /// Drop all elements and reset to empty (retains capacity for reuse).
    pub fn clear(&mut self) {
        self.slots.clear();
        self.free.clear();
        self.len = 0;
    }

    /// Iterate `(Handle, &T)` over the live elements, in dense storage order.
    pub fn iter(&self) -> impl Iterator<Item = (Handle, &T)> {
        self.slots.iter().enumerate().filter_map(|(i, s)| {
            s.value.as_ref().map(|v| {
                (
                    Handle {
                        index: i as u32,
                        generation: s.generation,
                    },
                    v,
                )
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;

    #[test]
    fn insert_get_get_mut_remove_roundtrip() {
        let mut a: SlotArena<u32> = SlotArena::new();
        assert!(a.is_empty());
        let h = a.insert(42);
        assert_eq!(a.len(), 1);
        assert!(!a.is_empty());
        assert_eq!(a.get(h), Some(&42));
        assert!(a.contains(h));
        *a.get_mut(h).expect("live handle") = 99;
        assert_eq!(a.get(h), Some(&99));
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
        assert_eq!(a.get(h), None);
        assert_eq!(a.get_mut(h), None);
        assert!(!a.contains(h));
    }

    #[test]
    fn double_remove_is_a_safe_none() {
        let mut a: SlotArena<i64> = SlotArena::new();
        let h = a.insert(-7);
        assert_eq!(a.remove(h), Some(-7));
        assert_eq!(a.remove(h), None);
        assert_eq!(a.len(), 0);
    }

    #[test]
    fn aba_defeated_across_removal_and_slot_reuse() {
        let mut a: SlotArena<u32> = SlotArena::new();
        let stale = a.insert(1);
        assert_eq!(a.remove(stale), Some(1));
        let fresh = a.insert(2);
        assert_ne!(stale, fresh, "recycled slot must carry a new generation");
        assert_eq!(a.get(stale), None);
        assert!(!a.contains(stale));
        assert_eq!(a.get(fresh), Some(&2));
    }

    #[test]
    fn many_handles_stay_independently_valid() {
        let mut a: SlotArena<u32> = SlotArena::new();
        let handles: Vec<Handle> = (0..8).map(|i| a.insert(i)).collect();
        assert_eq!(a.len(), 8);
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
        assert_eq!(size_of::<Handle>(), 8, "Handle must be 8 bytes");
        assert_eq!(
            size_of::<Option<Handle>>(),
            8,
            "Option<Handle> must niche-pack to 8 bytes"
        );
    }
}
