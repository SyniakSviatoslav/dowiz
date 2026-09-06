// scan_const.rs — B2 twin (ii), the "best form": schema (3 i64 columns) and predicate
// (c0 in [LO,HI) and c1 < Q, aggregate sum(c2)) are compile-time constants, exactly what
// bebop's generated scan_<digest>.bp is (docs/blueprints/B2-decisive-twins.md §3(ii)).
// Same LCG generator as bench/vs_rust/std_tests/gen_scan.py / bench/oracles/scan_twin.py.
// argv: n. REPS (env, default 1) — see join_hash.rs's header; no timing claimed yet.
use std::hint::black_box;

// LCG state MUST be u64: bebop's `>>` is LOGICAL (unsigned) on the 64-bit bit pattern (see
// selfhost/std/csr.bp's header comment), and so is python's (it masks to & (2^64-1) first).
// An i64 state would sign-extend on `>>` and silently diverge whenever the top bit is set.
const A: u64 = 6364136223846793005;
const C: u64 = 1442695040888963407;
const LO: i64 = 200000;
const HI: i64 = 500000;
const Q: i64 = 500;
const SEED: u64 = 5591;

fn lcg(x: u64) -> u64 { x.wrapping_mul(A).wrapping_add(C) }

fn scan(n: i64, seed: u64) -> i64 {
    let mut x = seed;
    let mut sum: i64 = 0;
    let mut i = 0;
    while i < n {
        x = lcg(x);
        let c0 = ((x >> 20) % 1000000) as i64;
        x = lcg(x);
        let c1 = ((x >> 40) % 1000) as i64;
        x = lcg(x);
        let c2 = ((x >> 10) % 1000000) as i64;
        let inrange = ((c0 >= LO) as i64) * ((c0 < HI) as i64) * ((c1 < Q) as i64);
        sum = sum.wrapping_add(inrange * c2);
        i += 1;
    }
    sum
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n: i64 = args.get(1).map(|s| s.parse().unwrap()).unwrap_or(20000);
    let reps: usize = std::env::var("REPS").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
    let t0 = std::time::Instant::now();
    let mut sum = 0;
    for _ in 0..reps {
        sum = scan(black_box(n), black_box(SEED));
    }
    let ms = t0.elapsed().as_secs_f64() * 1000.0 / reps as f64;
    eprintln!("{:.3}", ms);
    println!("sum {}", sum);
}
