//! ITEM 4 — Deterministic sin/cos WITHOUT libm: fixed-point CORDIC (integer-only).
//!
//! Objection (blueprint E13 / register #13): "transcendental float paths are NOT cross-target
//! bit-identical" — true for f64::sin (delegates to platform libm). Operator's rebuttal: the two
//! forms "are identical if the functions/equations inside are identical" — CORRECT if a portable
//! deterministic implementation replaces libm. This module IS that implementation.
//!
//! The RUNTIME kernel below (`cordic_sincos`) uses ONLY i64 add/sub/compare and ARITHMETIC SHIFT.
//! No f64, no libm, no platform-dependent op. The atan table + gain are FROZEN integer constants
//! (shipped like SHA3 round constants), derived once offline — never recomputed on any target.
//! f64 appears ONLY in the measurement/test harness (accuracy vs libm), never on the codec path.

// ---- FROZEN integer constants (Q30 fixed point). Generated once; shipped as literals. ----
const ATAN_Q30: [i64; 31] = [843314857, 497837829, 263043837, 133525159, 67021687, 33543516,
    16775851, 8388437, 4194283, 2097149, 1048576, 524288, 262144, 131072, 65536, 32768, 16384,
    8192, 4096, 2048, 1024, 512, 256, 128, 64, 32, 16, 8, 4, 2, 1];
const CORDIC_K_Q30: i64 = 652032874; // circular gain 0.60725293...
const HALF_PI_Q30: i64 = 1686629713;
const PI_Q30: i64 = 3373259426;
const TWO_PI_Q30: i64 = 6746518852;
const ONE_Q30: i64 = 1 << 30;

/// PURE-INTEGER sin/cos. Input angle in Q30 radians. Returns (cos_q30, sin_q30).
/// Zero float ops. Only: +, -, <, ==, arithmetic >> (sign-extending, well-defined for i64).
fn cordic_sincos(mut z: i64) -> (i64, i64) {
    // 1) Range-fold to [-pi, pi] with integer add/sub only.
    while z >  PI_Q30 { z -= TWO_PI_Q30; }
    while z < -PI_Q30 { z += TWO_PI_Q30; }
    // 2) Fold [-pi,-pi/2)U(pi/2,pi] into [-pi/2,pi/2], remembering to negate both outputs.
    let mut negate = false;
    if z >  HALF_PI_Q30 { z -= PI_Q30; negate = true; }
    else if z < -HALF_PI_Q30 { z += PI_Q30; negate = true; }
    // 3) CORDIC circular rotation. Seed x = gain (so final |(x,y)| = 1), y = 0.
    let mut x: i64 = CORDIC_K_Q30;
    let mut y: i64 = 0;
    for i in 0..31u32 {
        let dx = x >> i;               // x * 2^-i  (arithmetic shift; cross-target defined)
        let dy = y >> i;               // y * 2^-i
        if z >= 0 { x -= dy; y += dx; z -= ATAN_Q30[i as usize]; }
        else      { x += dy; y -= dx; z += ATAN_Q30[i as usize]; }
    }
    if negate { (-x, -y) } else { (x, y) }
}

// FNV-1a over the raw output stream — the exact digest a cross-arch CI job would compare.
fn digest(stream: &[i64]) -> u64 {
    let mut hsh: u64 = 0xcbf29ce484222325;
    for &v in stream { for b in v.to_le_bytes() { hsh ^= b as u64; hsh = hsh.wrapping_mul(0x100000001b3); } }
    hsh
}

fn main() {
    // Sweep angles across two full periods, step 1/4096 rad, in Q30 — integer-only construction.
    const STEP_Q30: i64 = ONE_Q30 / 4096;
    let lo = -2 * PI_Q30;
    let n = ((2 * TWO_PI_Q30) / STEP_Q30) as usize;

    let mut stream_a: Vec<i64> = Vec::with_capacity(2 * n);
    let mut z = lo;
    for _ in 0..n {
        let (c, s) = cordic_sincos(z);
        stream_a.push(c); stream_a.push(s);
        z += STEP_Q30;
    }
    // Independent second run (fresh state) — determinism check.
    let mut stream_b: Vec<i64> = Vec::with_capacity(2 * n);
    let mut z = lo;
    for _ in 0..n { let (c, s) = cordic_sincos(z); stream_b.push(c); stream_b.push(s); z += STEP_Q30; }

    let da = digest(&stream_a);
    let db = digest(&stream_b);
    println!("determinism: {} samples/run, digest_run1=0x{:016x} digest_run2=0x{:016x} -> {}",
        n, da, db, if da == db { "BIT-IDENTICAL" } else { "DIVERGED" });
    assert_eq!(stream_a, stream_b, "pure-integer CORDIC must be reproducible");

    // Accuracy vs libm (f64 used ONLY here in the harness, never in the codec).
    let scale = ONE_Q30 as f64;
    let mut max_err_sin = 0f64;
    let mut max_err_cos = 0f64;
    let mut z = -HALF_PI_Q30;
    let mut samples = 0;
    while z <= HALF_PI_Q30 {
        let (c, s) = cordic_sincos(z);
        let ang = z as f64 / scale;
        max_err_sin = max_err_sin.max(((s as f64 / scale) - ang.sin()).abs());
        max_err_cos = max_err_cos.max(((c as f64 / scale) - ang.cos()).abs());
        z += STEP_Q30; samples += 1;
    }
    println!("accuracy vs libm over [-pi/2,pi/2], {} samples: max|sin err|={:.2e}  max|cos err|={:.2e}  (~{:.1} bits)",
        samples, max_err_sin, max_err_cos, -(max_err_sin.max(max_err_cos)).log2());

    // Spot check
    let (c0, s0) = cordic_sincos(0);
    let (ch, sh) = cordic_sincos(HALF_PI_Q30);
    println!("spot: sincos(0)=(cos {:.6}, sin {:.6})  sincos(pi/2)=(cos {:.6}, sin {:.6})",
        c0 as f64/scale, s0 as f64/scale, ch as f64/scale, sh as f64/scale);

    println!("\nWHAT CLOSES THE CROSS-ARCH LOOP (cannot run ARM here, but the loop is small & explicit):");
    println!("  * runtime kernel touches only i64 +,-,<,==,>> (arithmetic shift). Rust guarantees");
    println!("    i64 wrapping/shift semantics identical on every target (unlike f64 transcendentals).");
    println!("  * CI addition: build this on x86_64 AND aarch64 (cross-compile or qemu), emit the");
    println!("    output digest 0x{:016x}, assert byte-equal across both. That single asserted digest", da);
    println!("    is the whole cross-target proof — no per-target float tolerance needed.");
}
