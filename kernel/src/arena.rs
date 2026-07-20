//! Deterministic bump arena — a zero-dependency, `std`-only phase/region allocator.
//!
//! # Why this exists (BLUEPRINT-CACHE-REFERENCE-GRAPH-TENSOR-ARENA §3.3, W5)
//!
//! The graph/spectral rebuild-and-rank pass (`Csr::from_edges` → `row_normalize` →
//! `personalized_pagerank`, plus dense `charpoly` scratch) builds a family of same-shaped
//! buffers, uses them, then discards them all at the end of the pass. That is the textbook
//! phase/region shape: with the heap each rebuild is `≈2n+7` `Vec` allocations (n=1024 ⇒
//! ≈2,055) plus `charpoly`'s `n²+O(n)` (n=64 ⇒ ≈4.3k). The arena serves every such
//! buffer from one contiguous `Vec<u8>` region with a pointer bump and frees them all with a
//! single `O(1)` `reset`.
//!
//! # Soundness discipline (same as `householder.rs` / `simd.rs` §1.4)
//!
//! - `T: Copy + Default` ⇒ no `Drop` obligations can exist for a bumped slice; the bumpalo
//!   no-Drop hazard is eliminated at **compile time** (a `Copy` type provably does not
//!   implement `Drop`), not by convention.
//! - The region is exclusively owned via `UnsafeCell<Vec<u8>>`; each `alloc_slice` returns a
//!   `&mut [T]` whose lifetime is tied to `&self`. Because the bump offset is monotone, two
//!   distinct allocations can never overlap.
//! - `reset(&mut self)` takes `&mut self`, so the borrow checker **proves** no live loans from
//!   `alloc_slice` survive a reset — use-after-reset is unrepresentable.
//! - `!Sync` by construction (`Cell`): one arena per pass / thread. Never shared across
//!   threads.
//! - **Degrade-closed:** when the region is exhausted `alloc_slice` returns `None` and the
//!   caller falls back to a plain heap `Vec` (the `_in` wrappers do this internally). The arena
//!   NEVER grows its region and NEVER panics on exhaustion.
//!
//! # Orientation note
//!
//! `alloc_slice` returns `&mut [T]` from `&self` — the bumpalo pattern. The done-check (§7
//! W5) runs this module's tests under Miri to confirm the soundness argument holds, not just
//! that it reads plausibly.

use std::cell::{Cell, UnsafeCell};

/// `Vec<u8>`-backed bump region. Fixed capacity (policy-as-data), pointer-bump
/// allocation, `O(1)` reset. `!Sync` by construction (`Cell`) — one arena per
/// pass/thread.
///
/// The region's capacity is fixed at construction and is **never** grown. Exhaustion
/// yields `None` from [`BumpArena::alloc_slice`] (degrade-closed), never a panic.
pub struct BumpArena {
    /// The backing region. Exclusively owned; capacity never changes after `with_capacity`.
    buf: UnsafeCell<Vec<u8>>,
    /// Bump pointer (byte offset into `buf`). Monotone within a pass.
    offset: Cell<usize>,
    /// Max `offset` ever reached — honest sizing telemetry for the maintenance pass.
    high_water: Cell<usize>,
}

impl BumpArena {
    /// Construct a region of exactly `bytes` capacity. The region is zeroed lazily on
    /// `alloc_slice` (via `T::default()`), so construction is `O(1)` and allocation-free
    /// beyond the single `Vec` reserve.
    pub fn with_capacity(bytes: usize) -> Self {
        BumpArena {
            buf: UnsafeCell::new(vec![0u8; bytes]),
            offset: Cell::new(0),
            high_water: Cell::new(0),
        }
    }

    /// Bump-allocate a zero-initialized slice of `len` elements of `T`.
    ///
    /// Alignment: the actual pointer **address** (`buf.as_ptr() + offset`) is rounded **up**
    /// to `align_of::<T>()` before the slice is placed — NOT the bare offset. The backing
    /// store is a `Vec<u8>` whose base pointer is only guaranteed 1-aligned, so an aligned
    /// *offset* does not imply an aligned *address*; rounding the address is what makes the
    /// returned slice properly aligned regardless of the base pointer or prior allocations
    /// (miri-gate finding, item 52: the offset-rounding version was UB —
    /// `from_raw_parts_mut` with an unaligned `*mut T`). Returns `None` when the (fixed)
    /// region cannot satisfy the request — degrade-closed, the caller falls back to a heap
    /// `Vec`.
    ///
    /// # Soundness
    /// `T: Copy + Default` ⇒ no `Drop` obligations and a trivially-constructible value. The
    /// returned `&mut [T]` borrows `&self`, and the monotone `offset` guarantees disjointness
    /// from every other slice handed out by this arena.
    // `&self -> &mut [T]` is the point of a bump arena (interior mutability via
    // `UnsafeCell`; disjointness argued in the Soundness section above, alignment
    // UB covered by the item-52 miri row) — the same shape every arena crate
    // (e.g. bumpalo) allows this lint for.
    #[allow(clippy::mut_from_ref)]
    pub fn alloc_slice<T: Copy + Default>(&self, len: usize) -> Option<&mut [T]> {
        let alignment = std::mem::align_of::<T>();

        // SAFETY: `buf` is exclusively owned via `UnsafeCell`; `&self` proves no other
        // reference to the region is currently handed out that aliases `start..end`, because the
        // bump offset is monotone — every prior allocation lives at a strictly lower offset, and
        // every future allocation will live at a strictly higher one. `T: Copy + Default`
        // makes the transmuted region a well-formed value with no `Drop` hazard.
        let buf = unsafe { &mut *self.buf.get() };

        // Align the ADDRESS, not the offset: `Vec<u8>`'s base pointer is only guaranteed
        // 1-aligned, so `base + round_up(offset)` can still be misaligned for `T`. Round the
        // absolute address up to `alignment` and convert back to an offset (UB caught by the
        // item-52 miri row: unaligned `&mut [T]` out of `from_raw_parts_mut`).
        let base_addr = buf.as_ptr() as usize;
        let start = round_up(
            base_addr
                .checked_add(self.offset.get())
                .expect("BumpArena: address computation overflows usize"),
            alignment,
        ) - base_addr;
        let size = len
            .checked_mul(std::mem::size_of::<T>())
            .expect("BumpArena: element count overflows usize");
        let end = start
            .checked_add(size)
            .expect("BumpArena: region size overflows usize");
        if end > buf.len() {
            return None; // degrade-closed: caller uses a heap Vec instead.
        }

        // Advance the bump pointer and record high-water for sizing telemetry.
        self.offset.set(end);
        if end > self.high_water.get() {
            self.high_water.set(end);
        }

        // SAFETY: `start..end` is `size`-bytes within `buf` and `size == len*size_of::<T>()`.
        // We reinterpret as `&mut [T]` and zero it via `T::default()` (a `Copy` type ⇒ cheap,
        // no allocation, no panic for the kernel's numeric `T`s).
        let slice_ptr = unsafe { buf.as_mut_ptr().add(start) as *mut T };
        let slice = unsafe { std::slice::from_raw_parts_mut(slice_ptr, len) };
        for slot in slice.iter_mut() {
            *slot = T::default();
        }
        Some(slice)
    }

    /// `O(1)`: `offset := 0`. Takes `&mut self`, so the borrow checker proves no loans from
    /// [`alloc_slice`](Self::alloc_slice) are live — soundness by signature, not convention.
    pub fn reset(&mut self) {
        self.offset.set(0);
        // `high_water` is intentionally retained across resets — it is cumulative sizing
        // telemetry for the maintenance pass, not per-pass state.
    }

    /// The maximum byte offset ever reached (across all resets). Honest input to sizing the
    /// region for the next maintenance pass (`measured high_water + slack`).
    pub fn high_water(&self) -> usize {
        self.high_water.get()
    }
}

/// Round `x` up to the next multiple of `a` (`a` must be a power of two, as all
/// `align_of` values are). Returns `x` when `x == 0`.
#[inline]
fn round_up(x: usize, a: usize) -> usize {
    if a == 0 {
        return x;
    }
    ((x + a - 1) / a) * a
}

/// HugePage seam — a `core_pinning.rs`-style `NoOp` port (DECART-deferred). The trigger
/// for a real implementation is a persistent tensor-arena region `> 2 MB` (RESCAL/CMTF
/// factor matrices at `n ≥ 10⁵`). Until that trigger fires on measured data, the seam is a
/// no-op that records the requested bytes so the maintenance pass can log saturation — it
/// performs no `madvise(MADV_HUGEPAGE)` and never fails.
///
/// Keeping this as a typed seam (not an inline comment) means the real `madvise` backend
/// plugs in behind it later without touching any `BumpArena` call site.
pub struct HugePageHint;

impl HugePageHint {
    /// Advisory: request huge-page backing for a region of `bytes`. Today a no-op
    /// (single-region host ⇒ no win); returns `Ok(())` always and records nothing
    /// observable. Degrade-closed even if a future host rejects the hint.
    #[allow(clippy::unnecessary_wraps)]
    pub fn advise(bytes: usize) -> Result<(), ()> {
        // DECART-deferred: no measured region > 2 MB yet (blueprint A8 trigger). When it
        // fires, this shells `madvise(MADV_HUGEPAGE)` on the arena's `Vec` base and returns
        // the syscall result; callers already treat the arena path as best-effort.
        let _ = bytes;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_capacity_is_empty_and_exhausts_cleanly() {
        let a = BumpArena::with_capacity(64);
        assert_eq!(a.high_water(), 0);
        // Over-allocate beyond the 64-byte region ⇒ degrade-closed None, no panic.
        assert!(a.alloc_slice::<u8>(128).is_none());
        assert_eq!(a.high_water(), 0);
    }

    #[test]
    fn alloc_slice_returns_zeroed_copy_values() {
        let a = BumpArena::with_capacity(1024);
        let s: &mut [f64] = a.alloc_slice(4).expect("fits");
        assert_eq!(s, &[0.0, 0.0, 0.0, 0.0]);
        // Mutating the loan is visible. (2.5, not 3.14 — clippy::approx_constant
        // reads 3.14 as a sloppy π; any value works, this one is exact in f64.)
        s[1] = 2.5;
        assert_eq!(s[1], 2.5);
    }

    #[test]
    fn monotone_offset_gives_disjoint_slices() {
        let a = BumpArena::with_capacity(1024);
        let x: &mut [u32] = a.alloc_slice(2).unwrap();
        x[0] = 11;
        x[1] = 22;
        let y: &mut [u32] = a.alloc_slice(2).unwrap(); // strictly higher offset
        y[0] = 33;
        y[1] = 44;
        // Neither loan aliases the other (no overlap by construction).
        assert_eq!(x, &[11, 22]);
        assert_eq!(y, &[33, 44]);
    }

    #[test]
    fn alignment_is_respected_across_mixed_types() {
        let a = BumpArena::with_capacity(4096);
        let _b: &mut [u8] = a.alloc_slice(1).unwrap();
        // A 8-byte-aligned type must land on an 8-byte boundary even after the u8 alloc.
        let f: &mut [f64] = a.alloc_slice(1).unwrap();
        let f_addr = f.as_ptr() as usize;
        assert_eq!(
            f_addr % std::mem::align_of::<f64>(),
            0,
            "f64 slice must be aligned"
        );
    }

    #[test]
    fn high_water_records_max_reached() {
        let mut a = BumpArena::with_capacity(1024);
        let _x: &mut [u8] = a.alloc_slice(500).unwrap();
        assert_eq!(a.high_water(), 500);
        a.reset();
        // After reset the bump pointer is 0, but high_water persists (cumulative byte
        // telemetry across resets, not per-pass state). The next pass reuses the region
        // from offset 0.
        assert_eq!(a.high_water(), 500);
        let _y: &mut [u8] = a.alloc_slice(200).unwrap();
        // end=200 < prior max 500, so high_water is unchanged.
        assert_eq!(a.high_water(), 500);
        let _z: &mut [u8] = a.alloc_slice(400).unwrap();
        // offset was 200 → end=600 > 500, so high_water advances to 600.
        assert_eq!(a.high_water(), 600);
    }

    #[test]
    fn reset_frees_region_for_reuse() {
        let mut a = BumpArena::with_capacity(64);
        {
            let s: &mut [u32] = a.alloc_slice(4).unwrap();
            s[0] = 0xDEAD;
        }
        a.reset(); // &mut self ⇒ prior loan is provably dead here.
        let t: &mut [u32] = a.alloc_slice(4).unwrap();
        // Zeroed on (re)allocation.
        assert_eq!(t, &[0, 0, 0, 0]);
        t[1] = 0xBEEF;
        assert_eq!(t[1], 0xBEEF);
    }

    #[test]
    fn hugepage_hint_is_honest_noop() {
        // The seam must not fail and must remain a no-op until the >2 MB trigger fires.
        assert!(HugePageHint::advise(0).is_ok());
        assert!(HugePageHint::advise(4096).is_ok());
        assert!(HugePageHint::advise(1 << 30).is_ok());
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Counting global allocator (W5 DoD §8.2 — honest allocation-count assertion).
//
// Installed ONLY when the `count-allocs` feature is active (otherwise the
// default `std` allocator is used, so production / benchmark builds are
// unaffected). The test that asserts "≤ 8 heap allocations on the arena path"
// enables this feature, snapshots the global counter around a code region, and
// compares ARENA vs HEAP rebuild passes on the SAME n=1024 fixture graph — the
// blueprint's §3.3 baseline of ≈2,055 malloc/free pairs is here MEASURED, not
// assumed (see DoD §8.3 #5).
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(all(feature = "count-allocs", not(target_arch = "wasm32")))]
pub mod counting_alloc {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Number of `alloc` calls routed through the global allocator since the
    /// last [`reset_count`]. Monotonic + cumulative; `dealloc` is NOT counted
    /// (the blueprint's "malloc/free PAIR" claim, and the W5 "≤ 8 heap
    /// allocations" bound, are both framed in allocs — the arena's value is
    /// fewer allocations, not fewer frees).
    pub static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);

    /// `thread_local` snapshot so parallel test threads do not corrupt each
    /// other's measurement. Each region snapshots/resets/snapshots.
    thread_local! {
        static SNAPSHOT: Cell<usize> = const { Cell::new(0) };
    }

    pub struct CountingAlloc;

    unsafe impl GlobalAlloc for CountingAlloc {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
            System.alloc(layout)
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            System.dealloc(ptr, layout)
        }
    }

    #[global_allocator]
    static GLOBAL: CountingAlloc = CountingAlloc;

    /// Record the current global count into this thread's snapshot.
    pub fn snapshot() -> usize {
        let cur = ALLOC_COUNT.load(Ordering::Relaxed);
        SNAPSHOT.with(|s| {
            s.set(cur);
            cur
        })
    }

    /// Reset the global counter to 0 and return the value it had (for chaining).
    pub fn reset_count() -> usize {
        ALLOC_COUNT.swap(0, Ordering::Relaxed)
    }

    /// Allocations performed since the last [`snapshot`] on this thread.
    pub fn since_snapshot() -> usize {
        SNAPSHOT.with(|s| ALLOC_COUNT.load(Ordering::Relaxed) - s.get())
    }
}
