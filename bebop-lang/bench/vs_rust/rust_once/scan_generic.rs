// scan_generic.rs — B2 twin (ii), the "honest generic" form: the schema (which columns exist),
// the predicate (which column, which bounds) and the aggregate column are read at RUNTIME from
// a descriptor, and applied by a small interpreter — a two-variant match over an enum, no
// allocation per row (docs/blueprints/B2-decisive-twins.md §3(ii) risk #4: "the generic Rust
// scan is strawmanned ... publish the source" — this is that source).
//
// 2026-09-08: like scan_const.rs, this now scans a MATERIALISED table (columns built once,
// untimed) rather than generating rows inside the timed loop — see gen_scan.py's header for
// why. That change is what makes the row informative: with the LCG out of the loop, the
// difference between this file and scan_const.rs IS the cost of not knowing the query at
// compile time, which is exactly the quantity a specialising compiler claims to remove.
//
// What "generic" honestly means here, and what it deliberately does NOT mean:
//   - the interpreter does not allocate, does not box, and does not go through a trait object;
//     it is a `match` over a two-variant `Copy` enum, the cheapest dynamic dispatch there is;
//   - columns are reached as `cols[p.col]` — a generic engine cannot know the column index at
//     compile time, and this indirection is a real part of being generic, not a handicap;
//   - `black_box` on the descriptor stops LLVM proving the predicate is the literal one below
//     and re-specialising the loop, which would quietly turn this file into scan_const.rs.
// If a reviewer thinks this is still a strawman, the fix is to make this file faster, not to
// discount the row: it is the denominator of the gate.
//
// argv: n, reps. Timing (per rep, stderr) covers the scan only.
use std::hint::black_box;

const A: u64 = 6364136223846793005;
const C: u64 = 1442695040888963407;
const SEED: u64 = 5591;

fn lcg(x: u64) -> u64 { x.wrapping_mul(A).wrapping_add(C) }

fn fill(n: usize) -> Vec<Vec<i64>> {
    let mut cols = vec![vec![0i64; n], vec![0i64; n], vec![0i64; n]];
    let mut x = SEED;
    for i in 0..n {
        x = lcg(x);
        cols[0][i] = ((x >> 20) % 1000000) as i64;
        x = lcg(x);
        cols[1][i] = ((x >> 40) % 1000) as i64;
        x = lcg(x);
        cols[2][i] = ((x >> 10) % 1000000) as i64;
    }
    cols
}

#[derive(Clone, Copy)]
enum Pred {
    Range(usize, i64, i64),
    Lt(usize, i64),
}

#[inline]
fn eval(p: &Pred, cols: &[Vec<i64>], i: usize) -> bool {
    match *p {
        Pred::Range(col, lo, hi) => {
            let v = cols[col][i];
            v >= lo && v < hi
        }
        Pred::Lt(col, bound) => cols[col][i] < bound,
    }
}

fn scan(cols: &[Vec<i64>], preds: &[Pred], agg_col: usize, n: usize) -> i64 {
    let mut sum: i64 = 0;
    for i in 0..n {
        let mut ok = true;
        for p in preds {
            ok = ok && eval(p, cols, i);
        }
        if ok {
            sum = sum.wrapping_add(cols[agg_col][i]);
        }
    }
    sum
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n: usize = args.get(1).map(|s| s.parse().unwrap()).unwrap_or(20000);
    let reps: usize = args.get(2).map(|s| s.parse().unwrap()).unwrap_or(1);
    let cols = fill(n);
    // schema/predicate come from a runtime-built descriptor, not a compile-time constant.
    let preds = black_box(vec![Pred::Range(0, 200000, 500000), Pred::Lt(1, 500)]);
    let agg_col = black_box(2usize);
    let t0 = std::time::Instant::now();
    let mut sum = 0;
    for _ in 0..reps {
        sum = scan(black_box(&cols), &preds, agg_col, black_box(n));
    }
    let ms = t0.elapsed().as_secs_f64() * 1000.0 / reps as f64;
    eprintln!("{:.4}", ms);
    println!("sum {}", sum);
}
