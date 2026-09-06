// scan_generic.rs — B2 twin (ii), the "honest generic" form: schema (column count, predicate
// columns, bounds) is read at RUNTIME through a `Vec<Pred>` (not const-generic, not baked in
// by the compiler) and applied by a small interpreter — a 3-branch match over an enum, no
// allocation per row (docs/blueprints/B2-decisive-twins.md §3(ii) risk #4: "the generic Rust
// scan is strawmanned ... publish the source" — this is that source). Same LCG generator as
// scan_const.rs / bench/vs_rust/std_tests/gen_scan.py / bench/oracles/scan_twin.py.
// argv: n. REPS (env, default 1) — see join_hash.rs's header; no timing claimed yet.
use std::hint::black_box;

// LCG state MUST be u64 — see scan_const.rs's header comment (bebop's/python's `>>` is
// LOGICAL on the 64-bit pattern; an i64 state would sign-extend and silently diverge).
const A: u64 = 6364136223846793005;
const C: u64 = 1442695040888963407;
const SEED: u64 = 5591;

fn lcg(x: u64) -> u64 { x.wrapping_mul(A).wrapping_add(C) }

#[derive(Clone, Copy)]
enum Pred {
    Range(usize, i64, i64),
    Lt(usize, i64),
}

fn eval(p: &Pred, row: &[i64; 3]) -> bool {
    match *p {
        Pred::Range(col, lo, hi) => row[col] >= lo && row[col] < hi,
        Pred::Lt(col, bound) => row[col] < bound,
    }
}

fn scan(n: i64, seed: u64, preds: &[Pred], agg_col: usize) -> i64 {
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
        let row = [c0, c1, c2];
        let mut ok = true;
        for p in preds {
            ok = ok && eval(p, &row);
        }
        if ok {
            sum = sum.wrapping_add(row[agg_col]);
        }
        i += 1;
    }
    sum
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n: i64 = args.get(1).map(|s| s.parse().unwrap()).unwrap_or(20000);
    let reps: usize = std::env::var("REPS").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
    // schema/predicate come from a runtime-built descriptor, not a compile-time constant —
    // black_box keeps LLVM from proving these are the literals below and re-specialising.
    let preds = black_box(vec![Pred::Range(0, 200000, 500000), Pred::Lt(1, 500)]);
    let agg_col = black_box(2usize);
    let t0 = std::time::Instant::now();
    let mut sum = 0;
    for _ in 0..reps {
        sum = scan(black_box(n), black_box(SEED), &preds, agg_col);
    }
    let ms = t0.elapsed().as_secs_f64() * 1000.0 / reps as f64;
    eprintln!("{:.3}", ms);
    println!("sum {}", sum);
}
