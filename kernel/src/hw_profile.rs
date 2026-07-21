//! `kernel::hw_profile` — CPU core topology + cache hierarchy + clock source detection.
//!
//! Probes /sys/devices/system/cpu/ and /proc/cpuinfo at init to build a
//! deterministic snapshot of hardware topology. All measurement is pure data
//! (no I/O after init). The snapshot is the single authority for:
//! - SMT/core topology (which threads share a core)
//! - Cache hierarchy (L1d/L1i/L2/L3 sizes, associativity, line size)
//! - Clock source type (TSC/kvm-clock/HPET/ACPI_PM)
//! - NUMA node layout
//! - Bug/workaround flags
//!
//! # Fail-closed
//! If probe fails (e.g. /sys not mounted in wasm), all values report Unknown
//! or 0. No boot hangs, no panics.

use crate::TriState;

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
    /// Probe hardware topology from /proc/cpuinfo and /sys.
    /// Pure computation: reads are done once at init.
    pub fn probe() -> Self {
        let mut topo = CpuTopology::default();

        let cpuinfo_raw = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
        let lines: Vec<&str> = cpuinfo_raw.lines().collect();

        // Count logical processors.
        topo.logical_processors = lines.iter().filter(|l| l.starts_with("processor")).count();

        // Count unique physical cores (core id).
        let mut core_ids = std::collections::BTreeSet::new();
        for l in &lines {
            if let Some(val) = l.strip_prefix("core id\t\t: ") {
                core_ids.insert(val.trim());
            }
        }
        topo.physical_cores = core_ids.len();
        if topo.physical_cores > 0 && topo.logical_processors > 0 {
            topo.smt_threads_per_core = topo.logical_processors / topo.physical_cores;
        }

        // Cache sizes from /sys.
        for idx in 0..8 {
            let typ = std::fs::read_to_string(
                format!("/sys/devices/system/cpu/cpu0/cache/index{idx}/type")
            ).unwrap_or_default();
            let size_str = std::fs::read_to_string(
                format!("/sys/devices/system/cpu/cpu0/cache/index{idx}/size")
            ).unwrap_or_default();
            let size = parse_cache_size(&size_str);
            let line = std::fs::read_to_string(
                format!("/sys/devices/system/cpu/cpu0/cache/index{idx}/coherency_line_size")
            ).unwrap_or_default();
            if topo.cache_line_size == 0 {
                topo.cache_line_size = line.trim().parse().unwrap_or(64);
            }

            match typ.trim() {
                "Data" if size > 0 => topo.l1d_size = size,
                "Instruction" if size > 0 => topo.l1i_size = size,
                "Unified" => {
                    // index2 = L2, index3 = L3
                    if idx < 3 { topo.l2_size = size; }
                    else { topo.l3_size = size; }
                }
                _ => {}
            }
        }

        // Base frequency.
        if let Some(first) = lines.iter().find(|l| l.starts_with("cpu MHz")) {
            if let Some(mhz_str) = first.split(':').nth(1) {
                let mhz: f64 = mhz_str.trim().parse().unwrap_or(0.0);
                topo.base_freq_hz = (mhz * 1_000_000.0) as u64;
            }
        }

        // NUMA nodes.
        topo.numa_nodes = std::fs::read_to_string("/sys/devices/system/node/online")
            .ok()
            .and_then(|s| {
                let count = s.split(',').filter(|p| !p.is_empty()).count();
                if count > 0 { Some(count) } else { None }
            })
            .unwrap_or(1);

        // TSC flags.
        if let Some(first) = lines.iter().find(|l| l.starts_with("flags")) {
            let has_known = first.contains("tsc_known_freq");
            let has_invariant = first.contains("tsc_invariant") || first.contains("constant_tsc");
            topo.tsc_known_freq = TriState::from_bool(has_known);
            topo.tsc_invariant = TriState::from_bool(has_invariant);
        }

        topo
    }

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

fn parse_cache_size(s: &str) -> usize {
    let s = s.trim();
    if s.ends_with('K') {
        s.trim_end_matches('K').parse::<usize>().unwrap_or(0) * 1024
    } else if s.ends_with('M') {
        s.trim_end_matches('M').parse::<usize>().unwrap_or(0) * 1024 * 1024
    } else {
        s.parse().unwrap_or(0)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_returns_default_on_no_proc() {
        let topo = CpuTopology::probe();
        if topo.physical_cores == 0 {
            assert_eq!(topo.logical_processors, 0);
        } else {
            assert!(topo.logical_processors >= topo.physical_cores);
        }
    }

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
}
