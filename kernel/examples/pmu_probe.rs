// Host PMU diagnostic (roadmap item 27, Tier B). Opens the four hardware counters via
// `dowiz_kernel::fdr::pmu::PmuStation` (hand-rolled perf_event_open(2)), brackets a fixed
// integer workload, and prints every Tier-B reading as a real Value or its NAMED absence —
// never a fabricated 0. Use it to verify host configuration (kernel.perf_event_paranoid,
// AppArmor, seccomp) end-to-end after a change: exit 0 = all four counters read real
// hardware Values; exit 1 = at least one is Unavailable (the named reason says why).
// Run: cd kernel && cargo run --release --example pmu_probe
use dowiz_kernel::fdr::pmu::{PmuStamp, PmuStation};
use dowiz_kernel::fdr::schema::Reading;

fn show(name: &str, r: Reading<u64>) -> bool {
    match r {
        Reading::Value(v) => {
            println!("  {name:<18} = {v} (real hardware Value)");
            true
        }
        Reading::Unavailable(a) => {
            println!("  {name:<18} = UNAVAILABLE — {a:?}");
            false
        }
    }
}

fn main() {
    let paranoid = std::fs::read_to_string("/proc/sys/kernel/perf_event_paranoid")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "unreadable".to_string());
    println!("pmu_probe: kernel.perf_event_paranoid = {paranoid}");

    let station = PmuStation::new();
    let (_, delta) = station.bracket(|| {
        // Fixed integer workload so working counters are strictly > 0.
        let mut acc = 0u64;
        for i in 0..5_000_000u64 {
            acc = acc.wrapping_add(i.wrapping_mul(2654435761) ^ (i >> 3));
        }
        std::hint::black_box(acc)
    });

    println!("pmu_probe: Tier-B deltas over the bracketed workload:");
    let mut all_live = true;
    for (name, r) in [
        ("hw_instructions", delta.hw_instructions),
        ("hw_cpu_cycles", delta.hw_cpu_cycles),
        ("hw_cache_misses", delta.hw_cache_misses),
        ("hw_branch_misses", delta.hw_branch_misses),
    ] {
        all_live &= show(name, r);
    }

    if let (Reading::Value(i), Reading::Value(c)) = (delta.hw_instructions, delta.hw_cpu_cycles) {
        if c > 0 {
            println!(
                "pmu_probe: IPC = {:.3} instructions/cycle",
                i as f64 / c as f64
            );
        }
    }

    // Keep the full stamp path exercised too (Tier A must always be live on Linux x86_64).
    let stamp: PmuStamp = station.sample();
    println!(
        "pmu_probe: Tier-A sanity: tsc={:?} minflt={:?}",
        stamp.tsc_cycles, stamp.minflt
    );

    if all_live {
        println!("pmu_probe: RESULT = LIVE (all four Tier-B counters read real Values)");
    } else {
        println!("pmu_probe: RESULT = DEGRADED (named absences above; check perf_event_paranoid / AppArmor / capabilities)");
        std::process::exit(1);
    }
}
