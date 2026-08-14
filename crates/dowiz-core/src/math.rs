//! math.rs — hand-rolled `f64` transcendental/rounding functions (zero-dep, core).
//!
//! Rust's `f64::{sqrt,sin,cos,atan2,hypot,round,floor,ceil,mul_add}` are
//! **std-only** (they link the system libm). To move the geometry modules (fft,
//! spherical, modular, trig, eigen) into the `no_std` core, these are
//! reimplemented here on `core::` primitives only.
//!
//! Accuracy targets:
//! - `sqrt` — **correctly rounded** (bit-exact vs hardware `fsqrt`). Implemented
//!   via exact integer square root (`u128`) + round-to-nearest-even, so it
//!   matches the IEEE 754 required result bit-for-bit. This is what the
//!   `householder::eig2x2_bit_capture_oracle` golden signatures depend on.
//! - `fma` — **correctly rounded** fused multiply-add (bit-exact vs hardware
//!   `fmadd`). Port of glibc's soft-float `s_fma.c` (exact `u128` product,
//!   exponent alignment, sticky-bit rounding).
//! - `hypot` — **bit-exact vs glibc** (Borges MyHypot3, FMA fast path), so it
//!   matches `f64::hypot` on aarch64 (`__FP_FAST_FMA`). This is the other half
//!   of the golden-signature guarantee.
//! - `sin`/`cos`/`atan2` — ~1 ULP (Cody–Waite/Taylor). NOT bit-exact (libm trig
//!   is not correctly rounded) — only used on non-golden (tolerance) paths.
//! - `floor`/`ceil`/`trunc`/`round` — exact bit manipulation (bit-exact).

use core::f64::consts::{FRAC_PI_2, PI};

/// `|x|` by clearing the sign bit (exact, no branch on NaN payload).
#[inline]
pub fn fabs(x: f64) -> f64 {
    f64::from_bits(x.to_bits() & 0x7fff_ffff_ffff_ffff)
}

// ── integer helpers (exact) ─────────────────────────────────────────────────

/// Floor of the integer square root of `n` (exact, Newton's method).
fn isqrt_u128(n: u128) -> u128 {
    if n < 2 {
        return n;
    }
    // Initial over-estimate: 2^(ceil(log2(n))/2 + 1).
    let mut x = 1u128 << (n.ilog2() / 2 + 1);
    loop {
        let y = (x + n / x) >> 1;
        if y >= x {
            return x;
        }
        x = y;
    }
}

const TWO63: f64 = f64::from_bits(0x43E0_0000_0000_0000); // 2^63

/// `x << n`; returns 0 when `n >= 64` (bits fully shifted out). Safe for any
/// runtime `n` (Rust's `<<` would panic/UB at `n == 64`).
#[inline]
fn shl(x: u64, n: u32) -> u64 {
    if n < 64 {
        x << n
    } else {
        0
    }
}

/// `x >> n`; returns 0 when `n >= 64`. Safe for any runtime `n`.
#[inline]
fn shr(x: u64, n: u32) -> u64 {
    if n < 64 {
        x >> n
    } else {
        0
    }
}

/// `x * 2^n`, exact for representable results (power-of-two scaling).
/// `x` must be finite; handles overflow → ±inf and underflow → subnormal/±0.
fn scalbn(x: f64, n: i32) -> f64 {
    if x == 0.0 || !x.is_finite() {
        return x;
    }
    let bits = x.to_bits();
    let sign = bits & 0x8000_0000_0000_0000;
    let frac = bits & 0x000f_ffff_ffff_ffff;
    let e = ((bits >> 52) & 0x7ff) as i64;
    let ne = e + n as i64;
    if ne >= 0x7ff {
        // overflow → ±inf
        f64::from_bits(sign | 0x7ff0_0000_0000_0000)
    } else if ne <= 0 {
        // Underflow → subnormal or ±0. The value is `mant · 2^(ne − 1075)` with
        // `mant = 2^52 + frac` the 53-bit significand; as a subnormal this is
        // `round(mant / 2^(1 − ne))` (round-to-nearest-even).
        let mant = (1u64 << 52) | frac;
        let shift = (1 - ne) as u32;
        if shift >= 64 {
            return f64::from_bits(sign); // mant / 2^shift < 2^-11 → ±0
        }
        let mut sub = mant >> shift;
        let dropped = mant & ((1u64 << shift) - 1);
        let half = 1u64 << (shift - 1);
        if dropped > half || (dropped == half && (sub & 1) == 1) {
            sub += 1;
        }
        f64::from_bits(sign | sub)
    } else {
        f64::from_bits(sign | ((ne as u64) << 52) | frac)
    }
}

// ── sqrt (correctly rounded) ───────────────────────────────────────────────

/// `sqrt(x)`, correctly rounded (bit-exact vs IEEE 754 `fsqrt`).
///
/// Algorithm: decompose `x = m·2^p` with `m` a 54-bit integer and `p` even, so
/// `sqrt(x) = sqrt(m)·2^(p/2)`. Compute `sqrt(m)` to enough guard bits via exact
/// integer square root, then round to nearest-even into the 53-bit significand.
pub fn sqrt(x: f64) -> f64 {
    if x.is_nan() || x < 0.0 {
        return f64::NAN;
    }
    if x == 0.0 {
        return x; // ±0 → ±0
    }
    if x == f64::INFINITY {
        return f64::INFINITY;
    }

    let bits = x.to_bits();
    let e = ((bits >> 52) & 0x7ff) as i64;
    let f = bits & 0x000f_ffff_ffff_ffff;

    // x = m·2^p, m integer, p integer.
    let (mut m, mut p): (u64, i64) = if e == 0 {
        (f, -1074) // subnormal: x = f·2^-1074
    } else {
        (f | (1u64 << 52), e - 1075) // normal: x = (2^52+f)·2^(e-1075)
    };

    // Normalize m up to have bit 52 set (so m ∈ [2^52, 2^53)).
    while m < (1u64 << 52) {
        m <<= 1;
        p -= 1;
    }
    // Make p even; then sqrt(x) = sqrt(m)·2^(p/2).
    if p & 1 != 0 {
        m <<= 1; // m ∈ [2^52, 2^54)
        p -= 1;
    }
    let pe = p / 2;

    // sqrt(m) ∈ [2^26, 2^27). q = floor(sqrt(m)·2^28) via exact integer sqrt of
    // m·2^56; q has 55 bits. rem > 0 ⇒ true sqrt(m) is strictly above q·2^-28.
    let m_scaled = (m as u128) << 56;
    let q = isqrt_u128(m_scaled);
    let rem = m_scaled - q * q;

    // Round q (55 bits) to 53 significant bits (drop the low 2 bits).
    let q_hi = (q >> 2) as u64; // top 53 bits
    let dropped = (q & 3) as u64; // low 2 bits
    let mut sig = q_hi;
    if dropped > 2 || (dropped == 2 && (rem > 0 || q_hi & 1 == 1)) {
        sig += 1;
    }

    // sig ∈ [2^52, 2^53]; on overflow to 2^53, renormalize.
    let (sig, exp_adj) = if sig >> 52 == 1 {
        (sig, 0i64)
    } else {
        (sig >> 1, 1i64)
    };

    // sqrt(m) = sig · 2^(exp_adj − 26): sig is a 53-bit significand (value
    // sig·2^−52), and m ∈ [2^52, 2^54) ⇒ sqrt(m) ∈ [2^26, 2^27), so the result
    // significand carries an exponent of 26 (52/2). Then sqrt(x) = sqrt(m)·2^pe
    // = sig · 2^(pe + exp_adj − 26).
    scalbn(sig as f64, (pe + exp_adj - 26) as i32)
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
    let t = trunc(x);
    let frac = fabs(x - t);
    if frac >= 0.5 {
        if x > 0.0 {
            t + 1.0
        } else {
            t - 1.0
        }
    } else {
        t
    }
}

// ── fma (correctly rounded fused multiply-add) ─────────────────────────────
// Port of glibc `sysdeps/ieee754/dbl-64/s_fma.c` (soft-float path). Value is
// represented as `m·2^e` with `m` a 54-bit integer; the exact product is a
// 128-bit integer; operands are exponent-aligned and summed exactly, then the
// 63-bit head (with sticky) is rounded into a double.

struct Num {
    m: u64,
    e: i32,
    sign: u64, // 0 or 0x800
}

fn normalize(x: f64) -> Num {
    let mut ix = x.to_bits();
    let top = (ix >> 52) as i32;
    let sign = (top & 0x800) as u64;
    let mut e = top & 0x7ff;
    if e == 0 {
        // subnormal: scale by 2^63 to reveal the exponent
        ix = (x * TWO63).to_bits();
        e = ((ix >> 52) & 0x7ff) as i32;
        e = if e != 0 { e - 63 } else { 0x800 };
    }
    let mut m = ix & 0x000f_ffff_ffff_ffff;
    m |= 1u64 << 52;
    m <<= 1;
    e -= 1023 + 52 + 1; // -= 1076
    Num { m, e, sign }
}

/// `x·y + z` with a single rounding (correctly rounded fused multiply-add).
pub fn fma(x: f64, y: f64, z: f64) -> f64 {
    const ZEROINFNAN: i32 = 0x7ff - 1023 - 52 - 1; // = 971

    let nx = normalize(x);
    let ny = normalize(y);
    let nz = normalize(z);

    if nx.e >= ZEROINFNAN || ny.e >= ZEROINFNAN {
        return x * y + z;
    }
    if nz.e >= ZEROINFNAN {
        if nz.e > ZEROINFNAN {
            return x * y; // z == 0
        } else if z.is_nan() {
            return f64::NAN;
        }
        return z;
    }

    // r = x·y (128-bit exact product; each operand is ≤ 54 bits).
    let prod = (nx.m as u128) * (ny.m as u128);
    let mut rhi = (prod >> 64) as u64;
    let mut rlo = prod as u64;

    // Align exponents.
    let mut e = nx.e + ny.e;
    let mut d = nz.e - e;
    let zhi: u64;
    let zlo: u64;
    if d > 0 {
        if d < 64 {
            zlo = nz.m << d;
            zhi = nz.m >> (64 - d);
        } else {
            zlo = 0;
            zhi = nz.m;
            e = nz.e - 64;
            d -= 64;
            if d < 64 {
                let sa = (64 - d) as u32; // ∈ [1, 64]; 64 when d == 0
                rlo = shl(rhi, sa) | (rlo >> d) | (shl(rlo, sa) != 0) as u64;
                rhi >>= d;
            } else {
                rlo = 1;
                rhi = 0;
            }
        }
    } else {
        zhi = 0;
        d = -d;
        if d == 0 {
            zlo = nz.m;
        } else if d < 64 {
            zlo = (nz.m >> d) | ((nz.m << (64 - d)) != 0) as u64;
        } else {
            zlo = 1;
        }
    }

    // Add (r += z) or subtract (r -= z) depending on signs.
    let mut sign = nx.sign ^ ny.sign != 0;
    let samesign = !(sign ^ (nz.sign != 0));
    let mut nonzero = true;
    if samesign {
        let (lo, carry) = rlo.overflowing_add(zlo);
        rlo = lo;
        rhi = rhi.wrapping_add(zhi.wrapping_add(carry as u64));
    } else {
        let t = rlo;
        rlo = rlo.wrapping_sub(zlo);
        let borrow = (t < rlo) as u64;
        rhi = rhi.wrapping_sub(zhi.wrapping_add(borrow));
        if rhi >> 63 != 0 {
            rlo = rlo.wrapping_neg();
            rhi = rhi.wrapping_neg().wrapping_sub((rlo != 0) as u64);
            sign = !sign;
        }
        nonzero = rhi != 0;
    }

    // Normalize the head to the top 63 bits (last bit is sticky).
    let dnorm: i32;
    if nonzero {
        e += 64;
        dnorm = rhi.leading_zeros() as i32 - 1;
        rhi = (rhi << dnorm) | shr(rlo, (64 - dnorm) as u32) | ((rlo << dnorm) != 0) as u64;
    } else if rlo != 0 {
        dnorm = rlo.leading_zeros() as i32 - 1;
        if dnorm < 0 {
            rhi = (rlo >> 1) | (rlo & 1);
        } else {
            rhi = rlo << dnorm;
        }
    } else {
        return x * y + z; // exact ±0
    }
    e -= dnorm;

    let mut i = rhi as i64;
    if sign {
        i = -i;
    }
    let mut r = i as f64; // |r| ∈ [2^62, 2^63)

    if e < -1022 - 62 {
        // Result is subnormal before rounding.
        if e == -1022 - 63 {
            let c = if sign { -TWO63 } else { TWO63 };
            if r == c {
                // Min normal after rounding.
                return if sign { -f64::MIN_POSITIVE } else { f64::MIN_POSITIVE };
            }
            // One bit is lost when scaled; round once at conversion.
            if rhi << 53 != 0 {
                let mut i2 = ((rhi >> 1) | (rhi & 1) | (1u64 << 62)) as i64;
                if sign {
                    i2 = -i2;
                }
                r = i2 as f64;
                r = 2.0 * r - c;
            }
        } else {
            let dn = 10;
            let mut i3 = (((rhi >> dn) | ((rhi << (64 - dn)) != 0) as u64) << dn) as i64;
            if sign {
                i3 = -i3;
            }
            r = i3 as f64;
        }
    }
    scalbn(r, e)
}

/// `hypot(x, y) = sqrt(x² + y²)`, bit-exact vs glibc (`__FP_FAST_FMA` path).
///
/// Port of glibc `sysdeps/ieee754/dbl-64/e_hypot.c` (Borges MyHypot3, FMA fast
/// path). Uses correctly-rounded `fma` + `sqrt`, matching aarch64 glibc exactly.
pub fn hypot(x: f64, y: f64) -> f64 {
    const SCALE: f64 = f64::from_bits(0x1A70_0000_0000_0000); // 2^-600
    const LARGE_VAL: f64 = f64::from_bits(0x5FE0_0000_0000_0000); // 2^511
    const TINY_VAL: f64 = f64::from_bits(0x2340_0000_0000_0000); // 2^-459
    const EPS: f64 = f64::from_bits(0x3C90_0000_0000_0000); // 2^-54

    if !x.is_finite() || !y.is_finite() {
        if x.is_infinite() || y.is_infinite() {
            return f64::INFINITY;
        }
        return x + y;
    }

    let x = fabs(x);
    let y = fabs(y);
    let ax = if x < y { y } else { x };
    let ay = if x < y { x } else { y };

    // ax is huge → scale both down.
    if ax > LARGE_VAL {
        if ay <= ax * EPS {
            return ax + ay;
        }
        return hypot_kernel(ax * SCALE, ay * SCALE) / SCALE;
    }

    // ay is tiny → scale both up.
    if ay < TINY_VAL {
        if ax >= ay / EPS {
            return ax + ay;
        }
        return hypot_kernel(ax / SCALE, ay / SCALE) * SCALE;
    }

    // Common case.
    if ay <= ax * EPS {
        return ax + ay;
    }
    hypot_kernel(ax, ay)
}

/// The hypot kernel. Inputs: `ax >= ay >= 0`, and squaring ax, ay, (ax−ay)
/// does not overflow or underflow. `__FP_FAST_FMA` fast path (aarch64).
#[inline]
fn hypot_kernel(ax: f64, ay: f64) -> f64 {
    let t1 = ay + ay;
    let t2 = ax - ay;
    if t1 >= ax {
        sqrt(fma(t1, ax, t2 * t2))
    } else {
        sqrt(fma(ax, ax, ay * ay))
    }
}

// ── sin / cos ───────────────────────────────────────────────────────────────

/// π/2 split into high + low parts (Cody–Waite) for accurate range reduction.
#[allow(clippy::approx_constant)] // deliberate high-precision split of π/2
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
    use core::f64::consts::TAU;

    fn close(a: f64, b: f64, tol: f64) {
        let d = fabs(a - b);
        assert!(d < tol, "mismatch: {a} vs {b} (diff {d})");
    }

    #[test]
    fn sqrt_matches_std() {
        for x in [0.0, 1.0, 2.0, 4.0, 0.5, 1e-8, 1e8, 12345.6789, PI] {
            assert_eq!(sqrt(x).to_bits(), x.sqrt().to_bits(), "sqrt({x})");
        }
    }

    #[test]
    fn fma_matches_std() {
        for (a, b, c) in [
            (3.0, 4.0, 5.0),
            (1e200, 1e200, -1e200),
            (0.1, 0.2, 0.3),
            (1.5, -2.5, 3.5),
            (1e-200, 1e-200, 1e-300),
        ] {
            assert_eq!(fma(a, b, c).to_bits(), a.mul_add(b, c).to_bits(), "fma({a},{b},{c})");
        }
    }

    #[test]
    fn hypot_matches_std() {
        for (x, y) in [(3.0, 4.0), (1.0, 1.0), (1e-8, 1e-8), (1e8, 1e8)] {
            assert_eq!(hypot(x, y).to_bits(), x.hypot(y).to_bits(), "hypot({x},{y})");
        }
        assert_eq!(hypot(3.0, 4.0).to_bits(), 5.0f64.to_bits());
    }

    #[test]
    fn sin_cos_match_std() {
        for x in [-10.0, -3.0, -1.0, 0.0, 0.5, 1.0, 3.0, TAU, 10.0, 100.0] {
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
    fn sin_cos_identities() {
        close(sin(PI), 0.0, 1e-15);
        close(cos(PI), -1.0, 1e-15);
        close(sin(FRAC_PI_2), 1.0, 1e-15);
        close(cos(FRAC_PI_2), 0.0, 1e-15);
        for x in [0.1, 0.7, 1.3, 2.9] {
            let s = sin(x);
            let c = cos(x);
            close(s * s + c * c, 1.0, 1e-12);
        }
    }
}
