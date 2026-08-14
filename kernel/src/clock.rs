//! clock.rs — single-authority time abstraction (no_std migration enabler).
//!
//! The no_std audit found ~34 "boundary" modules whose only `std` dependency is
//! `std::time::{Instant, SystemTime}`. A kernel module has no `std::time` — it
//! reads `jiffies`/`ktime_get_ns`. This module is the single seam: callers read
//! time only through [`now_ns`] (monotonic) and [`now_epoch_s`] (wall clock),
//! so the kernel port swaps the *impl*, not every call site.
//!
//! # Best-pattern notes
//! - Monotonic time is read *once per decision* and hoisted out of loops (the
//!   `token_bucket` pattern: `now` sampled before the critical section).
//! - All counters saturate (`saturating_sub`), never wrap/panic.

/// Monotonic clock: nanoseconds since an arbitrary fixed epoch (monotone).
/// Userspace = `std::time::Instant`; kernel = `ktime_get_ns`/jiffies (swap here).
pub trait Clock {
    /// Current monotonic time in nanoseconds.
    fn now_ns(&self) -> u64;
}

/// Wall clock: seconds since the Unix epoch. Kernel swap = `ktime_get_real_seconds`.
pub trait WallClock {
    fn now_epoch_s(&self) -> u64;
}

/// The userspace monotonic clock (`std::time::Instant`).
#[derive(Debug, Clone, Copy)]
pub struct MonoClock {
    /// Anchor `Instant` so `now_ns()` returns elapsed ns since construction —
    /// deterministic across a process and independent of the wall clock.
    epoch: std::time::Instant,
}

impl Default for MonoClock {
    fn default() -> Self {
        Self::new()
    }
}

impl MonoClock {
    pub fn new() -> Self {
        Self { epoch: std::time::Instant::now() }
    }
}

impl Clock for MonoClock {
    fn now_ns(&self) -> u64 {
        self.epoch.elapsed().as_nanos() as u64
    }
}

/// The userspace wall clock (`std::time::SystemTime`).
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl WallClock for SystemClock {
    fn now_epoch_s(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

/// Single authority for "now" (monotonic ns). Every hot path reads time here.
pub fn now_ns() -> u64 {
    MonoClock::new().now_ns()
}

/// Single authority for "now" in wall-clock epoch milliseconds (matches the
/// historical `crate::now_ms` semantics that resilience/wave/telemetry rely on).
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Single authority for wall-clock seconds. Every serialization timestamp
/// reads here (canonical, so the LLM-prompt prefix stays stable — see
/// `canonical`).
pub fn now_epoch_s() -> u64 {
    SystemClock.now_epoch_s()
}

/// Elapsed ns between two monotonic stamps (saturating, never negative).
#[inline(always)]
pub fn elapsed_ns(from_ns: u64, to_ns: u64) -> u64 {
    to_ns.saturating_sub(from_ns)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mono_clock_is_monotonic() {
        let c = MonoClock::new();
        let a = c.now_ns();
        // Do a little work.
        let mut acc = 0u64;
        for i in 0..100_000 {
            acc = acc.wrapping_add(i);
        }
        core::hint::black_box(acc);
        let b = c.now_ns();
        assert!(b >= a, "monotonic clock must not run backwards: {a} -> {b}");
    }

    #[test]
    fn elapsed_saturates() {
        assert_eq!(elapsed_ns(100, 50), 0);
        assert_eq!(elapsed_ns(50, 100), 50);
    }

    #[test]
    fn epoch_seconds_is_plausible() {
        let t = now_epoch_s();
        // After ~2020 and before 2100.
        assert!(t > 1_577_836_800, "epoch seconds too small: {t}");
        assert!(t < 4_102_444_800, "epoch seconds too large: {t}");
    }
}
