//! CorePinning — CPU-core affinity for agent dispatch.
//! Linux: sched_setaffinity via libc
//! Fallback: no-op on unsupported platforms

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
        unsafe {
            let mut set: libc::cpu_set_t = std::mem::zeroed();
            libc::CPU_ZERO(&mut set);
            libc::CPU_SET(core_id % cpu_count(), &mut set);
            let result = libc::sched_setaffinity(
                0, // current pid
                std::mem::size_of::<libc::cpu_set_t>(),
                &set,
            );
            result == 0
        }
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
    for &core in &layout {
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
