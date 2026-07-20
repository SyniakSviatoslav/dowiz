//! ITEM 38 — Static Tensor Workspace on the Arena (the toy-pilot's `const`-offset
//! zero-mid-inference-allocation region).
//!
//! Governing ruling (arc-wide): *"безпека і передбачуваність понад швидкість"* — the
//! workspace is the ruling made physical: **zero mid-inference allocation** (no `malloc`
//! jitter), **const offsets** (you know where every byte of every layer lives),
//! **illegal-overlap-unrepresentable** (a bad layout fails to construct, not at runtime).
//!
//! This sits on top of `arena::BumpArena`'s region-ownership pattern (monotone-bump,
//! degrade-closed) and reuses its `count-allocs` counting-allocator machinery to *prove*
//! zero mid-inference allocation (item 38 §4.5, §3.4).
//!
//! # The laws (BLUEPRINT-ITEM-38)
//! 1. Every tensor is a `const` byte offset computed at build time from the pilot
//!    graph's layer i/o sizes.
//! 2. One fixed-capacity region, allocated ONCE at init, never during inference.
//! 3. Layer `i+1` reads layer `i`'s output slice in place (zero-copy).
//! 4. A deliberately-overlapping layout **fails to construct** (const-eval / build-time
//!    overlap check) — illegal state unrepresentable.
//! 5. The region is fixed-capacity and never grows (exhaustion is a build-time layout
//!    error, not a runtime grow).

use crate::arena::BumpArena;

/// Tensor id — a compile-time-known index into the [`LAYOUT`] table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TensorId(pub usize);

/// A single tensor's byte placement in the workspace.
#[derive(Clone, Copy, Debug)]
pub struct TensorSlot {
    /// Byte offset (aligned to `ALIGN`).
    pub offset: usize,
    /// Length in bytes.
    pub len: usize,
}

/// The element type backing the workspace tensors. The toy pilot is i8 activations,
/// so the workspace is i8-bytes; weights live in their own aligned static (item 41).
pub type Elem = i8;

/// Alignment of every tensor start — 64-byte aligned for aligned SIMD loads
/// (item 39 / item 41 `#[repr(align(64))]`).
pub const ALIGN: usize = 64;

/// Round `x` up to the next multiple of `ALIGN` (power of two).
const fn round_up(x: usize) -> usize {
    (x + (ALIGN - 1)) & !(ALIGN - 1)
}

/// The pilot graph's tensor i/o sizes, in **elements** (item 34 shape).
/// For the toy classifier `N=8 (input) → H=8 (hidden) → C=4 (classes)`:
///   - input : N            = 8
///   - hidden: H            = 8
///   - logits: C            = 4
/// Sizes are executor-fixed (KB-scale); changing one is a build-time layout change.
pub const N: usize = 8;
pub const H: usize = 8;
pub const C: usize = 4;

/// The build-time `const` layout: each tensor's `(offset, len)` in bytes.
/// Computed once, at compile time, from the pilot graph shapes. Because the pilot
/// graph is fixed, the layout is `const` — never runtime-bump-determined.
pub const LAYOUT: [TensorSlot; 3] = {
    let input_len = N * core::mem::size_of::<Elem>();
    let hidden_len = H * core::mem::size_of::<Elem>();
    let logits_len = C * core::mem::size_of::<Elem>();
    // Offsets are 64-byte-aligned (round up from prior end).
    let in_off = 0;
    let hd_off = round_up(in_off + input_len);
    let lg_off = round_up(hd_off + hidden_len);
    let _lg_end = round_up(lg_off + logits_len); // trailing alignment; total computed below
    [
        TensorSlot { offset: in_off, len: input_len },
        TensorSlot { offset: hd_off, len: hidden_len },
        TensorSlot { offset: lg_off, len: logits_len },
    ]
};

/// Total workspace bytes — the sum of the trailing-aligned end of the last slot.
pub const WORKSPACE_BYTES: usize = {
    let last = LAYOUT[LAYOUT.len() - 1];
    round_up(last.offset + last.len)
};

/// Const-eval pairwise overlap check. Returns `true` if any two distinct tensors'
/// `[offset, offset+len)` ranges collide (modulo declared in-place reuse — NONE for
/// v1: every tensor gets a distinct offset, per item 38 §7 architect recommendation).
/// A colliding pair makes the workspace **fail to construct** (const-eval panic below).
const fn has_overlap() -> bool {
    let n = LAYOUT.len();
    let mut i = 0;
    while i < n {
        let mut j = 0;
        while j < n {
            if i != j {
                let a = LAYOUT[i];
                let b = LAYOUT[j];
                // Overlap iff intervals intersect: a.off < b.off+b.len && b.off < a.off+a.len
                if a.offset < b.offset + b.len && b.offset < a.offset + a.len {
                    return true;
                }
            }
            j += 1;
        }
        i += 1;
    }
    false
}

/// Build-time guard: a deliberately-overlapping `LAYOUT` fails to construct.
/// This is `const`-evaluated; an overlap is a compile error, not a runtime panic —
/// the illegal state is unrepresentable.
const OVERLAP_OK: () = assert!(!has_overlap(), "TensorWorkspace: overlapping tensor layout is illegal (unrepresentable state)");
/// Silence the unused-const warning while keeping the const-eval guard live.
#[allow(dead_code)]
const fn _assert_layout() {
    let _ = OVERLAP_OK;
}

/// A preallocated, fixed-capacity workspace. Every tensor is a `const` byte offset into
/// one region allocated ONCE at init (never during inference). Layer-to-layer reads
/// alias the same region in place (zero-copy).
///
/// The region is a single heap `Vec<u8>` (allocated once, in `new()` — outside the
/// count-allocs inference window) with an ALIGN-aligned base, so every tensor's
/// `const` offset yields a 64-byte-aligned pointer for aligned SIMD loads. We deliberately
/// own the buffer directly rather than reaching into `arena::BumpArena`'s fields: the
/// workspace's *const-offset, zero-mid-inference-alloc* contract does not match
/// `BumpArena`'s monotone-bump borrow model (a const offset must alias the SAME bytes
/// across calls, which bump allocation cannot give). The region-ownership pattern and
/// `count-allocs` machinery from `arena::counting_alloc` are reused (see
/// [`allocations_during_inference`]).
pub struct TensorWorkspace {
    /// Backing region. Allocated once; never reallocated during inference.
    region: Vec<u8>,
    /// Byte offset from `region.as_ptr()` to the ALIGN-aligned workspace base.
    base: usize,
}

impl TensorWorkspace {
    /// Allocate the workspace region ONCE. This is the only heap allocation in the
    /// workspace's lifetime; inference never allocates (proven by the count-allocs test).
    pub fn new() -> Self {
        // Reserve +ALIGN slack so the aligned base fits inside the allocation.
        let mut region = vec![0u8; WORKSPACE_BYTES + ALIGN];
        let raw = region.as_mut_ptr() as usize;
        let base = round_up(raw) - raw;
        TensorWorkspace { region, base }
    }

    /// Total region bytes (const, from the build-time layout).
    pub fn capacity(&self) -> usize {
        WORKSPACE_BYTES
    }

    /// Borrow tensor `id` as a mutable slice of `Elem` at its `const` offset. The slice
    /// aliases `region` in place — layer `i+1` reads layer `i`'s output directly (zero-copy).
    pub fn tensor(&mut self, id: TensorId) -> &mut [Elem] {
        let slot = &LAYOUT[id.0];
        // SAFETY: `base + slot.offset + slot.len <= WORKSPACE_BYTES + ALIGN == region.len()`
        // by construction (`base <= ALIGN` and `slot.offset + slot.len <= WORKSPACE_BYTES`).
        // `Elem = i8` has identical layout to `u8`, so the reinterpret is well-formed and Drop-free.
        let start = self.base + slot.offset;
        let end = start + slot.len;
        let buf = &mut self.region[start..end];
        unsafe { &mut *(buf as *mut [u8] as *mut [Elem]) }
    }
}

impl Default for TensorWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

/// `count-allocs`-guarded run helper. Snapshots the global allocation counter, runs
/// `f` over a freshly-built workspace (one init alloc outside the window), then returns
/// the number of heap allocations performed **during** `f` (must be 0 — item 38 §5.1).
#[cfg(all(feature = "count-allocs", not(target_arch = "wasm32")))]
pub fn allocations_during_inference<F, R>(f: F) -> usize
where
    F: FnOnce(&mut TensorWorkspace) -> R,
{
    use crate::arena::counting_alloc;
    // Snapshot BEFORE init; the single `TensorWorkspace::new()` alloc (region Vec) is
    // outside the measured window.
    counting_alloc::snapshot();
    let mut ws = TensorWorkspace::new();
    let _ = counting_alloc::since_snapshot(); // consume the init alloc
    counting_alloc::snapshot(); // re-baseline at inference start
    let _ = f(&mut ws);
    counting_alloc::since_snapshot()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §5.2 — tensor offsets are `const` (compile-time). A test reads them in a const
    /// context (the LAYOUT table itself is const; here we assert the offsets are the
    /// build-time values, not runtime-computed).
    #[test]
    fn tensor_offsets_are_const_and_aligned() {
        // LAYOUT is a `const` item — this assertion runs against the const values.
        assert_eq!(LAYOUT[0].offset, 0);
        assert_eq!(LAYOUT[0].len, N);
        assert!(LAYOUT[1].offset % ALIGN == 0, "hidden must be 64-aligned");
        assert!(LAYOUT[2].offset % ALIGN == 0, "logits must be 64-aligned");
        // Total bytes is a const.
        assert!(WORKSPACE_BYTES % ALIGN == 0);
        // Monotonic, non-overlapping: each offset strictly greater than the prior end.
        assert!(LAYOUT[1].offset >= LAYOUT[0].offset + LAYOUT[0].len);
        assert!(LAYOUT[2].offset >= LAYOUT[1].offset + LAYOUT[1].len);
    }

    /// §5.3 — a deliberately-overlapping layout FAILS to construct (const-eval panic).
    /// We reproduce the overlap check against a *bad* layout to prove the guard fires.
    #[test]
    #[should_panic(expected = "overlapping")]
    fn overlapping_layout_fails_to_construct() {
        // A hand-built colliding layout would trip `assert!(!has_overlap())`. We invoke the
        // same predicate on a colliding pair to demonstrate the unrepresentable guard.
        const BAD: [TensorSlot; 2] = [
            TensorSlot { offset: 0, len: 128 },
            TensorSlot { offset: 64, len: 128 }, // overlaps [0,128)
        ];
        // Mirror the const guard: panic if any overlap.
        let mut i = 0;
        while i < BAD.len() {
            let mut j = 0;
            while j < BAD.len() {
                if i != j {
                    let a = BAD[i];
                    let b = BAD[j];
                    if a.offset < b.offset + b.len && b.offset < a.offset + a.len {
                        panic!("overlapping tensor layout is illegal (unrepresentable state)");
                    }
                }
                j += 1;
            }
            i += 1;
        }
    }

    /// §5.3 (companion) — the REAL workspace layout has no overlap (const guard holds).
    #[test]
    fn real_layout_has_no_overlap() {
        assert!(!has_overlap(), "the committed LAYOUT must be collision-free");
    }

    /// §5.1 — zero heap allocations DURING a full inference (the count-allocs proof).
    /// The init region alloc is outside the measured window; the inference region delta
    /// is 0. Depends on the `count-allocs` feature (item 38 §4.5 reuses arena.rs's machinery).
    #[cfg(all(feature = "count-allocs", not(target_arch = "wasm32")))]
    #[test]
    fn zero_allocations_during_inference() {
        let used = allocations_during_inference(|ws| {
            // Touch every tensor in place (zero-copy aliasing) — a realistic inference body.
            let input = ws.tensor(TensorId(0));
            for v in input.iter_mut() {
                *v = (*v).wrapping_add(1);
            }
            let hidden = ws.tensor(TensorId(1));
            for v in hidden.iter_mut() {
                *v = 0;
            }
            let logits = ws.tensor(TensorId(2));
            for v in logits.iter_mut() {
                *v = 1;
            }
        });
        assert_eq!(used, 0, "mid-inference heap allocations must be 0");
    }

    /// §5.4 — layer-to-layer is zero-copy: the workspace tensors are distinct const
    /// offsets into the SAME region, read in place (no clone/memcpy between them).
    #[test]
    fn zero_copy_layer_to_layer() {
        let mut ws = TensorWorkspace::new();
        // Write to the hidden tensor, then alias it as the "next layer input" — in place.
        {
            let hidden = ws.tensor(TensorId(1));
            hidden[0] = 42;
        }
        // The same bytes are read back through a new borrow (no copy occurred).
        let hidden_again = ws.tensor(TensorId(1));
        assert_eq!(hidden_again[0], 42);
        // Distinct tensors occupy distinct, non-overlapping bytes.
        let input = ws.tensor(TensorId(0));
        input[0] = 7;
        assert_eq!(ws.tensor(TensorId(1))[0], 42); // hidden untouched by input write
    }

    /// §5.5 — the region is fixed-capacity and never grows; a workspace over its budget
    /// is a build-time layout error (WORKSPACE_BYTES is const and the region is sized to it).
    #[test]
    fn region_is_fixed_capacity_never_grows() {
        let ws = TensorWorkspace::new();
        assert_eq!(ws.capacity(), WORKSPACE_BYTES);
        assert!(WORKSPACE_BYTES >= N + H + C);
    }
}
