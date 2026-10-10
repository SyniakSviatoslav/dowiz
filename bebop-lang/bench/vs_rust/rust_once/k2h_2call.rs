// K2H2 honest twin (lane rfast-lsr, 2026-10-07, operator Q3 "show K2H both ways"): fib(25)
// with inlining forbidden AND tail-recursion elimination blocked. rust_once/k2h.rs lets LLVM
// turn the second recursive call into a loop with an accumulator (one `bl` per node, the
// research report §4 objdump); here `black_box` sits on the second call's result, so the
// add is no longer an accumulation LLVM may move into a loop and EVERY logical call stays a
// real `bl` -- the same work bench630/k2h2t.bp does. REPS=500 (bench/vs_rust/kernel_reps.txt
// k2h), stderr = ms PER REP.
#[inline(never)]
fn fib(n: i64) -> i64 {
    if n < 2 { n } else { fib(n - 1) + std::hint::black_box(fib(n - 2)) }
}
fn main() {
    let reps: i64 = std::hint::black_box(500);
    let t0 = std::time::Instant::now();
    let mut r: i64 = 0;
    for _ in 0..reps { r = r.wrapping_add(fib(std::hint::black_box(25))); }
    eprintln!("{:.3}", t0.elapsed().as_secs_f64() * 1000.0 / reps as f64);
    println!("{}", std::hint::black_box(r));
}
