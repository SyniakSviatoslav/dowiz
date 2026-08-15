#![allow(unused)]
use core::time::Duration;

/// thread.rs — thread seam (ledger item 4: thread → kthread).
///
/// The no_std audit found a small set of `std::thread` call sites in otherwise
/// no_std-ready modules: `sleep` (chronos), `available_parallelism`
/// (core_pinning, span_metrics). A kernel module has no `std::thread` — it
/// sleeps via `schedule_timeout`/`msleep` and counts CPUs via `num_online_cpus`.
/// This module is the single seam, in the same shape as [`crate::clock`]: a
/// no_std-compatible [`Thread`] trait (`core::time::Duration`, `usize` — no
/// `JoinHandle`), a userspace [`StdThread`] impl (std-gated), and free functions
/// ([`sleep`], [`available_parallelism`]) that are the single authority. The
/// kernel port swaps the impl, not the call sites.
///
/// # no_std fallbacks
/// Without the `std` feature, [`sleep`] is a no-op and
/// [`available_parallelism`] degrades-closed to `1` (the kernel/host must
/// supply its own impl: `schedule_timeout`/`msleep` and `num_online_cpus`).
///
/// # Out of scope (documented follow-up)
/// `std::thread::spawn` / `scope` (budget.rs) return `JoinHandle`/borrow stack
/// data — a `kthread_create` port needs a different handle type, so those stay
/// std until a `JoinHandle`-free spawn seam is designed.
pub trait Thread {
    /// Sleep the current thread for `d`.
    fn sleep(&self, d: Duration);
    /// Number of usable CPU cores (degrades-closed to `1`).
    fn available_parallelism(&self) -> usize;
}

#[cfg(feature = "std")]
mod std_impls {
    use super::*;

    /// The userspace thread impl (`std::thread`).
    pub struct StdThread;

    impl Thread for StdThread {
        fn sleep(&self, d: Duration) {
            std::thread::sleep(d);
        }
        fn available_parallelism(&self) -> usize {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
        }
    }
}

#[cfg(feature = "std")]
pub use std_impls::StdThread;

/// Single authority for "sleep". Kernel port swaps to `schedule_timeout`/`msleep`.
pub fn sleep(d: Duration) {
    #[cfg(feature = "std")]
    {
        std_impls::StdThread.sleep(d);
    }
    #[cfg(not(feature = "std"))]
    {
        let _ = d; // no_std: no-op sleep (kernel/host supplies its own impl).
    }
}

/// Single authority for "how many CPUs". Kernel port swaps to `num_online_cpus`.
pub fn available_parallelism() -> usize {
    #[cfg(feature = "std")]
    {
        std_impls::StdThread.available_parallelism()
    }
    #[cfg(not(feature = "std"))]
    {
        1 // no_std: degrade-closed to a single CPU.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parallelism_is_positive() {
        assert!(available_parallelism() >= 1, "at least one CPU");
    }

    #[test]
    fn sleep_zero_is_instant() {
        // Must not hang or panic; zero-duration sleep is a no-op.
        sleep(Duration::from_millis(0));
    }
}
