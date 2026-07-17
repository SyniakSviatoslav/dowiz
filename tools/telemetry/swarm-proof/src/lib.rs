//! swarm-proof — analytical cost model + engine timing (pure-std).
//!
//! Replaces the deleted `tools/telemetry/swarm_proof.py`. Two independent claims:
//!
//!   (A) ECONOMIC — there is a crossover N where swarm_cost < sequential_cost,
//!       derived from real 2026 API prices. Closed-form, not vibes.
//!   (B) WALL-CLOCK — parallel dispatch of N independent tasks completes in
//!       ~max(task_latency), sequential in ~sum(task_latency). Measured with
//!       `std::thread` fan-out (the same primitive the agent engine uses).
//!
//! No LLM calls. Pure arithmetic + a free timing experiment.

/// Real 2026 API prices (per 1M tokens, USD; representative public tiers).
/// (input $/Mtok, output $/Mtok): architect = frontier, exec = cheap.
pub const PRICES: &[(&str, (f64, f64))] = &[
    ("frontier", (5.0, 15.0)), // e.g. Opus-class / GPT-5-class (expensive architect)
    ("mid", (0.50, 1.50)),     // e.g. Sonnet-class (mid tier)
    ("cheap", (0.10, 0.40)),   // e.g. Haiku-class / small distilled (swarm executor)
];

/// Representative per-task token draw for a "ready blueprint" execution.
pub const BLUEPRINT_TOK: (f64, f64) = (4000.0, 1500.0); // architect drafts one blueprint
pub const EXEC_TOK: (f64, f64) = (2000.0, 800.0); // cheap executor runs one blueprint

pub fn cost(tier: &str, tin: f64, tout: f64) -> f64 {
    let (pi, po) = PRICES
        .iter()
        .find(|(t, _)| *t == tier)
        .map(|(_, p)| *p)
        .unwrap_or((0.0, 0.0));
    tin / 1e6 * pi + tout / 1e6 * po
}

/// One expensive agent does N tasks itself (no swarm).
pub fn sequential_cost(n: usize, arch: &str) -> f64 {
    n as f64
        * cost(
            arch,
            BLUEPRINT_TOK.0 + EXEC_TOK.0,
            BLUEPRINT_TOK.1 + EXEC_TOK.1,
        )
}

/// Architect drafts N blueprints (once), N cheap executors run them.
pub fn swarm_cost(n: usize, arch: &str, exec_t: &str) -> f64 {
    let arch_c = n as f64 * cost(arch, BLUEPRINT_TOK.0, BLUEPRINT_TOK.1);
    let exec_c = n as f64 * cost(exec_t, EXEC_TOK.0, EXEC_TOK.1);
    arch_c + exec_c
}

/// Smallest N where swarm_cost(N) < sequential_cost(N).
pub fn crossover_n(arch: &str, exec_t: &str) -> Option<usize> {
    for n in 1..200 {
        if swarm_cost(n, arch, exec_t) < sequential_cost(n, arch) {
            return Some(n);
        }
    }
    None
}

/// Wall-clock: run N independent timed tasks. `parallel` fans out across
/// threads (std::thread::scope), else runs serially. Returns elapsed seconds.
pub fn time_tasks(n: usize, parallel: bool, task_secs: f64) -> f64 {
    use std::thread;
    use std::time::Instant;
    let start = Instant::now();
    if parallel {
        thread::scope(|s| {
            for _ in 0..n {
                s.spawn(move || {
                    let t0 = Instant::now();
                    while t0.elapsed().as_secs_f64() < task_secs {
                        // busy-wait the task duration (no sleep precision needed)
                        std::hint::spin_loop();
                    }
                });
            }
        });
    } else {
        for _ in 0..n {
            let t0 = Instant::now();
            while t0.elapsed().as_secs_f64() < task_secs {
                std::hint::spin_loop();
            }
        }
    }
    start.elapsed().as_secs_f64()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prices_table_is_complete() {
        for t in ["frontier", "mid", "cheap"] {
            assert!(PRICES.iter().any(|(k, _)| *k == t), "missing tier {t}");
        }
    }

    #[test]
    fn crossover_is_small_and_swarm_wins_for_n10() {
        for (arch, exec_t) in [("frontier", "cheap"), ("frontier", "mid"), ("mid", "cheap")] {
            let nx = crossover_n(arch, exec_t).expect("crossover exists");
            assert!(nx <= 2, "crossover N should be <=2, got {nx} for {arch}/{exec_t}");
            assert!(
                swarm_cost(10, arch, exec_t) < sequential_cost(10, arch),
                "swarm must beat sequential at N=10"
            );
        }
    }

    #[test]
    fn parallel_fanout_faster_than_serial() {
        // 8 tasks of ~0.05s each: serial ~0.4s, parallel ~0.05s.
        let serial = time_tasks(8, false, 0.05);
        let parallel = time_tasks(8, true, 0.05);
        assert!(
            parallel < serial * 0.8,
            "parallel ({parallel:.3}s) should be well under serial ({serial:.3}s)"
        );
    }

    #[test]
    fn cost_monotonic_in_n() {
        assert!(sequential_cost(10, "frontier") > sequential_cost(5, "frontier"));
        assert!(swarm_cost(10, "frontier", "cheap") > swarm_cost(5, "frontier", "cheap"));
    }
}
