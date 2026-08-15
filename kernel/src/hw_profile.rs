//! hw_profile.rs — std host shim (pure `CpuTopology`/`parse_cpuinfo`/
//! `parse_cache_size` live in `dowiz_core::hw_profile`; the VFS `probe()` seam
//! stays here).
//!
//! `probe()` reads /proc/cpuinfo and /sys through `crate::vfs`, then delegates
//! the pure parsing to the no_std core. Fail-closed: any unreadable file →
//! Unknown/0, never a panic.

pub use dowiz_core::hw_profile::*;

/// Probe hardware topology from /proc/cpuinfo and /sys (fills the
/// /sys-derived cache + NUMA fields the no_std core leaves at default).
pub fn probe() -> CpuTopology {
    let cpuinfo_raw = crate::vfs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let mut topo = parse_cpuinfo(&cpuinfo_raw);

    // Cache sizes from /sys.
    for idx in 0..8 {
        let typ = crate::vfs::read_to_string(
            format!("/sys/devices/system/cpu/cpu0/cache/index{idx}/type")
        ).unwrap_or_default();
        let size_str = crate::vfs::read_to_string(
            format!("/sys/devices/system/cpu/cpu0/cache/index{idx}/size")
        ).unwrap_or_default();
        let size = parse_cache_size(&size_str);
        let line = crate::vfs::read_to_string(
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

    // NUMA nodes.
    topo.numa_nodes = crate::vfs::read_to_string("/sys/devices/system/node/online")
        .ok()
        .and_then(|s| {
            let count = s.split(',').filter(|p| !p.is_empty()).count();
            if count > 0 { Some(count) } else { None }
        })
        .unwrap_or(1);

    topo
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_returns_default_on_no_proc() {
        let topo = probe();
        if topo.physical_cores == 0 {
            assert_eq!(topo.logical_processors, 0);
        } else {
            assert!(topo.logical_processors >= topo.physical_cores);
        }
    }
}
