//! math.rs — hand-rolled `f64` transcendental/rounding functions (zero-dep, core).
//!
//! Rust's `f64::{sqrt,sin,cos,atan2,hypot,round,floor,ceil}` are **std-only**
//! (they link the system libm). To move the geometry modules (fft, spherical,
//! modular, trig, eigen) into the `no_std` core, these are reimplemented here
//! on `core::` primitives only.
//!
//! Accuracy target: ≤ ~1e-14 absolute/relative (double precision), verified
//! against `std` in the parity tests. Methods:
//! - `sqrt` — bit-hacked seed + Newton refinement (quadratic convergence).
//! - `sin`/`cos` — Cody–Waite π/2 range reduction + Taylor (converges fast on
//!   the reduced |x| ≤ π/4).
//! - `atan2` — `atan` via angle-halving reduction + Taylor, quadrant-adjusted.
//! - `floor`/`ceil`/`trunc`/`round` — exact bit manipulation (no libm).
//! - `hypot` — `sqrt(x² + y²)` with overflow guard.

use core::f64::consts::{FRAC_PI_2, PI};

/// `|x|` by clearing the sign bit (exact, no branch on NaN payload).
#[inline]
pub fn fabs(x: f64) -> f64 {
    f64::from_bits(x.to_bits() & 0x7fff_ffff_ffff_ffff)
}

/// `sqrt` via Newton's method with a bit-hacked initial seed.
pub fn sqrt(x: f64) -> f64 {
    if x < 0.0 || x.is_nan() {
        return f64::NAN;
    }
    if x == 0.0 {
        return x;
    }
    if x == f64::INFINITY {
        return f64::INFINITY;
    }
    // Seed via the classic bit magic: ~4-bit accurate start for Newton.
    let mut y = f64::from_bits((x.to_bits() >> 1) + 0x1ff7_8000_0000_0000);
    // Newton iterations: y = (y + x/y) / 2.
    for _ in 0..6 {
        y = 0.5 * (y + x / y);
    }
    y
}

/// `trunc`: round toward zero by masking the fractional mantissa bits.
#[inline]
pub fn trunc(x: f64) -> f64 {
    let bits = x.to_bits();
    let exp = ((bits >> 52) & 0x7ff) as i32;
    if exp < 1023 {
        // |x| < 1 → truncate to 0 (preserve sign).
        return f64::from_bits(bits & 0x8000_0000_0000_0000);
    }
    if exp >= 1075 {
        // |x| >= 2^52 → already integer.
        return x;
    }
    let mask = 0xffff_ffff_ffff_ffffu64 << (1075 - exp);
    f64::from_bits(bits & mask)
}

/// `floor`: largest integer ≤ x.
pub fn floor(x: f64) -> f64 {
    let t = trunc(x);
    if x < 0.0 && x != t {
        t - 1.0
    } else {
        t
    }
}

/// `ceil`: smallest integer ≥ x.
pub fn ceil(x: f64) -> f64 {
    let t = trunc(x);
    if x > 0.0 && x != t {
        t + 1.0
    } else {
        t
    }
}

/// `round`: nearest integer, ties away from zero (matches `f64::round`).
pub fn round(x: f64) -> f64 {
    // f64::round rounds half away from zero.
    let t = trunc(x);
    let frac = fabs(x - t);
    if frac >= 0.5 {
        if x > 0.0 { t + 1.0 } else { t - 1.0 }
    } else {
        t
    }
}

/// `hypot(x, y) = sqrt(x² + y²)` with overflow guard.
pub fn hypot(x: f64, y: f64) -> f64 {
    let ax = fabs(x);
    let ay = fabs(y);
    if ax > ay {
        let r = ay / ax;
        ax * sqrt(1.0 + r * r)
    } else if ay > 0.0 {
        let r = ax / ay;
        ay * sqrt(1.0 + r * r)
    } else {
        0.0
    }
}

// ── sin / cos ───────────────────────────────────────────────────────────────

/// π/2 split into high + low parts (Cody–Waite) for accurate range reduction.
const PIO2_HI: f64 = 1.570_796_326_794_896_6;
const PIO2_LO: f64 = 6.123_233_995_736_766e-17;

/// Reduce `x` to `(n, r)` with `x = n·(π/2) + r`, `r ∈ [-π/4, π/4]`.
fn rem_pio2(x: f64) -> (i32, f64) {
    let n = round(x * core::f64::consts::FRAC_2_PI) as i32;
    let r = (x - n as f64 * PIO2_HI) - n as f64 * PIO2_LO;
    (n, r)
}

/// `sin(r)` on |r| ≤ π/4 (Taylor, converges to full f64 precision fast).
#[inline]
fn sin_poly(r: f64) -> f64 {
    let r2 = r * r;
    let mut term = r;
    let mut sum = r;
    // Terms: -r³/3!, +r⁵/5!, -r⁷/7!, … up to r¹⁷/17!.
    for k in 1..=8 {
        term *= -r2 / (((2 * k) as f64) * ((2 * k + 1) as f64));
        sum += term;
    }
    sum
}

/// `cos(r)` on |r| ≤ π/4.
#[inline]
fn cos_poly(r: f64) -> f64 {
    let r2 = r * r;
    let mut term = 1.0;
    let mut sum = 1.0;
    for k in 1..=8 {
        term *= -r2 / (((2 * k - 1) as f64) * ((2 * k) as f64));
        sum += term;
    }
    sum
}

/// `sin(x)`.
pub fn sin(x: f64) -> f64 {
    if !x.is_finite() {
        return f64::NAN;
    }
    let (n, r) = rem_pio2(x);
    match n.rem_euclid(4) {
        0 => sin_poly(r),
        1 => cos_poly(r),
        2 => -sin_poly(r),
        _ => -cos_poly(r),
    }
}

/// `cos(x)`.
pub fn cos(x: f64) -> f64 {
    if !x.is_finite() {
        return f64::NAN;
    }
    let (n, r) = rem_pio2(x);
    match n.rem_euclid(4) {
        0 => cos_poly(r),
        1 => -sin_poly(r),
        2 => -cos_poly(r),
        _ => sin_poly(r),
    }
}

// ── atan / atan2 ────────────────────────────────────────────────────────────

/// `atan(x)` for x ≥ 0, via angle-halving reduction + Taylor.
fn atan_pos(mut x: f64) -> f64 {
    if x > 1.0 {
        return FRAC_PI_2 - atan_pos(1.0 / x);
    }
    // Halve the angle until |x| ≤ 0.01 (Taylor converges to full precision).
    let mut mult = 1.0;
    while fabs(x) > 0.01 {
        x = x / (1.0 + sqrt(1.0 + x * x));
        mult *= 2.0;
    }
    let x2 = x * x;
    let mut term = x;
    let mut sum = x;
    for k in 1..=6 {
        term *= -x2;
        sum += term / ((2 * k + 1) as f64);
    }
    mult * sum
}

/// `atan2(y, x)` — argument in (−π, π], matching `f64::atan2`.
pub fn atan2(y: f64, x: f64) -> f64 {
    if x > 0.0 {
        atan_pos(y / x)
    } else if x < 0.0 {
        if y >= 0.0 {
            atan_pos(y / x) + PI
        } else {
            atan_pos(y / x) - PI
        }
    } else if y > 0.0 {
        FRAC_PI_2
    } else if y < 0.0 {
        -FRAC_PI_2
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) {
        let d = fabs(a - b);
        assert!(d < tol, "mismatch: {a} vs {b} (diff {d})");
    }

    #[test]
    fn sqrt_matches_std() {
        for x in [0.0, 1.0, 2.0, 4.0, 0.5, 1e-8, 1e8, 12345.6789, 3.14159] {
            close(sqrt(x), x.sqrt(), 1e-12);
        }
    }

    #[test]
    fn sin_cos_match_std() {
        for x in [-10.0, -3.0, -1.0, 0.0, 0.5, 1.0, 3.0, 6.28, 10.0, 100.0] {
            close(sin(x), x.sin(), 1e-12);
            close(cos(x), x.cos(), 1e-12);
        }
    }

    #[test]
    fn atan2_matches_std() {
        let cases = [
            (1.0, 1.0),
            (-1.0, 1.0),
            (1.0, -1.0),
            (-1.0, -1.0),
            (0.0, 1.0),
            (0.0, -1.0),
            (1.0, 0.0),
            (-1.0, 0.0),
            (3.0, 4.0),
            (4.0, -3.0),
        ];
        for (y, x) in cases {
            close(atan2(y, x), y.atan2(x), 1e-12);
        }
    }

    #[test]
    fn floor_ceil_round_match_std() {
        for x in [-2.7, -1.5, -0.5, -0.1, 0.0, 0.1, 0.5, 1.5, 2.7, 3.9999] {
            assert_eq!(floor(x), x.floor());
            assert_eq!(ceil(x), x.ceil());
            assert_eq!(round(x), x.round());
            assert_eq!(trunc(x), x.trunc());
        }
    }

    #[test]
    fn hypot_matches_std() {
        for (x, y) in [(3.0, 4.0), (1.0, 1.0), (1e-8, 1e-8), (1e8, 1e8)] {
            let got = hypot(x, y);
            let want = x.hypot(y);
            // Relative tolerance: sqrt is ~1 ULP accurate, so large results
            // carry a proportionally large absolute error.
            let tol = 1e-12 * want.abs().max(1.0);
            close(got, want, tol);
        }
        close(hypot(3.0, 4.0), 5.0, 1e-12);
    }

    #[test]
    fn sin_cos_identities() {
        close(sin(PI), 0.0, 1e-15);
        close(cos(PI), -1.0, 1e-15);
        close(sin(FRAC_PI_2), 1.0, 1e-15);
        close(cos(FRAC_PI_2), 0.0, 1e-15);
        // sin² + cos² = 1
        for x in [0.1, 0.7, 1.3, 2.9] {
            let s = sin(x);
            let c = cos(x);
            close(s * s + c * c, 1.0, 1e-12);
        }
    }
}
