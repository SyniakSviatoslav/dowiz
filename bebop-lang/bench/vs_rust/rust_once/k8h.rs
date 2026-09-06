// K8h honest twin (D14 item 5 / B9 falsifier for T52-T54): LCG-driven state, branch on a HIGH
// bit of the stream ((x >> 60) & 1 -- a ~50% coin flip; the low bit of an LCG alternates and
// would be perfectly predicted). The two arms do different real work: acc = acc+x vs acc-i.
// 20000 inner iterations per rep, REPS=100 (2M branches total); black_box on the seed, the
// iteration count, and the carried acc/x at each rep boundary (same convention as k3h.rs/k4h.rs)
// and the final acc. stderr = ms PER REP. LLVM may pick csel here -- that is the point of the
// row: bebop's branch vs LLVM's choice.
fn main() {
    let seed: i64 = std::hint::black_box(1);
    let n: i64 = std::hint::black_box(20000);
    let reps: i64 = std::hint::black_box(100);
    let t0 = std::time::Instant::now();
    let mut acc: i64 = 0;
    let mut x: i64 = seed;
    for _ in 0..reps {
        acc = std::hint::black_box(acc);
        x = std::hint::black_box(x);
        let mut i: i64 = n;
        while i > 0 {
            x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let bit = (x >> 60) & 1;
            acc = if bit == 1 { acc.wrapping_add(x) } else { acc.wrapping_sub(i) };
            i -= 1;
        }
    }
    eprintln!("{:.3}", t0.elapsed().as_secs_f64() * 1000.0 / reps as f64);
    println!("{}", std::hint::black_box(acc));
}
