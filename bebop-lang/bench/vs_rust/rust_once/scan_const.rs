// scan_const.rs — B2 twin (ii), the "best form": schema (3 i64 columns) and predicate
// (c0 in [LO,HI) and c1 < Q, aggregate sum(c2)) are compile-time constants, exactly what
// bebop's generated scan_<digest>.bp is (docs/blueprints/B2-decisive-twins.md §3(ii)).
//
// 2026-09-08: the twin now scans a MATERIALISED table (three i64 column arrays built once,
// outside the timed region) instead of generating each row from the LCG inside the timed loop.
// Reason in full in bench/vs_rust/std_tests/gen_scan.py's header: the old shape timed three
// multiply-adds per row and only incidentally the predicate, which diluted the one quantity the
// row exists to measure (specialised vs interpreted predicate) into a shared arithmetic
// constant. The value stream is unchanged, so bench/oracles/scan_twin.py is still the oracle.
//
// argv: n, reps. Timing (per rep, stderr) covers the scan only; the fill is untimed on every
// engine. REPS on the command line rather than in the env because this binary is now always
// run with an explicit rep count — bench/vs_rust/kernel_reps.txt: a timed run must last ~100 ms
// or it measured the box, and one 1M-row scan is ~1 ms.
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

fn fill(n: usize) -> (Vec<i64>, Vec<i64>, Vec<i64>) {
    let mut c0 = vec![0i64; n];
    let mut c1 = vec![0i64; n];
    let mut c2 = vec![0i64; n];
    let mut x = SEED;
    for i in 0..n {
        x = lcg(x);
        c0[i] = ((x >> 20) % 1000000) as i64;
        x = lcg(x);
        c1[i] = ((x >> 40) % 1000) as i64;
        x = lcg(x);
        c2[i] = ((x >> 10) % 1000000) as i64;
    }
    (c0, c1, c2)
}

/// Branchless, exactly as bebop's generated kernel is: the predicate collapses to a product of
/// three 0/1 flags multiplying the aggregate column.
fn scan(c0: &[i64], c1: &[i64], c2: &[i64]) -> i64 {
    let mut sum: i64 = 0;
    for i in 0..c0.len() {
        let v = c0[i];
        let inrange = ((v >= LO) as i64) * ((v < HI) as i64) * ((c1[i] < Q) as i64);
        sum = sum.wrapping_add(inrange * c2[i]);
    }
    sum
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n: usize = args.get(1).map(|s| s.parse().unwrap()).unwrap_or(20000);
    let reps: usize = args.get(2).map(|s| s.parse().unwrap()).unwrap_or(1);
    let (c0, c1, c2) = fill(n);
    let t0 = std::time::Instant::now();
    let mut sum = 0;
    for _ in 0..reps {
        sum = scan(black_box(&c0), black_box(&c1), black_box(&c2));
    }
    let ms = t0.elapsed().as_secs_f64() * 1000.0 / reps as f64;
    eprintln!("{:.4}", ms);
    println!("sum {}", sum);
}
