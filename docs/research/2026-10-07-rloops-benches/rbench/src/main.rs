//! R-LOOPS benches. Every replacement is checked for an identical result before timing.
//! Usage: rbench <router|journal|orders|bucket|logwalk|ppr|all> [reps]
use std::time::Instant;

mod bucket;
mod journal;
mod logwalk;
mod orders;
mod ppr;
mod router;

/// (min, median, max) of `reps` runs of `f` (each run returns its own wall-time in ns).
pub fn median_ns(reps: usize, mut f: impl FnMut() -> u128) -> (u128, u128, u128) {
    let mut v: Vec<u128> = (0..reps).map(|_| f()).collect();
    v.sort();
    (v[0], v[v.len() / 2], v[v.len() - 1])
}

pub fn time_ns(f: impl FnOnce()) -> u128 {
    let t = Instant::now();
    f();
    t.elapsed().as_nanos()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let what = args.get(1).map(String::as_str).unwrap_or("all");
    let reps: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(7);
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    let mhz = std::fs::read_to_string("/sys/devices/system/cpu/cpu4/cpufreq/scaling_cur_freq").map(|s| s.trim().parse::<u64>().unwrap_or(0) / 1000).unwrap_or(0);
    println!("RBENCH profile={profile} reps={reps} cpu4_mhz={mhz}");
    let all = what == "all";
    if all || what == "router" { router::run(reps); }
    if all || what == "journal" { journal::run(reps); }
    if all || what == "orders" { orders::run(reps); }
    if all || what == "bucket" { bucket::run(reps); }
    if all || what == "logwalk" { logwalk::run(reps); }
    if all || what == "ppr" { ppr::run(reps); }
}
