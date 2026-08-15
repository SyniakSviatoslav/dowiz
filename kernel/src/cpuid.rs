//! cpuid.rs — std shim over the pure no_std core.
//!
//! The pure parsing (`CpuCaps`, [`detect_from`], [`parse_cache_size_kb`]) lives in
//! `dowiz_core::cpuid` and is re-exported here. This shim adds the std `detect()`:
//! reads `/proc/cpuinfo` + `/proc/meminfo` + the L3 cache size once (via `OnceLock`) and
//! caches the snapshot — zero I/O after the first call.

pub use dowiz_core::cpuid::*;

use std::sync::OnceLock;

static CAPS: OnceLock<CpuCaps> = OnceLock::new();

/// Detect CPU capabilities once; subsequent calls return the cached result.
///
/// Safe to call from multiple threads — `OnceLock` guarantees exactly-one parse.
pub fn detect() -> &'static CpuCaps {
    CAPS.get_or_init(detect_fresh)
}

fn detect_fresh() -> CpuCaps {
    let cpuinfo = crate::vfs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let meminfo = crate::vfs::read_to_string("/proc/meminfo").unwrap_or_default();
    let l3 = crate::vfs::read_to_string("/sys/devices/system/cpu/cpu0/cache/index3/size")
        .unwrap_or_default();
    detect_from(&cpuinfo, &meminfo, &l3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_returns_non_panicking() {
        let caps = detect();
        // On a real Linux host we will see real values; on wasm/non-Linux all zeros.
        // Either way the call must not panic.
        assert!(caps.avx2 || !caps.avx2); // always a valid bool
        assert!(caps.cores > 0 || caps.cores == 0);
    }

    #[test]
    fn detect_is_idempotent() {
        let a = detect();
        let b = detect();
        assert_eq!(a as *const CpuCaps, b as *const CpuCaps, "OnceLock must return same &'static ref");
    }
}
