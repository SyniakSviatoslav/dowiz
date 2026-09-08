// K2h honest twin (D11-C): fib(25) with inlining forbidden — every logical call is a
// real call, as in bebop (which never inlines). REPS=100 reps, stderr = ms PER REP.
#[inline(never)]
fn fib(n: i64) -> i64 { if n < 2 { n } else { fib(n - 1) + fib(n - 2) } }
// REPS raised 100 -> 500 on 2026-09-08 together with bench630/k2ht.bp; see
// bench/vs_rust/kernel_reps.txt for why.
fn main() {
    let reps: i64 = std::hint::black_box(500);
    let t0 = std::time::Instant::now();
    let mut r: i64 = 0;
    for _ in 0..reps { r = r.wrapping_add(fib(std::hint::black_box(25))); }
    eprintln!("{:.3}", t0.elapsed().as_secs_f64() * 1000.0 / reps as f64);
    println!("{}", std::hint::black_box(r));
}
