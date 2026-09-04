// K1h honest twin (D11-C): loop-carried NONLINEAR recurrence s = s*3 + i (wrapping),
// black_box only on the input and the output — LLVM keeps the loop (SCEV cannot
// close-form it) and does not vectorize it. Twin of kernels/k1h.bp.
fn main() {
    let n: i64 = std::hint::black_box(1_000_000);
    let t0 = std::time::Instant::now();
    let mut s: i64 = 0;
    let mut i: i64 = n;
    while i > 0 { s = s.wrapping_mul(3).wrapping_add(i); i -= 1; }
    eprintln!("{:.3}", t0.elapsed().as_secs_f64() * 1000.0);
    println!("{}", std::hint::black_box(s));
}
