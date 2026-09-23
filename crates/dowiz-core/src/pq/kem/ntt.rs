//! pq/kem/ntt — the FIPS 203 §4.3 number-theoretic transform for ML-KEM.
//!
//! ML-KEM's public key is `ByteEncode12(t̂)` where t̂ lives in the NTT domain, so
//! a schoolbook ring multiply can never reproduce the standard's bytes: the
//! transform is part of the wire format, not an optimisation. This file is a
//! direct transcription of Algorithms 9 (NTT), 10 (NTT⁻¹), 11 (MultiplyNTTs)
//! and 12 (BaseCaseMultiply), with ζ = 17 (a primitive 256th root of unity
//! mod q = 3329) and the two constant tables of §4.3 / Appendix A computed at
//! compile time from that one number rather than pasted.
//!
//! Coefficients are kept in [0, q) throughout; every product is at most
//! (q-1)² < 2^24, so i32/i64 arithmetic never overflows.

use super::{N, Q};

/// BitRev7(i): reverse the low 7 bits of i (FIPS 203 §2.3).
const fn bitrev7(i: usize) -> usize {
    let mut r = 0;
    let mut b = 0;
    while b < 7 {
        r |= ((i >> b) & 1) << (6 - b);
        b += 1;
    }
    r
}

const fn pow_mod(base: i64, mut e: usize) -> i32 {
    let q = Q as i64;
    let mut acc = 1i64;
    let mut b = base % q;
    while e > 0 {
        if e & 1 == 1 {
            acc = acc * b % q;
        }
        b = b * b % q;
        e >>= 1;
    }
    acc as i32
}

/// ZETAS[i] = ζ^BitRev7(i) mod q, i ∈ [0, 128) — the NTT twiddles (Appendix A).
pub(super) const ZETAS: [i32; 128] = {
    let mut t = [0i32; 128];
    let mut i = 0;
    while i < 128 {
        t[i] = pow_mod(17, bitrev7(i));
        i += 1;
    }
    t
};

/// GAMMAS[i] = ζ^(2·BitRev7(i)+1) mod q — the BaseCaseMultiply moduli (Appendix A).
pub(super) const GAMMAS: [i32; 128] = {
    let mut t = [0i32; 128];
    let mut i = 0;
    while i < 128 {
        t[i] = pow_mod(17, 2 * bitrev7(i) + 1);
        i += 1;
    }
    t
};

#[inline]
fn mulq(a: i32, b: i32) -> i32 {
    ((a as i64 * b as i64) % Q as i64) as i32
}

#[inline]
fn addq(a: i32, b: i32) -> i32 {
    let s = a + b;
    if s >= Q {
        s - Q
    } else {
        s
    }
}

#[inline]
fn subq(a: i32, b: i32) -> i32 {
    let s = a - b;
    if s < 0 {
        s + Q
    } else {
        s
    }
}

/// Algorithm 9: NTT(f) → f̂. Input and output coefficients in [0, q).
pub(super) fn ntt(f: &[i32; N]) -> [i32; N] {
    let mut f = *f;
    let mut i = 1;
    let mut len = 128;
    while len >= 2 {
        let mut start = 0;
        while start < N {
            let zeta = ZETAS[i];
            i += 1;
            for j in start..start + len {
                let t = mulq(zeta, f[j + len]);
                f[j + len] = subq(f[j], t);
                f[j] = addq(f[j], t);
            }
            start += 2 * len;
        }
        len /= 2;
    }
    f
}

/// Algorithm 10: NTT⁻¹(f̂) → f, ending with the multiply by 3303 = 128⁻¹ mod q.
pub(super) fn ntt_inv(f: &[i32; N]) -> [i32; N] {
    let mut f = *f;
    let mut i = 127;
    let mut len = 2;
    while len <= 128 {
        let mut start = 0;
        while start < N {
            let zeta = ZETAS[i];
            i -= 1;
            for j in start..start + len {
                let t = f[j];
                f[j] = addq(t, f[j + len]);
                f[j + len] = mulq(zeta, subq(f[j + len], t));
            }
            start += 2 * len;
        }
        len *= 2;
    }
    for x in f.iter_mut() {
        *x = mulq(*x, 3303);
    }
    f
}

/// Algorithm 11 (+ Algorithm 12 inlined): the product of two NTT-domain
/// polynomials, as 128 degree-one products modulo (X² − γ_i).
pub(super) fn multiply_ntts(f: &[i32; N], g: &[i32; N]) -> [i32; N] {
    let mut h = [0i32; N];
    for i in 0..128 {
        let (a0, a1) = (f[2 * i], f[2 * i + 1]);
        let (b0, b1) = (g[2 * i], g[2 * i + 1]);
        h[2 * i] = addq(mulq(a0, b0), mulq(mulq(a1, b1), GAMMAS[i]));
        h[2 * i + 1] = addq(mulq(a0, b1), mulq(a1, b0));
    }
    h
}

/// h + g coefficient-wise in [0, q).
pub(super) fn poly_add(a: &[i32; N], b: &[i32; N]) -> [i32; N] {
    let mut r = [0i32; N];
    for i in 0..N {
        r[i] = addq(a[i], b[i]);
    }
    r
}

/// h − g coefficient-wise in [0, q).
pub(super) fn poly_sub(a: &[i32; N], b: &[i32; N]) -> [i32; N] {
    let mut r = [0i32; N];
    for i in 0..N {
        r[i] = subq(a[i], b[i]);
    }
    r
}
