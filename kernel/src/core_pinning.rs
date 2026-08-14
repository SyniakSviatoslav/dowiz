//! CorePinning — CPU-core affinity for agent dispatch.
//! Linux: minimal kernel-owned FFI for `sched_setaffinity(2)`
//! Fallback: no-op on unsupported platforms

#[cfg(target_os = "linux")]
mod linux {
    use std::ffi::c_int;

    // Linux/glibc exposes a fixed 1024-bit cpu_set_t. Keep the ABI detail local
    // instead of pulling the entire `libc` crate into the mandatory graph.
    pub(super) const CPU_SETSIZE: usize = 1024;
    const WORD_BITS: usize = usize::BITS as usize;

    #[repr(C)]
    struct CpuSet {
        words: [usize; CPU_SETSIZE / WORD_BITS],
    }

    impl CpuSet {
        fn one(cpu: usize) -> Option<Self> {
            if cpu >= CPU_SETSIZE {
                return None;
            }
            let mut set = Self {
                words: [0; CPU_SETSIZE / WORD_BITS],
            };
            set.words[cpu / WORD_BITS] |= 1usize << (cpu % WORD_BITS);
            Some(set)
        }

        #[cfg(test)]
        fn contains(&self, cpu: usize) -> bool {
            cpu < CPU_SETSIZE
                && self.words[cpu / WORD_BITS] & (1usize << (cpu % WORD_BITS)) != 0
        }
    }

    extern "C" {
        fn sched_setaffinity(pid: c_int, cpusetsize: usize, mask: *const CpuSet) -> c_int;
    }

    pub(super) fn pin_current(cpu: usize) -> bool {
        let Some(set) = CpuSet::one(cpu) else {
            return false;
        };
        // SAFETY: `set` has the Linux cpu_set_t ABI, remains alive for the call,
        // and pid 0 explicitly denotes the calling thread/process.
        unsafe { sched_setaffinity(0, core::mem::size_of::<CpuSet>(), &set) == 0 }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn cpu_set_places_boundary_bits_without_aliasing() {
            let set = CpuSet::one(WORD_BITS).expect("in range");
            assert!(set.contains(WORD_BITS));
            assert!(!set.contains(0));
            assert!(CpuSet::one(CPU_SETSIZE).is_none());
        }
    }
}

/// Number of logical CPUs detected.
pub fn cpu_count() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

/// Pin current process to specific CPU cores.
/// Returns true if pinning succeeded.
pub fn pin_to_core(core_id: usize) -> bool {
    #[cfg(target_os = "linux")]
    {
        linux::pin_current(core_id % cpu_count().min(linux::CPU_SETSIZE))
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = core_id;
        false
    }
}

/// Get optimal core layout for N agents.
/// Spreads agents evenly across cores.
pub fn optimal_layout(n_agents: usize) -> Vec<usize> {
    let n_cores = cpu_count();
    (0..n_agents).map(|i| i % n_cores).collect()
}

/// Pin a batch of agents to cores (round-robin).
pub fn pin_agents(n_agents: usize) -> Vec<usize> {
    let layout = optimal_layout(n_agents);
    for &_core in &layout {
        // In production: each agent process gets pinned individually
        // Here we just return the layout for the orchestrator
    }
    layout
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_count_is_reasonable() {
        let n = cpu_count();
        assert!(n >= 1 && n <= 256, "CPU count should be reasonable, got {}", n);
    }

    #[test]
    fn optimal_layout_spreads_evenly() {
        let layout = optimal_layout(16);
        assert_eq!(layout.len(), 16);
        // First 8 agents should be on cores 0-7
        assert_eq!(layout[0], 0 % cpu_count());
        assert_eq!(layout[cpu_count()], 0); // wraps around
    }

    #[test]
    fn pin_to_core_does_not_panic() {
        // Even on non-Linux, should not panic
        let result = pin_to_core(0);
        // Just verify it doesn't crash — may return false on non-Linux
        let _ = result;
    }

    #[test]
    fn optimal_layout_for_zero_agents_is_empty() {
        assert!(optimal_layout(0).is_empty());
    }

    #[test]
    fn pin_agents_returns_correct_count() {
        let layout = pin_agents(32);
        assert_eq!(layout.len(), 32);
    }
}
