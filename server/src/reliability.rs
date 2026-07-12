//! Reliability ratchet (Tier-0 B substrate seed).
//!
//! A degrade-storm gate: when the process starts it enters a boot-grace window
//! during which transient failures are tolerated. After the grace window lapses
//! the storm flag latches if it was tripped; otherwise the process is considered
//! healthy. The flag lives in a process-global `LazyLock` so it is trivially
//! observable (and reset) across the whole server and from integration tests —
//! it is intentionally NOT part of `AppState` (which is cloned per request).
//!
//! RED condition (roadmap MASTER-BUILD-SEQUENCE): "flags reset on restart".
//! Because `Reliability` is a `LazyLock` initialized at first access, a fresh
//! process starts with a clean flag set — proven by `red_flags_reset_on_restart`.
//!
//! ponytail: boot-grace duration is a `const`; if it ever needs to be
//! configurable per-env, promote it to an `AtomicU64` ms override or read
//! `DOWIZ_BOOT_GRACE_MS`. No other deps.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::LazyLock;
use std::time::{Duration, Instant};

/// Boot-grace window. During this period the storm flag may trip without
/// latching (the system is still warming up).
pub const BOOT_GRACE: Duration = Duration::from_secs(30);

/// Process-global reliability handle. Initialized on first use; every process
/// begins in a clean state (the RED "reset on restart" guarantee).
pub static RELIABILITY: LazyLock<Reliability> = LazyLock::new(Reliability::new);

struct Inner {
    /// Set true once the boot-grace window has elapsed.
    boot_grace_elapsed: AtomicBool,
    /// Latched when a storm was detected after boot grace.
    storm: AtomicBool,
    /// Count of storm trips observed this process.
    storm_trips: AtomicU64,
    /// Monotonic process-start instant (for boot-grace math).
    started_at: Instant,
}

pub struct Reliability {
    inner: Inner,
}

impl Reliability {
    pub fn new() -> Self {
        Reliability {
            inner: Inner {
                boot_grace_elapsed: AtomicBool::new(false),
                storm: AtomicBool::new(false),
                storm_trips: AtomicU64::new(0),
                started_at: Instant::now(),
            },
        }
    }

    /// Advance the boot-grace clock. Call this from the server's main loop or a
    /// periodic tick; idempotent. Until this returns true, the process is in its
    /// boot-grace window.
    pub fn tick_boot_grace(&self) -> bool {
        let elapsed = self.inner.started_at.elapsed() >= BOOT_GRACE;
        self.inner.boot_grace_elapsed.store(elapsed, Ordering::SeqCst);
        elapsed
    }

    /// True while the process is still inside its boot-grace window.
    pub fn in_boot_grace(&self) -> bool {
        !self.inner.boot_grace_elapsed.load(Ordering::SeqCst)
    }

    /// Record a storm signal. Returns true if this trip LATCHED a storm (i.e. it
    /// happened after boot grace). During boot grace the trip is counted but the
    /// storm flag is not latched, so a noisy startup does not degrade the system.
    pub fn trip_storm(&self) -> bool {
        // Count the trip; only latch the storm flag after boot grace.
        self.inner.storm_trips.fetch_add(1, Ordering::SeqCst);
        if self.in_boot_grace() {
            return false;
        }
        self.inner.storm.store(true, Ordering::SeqCst);
        true
    }

    /// Current storm state.
    pub fn is_storm(&self) -> bool {
        self.inner.storm.load(Ordering::SeqCst)
    }

    /// Number of storm trips observed this process.
    pub fn storm_trips(&self) -> u64 {
        self.inner.storm_trips.load(Ordering::SeqCst)
    }

    /// Snapshot for `/api/healthz`.
    pub fn status(&self) -> ReliabilityStatus {
        ReliabilityStatus {
            boot_grace: self.in_boot_grace(),
            storm: self.is_storm(),
            storm_trips: self.storm_trips(),
        }
    }
}

/// JSON-serializable status view.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReliabilityStatus {
    pub boot_grace: bool,
    pub storm: bool,
    pub storm_trips: u64,
}
