//! T8 / A6 — Deterministic sin/cos WITHOUT libm: fixed-point CORDIC (integer-only).
//!
//! Promoted from `docs/design/bebop2-mesh-tensor-hermetic-2026-07-17/reexam-builds/item4_cordic.rs`
//! (doc 20 item 4, FLIP) into `tools/eqc-rs` as the fixed-point-transcendental substrate.
//!
//! Objection (blueprint E13 / register #13): "transcendental float paths are NOT cross-target
//! bit-identical" — true for `f64::sin` (delegates to platform libm). This module is the
//! operator's rebuttal: a portable deterministic implementation that replaces libm.
//!
//! The RUNTIME kernel below (`cordic_sincos`) uses ONLY `i64` add/sub/compare and ARITHMETIC
//! SHIFT. No `f64`, no libm, no platform-dependent op. The atan table + gain are FROZEN integer
//! constants (shipped like SHA3 round constants), derived once offline — never recomputed on any
//! target. `f64` appears ONLY in the accuracy harness (not provided here), never on the codec
//! path.
//!
//! Determinism is proven by a FNV-1a digest over the raw output stream of a fixed sample sweep.
//! `CORDIC_SINCOS_DIGEST` is the value measured bit-identical twice in doc 20 over 51,471 samples;
//! the digest test (see `tests/cordic_digest.rs`) *recomputes* it from this module's output and
//! asserts equality — never a hardcoded claim. The `ITERS-1` teeth test proves the digest is
//! sensitive (an adversarial one-fewer-iteration run MUST differ).

// ---- FROZEN integer constants (Q30 fixed point). Generated once; shipped as literals. ----
const ATAN_Q30: [i64; 31] = [843314857, 497837829, 263043837, 133525159, 67021687, 33543516,
    16775851, 8388437, 4194283, 2097149, 1048576, 524288, 262144, 131072, 65536, 32768, 16384,
    8192, 4096, 2048, 1024, 512, 256, 128, 64, 32, 16, 8, 4, 2, 1];
const CORDIC_K_Q30: i64 = 652032874; // circular gain 0.60725293...
const HALF_PI_Q30: i64 = 1686629713;
const PI_Q30: i64 = 3373259426;
const TWO_PI_Q30: i64 = 6746518852;
const ONE_Q30: i64 = 1 << 30;

/// Number of CORDIC rotation iterations in the canonical (pinned) kernel.
pub const ITERS: u32 = 31;

/// Cross-target determinism proof for the Q30 CORDIC sin/cos primitive:
/// FNV-1a digest over 51,471 samples, measured twice bit-identical
/// (20-BUILD-TEST-FIRST-REEXAMINATION.md:181,207-208).
///
/// This is the EXPECTED pin. The digest test recomputes the digest from this module's
/// output and asserts it equals this constant — the constant is documented intent, the
/// test is the proof that the implementation still reproduces it.
pub const CORDIC_SINCOS_DIGEST: u64 = 0x9d1c_0e89_c65c_be08;

/// Sample-sweep step (Q30 radians): 1/4096 rad.
const STEP_Q30: i64 = ONE_Q30 / 4096;
/// Sample-sweep lower bound: two full periods below zero (Q30 radians).
const LO_Q30: i64 = -2 * PI_Q30;

/// PURE-INTEGER sin/cos. Input angle in Q30 radians. Returns (cos_q30, sin_q30).
/// Zero float ops. Only: +, -, <, ==, arithmetic >> (sign-extending, well-defined for `i64`)
/// — identical on every target.
pub fn cordic_sincos(z: i64) -> (i64, i64) {
    cordic_sincos_iters(z, ITERS)
}

/// Parameterized variant used ONLY by the adversarial `ITERS-1` teeth test. The default
/// `cordic_sincos` always runs exactly `ITERS` rotations; this lets the test prove the digest
/// is sensitive to iteration count without altering the canonical kernel.
pub fn cordic_sincos_iters(mut z: i64, iters: u32) -> (i64, i64) {
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
    for i in 0..iters {
        let dx = x >> i;               // x * 2^-i  (arithmetic shift; cross-target defined)
        let dy = y >> i;               // y * 2^-i
        if z >= 0 { x -= dy; y += dx; z -= ATAN_Q30[i as usize]; }
        else      { x += dy; y -= dx; z += ATAN_Q30[i as usize]; }
    }
    if negate { (-x, -y) } else { (x, y) }
}

/// FNV-1a over the raw output stream — the exact digest a cross-arch CI job would compare.
pub fn digest(stream: &[i64]) -> u64 {
    let mut hsh: u64 = 0xcbf29ce484222325;
    for &v in stream {
        for b in v.to_le_bytes() {
            hsh ^= b as u64;
            hsh = hsh.wrapping_mul(0x100000001b3);
        }
    }
    hsh
}

/// The exact 51,471-sample sweep used to pin the digest (replicates doc 20 item 4):
/// two full periods, step 1/4096 rad, in Q30 — integer-only construction.
pub fn sample_stream() -> Vec<i64> {
    let n = ((2 * TWO_PI_Q30) / STEP_Q30) as usize;
    let mut stream = Vec::with_capacity(2 * n);
    let mut z = LO_Q30;
    for _ in 0..n {
        let (c, s) = cordic_sincos(z);
        stream.push(c);
        stream.push(s);
        z += STEP_Q30;
    }
    stream
}

/// The pinned digest, recomputed live from this module's output over the canonical sweep.
/// The digest test compares this to `CORDIC_SINCOS_DIGEST`.
pub fn compute_digest() -> u64 {
    digest(&sample_stream())
}

/// Same sweep, but using `iters` rotations instead of `ITERS`. Used by the teeth test to prove
/// the digest changes when the iteration count is wrong.
pub fn compute_digest_with_iters(iters: u32) -> u64 {
    let n = ((2 * TWO_PI_Q30) / STEP_Q30) as usize;
    let mut stream = Vec::with_capacity(2 * n);
    let mut z = LO_Q30;
    for _ in 0..n {
        let (c, s) = cordic_sincos_iters(z, iters);
        stream.push(c);
        stream.push(s);
        z += STEP_Q30;
    }
    digest(&stream)
}
