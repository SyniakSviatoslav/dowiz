//! cpuid.rs — Hardware capability detection (pure no_std core).
//!
//! The pure half: the [`CpuCaps`] feature-flag struct, the cache-size parser, and the
//! pure [`detect_from`] parser (given `/proc/cpuinfo` + `/proc/meminfo` + the L3 cache
//! size string, it derives the capability snapshot with no I/O). The std half — the
//! lazy-idempotent `detect()` that reads `/proc`/`/sys` once via the vfs seam and caches
//! the result — lives in the kernel shim (`dowiz-kernel`'s `cpuid::detect`).
//!
//! # Fail-closed
//! If `/proc` or `/sys` is unavailable (wasm, non-Linux), all booleans are false,
//! counts are zero. No panics, no hangs.

use alloc::vec::Vec;

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
    /// L3 cache size in KiB (or 0 when unreadable).
    pub l3_cache_kb: usize,
    /// Total system RAM in MiB (or 0 when unreadable).
    pub ram_total_mb: usize,
    /// Number of logical processors.
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

/// Derive a capability snapshot from the raw `/proc`/`/sys` strings (pure, no I/O). The
/// kernel shim's `detect()` reads the files and forwards them here.
pub fn detect_from(cpuinfo: &str, meminfo: &str, l3_cache_size: &str) -> CpuCaps {
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
    let l3_cache_kb = parse_cache_size_kb(l3_cache_size).unwrap_or(0);

    // ── RAM from /proc/meminfo ──
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
pub fn parse_cache_size_kb(s: &str) -> Option<usize> {
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

    #[test]
    fn detect_from_parses_flags_and_counts() {
        let cpuinfo = "processor : 0\nprocessor : 1\nflags : fpu vme avx2 fma aes bmi2 fsrm\n";
        let meminfo = "MemTotal: 16777216 kB\n";
        let caps = detect_from(cpuinfo, meminfo, "32M");
        assert!(caps.avx2);
        assert!(caps.fma);
        assert!(caps.aes_ni);
        assert!(caps.bmi2);
        assert!(caps.fsrm);
        assert!(!caps.sha_ni, "sha_ni not in flags");
        assert_eq!(caps.cores, 2);
        assert_eq!(caps.l3_cache_kb, 32 * 1024);
        assert_eq!(caps.ram_total_mb, 16384);
    }
}
