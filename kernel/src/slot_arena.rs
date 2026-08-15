//! slot_arena.rs — held-handle shim (the pure no_std generational arena now
//! lives in `dowiz_core::slot_arena`; no external `thunderdome` dependency).
//!
//! The backing crate was swapped for the hand-rolled `SlotArena` (deep-dive §3):
//! `Vec<Slot<T>>` + `NonZeroU32` generation + free-list, zero runtime deps. The
//! public surface ([`Handle`] / [`SlotArena`]) is unchanged, so call sites built
//! against this API never touched `thunderdome::*` — the swap was seamless.

pub use dowiz_core::slot_arena::*;
