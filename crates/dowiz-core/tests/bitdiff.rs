//! Differential bit-exactness harness: compares dowiz-core hand-rolled math
//! against the host libm (std) at the BIT level (to_bits). This is the probe
//! for the eig2x2_bit_capture_oracle blocker — golden signatures are bit-exact.
//!
//! Run: `cargo test -p dowiz-core --test bitdiff -- --nocapture`

use dowiz_core::math::{atan2, cos, fma, floor, hypot, round, sin, sqrt};

/// Deterministic SplitMix64 (no external dep).
struct SplitMix64(u64);
impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform finite f64 over the FULL bit range (log-uniform magnitude,
    /// includes subnormals and near-max; rejects NaN/Inf).
    fn next_f64(&mut self) -> f64 {
        loop {
            let b = self.next_u64();
            let exp = (b >> 52) & 0x7FF;
            if exp == 0x7FF {
                continue;
            }
            return f64::from_bits(b);
        }
    }
    /// Uniform f64 in [0, 1) (dense mantissa coverage near zero).
    fn next_unit(&mut self) -> f64 {
        f64::from_bits((self.next_u64() >> 12) | 0x3FF0_0000_0000_0000) - 1.0
    }
}

fn report(name: &str, total: usize, mismatches: usize) {
    println!("{name:>8}: {mismatches}/{total} bit-mismatches");
}

#[test]
fn sqrt_bit_exact() {
    let mut rng = SplitMix64(0x1234_5678_9abc_def0);
    let mut total = 0usize;
    let mut mismatches = 0usize;
    let mut first = true;
    for _ in 0..4_000_000 {
        let x = if rng.next_u64() & 1 == 0 {
            rng.next_f64() // full range (log-uniform)
        } else {
            rng.next_unit() * 1e300 // dense near zero but wide
        };
        if !x.is_finite() {
            continue;
        }
        total += 1;
        let mine = sqrt(x);
        let stdv = x.sqrt();
        if mine.to_bits() != stdv.to_bits() {
            mismatches += 1;
            if first {
                println!(
                    "  sqrt mismatch: x={x:e} mine={mine:e} std={stdv:e} (bits {:#018x} vs {:#018x})",
                    mine.to_bits(),
                    stdv.to_bits()
                );
                first = false;
            }
        }
    }
    report("sqrt", total, mismatches);
    assert_eq!(mismatches, 0, "sqrt must be correctly rounded (bit-exact vs libm)");
}

#[test]
fn hypot_bit_exact() {
    let mut rng = SplitMix64(0xfeed_face_cafe_beef);
    let mut total = 0usize;
    let mut mismatches = 0usize;
    let mut first = true;
    for _ in 0..4_000_000 {
        let a = rng.next_f64();
        let b = rng.next_f64();
        if !a.is_finite() || !b.is_finite() {
            continue;
        }
        total += 1;
        let mine = hypot(a, b);
        let stdv = a.hypot(b);
        if mine.to_bits() != stdv.to_bits() {
            mismatches += 1;
            if first {
                println!(
                    "  hypot mismatch: a={a:e} b={b:e} mine={mine:e} std={stdv:e} (bits {:#018x} vs {:#018x})",
                    mine.to_bits(),
                    stdv.to_bits()
                );
                first = false;
            }
        }
    }
    report("hypot", total, mismatches);
    assert_eq!(mismatches, 0, "hypot must be bit-exact vs libm");
}

#[test]
fn fma_bit_exact() {
    let mut rng = SplitMix64(0x0bad_c0de_5eed_1234);
    let mut total = 0usize;
    let mut mismatches = 0usize;
    let mut first = true;
    for _ in 0..4_000_000 {
        let a = rng.next_f64();
        let b = rng.next_f64();
        let c = rng.next_f64();
        if !a.is_finite() || !b.is_finite() || !c.is_finite() {
            continue;
        }
        total += 1;
        let mine = fma(a, b, c);
        let stdv = a.mul_add(b, c);
        if mine.to_bits() != stdv.to_bits() {
            mismatches += 1;
            if first {
                println!(
                    "  fma mismatch: a={a:e} b={b:e} c={c:e} mine={mine:e} std={stdv:e} (bits {:#018x} vs {:#018x})",
                    mine.to_bits(),
                    stdv.to_bits()
                );
                first = false;
            }
        }
    }
    report("fma", total, mismatches);
    assert_eq!(mismatches, 0, "fma must be correctly rounded (bit-exact vs hardware fmadd)");
}

#[test]
fn trig_bit_exact_report() {
    // Informational only — NOT asserted: libm sin/cos/atan2 are NOT correctly
    // rounded, so bit-exactness is not achievable by any correct reimplementation
    // unless we replicate glibc's exact polynomial. Report the rate for the record.
    let mut rng = SplitMix64(0xdead_beef_0123_4567);
    let (mut st, mut sm, mut ct, mut cm, mut at, mut am, mut rt, mut rm, mut ft, mut fm) =
        (0usize, 0usize, 0usize, 0usize, 0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
    for _ in 0..1_000_000 {
        let x = (rng.next_unit() - 0.5) * 1e6;
        let y = (rng.next_unit() - 0.5) * 1e6;
        st += 1;
        if sin(x).to_bits() != x.sin().to_bits() {
            sm += 1;
        }
        ct += 1;
        if cos(x).to_bits() != x.cos().to_bits() {
            cm += 1;
        }
        at += 1;
        if atan2(y, x).to_bits() != y.atan2(x).to_bits() {
            am += 1;
        }
        rt += 1;
        if round(x).to_bits() != x.round().to_bits() {
            rm += 1;
        }
        ft += 1;
        if floor(x).to_bits() != x.floor().to_bits() {
            fm += 1;
        }
    }
    report("sin", st, sm);
    report("cos", ct, cm);
    report("atan2", at, am);
    report("round", rt, rm);
    report("floor", ft, fm);
}
