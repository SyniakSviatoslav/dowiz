//! A6 / T8 runnable probe — demonstrates that `Sin`/`Cos` integer-exact emission
//! now routes through the digest-pinned Q30 CORDIC substrate instead of hard-
//! refusing, and measures the Q30 parity error against the f64 reference.
//!
//! Run: `cargo run --release --bin cordic_probe`
//!
//! Conventions used by the emitted int code (must match `emit_int_checked` in
//! `src/lib.rs`):
//!   * the angle argument is Q30 fixed-point RADIANS (1 rad = 1<<30),
//!   * the returned sine/cosine is Q30 unit-magnitude (|value| <= 1<<30).

use eqc_rs::{Equation, Expr};

const ONE_Q30: f64 = (1i64 << 30) as f64;

fn main() {
    // 1) Emit the integer-exact Rust for sin(theta) and cos(theta). This must now
    //    SUCCEED (it used to hard-refuse with `IntEmissionUnsupported`).
    let theta = Expr::sym("theta");
    let sin_eq = Equation::new("my_sin", &["theta"], theta.clone().sin());
    let cos_eq = Equation::new("my_cos", &["theta"], theta.cos());

    let sin_src = sin_eq
        .emit_int_checked_rust()
        .expect("A6/T8: Sin must now emit in integer-exact mode (no longer refused)");
    let cos_src = cos_eq
        .emit_int_checked_rust()
        .expect("A6/T8: Cos must now emit in integer-exact mode (no longer refused)");

    println!("=== emitted integer-exact sin ===\n{sin_src}\n");
    println!("=== emitted integer-exact cos ===\n{cos_src}\n");

    assert!(
        sin_src.contains("cordic::cordic_sincos"),
        "wiring broken: emitted sin does not call the CORDIC substrate"
    );
    assert!(
        cos_src.contains("cordic::cordic_sincos"),
        "wiring broken: emitted cos does not call the CORDIC substrate"
    );
    println!("wiring OK: both Sin/Cos emit through eqc_rs::cordic::cordic_sincos\n");

    // 2) Measure max Q30 parity error vs the f64 reference across a sweep that
    //    also exercises range-reduction (angles beyond [-pi/2, pi/2] and beyond
    //    [-pi, pi]).
    let mut max_sin: i64 = 0;
    let mut max_cos: i64 = 0;
    let mut worst_theta_sin: i64 = 0;
    let mut worst_theta_cos: i64 = 0;

    // Sample over ~4 full periods so range-reduction paths are hit.
    let step = (1i64 << 30) / 1024; // ~1/1024 rad
    let lo = -4 * (1i64 << 30);
    let hi = 4 * (1i64 << 30);
    let mut z = lo;
    while z <= hi {
        let (c, s) = eqc_rs::cordic::cordic_sincos(z);
        let rad = z as f64 / ONE_Q30;
        let ref_s = (rad.sin() * ONE_Q30).round() as i64;
        let ref_c = (rad.cos() * ONE_Q30).round() as i64;
        let es = (s - ref_s).abs();
        let ec = (c - ref_c).abs();
        if es > max_sin {
            max_sin = es;
            worst_theta_sin = z;
        }
        if ec > max_cos {
            max_cos = ec;
            worst_theta_cos = z;
        }
        z += step;
    }

    println!(
        "max |cordic.sin - f64.sin| over sweep: {max_sin} (Q30 units, ~{:.3e} rad)",
        max_sin as f64 / ONE_Q30
    );
    println!(
        "max |cordic.cos - f64.cos| over sweep: {max_cos} (Q30 units, ~{:.3e} rad)",
        max_cos as f64 / ONE_Q30
    );
    println!(
        "worst-theta sin = {worst_theta_sin} ({} rad)",
        worst_theta_sin as f64 / ONE_Q30
    );
    println!(
        "worst-theta cos = {worst_theta_cos} ({} rad)",
        worst_theta_cos as f64 / ONE_Q30
    );

    // 3) A few human-readable sample points.
    println!("\n   theta(rad)    sin_cordic/Q30   sin_f64/Q30    cos_cordic/Q30   cos_f64/Q30");
    for &theta_rad in &[0.0_f64, 0.5, 1.0, std::f64::consts::FRAC_PI_2, 2.0, -1.3, 3.5] {
        let q = (theta_rad * ONE_Q30).round() as i64;
        let (c, s) = eqc_rs::cordic::cordic_sincos(q);
        println!(
            "   {:>9.4}   {:>14}   {:>11.4}   {:>14}   {:>11.4}",
            theta_rad,
            s,
            (theta_rad.sin() * ONE_Q30).round(),
            c,
            (theta_rad.cos() * ONE_Q30).round(),
        );
    }

    println!("\ncordic_probe OK");
}
