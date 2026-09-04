// K2h honest twin (D11-C): fib(25) with inlining forbidden — every logical call is a
// real call, as in bebop (which never inlines).
#[inline(never)]
fn fib(n: i64) -> i64 { if n < 2 { n } else { fib(n - 1) + fib(n - 2) } }
fn main() { let t0 = std::time::Instant::now(); let r = fib(std::hint::black_box(25)); eprintln!("{:.3}", t0.elapsed().as_secs_f64() * 1000.0); println!("{}", r); }
