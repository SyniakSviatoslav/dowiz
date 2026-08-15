#![allow(unused)]
//! hw_profile.rs — CPU core topology + cache hierarchy + clock source detection.
//!
//! The no_std core ships the pure data type (`CpuTopology`) + the cpuinfo parser
//! (`parse_cpuinfo`) + cache-size parser (`parse_cache_size`). The `probe()`
//! entry point (which reads /proc/cpuinfo + /sys via the VFS) lives in the
//! kernel held-handle shim; the /sys-derived fields (cache sizes, NUMA nodes)
//! are filled there, never here.
//!
//! # Fail-closed
//! If probe fails (e.g. /sys not mounted in wasm), all values report Unknown
//! or 0. No boot hangs, no panics.

use crate::TriState;
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;

/// CPU topology snapshot (single-socket, single-NUMA-node).
#[derive(Debug, Clone)]
pub struct CpuTopology {
    /// Number of logical processors (SMT threads).
    pub logical_processors: usize,
    /// Number of physical cores.
    pub physical_cores: usize,
    /// SMT threads per core (0 = unknown).
    pub smt_threads_per_core: usize,
    /// Per-core cache sizes in bytes.
    pub l1d_size: usize,
    pub l1i_size: usize,
    pub l2_size: usize,
    pub l3_size: usize,
    /// Cache line size (bytes).
    pub cache_line_size: usize,
    /// Base clock frequency (Hz).
    pub base_freq_hz: u64,
    /// NUMA node count.
    pub numa_nodes: usize,
    /// Whether TSC has known frequency (invariant TSC).
    pub tsc_invariant: TriState,
    pub tsc_known_freq: TriState,
}

impl Default for CpuTopology {
    fn default() -> Self {
        CpuTopology {
            logical_processors: 0,
            physical_cores: 0,
            smt_threads_per_core: 0,
            l1d_size: 0,
            l1i_size: 0,
            l2_size: 0,
            l3_size: 0,
            cache_line_size: 0,
            base_freq_hz: 0,
            numa_nodes: 1,
            tsc_invariant: TriState::Unknown,
            tsc_known_freq: TriState::Unknown,
        }
    }
}

impl CpuTopology {
    /// Summary of topology for dashboard.
    pub fn dashboard(&self) -> String {
        let mut out = String::with_capacity(256);
        out.push_str("HW Profile\n");
        out.push_str(&format!("  CPU:       {} logical / {} cores ({} SMT/core)\n",
            self.logical_processors, self.physical_cores, self.smt_threads_per_core));
        out.push_str(&format!("  Freq:      {} MHz\n", self.base_freq_hz / 1_000_000));
        out.push_str(&format!("  L1d:       {} KB\n", self.l1d_size / 1024));
        out.push_str(&format!("  L1i:       {} KB\n", self.l1i_size / 1024));
        out.push_str(&format!("  L2:        {} KB\n", self.l2_size / 1024));
        out.push_str(&format!("  L3:        {} KB\n", self.l3_size / 1024));
        out.push_str(&format!("  Cacheline: {} B\n", self.cache_line_size));
        out.push_str(&format!("  TSC known: {}\n", self.tsc_known_freq));
        out
    }
}

/// Parse a /sys cache size string ("32K", "512K", "32M") into bytes.
pub fn parse_cache_size(s: &str) -> usize {
    let s = s.trim();
    if s.ends_with('K') {
        s.trim_end_matches('K').parse::<usize>().unwrap_or(0) * 1024
    } else if s.ends_with('M') {
        s.trim_end_matches('M').parse::<usize>().unwrap_or(0) * 1024 * 1024
    } else {
        s.parse().unwrap_or(0)
    }
}

/// Parse /proc/cpuinfo text into a topology snapshot (pure — no I/O).
///
/// Fills logical/physical cores, SMT ratio, base frequency, and TSC flags.
/// The /sys-derived fields (cache sizes, NUMA nodes) are left at default and
/// filled by the kernel shim's `probe()`.
pub fn parse_cpuinfo(cpuinfo_raw: &str) -> CpuTopology {
    let mut topo = CpuTopology::default();
    let lines: Vec<&str> = cpuinfo_raw.lines().collect();

    // Count logical processors.
    topo.logical_processors = lines.iter().filter(|l| l.starts_with("processor")).count();

    // Count unique physical cores (core id).
    let mut core_ids = BTreeSet::new();
    for l in &lines {
        if let Some(val) = l.strip_prefix("core id\t\t: ") {
            core_ids.insert(val.trim());
        }
    }
    topo.physical_cores = core_ids.len();
    // ARM64 (aarch64) /proc/cpuinfo has no `core id` field — each
    // `processor` entry is one physical core (SMT is not exposed there).
    // Fall back to 1:1 so the snapshot stays consistent instead of
    // reporting 0 physical cores alongside N logical processors.
    if topo.physical_cores == 0 && topo.logical_processors > 0 {
        topo.physical_cores = topo.logical_processors;
    }
    if topo.physical_cores > 0 && topo.logical_processors > 0 {
        topo.smt_threads_per_core = topo.logical_processors / topo.physical_cores;
    }

    // Base frequency.
    if let Some(first) = lines.iter().find(|l| l.starts_with("cpu MHz")) {
        if let Some(mhz_str) = first.split(':').nth(1) {
            let mhz: f64 = mhz_str.trim().parse().unwrap_or(0.0);
            topo.base_freq_hz = (mhz * 1_000_000.0) as u64;
        }
    }

    // TSC flags.
    if let Some(first) = lines.iter().find(|l| l.starts_with("flags")) {
        let has_known = first.contains("tsc_known_freq");
        let has_invariant = first.contains("tsc_invariant") || first.contains("constant_tsc");
        topo.tsc_known_freq = TriState::from_bool(has_known);
        topo.tsc_invariant = TriState::from_bool(has_invariant);
    }

    topo
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_size_parsing() {
        assert_eq!(parse_cache_size("32K"), 32 * 1024);
        assert_eq!(parse_cache_size("512K"), 512 * 1024);
        assert_eq!(parse_cache_size("32M"), 32 * 1024 * 1024);
        assert_eq!(parse_cache_size(""), 0);
    }

    #[test]
    fn dashboard_contains_cpu() {
        let topo = CpuTopology::default();
        let d = topo.dashboard();
        assert!(d.contains("HW Profile"));
        assert!(d.contains("CPU:"));
    }

    #[test]
    fn default_topo_sane() {
        let topo = CpuTopology::default();
        assert_eq!(topo.logical_processors, 0);
        assert_eq!(topo.numa_nodes, 1);
        assert_eq!(topo.tsc_invariant, TriState::Unknown);
    }

    #[test]
    fn parse_cpuinfo_counts_processors() {
        let raw = "processor\t: 0\ncore id\t\t: 0\nprocessor\t: 1\ncore id\t\t: 0\nprocessor\t: 2\ncore id\t\t: 1\nprocessor\t: 3\ncore id\t\t: 1\n";
        let topo = parse_cpuinfo(raw);
        assert_eq!(topo.logical_processors, 4);
        assert_eq!(topo.physical_cores, 2);
        assert_eq!(topo.smt_threads_per_core, 2);
    }
}
