// K4h honest twin (D11-C, 2026-09-06): v = (v + i*7)*3 - 11 wrapping, 2M iterations,
// black_box only on input/output (k4.rs keeps black_box INSIDE the loop = the 2.85 ms
// "twin" of SPEEDUP-ANALYSIS; this one lets LLVM do whatever it can with the loop, which
// is nothing close-form: the recurrence is multiplicative). REPS=100, stderr = ms PER REP.
fn main() {
    let n: i64 = std::hint::black_box(2_000_000);
    let reps: i64 = std::hint::black_box(100);
    let t0 = std::time::Instant::now();
    let mut v: i64 = 1;
    for _ in 0..reps {
        v = std::hint::black_box(v);
        let mut i: i64 = n;
        while i > 0 { v = v.wrapping_add(i.wrapping_mul(7)).wrapping_mul(3).wrapping_sub(11); i -= 1; }
    }
    eprintln!("{:.3}", t0.elapsed().as_secs_f64() * 1000.0 / reps as f64);
    println!("{}", std::hint::black_box(v));
}
