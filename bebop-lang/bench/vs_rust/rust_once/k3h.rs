// K3h honest twin (D11-C): 300x300 nested loops with a nonlinear carried accumulator
// a = a*3 + x*2 + y*3 (wrapping); black_box only on input/output.
fn main() {
    let n: i64 = std::hint::black_box(300);
    let t0 = std::time::Instant::now();
    let mut a: i64 = 0;
    let mut x: i64 = n;
    while x > 0 {
        let mut y: i64 = n;
        while y > 0 { a = a.wrapping_mul(3).wrapping_add(x * 2 + y * 3); y -= 1; }
        x -= 1;
    }
    eprintln!("{:.3}", t0.elapsed().as_secs_f64() * 1000.0);
    println!("{}", std::hint::black_box(a));
}
