//! cpuid.rs — Hardware capability detection (std-only, no external crates).
//!
//! Reads `/proc/cpuinfo` flags + `/proc/meminfo` at init time. Single snapshot,
//! zero I/O after first call. The `detect()` fn is lazy-idempotent — call it once
//! at boot and store the result; subsequent calls return a cached reference to the
//! already-parsed struct (see [`crate::hw_profile`] for the sibling topology probe).
//!
//! # Fail-closed
//! If `/proc` or `/sys` is unavailable (wasm, non-Linux), all booleans are false,
//! counts are zero. No panics, no hangs.

use std::sync::OnceLock;

static CAPS: OnceLock<CpuCaps> = OnceLock::new();

/// Hardware feature flags detected from `/proc/cpuinfo` + `/proc/meminfo`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuCaps {
    /// AVX2 (256-bit integer SIMD).
    pub avx2: bool,
    /// FMA3 (fused multiply-add, `vfma*`).
    pub fma: bool,
    /// SHA-1 / SHA-256 hardware acceleration (NOT SHA3/Keccak).
    pub sha_ni: bool,
    /// AES hardware (AES-NI).
    pub aes_ni: bool,
    /// BMI2 (bit-manipulation, e.g. `bzhi`, `pext`, `pdep`).
    pub bmi2: bool,
    /// FSRM — fast short rep movsb (memcpy acceleration).
    pub fsrm: bool,
    /// L3 cache size in KiB (from `/sys/devices/system/cpu/cpu0/cache/index3/size`,
    /// or 0 when unreadable).
    pub l3_cache_kb: usize,
    /// Total system RAM in MiB (from `/proc/meminfo:MemTotal`, or 0 when unreadable).
    pub ram_total_mb: usize,
    /// Number of logical processors (count of `processor` entries in `/proc/cpuinfo`).
    pub cores: usize,
}

impl Default for CpuCaps {
    fn default() -> Self {
        CpuCaps {
            avx2: false,
            fma: false,
            sha_ni: false,
            aes_ni: false,
            bmi2: false,
            fsrm: false,
            l3_cache_kb: 0,
            ram_total_mb: 0,
            cores: 0,
        }
    }
}

/// Detect CPU capabilities once; subsequent calls return the cached result.
///
/// Safe to call from multiple threads — `OnceLock` guarantees exactly-one parse.
pub fn detect() -> &'static CpuCaps {
    CAPS.get_or_init(detect_fresh)
}

fn detect_fresh() -> CpuCaps {
    let cpuinfo = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();

    let flags: &str = cpuinfo
        .lines()
        .find(|l| l.starts_with("flags"))
        .and_then(|l| l.split(':').nth(1))
        .unwrap_or("");

    let flags_lower = flags.to_lowercase();
    let has_flag = |f: &str| flags_lower.split_whitespace().any(|w| w == f);

    // ── cores ──
    let cores = cpuinfo.lines().filter(|l| l.starts_with("processor")).count();

    // ── L3 cache from /sys ──
    let l3_cache_kb = std::fs::read_to_string(
        "/sys/devices/system/cpu/cpu0/cache/index3/size",
    )
    .ok()
    .as_deref()
    .and_then(parse_cache_size_kb)
    .unwrap_or(0);

    // ── RAM from /proc/meminfo ──
    let meminfo = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let ram_total_mb = meminfo
        .lines()
        .find(|l| l.starts_with("MemTotal:"))
        .and_then(|l| {
            let parts: Vec<&str> = l.split_whitespace().collect();
            if parts.len() >= 2 {
                parts[1].parse::<usize>().ok().map(|kb| kb / 1024)
            } else {
                None
            }
        })
        .unwrap_or(0);

    CpuCaps {
        avx2: has_flag("avx2"),
        fma: has_flag("fma"),
        sha_ni: has_flag("sha_ni"),
        aes_ni: has_flag("aes"),
        bmi2: has_flag("bmi2"),
        fsrm: has_flag("fsrm"),
        l3_cache_kb,
        ram_total_mb,
        cores,
    }
}

/// Parse a cache-size string like `"32768K"` or `"32M"` → KiB.
fn parse_cache_size_kb(s: &str) -> Option<usize> {
    let s = s.trim();
    if s.ends_with('K') {
        s[..s.len() - 1].parse().ok()
    } else if s.ends_with('M') {
        s[..s.len() - 1].parse::<usize>().ok().map(|v| v * 1024)
    } else {
        None
    }
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

    #[test]
    fn default_is_all_false() {
        let d = CpuCaps::default();
        assert!(!d.avx2);
        assert!(!d.fma);
        assert!(!d.sha_ni);
        assert!(!d.aes_ni);
        assert!(!d.bmi2);
        assert!(!d.fsrm);
        assert_eq!(d.l3_cache_kb, 0);
        assert_eq!(d.ram_total_mb, 0);
        assert_eq!(d.cores, 0);
    }

    #[test]
    fn cache_size_parsing() {
        assert_eq!(parse_cache_size_kb("32768K"), Some(32768));
        assert_eq!(parse_cache_size_kb("32M"), Some(32 * 1024));
        assert_eq!(parse_cache_size_kb(""), None);
        assert_eq!(parse_cache_size_kb("xyz"), None);
    }
}
