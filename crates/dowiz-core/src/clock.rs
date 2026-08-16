#![allow(unused)]
use core::time::Duration;

/// clock.rs — single-authority time abstraction (no_std migration enabler).
///
/// The no_std core: `Clock` trait + `elapsed_ns` helper (pure core).
/// Std-gated: `MonoClock`, `SystemClock`, `now_ns()`, `now_ms()`, `now_epoch_s()`.
/// no_std fallback: returns 0 (kernel/host must provide their own implementation).

/// Monotonic clock: nanoseconds since an arbitrary fixed epoch (monotone).
/// Userspace = `std::time::Instant`; kernel = `ktime_get_ns`/jiffies (swap here).
pub trait Clock {
    /// Current monotonic time in nanoseconds.
    fn now_ns(&self) -> u64;
}

/// Helper: monotonic elapsed ns, saturating (never panics on wrap).
pub fn elapsed_ns(from_ns: u64, to_ns: u64) -> u64 {
    to_ns.saturating_sub(from_ns)
}

#[cfg(feature = "std")]
mod std_impls {
    use super::*;

    /// Userspace monotonic clock (wraps `std::time::Instant`).
    #[derive(Debug, Clone, Copy)]
    pub struct MonoClock {
        epoch: std::time::Instant,
    }

    impl Default for MonoClock {
        fn default() -> Self {
            Self { epoch: std::time::Instant::now() }
        }
    }

    impl MonoClock {
        pub fn new() -> Self { Default::default() }
    }

    impl Clock for MonoClock {
        fn now_ns(&self) -> u64 {
            self.epoch.elapsed().as_nanos() as u64
        }
    }

    /// Wall-clock system time (std-gated).
    #[derive(Debug, Clone, Copy)]
    pub struct SystemClock;

    impl Clock for SystemClock {
        fn now_ns(&self) -> u64 {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0)
        }
    }

    impl SystemClock {
        pub fn now_epoch_s() -> u64 {
            Self.now_ns() / 1_000_000_000
        }
    }
}

#[cfg(feature = "std")]
pub use std_impls::{MonoClock, SystemClock};

/// Monotonic clock: nanoseconds (std: delegates to MonoClock; no_std: returns 0).
pub fn now_ns() -> u64 {
    #[cfg(feature = "std")]
    { std_impls::MonoClock::new().now_ns() }
    #[cfg(not(feature = "std"))]
    { 0 }
}

/// Monotonic clock: milliseconds.
pub fn now_ms() -> u64 {
    now_ns() / 1_000_000
}

/// Wall-clock epoch seconds (std-gated only).
#[cfg(feature = "std")]
pub fn now_epoch_s() -> u64 {
    std_impls::SystemClock::now_epoch_s()
}

/// Wall-clock epoch seconds as a signed count (for arithmetic like
/// `now - tolerance - slack`). no_std fallback returns 0; the kernel shadows this
/// with the real `SystemTime` clock (mirroring `now_ns` / `now_ms`).
pub fn now_epoch_secs() -> i64 {
    #[cfg(feature = "std")]
    {
        std_impls::SystemClock::now_epoch_s() as i64
    }
    #[cfg(not(feature = "std"))]
    {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elapsed_ns_is_saturating() {
        assert_eq!(elapsed_ns(100, 50), 0);
        assert_eq!(elapsed_ns(100, 200), 100);
    }

    #[cfg(feature = "std")]
    #[test]
    fn mono_clock_is_monotonic() {
        let c = std_impls::MonoClock::new();
        let a = c.now_ns();
        let b = c.now_ns();
        assert!(b >= a);
    }
}
