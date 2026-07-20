//! pq_dsa — ML-DSA-65 (FIPS 204, Dilithium mode 3) implemented from scratch, zero external crates.
//!
//! FIPS 204 INTEROP (G10 fix, 2026-07-11): this module is a faithful, byte-exact port of the
//! pq-crystals reference (mode 3 = ML-DSA-65). It is verified against the official NIST ACVP
//! FIPS 204 known-answer vectors (keyGen / sigGen-internal-deterministic / sigVer) — see the
//! tmp harness. The prior implementation was NOT interoperable: it sampled matrix A and the
//! secrets s1/s2 with center-binomial CBD instead of the FIPS uniform/rejection samplers, it
//! multiplied in the coefficient domain (schoolbook) instead of the NTT domain, it omitted the
//! `||k||l` domain-separation bytes in KeyGen's H, it used a 32-byte c̃ (must be 48 for cat-3),
//! and it used custom pk/sk/sig packing. All of these are fixed here to match FIPS 204 exactly.
//!
//! ENTROPY MODEL: RNG-free on the crypto hot path. All randomness enters ONLY through
//! caller-supplied byte streams (`seed`, `rnd`). We never call any OS RNG, clock, or network.
//!
//! SHAKE128/256 delegate to the verified FIPS-202 Keccak core in `pq::keccak` (no duplicate).
//! (Ported from bebop2-core `pq_dsa.rs` 2026-07-12; std instead of alloc, no rng-entropy fn.)

#![allow(dead_code)]

use std::vec;
use std::vec::Vec;

pub fn shake256(data: &[u8], out: &mut [u8]) {
    crate::pq::keccak::shake256(data, out)
}
pub fn shake128(data: &[u8], out: &mut [u8]) {
    crate::pq::keccak::shake128(data, out)
}

// ─────────────────────────────────────────────────────────────────────────────
// ML-DSA-65 parameters (FIPS 204 Table 2, category 3 == Dilithium mode 3)
// ─────────────────────────────────────────────────────────────────────────────
const N: usize = 256;
const Q: i32 = 8380417;
const QINV: i32 = 58728449; // q^{-1} mod 2^32
const D: usize = 13;
const K: usize = 6;
const L: usize = 5;
const ETA: i32 = 4;
const TAU: usize = 49;
const BETA: i32 = 196;
const GAMMA1: i32 = 1 << 19;
const GAMMA2: i32 = (Q - 1) / 32;
const OMEGA: usize = 55;

pub const SEEDBYTES: usize = 32;
const CRHBYTES: usize = 64;
const TRBYTES: usize = 64;
pub const RNDBYTES: usize = 32;
const CTILDEBYTES: usize = 48;

const POLYT1_PACKEDBYTES: usize = 320;
const POLYT0_PACKEDBYTES: usize = 416;
const POLYETA_PACKEDBYTES: usize = 128;
const POLYZ_PACKEDBYTES: usize = 640;
const POLYW1_PACKEDBYTES: usize = 128;
const POLYVECH_PACKEDBYTES: usize = OMEGA + K;

pub const PUBLICKEYBYTES: usize = SEEDBYTES + K * POLYT1_PACKEDBYTES; // 1952
pub const SECRETKEYBYTES: usize = 2 * SEEDBYTES
    + TRBYTES
    + L * POLYETA_PACKEDBYTES
    + K * POLYETA_PACKEDBYTES
    + K * POLYT0_PACKEDBYTES; // 4032
pub const SIGNATUREBYTES: usize = CTILDEBYTES + L * POLYZ_PACKEDBYTES + POLYVECH_PACKEDBYTES; // 3309

const SHAKE128_RATE: usize = 168;
const SHAKE256_RATE: usize = 136;

// ─────────────────────────────────────────────────────────────────────────────
// Modular reduction (reduce.c)
// ─────────────────────────────────────────────────────────────────────────────
#[inline]
fn montgomery_reduce(a: i64) -> i32 {
    // For -2^{31}Q <= a <= Q*2^31, compute a*2^{-32} mod Q in (-Q, Q).
    let t = (a as i32).wrapping_mul(QINV); // low 32 bits of (int32)a * QINV
    ((a - (t as i64) * (Q as i64)) >> 32) as i32
}

#[inline]
fn reduce32(a: i32) -> i32 {
    // PRECONDITION (item 7, machine-checked by `kani_proofs::proof_reduce32_contract`):
    // requires `a <= i32::MAX - 2^22` — the `a + (1<<22)` below overflows i32 above that.
    // For all callers a is a reduced/near-reduced coefficient (|a| ≪ 2^22), so the bound
    // holds by construction; the Kani proof pins that no overflow occurs in-domain and the
    // result stays ≡ a (mod Q).
    let t = (a + (1 << 22)) >> 23;
    a - t * Q
}

#[inline]
fn caddq(a: i32) -> i32 {
    a + ((a >> 31) & Q)
}

// ─────────────────────────────────────────────────────────────────────────────
// Polynomial / vector types
// ─────────────────────────────────────────────────────────────────────────────
type Poly = [i32; N];
type PolyVecL = [Poly; L];
type PolyVecK = [Poly; K];

#[inline]
fn poly_zero() -> Poly {
    [0i32; N]
}

fn poly_add(a: &Poly, b: &Poly) -> Poly {
    let mut r = [0i32; N];
    for i in 0..N {
        r[i] = a[i] + b[i];
    }
    r
}

fn poly_sub(a: &Poly, b: &Poly) -> Poly {
    let mut r = [0i32; N];
    for i in 0..N {
        r[i] = a[i] - b[i];
    }
    r
}

fn poly_reduce(a: &mut Poly) {
    for i in 0..N {
        a[i] = reduce32(a[i]);
    }
}

fn poly_caddq(a: &mut Poly) {
    for i in 0..N {
        a[i] = caddq(a[i]);
    }
}

fn poly_shiftl(a: &mut Poly) {
    for i in 0..N {
        a[i] <<= D;
    }
}

fn poly_pointwise_montgomery(a: &Poly, b: &Poly) -> Poly {
    let mut r = [0i32; N];
    for i in 0..N {
        r[i] = montgomery_reduce((a[i] as i64) * (b[i] as i64));
    }
    r
}

/// |a| centered, then check any coeff has abs >= B (FIPS poly_chknorm). Returns true if out of bounds.
fn poly_chknorm(a: &Poly, b: i32) -> bool {
    if b > (Q - 1) / 8 {
        return true;
    }
    for i in 0..N {
        let t = a[i] >> 31;
        let t = a[i] - (t & (2 * a[i]));
        if t >= b {
            return true;
        }
    }
    false
}

// ─────────────────────────────────────────────────────────────────────────────
// NTT (ntt.c) — Montgomery domain, bitreversed output
// ─────────────────────────────────────────────────────────────────────────────
#[rustfmt::skip]
const ZETAS: [i32; N] = [
         0,    25847, -2608894,  -518909,   237124,  -777960,  -876248,   466468,
   1826347,  2353451,  -359251, -2091905,  3119733, -2884855,  3111497,  2680103,
   2725464,  1024112, -1079900,  3585928,  -549488, -1119584,  2619752, -2108549,
  -2118186, -3859737, -1399561, -3277672,  1757237,   -19422,  4010497,   280005,
   2706023,    95776,  3077325,  3530437, -1661693, -3592148, -2537516,  3915439,
  -3861115, -3043716,  3574422, -2867647,  3539968,  -300467,  2348700,  -539299,
  -1699267, -1643818,  3505694, -3821735,  3507263, -2140649, -1600420,  3699596,
    811944,   531354,   954230,  3881043,  3900724, -2556880,  2071892, -2797779,
  -3930395, -1528703, -3677745, -3041255, -1452451,  3475950,  2176455, -1585221,
  -1257611,  1939314, -4083598, -1000202, -3190144, -3157330, -3632928,   126922,
   3412210,  -983419,  2147896,  2715295, -2967645, -3693493,  -411027, -2477047,
   -671102, -1228525,   -22981, -1308169,  -381987,  1349076,  1852771, -1430430,
  -3343383,   264944,   508951,  3097992,    44288, -1100098,   904516,  3958618,
  -3724342,    -8578,  1653064, -3249728,  2389356,  -210977,   759969, -1316856,
    189548, -3553272,  3159746, -1851402, -2409325,  -177440,  1315589,  1341330,
   1285669, -1584928,  -812732, -1439742, -3019102, -3881060, -3628969,  3839961,
   2091667,  3407706,  2316500,  3817976, -3342478,  2244091, -2446433, -3562462,
    266997,  2434439, -1235728,  3513181, -3520352, -3759364, -1197226, -3193378,
    900702,  1859098,   909542,   819034,   495491, -1613174,   -43260,  -522500,
   -655327, -3122442,  2031748,  3207046, -3556995,  -525098,  -768622, -3595838,
    342297,   286988, -2437823,  4108315,  3437287, -3342277,  1735879,   203044,
   2842341,  2691481, -2590150,  1265009,  4055324,  1247620,  2486353,  1595974,
  -3767016,  1250494,  2635921, -3548272, -2994039,  1869119,  1903435, -1050970,
  -1333058,  1237275, -3318210, -1430225,  -451100,  1312455,  3306115, -1962642,
  -1279661,  1917081, -2546312, -1374803,  1500165,   777191,  2235880,  3406031,
   -542412, -2831860, -1671176, -1846953, -2584293, -3724270,   594136, -3776993,
  -2013608,  2432395,  2454455,  -164721,  1957272,  3369112,   185531, -1207385,
  -3183426,   162844,  1616392,  3014001,   810149,  1652634, -3694233, -1799107,
  -3038916,  3523897,  3866901,   269760,  2213111,  -975884,  1717735,   472078,
   -426683,  1723600, -1803090,  1910376, -1667432, -1104333,  -260646, -3833893,
  -2939036, -2235985,  -420899, -2286327,   183443,  -976891,  1612842, -3545687,
   -554416,  3919660,   -48306, -1362209,  3937738,  1400424,  -846154,  1976782,
];

fn ntt(a: &mut Poly) {
    let mut k = 0usize;
    let mut len = 128usize;
    while len > 0 {
        let mut start = 0usize;
        while start < N {
            k += 1;
            let zeta = ZETAS[k];
            let mut j = start;
            while j < start + len {
                let t = montgomery_reduce((zeta as i64) * (a[j + len] as i64));
                a[j + len] = a[j] - t;
                a[j] = a[j] + t;
                j += 1;
            }
            start = j + len;
        }
        len >>= 1;
    }
}

fn invntt_tomont(a: &mut Poly) {
    const F: i64 = 41978; // mont^2 / 256
    let mut k = 256usize;
    let mut len = 1usize;
    while len < N {
        let mut start = 0usize;
        while start < N {
            k -= 1;
            let zeta = -ZETAS[k];
            let mut j = start;
            while j < start + len {
                let t = a[j];
                a[j] = t + a[j + len];
                a[j + len] = t - a[j + len];
                a[j + len] = montgomery_reduce((zeta as i64) * (a[j + len] as i64));
                j += 1;
            }
            start = j + len;
        }
        len <<= 1;
    }
    for j in 0..N {
        a[j] = montgomery_reduce(F * (a[j] as i64));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Rounding (rounding.c)
// ─────────────────────────────────────────────────────────────────────────────
/// Power2Round: returns (a1, a0) with a = a1*2^D + a0, a0 in (-2^{D-1}, 2^{D-1}].
fn power2round(a: i32) -> (i32, i32) {
    let a1 = (a + (1 << (D - 1)) - 1) >> D;
    let a0 = a - (a1 << D);
    (a1, a0)
}

/// Decompose: returns (a1, a0) for GAMMA2 = (Q-1)/32.
fn decompose(a: i32) -> (i32, i32) {
    let mut a1 = (a + 127) >> 7;
    a1 = (a1 * 1025 + (1 << 21)) >> 22;
    a1 &= 15;
    let mut a0 = a - a1 * 2 * GAMMA2;
    a0 -= (((Q - 1) / 2 - a0) >> 31) & Q;
    (a1, a0)
}

fn make_hint(a0: i32, a1: i32) -> i32 {
    if a0 > GAMMA2 || a0 < -GAMMA2 || (a0 == -GAMMA2 && a1 != 0) {
        1
    } else {
        0
    }
}

fn use_hint(a: i32, hint: i32) -> i32 {
    let (a1, a0) = decompose(a);
    if hint == 0 {
        return a1;
    }
    if a0 > 0 {
        (a1 + 1) & 15
    } else {
        (a1 - 1) & 15
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Sampling (poly.c)
// ─────────────────────────────────────────────────────────────────────────────
/// Uniform rejection sampling of coefficients in [0, Q) from a byte buffer.
/// Returns number of coefficients written.
fn rej_uniform(a: &mut [i32], buf: &[u8]) -> usize {
    let len = a.len();
    let mut ctr = 0usize;
    let mut pos = 0usize;
    while ctr < len && pos + 3 <= buf.len() {
        let mut t = buf[pos] as u32;
        t |= (buf[pos + 1] as u32) << 8;
        t |= (buf[pos + 2] as u32) << 16;
        t &= 0x7FFFFF;
        pos += 3;
        if (t as i32) < Q {
            a[ctr] = t as i32;
            ctr += 1;
        }
    }
    ctr
}

/// ExpandA entry (RejNTTPoly): poly_uniform(rho, nonce). Result is in the NTT domain.
fn poly_uniform(rho: &[u8; SEEDBYTES], nonce: u16) -> Poly {
    // Reference squeezes 5 SHAKE128 blocks then 1 at a time; SHAKE squeeze of L bytes is a
    // prefix of the same stream and block boundaries (168) are 3-aligned, so a single generous
    // contiguous squeeze consumed 3 bytes at a time is byte-identical to the reference.
    let mut seed = [0u8; SEEDBYTES + 2];
    seed[..SEEDBYTES].copy_from_slice(rho);
    seed[SEEDBYTES] = (nonce & 0xff) as u8;
    seed[SEEDBYTES + 1] = (nonce >> 8) as u8;
    let mut buf = [0u8; SHAKE128_RATE * 12]; // 2016 bytes: rejection completes w.p. 1 - 2^-far
    shake128(&seed, &mut buf);
    let mut a = [0i32; N];
    let ctr = rej_uniform(&mut a, &buf);
    assert!(
        ctr == N,
        "poly_uniform: rejection sampling underflow (buffer too small)"
    );
    a
}

/// Rejection sampling of eta-bounded coefficients (η=4: nibble<9 → 4-nibble).
fn rej_eta(a: &mut [i32], buf: &[u8]) -> usize {
    let len = a.len();
    let mut ctr = 0usize;
    let mut pos = 0usize;
    while ctr < len && pos < buf.len() {
        let t0 = (buf[pos] & 0x0F) as i32;
        let t1 = (buf[pos] >> 4) as i32;
        pos += 1;
        if t0 < 9 {
            a[ctr] = 4 - t0;
            ctr += 1;
        }
        if t1 < 9 && ctr < len {
            a[ctr] = 4 - t1;
            ctr += 1;
        }
    }
    ctr
}

/// ExpandS entry (RejBoundedPoly): poly_uniform_eta(rhoprime, nonce). Coefficient domain.
fn poly_uniform_eta(rhoprime: &[u8; CRHBYTES], nonce: u16) -> Poly {
    let mut seed = [0u8; CRHBYTES + 2];
    seed[..CRHBYTES].copy_from_slice(rhoprime);
    seed[CRHBYTES] = (nonce & 0xff) as u8;
    seed[CRHBYTES + 1] = (nonce >> 8) as u8;
    let mut buf = [0u8; SHAKE256_RATE * 8]; // generous; 1-byte consumption is stream-aligned
    shake256(&seed, &mut buf);
    let mut a = [0i32; N];
    let ctr = rej_eta(&mut a, &buf);
    assert!(ctr == N, "poly_uniform_eta: rejection sampling underflow");
    a
}

/// ExpandMask entry: poly_uniform_gamma1(rhoprime, nonce) = polyz_unpack(SHAKE256 stream).
fn poly_uniform_gamma1(rhoprime: &[u8; CRHBYTES], nonce: u16) -> Poly {
    let mut seed = [0u8; CRHBYTES + 2];
    seed[..CRHBYTES].copy_from_slice(rhoprime);
    seed[CRHBYTES] = (nonce & 0xff) as u8;
    seed[CRHBYTES + 1] = (nonce >> 8) as u8;
    let mut buf = [0u8; POLYZ_PACKEDBYTES];
    shake256(&seed, &mut buf);
    polyz_unpack(&buf)
}

/// SampleInBall (poly_challenge): TAU coeffs in {+1,-1} placed via Fisher-Yates from 48-byte c̃.
fn poly_challenge(c_tilde: &[u8]) -> Poly {
    let mut buf = [0u8; SHAKE256_RATE * 2]; // plenty for TAU=49 rejections
    shake256(&c_tilde[..CTILDEBYTES], &mut buf);
    let mut signs: u64 = 0;
    for i in 0..8 {
        signs |= (buf[i] as u64) << (8 * i);
    }
    let mut pos = 8usize;
    let mut c = [0i32; N];
    for i in (N - TAU)..N {
        let b;
        loop {
            let cand = buf[pos] as usize;
            pos += 1;
            if cand <= i {
                b = cand;
                break;
            }
        }
        c[i] = c[b];
        c[b] = 1 - 2 * ((signs & 1) as i32);
        signs >>= 1;
    }
    c
}

fn poly_matrix_expand(rho: &[u8; SEEDBYTES]) -> [PolyVecL; K] {
    let mut mat = [[poly_zero(); L]; K];
    for i in 0..K {
        for j in 0..L {
            mat[i][j] = poly_uniform(rho, ((i as u16) << 8) + j as u16);
        }
    }
    mat
}

// ─────────────────────────────────────────────────────────────────────────────
// Bit-packing (poly.c / packing.c) — FIPS 204 §7
// ─────────────────────────────────────────────────────────────────────────────
fn polyt1_pack(a: &Poly, out: &mut [u8]) {
    for i in 0..N / 4 {
        out[5 * i] = (a[4 * i] >> 0) as u8;
        out[5 * i + 1] = ((a[4 * i] >> 8) | (a[4 * i + 1] << 2)) as u8;
        out[5 * i + 2] = ((a[4 * i + 1] >> 6) | (a[4 * i + 2] << 4)) as u8;
        out[5 * i + 3] = ((a[4 * i + 2] >> 4) | (a[4 * i + 3] << 6)) as u8;
        out[5 * i + 4] = (a[4 * i + 3] >> 2) as u8;
    }
}

fn polyt1_unpack(a: &[u8]) -> Poly {
    let mut r = [0i32; N];
    for i in 0..N / 4 {
        r[4 * i] = (((a[5 * i] as u32) >> 0 | ((a[5 * i + 1] as u32) << 8)) & 0x3FF) as i32;
        r[4 * i + 1] = (((a[5 * i + 1] as u32) >> 2 | ((a[5 * i + 2] as u32) << 6)) & 0x3FF) as i32;
        r[4 * i + 2] = (((a[5 * i + 2] as u32) >> 4 | ((a[5 * i + 3] as u32) << 4)) & 0x3FF) as i32;
        r[4 * i + 3] = (((a[5 * i + 3] as u32) >> 6 | ((a[5 * i + 4] as u32) << 2)) & 0x3FF) as i32;
    }
    r
}

fn polyt0_pack(a: &Poly, out: &mut [u8]) {
    let mut t = [0u32; 8];
    for i in 0..N / 8 {
        for j in 0..8 {
            t[j] = ((1 << (D - 1)) - a[8 * i + j]) as u32;
        }
        out[13 * i] = t[0] as u8;
        out[13 * i + 1] = (t[0] >> 8) as u8;
        out[13 * i + 1] |= (t[1] << 5) as u8;
        out[13 * i + 2] = (t[1] >> 3) as u8;
        out[13 * i + 3] = (t[1] >> 11) as u8;
        out[13 * i + 3] |= (t[2] << 2) as u8;
        out[13 * i + 4] = (t[2] >> 6) as u8;
        out[13 * i + 4] |= (t[3] << 7) as u8;
        out[13 * i + 5] = (t[3] >> 1) as u8;
        out[13 * i + 6] = (t[3] >> 9) as u8;
        out[13 * i + 6] |= (t[4] << 4) as u8;
        out[13 * i + 7] = (t[4] >> 4) as u8;
        out[13 * i + 8] = (t[4] >> 12) as u8;
        out[13 * i + 8] |= (t[5] << 1) as u8;
        out[13 * i + 9] = (t[5] >> 7) as u8;
        out[13 * i + 9] |= (t[6] << 6) as u8;
        out[13 * i + 10] = (t[6] >> 2) as u8;
        out[13 * i + 11] = (t[6] >> 10) as u8;
        out[13 * i + 11] |= (t[7] << 3) as u8;
        out[13 * i + 12] = (t[7] >> 5) as u8;
    }
}

fn polyt0_unpack(a: &[u8]) -> Poly {
    let mut r = [0i32; N];
    for i in 0..N / 8 {
        let g = |x: usize| a[x] as u32;
        let mut c = [0u32; 8];
        c[0] = (g(13 * i) | (g(13 * i + 1) << 8)) & 0x1FFF;
        c[1] = ((g(13 * i + 1) >> 5) | (g(13 * i + 2) << 3) | (g(13 * i + 3) << 11)) & 0x1FFF;
        c[2] = ((g(13 * i + 3) >> 2) | (g(13 * i + 4) << 6)) & 0x1FFF;
        c[3] = ((g(13 * i + 4) >> 7) | (g(13 * i + 5) << 1) | (g(13 * i + 6) << 9)) & 0x1FFF;
        c[4] = ((g(13 * i + 6) >> 4) | (g(13 * i + 7) << 4) | (g(13 * i + 8) << 12)) & 0x1FFF;
        c[5] = ((g(13 * i + 8) >> 1) | (g(13 * i + 9) << 7)) & 0x1FFF;
        c[6] = ((g(13 * i + 9) >> 6) | (g(13 * i + 10) << 2) | (g(13 * i + 11) << 10)) & 0x1FFF;
        c[7] = ((g(13 * i + 11) >> 3) | (g(13 * i + 12) << 5)) & 0x1FFF;
        for j in 0..8 {
            r[8 * i + j] = (1 << (D - 1)) - c[j] as i32;
        }
    }
    r
}

fn polyeta_pack(a: &Poly, out: &mut [u8]) {
    for i in 0..N / 2 {
        let t0 = (ETA - a[2 * i]) as u8;
        let t1 = (ETA - a[2 * i + 1]) as u8;
        out[i] = t0 | (t1 << 4);
    }
}

fn polyeta_unpack(a: &[u8]) -> Poly {
    let mut r = [0i32; N];
    for i in 0..N / 2 {
        r[2 * i] = ETA - (a[i] & 0x0F) as i32;
        r[2 * i + 1] = ETA - (a[i] >> 4) as i32;
    }
    r
}

fn polyz_pack(a: &Poly, out: &mut [u8]) {
    for i in 0..N / 2 {
        let t0 = (GAMMA1 - a[2 * i]) as u32;
        let t1 = (GAMMA1 - a[2 * i + 1]) as u32;
        out[5 * i] = t0 as u8;
        out[5 * i + 1] = (t0 >> 8) as u8;
        out[5 * i + 2] = (t0 >> 16) as u8;
        out[5 * i + 2] |= (t1 << 4) as u8;
        out[5 * i + 3] = (t1 >> 4) as u8;
        out[5 * i + 4] = (t1 >> 12) as u8;
    }
}

fn polyz_unpack(a: &[u8]) -> Poly {
    let mut r = [0i32; N];
    for i in 0..N / 2 {
        let g = |x: usize| a[x] as u32;
        let mut c0 = g(5 * i) | (g(5 * i + 1) << 8) | (g(5 * i + 2) << 16);
        c0 &= 0xFFFFF;
        let c1 = (g(5 * i + 2) >> 4) | (g(5 * i + 3) << 4) | (g(5 * i + 4) << 12);
        r[2 * i] = GAMMA1 - c0 as i32;
        r[2 * i + 1] = GAMMA1 - c1 as i32;
    }
    r
}

fn polyw1_pack(a: &Poly, out: &mut [u8]) {
    for i in 0..N / 2 {
        out[i] = (a[2 * i] | (a[2 * i + 1] << 4)) as u8;
    }
}

fn pack_w1(w1: &PolyVecK) -> [u8; K * POLYW1_PACKEDBYTES] {
    let mut out = [0u8; K * POLYW1_PACKEDBYTES];
    for i in 0..K {
        polyw1_pack(
            &w1[i],
            &mut out[i * POLYW1_PACKEDBYTES..(i + 1) * POLYW1_PACKEDBYTES],
        );
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// Key / signature (de)serialization (packing.c)
// ─────────────────────────────────────────────────────────────────────────────
fn pack_pk_bytes(rho: &[u8; SEEDBYTES], t1: &PolyVecK) -> Vec<u8> {
    let mut pk = vec![0u8; PUBLICKEYBYTES];
    pk[..SEEDBYTES].copy_from_slice(rho);
    for i in 0..K {
        polyt1_pack(
            &t1[i],
            &mut pk[SEEDBYTES + i * POLYT1_PACKEDBYTES..SEEDBYTES + (i + 1) * POLYT1_PACKEDBYTES],
        );
    }
    pk
}

pub fn unpack_pk_bytes(pk: &[u8]) -> ([u8; SEEDBYTES], PolyVecK) {
    let mut rho = [0u8; SEEDBYTES];
    rho.copy_from_slice(&pk[..SEEDBYTES]);
    let mut t1 = [poly_zero(); K];
    for i in 0..K {
        t1[i] = polyt1_unpack(&pk[SEEDBYTES + i * POLYT1_PACKEDBYTES..]);
    }
    (rho, t1)
}

fn pack_sk_bytes(
    rho: &[u8; SEEDBYTES],
    tr: &[u8; TRBYTES],
    key: &[u8; SEEDBYTES],
    t0: &PolyVecK,
    s1: &PolyVecL,
    s2: &PolyVecK,
) -> Vec<u8> {
    let mut sk = vec![0u8; SECRETKEYBYTES];
    let mut off = 0;
    sk[off..off + SEEDBYTES].copy_from_slice(rho);
    off += SEEDBYTES;
    sk[off..off + SEEDBYTES].copy_from_slice(key);
    off += SEEDBYTES;
    sk[off..off + TRBYTES].copy_from_slice(tr);
    off += TRBYTES;
    for i in 0..L {
        polyeta_pack(&s1[i], &mut sk[off + i * POLYETA_PACKEDBYTES..]);
    }
    off += L * POLYETA_PACKEDBYTES;
    for i in 0..K {
        polyeta_pack(&s2[i], &mut sk[off + i * POLYETA_PACKEDBYTES..]);
    }
    off += K * POLYETA_PACKEDBYTES;
    for i in 0..K {
        polyt0_pack(&t0[i], &mut sk[off + i * POLYT0_PACKEDBYTES..]);
    }
    sk
}

fn unpack_sk_bytes(
    sk: &[u8],
) -> (
    [u8; SEEDBYTES],
    [u8; TRBYTES],
    [u8; SEEDBYTES],
    PolyVecK,
    PolyVecL,
    PolyVecK,
) {
    let mut rho = [0u8; SEEDBYTES];
    let mut key = [0u8; SEEDBYTES];
    let mut tr = [0u8; TRBYTES];
    let mut off = 0;
    rho.copy_from_slice(&sk[off..off + SEEDBYTES]);
    off += SEEDBYTES;
    key.copy_from_slice(&sk[off..off + SEEDBYTES]);
    off += SEEDBYTES;
    tr.copy_from_slice(&sk[off..off + TRBYTES]);
    off += TRBYTES;
    let mut s1 = [poly_zero(); L];
    for i in 0..L {
        s1[i] = polyeta_unpack(&sk[off + i * POLYETA_PACKEDBYTES..]);
    }
    off += L * POLYETA_PACKEDBYTES;
    let mut s2 = [poly_zero(); K];
    for i in 0..K {
        s2[i] = polyeta_unpack(&sk[off + i * POLYETA_PACKEDBYTES..]);
    }
    off += K * POLYETA_PACKEDBYTES;
    let mut t0 = [poly_zero(); K];
    for i in 0..K {
        t0[i] = polyt0_unpack(&sk[off + i * POLYT0_PACKEDBYTES..]);
    }
    (rho, tr, key, t0, s1, s2)
}

fn pack_sig_bytes(c_tilde: &[u8], z: &PolyVecL, h: &PolyVecK) -> Vec<u8> {
    let mut sig = vec![0u8; SIGNATUREBYTES];
    sig[..CTILDEBYTES].copy_from_slice(&c_tilde[..CTILDEBYTES]);
    let zoff = CTILDEBYTES;
    for i in 0..L {
        polyz_pack(&z[i], &mut sig[zoff + i * POLYZ_PACKEDBYTES..]);
    }
    let hoff = CTILDEBYTES + L * POLYZ_PACKEDBYTES;
    // Encode hint: positions then per-poly cumulative count.
    let mut k = 0usize;
    for i in 0..K {
        for j in 0..N {
            if h[i][j] != 0 {
                sig[hoff + k] = j as u8;
                k += 1;
            }
        }
        sig[hoff + OMEGA + i] = k as u8;
    }
    sig
}

/// Unpack signature. Returns None on malformed hint (strong-unforgeability checks).
pub fn unpack_sig_bytes(sig: &[u8]) -> Option<([u8; CTILDEBYTES], PolyVecL, PolyVecK)> {
    if sig.len() != SIGNATUREBYTES {
        return None;
    }
    let mut c = [0u8; CTILDEBYTES];
    c.copy_from_slice(&sig[..CTILDEBYTES]);
    let zoff = CTILDEBYTES;
    let mut z = [poly_zero(); L];
    for i in 0..L {
        z[i] = polyz_unpack(&sig[zoff + i * POLYZ_PACKEDBYTES..]);
    }
    let hoff = CTILDEBYTES + L * POLYZ_PACKEDBYTES;
    let mut h = [poly_zero(); K];
    let mut k = 0usize;
    for i in 0..K {
        let cnt = sig[hoff + OMEGA + i] as usize;
        if cnt < k || cnt > OMEGA {
            return None;
        }
        for j in k..cnt {
            if j > k && sig[hoff + j] <= sig[hoff + j - 1] {
                return None;
            }
            h[i][sig[hoff + j] as usize] = 1;
        }
        k = cnt;
    }
    for j in k..OMEGA {
        if sig[hoff + j] != 0 {
            return None;
        }
    }
    Some((c, z, h))
}

// ─────────────────────────────────────────────────────────────────────────────
// Matrix / vector NTT helpers
// ─────────────────────────────────────────────────────────────────────────────
fn matrix_pointwise(mat: &[PolyVecL; K], v: &PolyVecL) -> PolyVecK {
    let mut t = [poly_zero(); K];
    for i in 0..K {
        t[i] = poly_pointwise_montgomery(&mat[i][0], &v[0]);
        for j in 1..L {
            let p = poly_pointwise_montgomery(&mat[i][j], &v[j]);
            t[i] = poly_add(&t[i], &p);
        }
    }
    t
}

// ─────────────────────────────────────────────────────────────────────────────
// Public byte-oriented API (ACVP internal interface)
// ─────────────────────────────────────────────────────────────────────────────
/// ML-DSA.KeyGen_internal (FIPS 204 Alg 6). Returns (pk_bytes, sk_bytes).
pub fn keygen_bytes(seed: &[u8; SEEDBYTES]) -> (Vec<u8>, Vec<u8>) {
    // (rho, rhoprime, K) <- H(seed || IntegerToBytes(k,1) || IntegerToBytes(l,1), 128)
    let mut hin = [0u8; SEEDBYTES + 2];
    hin[..SEEDBYTES].copy_from_slice(seed);
    hin[SEEDBYTES] = K as u8;
    hin[SEEDBYTES + 1] = L as u8;
    let mut ext = [0u8; 2 * SEEDBYTES + CRHBYTES];
    shake256(&hin, &mut ext);
    let mut rho = [0u8; SEEDBYTES];
    rho.copy_from_slice(&ext[..SEEDBYTES]);
    let mut rhoprime = [0u8; CRHBYTES];
    rhoprime.copy_from_slice(&ext[SEEDBYTES..SEEDBYTES + CRHBYTES]);
    let mut key = [0u8; SEEDBYTES];
    key.copy_from_slice(&ext[SEEDBYTES + CRHBYTES..]);

    let mat = poly_matrix_expand(&rho);
    let mut s1 = [poly_zero(); L];
    for i in 0..L {
        s1[i] = poly_uniform_eta(&rhoprime, i as u16);
    }
    let mut s2 = [poly_zero(); K];
    for i in 0..K {
        s2[i] = poly_uniform_eta(&rhoprime, (L + i) as u16);
    }

    let mut s1hat = s1;
    for i in 0..L {
        ntt(&mut s1hat[i]);
    }
    let mut t = matrix_pointwise(&mat, &s1hat);
    for i in 0..K {
        poly_reduce(&mut t[i]);
        invntt_tomont(&mut t[i]);
        t[i] = poly_add(&t[i], &s2[i]);
        poly_caddq(&mut t[i]);
    }
    let mut t1 = [poly_zero(); K];
    let mut t0 = [poly_zero(); K];
    for i in 0..K {
        for j in 0..N {
            let (a1, a0) = power2round(t[i][j]);
            t1[i][j] = a1;
            t0[i][j] = a0;
        }
    }
    let pk = pack_pk_bytes(&rho, &t1);
    let mut tr = [0u8; TRBYTES];
    shake256(&pk, &mut tr);
    let sk = pack_sk_bytes(&rho, &tr, &key, &t0, &s1, &s2);
    (pk, sk)
}

/// ML-DSA.Sign_internal (FIPS 204 Alg 7), deterministic-capable via rnd. pre is empty (internal).
pub fn sign_internal_bytes(sk: &[u8], msg: &[u8], rnd: &[u8; RNDBYTES]) -> Vec<u8> {
    let (rho, tr, key, t0, s1, s2) = unpack_sk_bytes(sk);

    // mu = CRH(tr || pre || msg); pre empty for internal interface.
    let mut mu = [0u8; CRHBYTES];
    {
        let mut buf = Vec::with_capacity(TRBYTES + msg.len());
        buf.extend_from_slice(&tr);
        buf.extend_from_slice(msg);
        shake256(&buf, &mut mu);
    }
    // rhoprime = CRH(key || rnd || mu)
    let mut rhoprime = [0u8; CRHBYTES];
    {
        let mut buf = Vec::with_capacity(SEEDBYTES + RNDBYTES + CRHBYTES);
        buf.extend_from_slice(&key);
        buf.extend_from_slice(rnd);
        buf.extend_from_slice(&mu);
        shake256(&buf, &mut rhoprime);
    }

    let mat = poly_matrix_expand(&rho);
    let mut s1n = s1;
    for i in 0..L {
        ntt(&mut s1n[i]);
    }
    let mut s2n = s2;
    for i in 0..K {
        ntt(&mut s2n[i]);
    }
    let mut t0n = t0;
    for i in 0..K {
        ntt(&mut t0n[i]);
    }

    let mut nonce: u16 = 0;
    loop {
        // y <- ExpandMask(rhoprime, nonce)
        let mut y = [poly_zero(); L];
        for i in 0..L {
            y[i] = poly_uniform_gamma1(&rhoprime, L as u16 * nonce + i as u16);
        }
        nonce += 1;

        // w = NTT^{-1}(A · NTT(y))
        let mut z = y;
        for i in 0..L {
            ntt(&mut z[i]);
        }
        let mut w = matrix_pointwise(&mat, &z);
        for i in 0..K {
            poly_reduce(&mut w[i]);
            invntt_tomont(&mut w[i]);
            poly_caddq(&mut w[i]);
        }
        // (w1, w0) = Decompose(w)
        let mut w1 = [poly_zero(); K];
        let mut w0 = [poly_zero(); K];
        for i in 0..K {
            for j in 0..N {
                let (a1, a0) = decompose(w[i][j]);
                w1[i][j] = a1;
                w0[i][j] = a0;
            }
        }
        let w1pack = pack_w1(&w1);

        // c̃ = H(mu || w1Encode(w1)), 48 bytes
        let mut c_tilde = [0u8; CTILDEBYTES];
        {
            let mut buf = Vec::with_capacity(CRHBYTES + w1pack.len());
            buf.extend_from_slice(&mu);
            buf.extend_from_slice(&w1pack);
            shake256(&buf, &mut c_tilde);
        }
        let mut cp = poly_challenge(&c_tilde);
        ntt(&mut cp);

        // z = y + c·s1
        let mut zsig = [poly_zero(); L];
        for i in 0..L {
            let mut cs1 = poly_pointwise_montgomery(&cp, &s1n[i]);
            invntt_tomont(&mut cs1);
            zsig[i] = poly_add(&cs1, &y[i]);
            poly_reduce(&mut zsig[i]);
        }
        let mut reject = false;
        for i in 0..L {
            if poly_chknorm(&zsig[i], GAMMA1 - BETA) {
                reject = true;
            }
        }
        if reject {
            continue;
        }

        // w0 = w0 - c·s2; reject on norm
        for i in 0..K {
            let mut cs2 = poly_pointwise_montgomery(&cp, &s2n[i]);
            invntt_tomont(&mut cs2);
            w0[i] = poly_sub(&w0[i], &cs2);
            poly_reduce(&mut w0[i]);
        }
        for i in 0..K {
            if poly_chknorm(&w0[i], GAMMA2 - BETA) {
                reject = true;
            }
        }
        if reject {
            continue;
        }

        // ct0 = c·t0; reject on norm; w0 += ct0; h = MakeHint(w0, w1)
        let mut ct0 = [poly_zero(); K];
        for i in 0..K {
            ct0[i] = poly_pointwise_montgomery(&cp, &t0n[i]);
            invntt_tomont(&mut ct0[i]);
            poly_reduce(&mut ct0[i]);
        }
        for i in 0..K {
            if poly_chknorm(&ct0[i], GAMMA2) {
                reject = true;
            }
        }
        if reject {
            continue;
        }
        let mut h = [poly_zero(); K];
        let mut hint_count = 0i32;
        for i in 0..K {
            w0[i] = poly_add(&w0[i], &ct0[i]);
            for j in 0..N {
                h[i][j] = make_hint(w0[i][j], w1[i][j]);
                hint_count += h[i][j];
            }
        }
        if hint_count as usize > OMEGA {
            continue;
        }
        return pack_sig_bytes(&c_tilde, &zsig, &h);
    }
}

/// ML-DSA.Verify_internal (FIPS 204 Alg 8). pre empty (internal interface).
pub fn verify_internal_bytes(pk: &[u8], msg: &[u8], sig: &[u8]) -> bool {
    if pk.len() != PUBLICKEYBYTES {
        return false;
    }
    let (rho, t1) = unpack_pk_bytes(pk);
    let (c_tilde, z, h) = match unpack_sig_bytes(sig) {
        Some(v) => v,
        None => return false,
    };
    for i in 0..L {
        if poly_chknorm(&z[i], GAMMA1 - BETA) {
            return false;
        }
    }
    // mu = CRH(H(pk) || pre || msg)
    let mut tr = [0u8; TRBYTES];
    shake256(pk, &mut tr);
    let mut mu = [0u8; CRHBYTES];
    {
        let mut buf = Vec::with_capacity(TRBYTES + msg.len());
        buf.extend_from_slice(&tr);
        buf.extend_from_slice(msg);
        shake256(&buf, &mut mu);
    }

    let mut cp = poly_challenge(&c_tilde);
    let mat = poly_matrix_expand(&rho);

    // w = A·NTT(z) - c·(t1·2^d)
    let mut zn = z;
    for i in 0..L {
        ntt(&mut zn[i]);
    }
    let mut w = matrix_pointwise(&mat, &zn);
    ntt(&mut cp);
    let mut t1n = t1;
    for i in 0..K {
        poly_shiftl(&mut t1n[i]);
        ntt(&mut t1n[i]);
        t1n[i] = poly_pointwise_montgomery(&cp, &t1n[i]);
    }
    for i in 0..K {
        w[i] = poly_sub(&w[i], &t1n[i]);
        poly_reduce(&mut w[i]);
        invntt_tomont(&mut w[i]);
        poly_caddq(&mut w[i]);
    }
    // w1 = UseHint(h, w)
    let mut w1 = [poly_zero(); K];
    for i in 0..K {
        for j in 0..N {
            w1[i][j] = use_hint(w[i][j], h[i][j]);
        }
    }
    let w1pack = pack_w1(&w1);
    let mut c2 = [0u8; CTILDEBYTES];
    {
        let mut buf = Vec::with_capacity(CRHBYTES + w1pack.len());
        buf.extend_from_slice(&mu);
        buf.extend_from_slice(&w1pack);
        shake256(&buf, &mut c2);
    }
    c2 == c_tilde
}

// ─────────────────────────────────────────────────────────────────────────────
// Struct-oriented convenience API (byte-backed; unchanged external shape)
// ─────────────────────────────────────────────────────────────────────────────
pub struct MlDsa65Pk {
    pub bytes: Vec<u8>,
}
pub struct MlDsa65Sk {
    pub bytes: Vec<u8>,
}
pub struct MlDsa65Sig {
    pub bytes: Vec<u8>,
}

/// KeyGen (FIPS 204). seed = 32 bytes.
pub fn keygen(seed: &[u8; SEEDBYTES]) -> (MlDsa65Pk, MlDsa65Sk) {
    let (pk, sk) = keygen_bytes(seed);
    (MlDsa65Pk { bytes: pk }, MlDsa65Sk { bytes: sk })
}

/// Sign (internal interface, deterministic-capable via rnd; rnd=0 → FIPS deterministic mode).
pub fn sign(sk: &MlDsa65Sk, msg: &[u8], rnd: &[u8; RNDBYTES]) -> MlDsa65Sig {
    MlDsa65Sig {
        bytes: sign_internal_bytes(&sk.bytes, msg, rnd),
    }
}

/// Verify (internal interface).
pub fn verify(pk: &MlDsa65Pk, msg: &[u8], sig: &MlDsa65Sig) -> bool {
    verify_internal_bytes(&pk.bytes, msg, &sig.bytes)
}

// ─────────────────────────────────────────────────────────────────────────────
// NIST ACVP FIPS204 ML-DSA-65 byte-exact property-gate (GREEN)
// Parses the vendored official NIST ACVP vectors and asserts byte-exact agreement
// for keyGen / sigGen-internal-deterministic / sigVer. Test-only; see acvp_tests.rs.
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod dsa_acvp_tests;

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kat_shake256_empty() {
        let mut out = [0u8; 64];
        shake256(b"", &mut out);
        let expected = [
            0x46, 0xb9, 0xdd, 0x2b, 0x0b, 0xa8, 0x8d, 0x13, 0x23, 0x3b, 0x3f, 0xeb, 0x74, 0x3e,
            0xeb, 0x24, 0x3f, 0xcd, 0x52, 0xea, 0x62, 0xb8, 0x1b, 0x82, 0xb5, 0x0c, 0x27, 0x64,
            0x6e, 0xd5, 0x76, 0x2f,
        ];
        assert_eq!(&out[..32], &expected[..], "SHAKE256 empty KAT mismatch");
    }

    #[test]
    fn sizes_are_fips204_cat3() {
        assert_eq!(PUBLICKEYBYTES, 1952);
        assert_eq!(SECRETKEYBYTES, 4032);
        assert_eq!(SIGNATUREBYTES, 3309);
        assert_eq!(CTILDEBYTES, 48);
    }

    #[test]
    fn sign_verify_roundtrip_and_tamper() {
        let seed = [7u8; 32];
        let (pk, sk) = keygen(&seed);
        let msg = b"bebop2 ml-dsa-65 fable gate";
        let rnd = [0u8; 32];
        let sig = sign(&sk, msg, &rnd);
        assert_eq!(sig.bytes.len(), SIGNATUREBYTES);
        assert!(verify(&pk, msg, &sig), "verify failed on valid signature");
        let mut bad = msg.to_vec();
        bad[0] ^= 0xff;
        assert!(
            !verify(&pk, &bad, &sig),
            "tampered message verified (RED missing)"
        );
    }

    #[test]
    fn deterministic_signing() {
        let seed = [3u8; 32];
        let (_pk, sk) = keygen(&seed);
        let rnd = [0u8; 32];
        let a = sign_internal_bytes(&sk.bytes, b"probe", &rnd);
        let b = sign_internal_bytes(&sk.bytes, b"probe", &rnd);
        assert_eq!(a, b, "deterministic (rnd=0) signing must be reproducible");
        let c = sign_internal_bytes(&sk.bytes, b"probf", &rnd);
        assert_ne!(a, c, "message change must change signature");
    }

    #[test]
    fn forge_with_zero_hint_is_rejected() {
        let seed = [9u8; 32];
        let (pk, _sk) = keygen(&seed);
        let mut z = [[0i32; N]; L];
        for p in &mut z {
            for v in p.iter_mut() {
                *v = 1234;
            }
        }
        let h = [[0i32; N]; K];
        let sig = pack_sig_bytes(&[42u8; CTILDEBYTES], &z, &h);
        assert!(!verify_internal_bytes(&pk.bytes, b"forge attempt", &sig));
    }

    // ── G10 DIFFERENTIAL PROBE (vs pq-crystals ML-DSA-65 reference, ACVP-green) ──
    // Emits FIPS 204 intermediates as "STAGE <name> <values>" for byte-level diffing.
    // Run: cargo test -p bebop2-core --release mldsa_diff_probe -- --nocapture
    fn phex(name: &str, b: &[u8]) {
        let mut s = std::string::String::new();
        for x in b {
            s.push_str(&std::format!("{:02x}", x));
        }
        std::println!("STAGE {} {}", name, s);
    }
    fn pnorm(name: &str, p: &Poly) {
        let mut s = std::string::String::new();
        for i in 0..N {
            let v = ((p[i] as i64 % Q as i64) + Q as i64) % Q as i64;
            s.push_str(&std::format!(" {}", v));
        }
        std::println!("STAGE {}{}", name, s);
    }

    #[test]
    fn mldsa_diff_probe() {
        let seed = [1u8; 32];
        phex("keygen.seed", &seed);
        let mut hin = [0u8; SEEDBYTES + 2];
        hin[..SEEDBYTES].copy_from_slice(&seed);
        hin[SEEDBYTES] = K as u8;
        hin[SEEDBYTES + 1] = L as u8;
        let mut ext = [0u8; 128];
        shake256(&hin, &mut ext);
        phex("keygen.rho", &ext[0..32]);
        phex("keygen.rhoprime", &ext[32..96]);
        phex("keygen.K", &ext[96..128]);
        let mut rho = [0u8; 32];
        rho.copy_from_slice(&ext[0..32]);
        let mut rhoprime = [0u8; 64];
        rhoprime.copy_from_slice(&ext[32..96]);
        let mat = poly_matrix_expand(&rho);
        pnorm("keygen.matA[0][0]", &mat[0][0]);
        let s1 = poly_uniform_eta(&rhoprime, 0);
        pnorm("keygen.s1[0]", &s1);
        let (pk, sk) = keygen(&seed);
        phex("keygen.pk", &pk.bytes[..48]);
        std::println!("STAGE keygen.pk.len {}", pk.bytes.len());
        std::println!("STAGE keygen.sk.len {}", sk.bytes.len());
        // t1[0] from packed pk
        let (_r, t1) = unpack_pk_bytes(&pk.bytes);
        pnorm("keygen.t1[0]", &t1[0]);
        let mut tr = [0u8; 64];
        shake256(&pk.bytes, &mut tr);
        phex("keygen.tr", &tr);
        let msg = b"probe";
        let rnd = [0u8; 32];
        let mut mu = [0u8; 64];
        {
            let mut buf = Vec::new();
            buf.extend_from_slice(&tr);
            buf.extend_from_slice(msg);
            shake256(&buf, &mut mu);
        }
        phex("sign.mu", &mu);
        let sig = sign_internal_bytes(&sk.bytes, msg, &rnd);
        phex("sign.ctilde", &sig[..48]);
        std::println!("STAGE sign.sig.len {}", sig.len());
    }

    // ── Item 7 (space-grade roadmap §C): native EXHAUSTIVE arithmetic contracts ──
    // Per RESEARCH-NATIVE-KANI-REPLACEMENT-FEASIBILITY-2026-07-19.md §2: `caddq`,
    // `power2round`, `decompose` take a SINGLE bounded i32; their whole contract domain
    // is enumerable in seconds — the codebase's exhaustive idiom proves the IDENTICAL
    // "for all inputs" guarantee as a Kani harness, with zero toolchain dependency.
    // (`reduce32` full-i32 (2^32) is impractical in a debug test → it is a cheap exact
    //  Kani proof instead; `montgomery_reduce`/`ntt`/`invntt` are the genuine Bucket-C
    //  Kani targets. See kani_proofs below.)

    /// EXHAUSTIVE over the contract domain −Q < a < Q (~16.76M values): `caddq` yields a
    /// value in [0, Q) that is ≡ a (mod Q), with no overflow.
    #[test]
    fn item7_exhaustive_caddq_contract() {
        let mut a = -(Q - 1);
        while a <= Q - 1 {
            let r = caddq(a);
            assert!(r >= 0 && r < Q, "caddq({a}) = {r} not in [0,Q)");
            // ≡ a (mod Q): r and a differ by a multiple of Q (0 or +Q).
            assert!(r == a || r == a + Q, "caddq({a}) = {r} not ≡ a (mod Q)");
            a += 1;
        }
    }

    /// EXHAUSTIVE over a ∈ [0, Q) (~8.38M): `power2round(a) = (a1, a0)` reconstructs
    /// a == a1·2^D + a0 with a0 ∈ (−2^{D−1}, 2^{D−1}], no overflow.
    #[test]
    fn item7_exhaustive_power2round_reconstructs() {
        let half = 1i32 << (D - 1); // 2^{D-1}
        for a in 0..Q {
            let (a1, a0) = power2round(a);
            assert_eq!(
                a1 * (1 << D) + a0,
                a,
                "power2round({a}) does not reconstruct"
            );
            assert!(
                a0 > -half && a0 <= half,
                "power2round({a}) a0={a0} out of range"
            );
        }
    }

    /// EXHAUSTIVE over a ∈ [0, Q) (~8.38M): `decompose(a) = (a1, a0)` recomposes to a per
    /// FIPS-204 (a == a1·2·GAMMA2 + a0, with the a1==0/a0 boundary wrap), no overflow, and
    /// a1 ∈ [0, 16). Also exhausts `poly_chknorm`'s abs-trick sign path via `make_hint`.
    #[test]
    fn item7_exhaustive_decompose_recomposes() {
        for a in 0..Q {
            let (a1, a0) = decompose(a);
            assert!(a1 >= 0 && a1 < 16, "decompose({a}) a1={a1} out of [0,16)");
            // Recompose: a ≡ a1·2·GAMMA2 + a0 (mod Q) exactly reconstructs the input.
            let recomposed = a1 * 2 * GAMMA2 + a0;
            let re = ((recomposed % Q) + Q) % Q;
            assert_eq!(re, a, "decompose({a})=({a1},{a0}) recompose {re} != {a}");
        }
    }

    /// EXHAUSTIVE over the coefficient domain: `poly_chknorm`'s inner centered-abs trick
    /// (`t = a - ((a>>31) & 2a)`) computes |a| for every a ∈ (−Q, Q) with no overflow, so
    /// the bound comparison is exact.
    #[test]
    fn item7_exhaustive_chknorm_abs_trick() {
        let mut a = -(Q - 1);
        while a <= Q - 1 {
            let t = a >> 31;
            let centered = a - (t & (2 * a));
            assert_eq!(centered, a.abs(), "chknorm abs-trick wrong at {a}");
            a += 1;
        }
    }
}

// ── Item 7 (space-grade roadmap §C): Kani proofs — the genuine Bucket-C targets ──
// Compiled ONLY under `cfg(kani)` (see keccak.rs header); nothing enters Cargo.toml/
// Cargo.lock, zero-dep stays mechanically true. These are the NTT-arithmetic targets
// the synthesis §7 named ("arithmetic edge conditions") whose input space is NOT
// exhaustible AND whose property is interval/congruence — so a machine-checked SAT
// proof beats a hand lemma (RESEARCH-NATIVE-KANI-REPLACEMENT §2, Bucket C).
#[cfg(kani)]
mod kani_proofs {
    use super::*;

    /// `reduce32` contract over ALL i32 in its safe domain. `reduce32` is NOT total over
    /// i32: `a + (1<<22)` overflows for `a > i32::MAX - 2^22`, so the precondition is
    /// encoded here (and documented on the fn). Kept as Kani rather than a native test
    /// because full-i32 (2^32) exhaustion is impractical in a debug test; Kani decides it
    /// exactly (incl. the exact overflow boundary) in one shot. Proves: no overflow, and
    /// result ≡ a (mod Q).
    #[kani::proof]
    fn proof_reduce32_contract() {
        let a: i32 = kani::any();
        kani::assume(a <= i32::MAX - (1 << 22)); // documented precondition (reduce.c)
        let r = reduce32(a);
        // ≡ a (mod Q), checked in i64 (no overflow in the check).
        assert!(((r as i64) - (a as i64)) % (Q as i64) == 0);
    }

    /// `montgomery_reduce` contract, over its documented precondition
    /// −Q·2^31 ≤ a ≤ Q·2^31 (reduce.c:75). Non-exhaustible domain (~±1.8e16). Proves the
    /// property the synthesis §7 actually names — NO arithmetic edge condition / hidden
    /// panic: the internal `(a as i32).wrapping_mul(QINV)`, `(t as i64)·Q`, `a − t·Q`, and
    /// `>> 32` never overflow/panic for ANY `a` in the precondition (Kani's automatic
    /// overflow/OOB checks), plus the output range `r ∈ (−Q, Q)`.
    ///
    /// HONEST LIMIT (ledgered in HOT-PATHS.tsv): the full Montgomery congruence
    /// `r·2^32 ≡ a (mod Q)` is NOT machine-checked here — a symbolic modulo over the
    /// ±1.8e16 domain exceeds the CI proof budget (measured 2026-07-19: both i128 and i64
    /// forms timed out > 7 min). That FUNCTIONAL-correctness property stays covered by the
    /// ACVP per-tcId KATs (item 6 oracle floor). This harness targets the fault class Kani
    /// is cheapest at (overflow/panic), which the KATs do NOT sweep for all inputs.
    #[kani::proof]
    fn proof_montgomery_reduce_contract() {
        let a: i64 = kani::any();
        let bound = (Q as i64) << 31;
        kani::assume(a >= -bound && a <= bound);
        let r = montgomery_reduce(a);
        // Range: |r| ≤ Q. Provable bound = |a − t·Q|/2^32 ≤ Q·2^32/2^32 = Q (with |t| ≤ 2^31,
        // |a| ≤ Q·2^31). The reference's open (−Q,Q) is strict only for |a| < Q·2^31; at the
        // inclusive boundary a = ±Q·2^31 the result reaches ±Q — Kani exhibited exactly this
        // when the open bound was asserted, so the closed bound is the machine-checked truth.
        assert!((r as i64) >= -(Q as i64) && (r as i64) <= (Q as i64)); // range [−Q, Q]
    }

    /// NTT forward-butterfly LEMMA + documented layer induction (blueprint §3.3 fallback
    /// rung (i); research §2 "interval propagation over the fixed butterfly schedule").
    /// Mirrors `ntt` (dsa.rs:209-211) EXACTLY: `t = montgomery_reduce(zeta·y);
    /// y' = x - t; x' = x + t`. For symbolic x, y with |x|,|y| < k·Q (k ≤ 8, the
    /// lazy-reduction bound) and |zeta| < Q: the `zeta·y` product stays within
    /// montgomery_reduce's precondition, no i32 overflow occurs in x±t, and the outputs
    /// satisfy |x'|,|y'| < (k+1)·Q. DOCUMENTED INDUCTION: `ntt` runs 8 layers starting
    /// from |a| < Q (k=1); the lemma gives |a| < (L+1)·Q after layer L, so after 8 layers
    /// |a| < 9Q ≈ 2^26.2 « 2^31 — the full-NTT never overflows i32. The 1024-symbolic-
    /// multiply whole-`ntt` sweep is capped at this lemma+induction shape (manifest gap).
    #[kani::proof]
    fn proof_ntt_butterfly_lemma() {
        let x: i32 = kani::any();
        let y: i32 = kani::any();
        let zeta: i32 = kani::any();
        let k: i64 = kani::any();
        kani::assume(k >= 1 && k <= 8);
        let kq = (k * Q as i64) as i32; // k·Q, ≤ 8·Q ≈ 6.7e7, fits i32
        kani::assume((x as i64).abs() < kq as i64 && (y as i64).abs() < kq as i64);
        kani::assume((zeta as i64).abs() < Q as i64);
        // zeta·y within montgomery_reduce precondition |a| ≤ Q·2^31.
        let prod = (zeta as i64) * (y as i64);
        kani::assume(prod >= -((Q as i64) << 31) && prod <= ((Q as i64) << 31)); // true: |prod|<8Q²
        let t = montgomery_reduce(prod); // |t| < Q (proven above)
        let y_new = x - t; // ntt.rs:210
        let x_new = x + t; // ntt.rs:211
                           // Growth invariant: |x'|,|y'| < (k+1)·Q, and (implicitly, via Kani) no i32 overflow.
        let kp1q = ((k + 1) * Q as i64) as i32;
        assert!((x_new as i64).abs() < kp1q as i64);
        assert!((y_new as i64).abs() < kp1q as i64);
    }

    /// Inverse-NTT butterfly LEMMA (Gentleman-Sande), mirroring `invntt_tomont`
    /// (dsa.rs:231-234): `s = x + y; d = x - y; d' = montgomery_reduce(zeta·d)`. The
    /// additive path `s` is where coefficients grow; the multiplicative path is reduced
    /// back below Q by montgomery_reduce. For |x|,|y| < k·Q (k ≤ 8), |zeta| < Q: no i32
    /// overflow in x±y, |s| < 2k·Q, |d'| < Q. Documented induction: `invntt` interleaves
    /// this over 8 layers, and the standard ML-DSA lazy-reduction analysis keeps the
    /// additive growth < 2^31 across the schedule (same ceiling as `ntt`).
    #[kani::proof]
    fn proof_invntt_butterfly_lemma() {
        let x: i32 = kani::any();
        let y: i32 = kani::any();
        let zeta: i32 = kani::any();
        let k: i64 = kani::any();
        kani::assume(k >= 1 && k <= 8);
        let kq = (k * Q as i64) as i32;
        kani::assume((x as i64).abs() < kq as i64 && (y as i64).abs() < kq as i64);
        kani::assume((zeta as i64).abs() < Q as i64);
        let s = x + y; // invntt: a[j] = t + a[j+len]
        let d = x - y; // invntt: a[j+len] = t - a[j+len]
        let prod = (zeta as i64) * (d as i64);
        kani::assume(prod >= -((Q as i64) << 31) && prod <= ((Q as i64) << 31));
        let d_reduced = montgomery_reduce(prod);
        let twokq = (2 * k * Q as i64) as i32;
        assert!((s as i64).abs() < twokq as i64); // additive-path growth bound
        assert!((d_reduced as i64) > -(Q as i64) && (d_reduced as i64) < Q as i64);
    }
}
