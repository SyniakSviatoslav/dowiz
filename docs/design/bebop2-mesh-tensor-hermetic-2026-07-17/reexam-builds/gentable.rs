fn main() {
    // Q30 fixed point. Emit the CORDIC atan table + gain as INTEGER literals to be frozen
    // as shipped constants (like SHA3 round constants). Derived once with f64::atan; the
    // shipped runtime never recomputes them, so no target ever runs a transcendental.
    let scale = (1i64 << 30) as f64;
    print!("const ATAN_Q30: [i64; 31] = [");
    for i in 0..31 {
        let a = (2f64.powi(-(i as i32))).atan();
        print!("{}, ", (a * scale).round() as i64);
    }
    println!("];");
    // gain K = prod 1/sqrt(1+2^-2i)
    let mut k = 1.0f64;
    for i in 0..31 { k *= 1.0 / (1.0 + 2f64.powi(-2*(i as i32))).sqrt(); }
    println!("const CORDIC_K_Q30: i64 = {}; // {}", (k*scale).round() as i64, k);
    println!("const HALF_PI_Q30: i64 = {};", (std::f64::consts::FRAC_PI_2 * scale).round() as i64);
    println!("const PI_Q30: i64 = {};", (std::f64::consts::PI * scale).round() as i64);
}
