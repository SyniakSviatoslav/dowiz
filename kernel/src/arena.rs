//! `kernel::arena` — re-export of the no_std bump arena (now `dowiz_core::arena`)
//! plus the kernel-only counting-allocator bench harness.
//!
//! The bump-allocator core (`BumpArena`, `HugePageHint`) was extracted to
//! `dowiz-core` (no_std). This module keeps `crate::arena::BumpArena` resolving
//! unchanged and hosts the `count-allocs`-gated global counting allocator that
//! is kernel-specific (it installs a `#[global_allocator]` for the W5 DoD §8.2
//! allocation-count assertion and depends on `std`).

pub use dowiz_core::arena::{BumpArena, HugePageHint};

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
    use core::cell::Cell;
    use core::sync::atomic::{AtomicUsize, Ordering};

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
