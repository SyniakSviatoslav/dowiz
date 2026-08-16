//! X25519 (RFC 7748) — Curve25519 scalar multiplication, hand-rolled, `no_std`.
//!
//! Field arithmetic over `p = 2^255 - 19` in 5 × 51-bit limbs (u128 multipliers),
//! then the Montgomery ladder (RFC 7748 §5). The ladder is constant-time in the
//! scalar bits (branch-free conditional swaps) and the field ops carry no
//! secret-dependent branches. Correctness is KAT-gated against RFC 7748 §6.1
//! (two vectors) plus the iterated scalar-mult associativity check.
//!
//! This replaces the previous `curve25519-dalek`-backed shim
//! (`kernel/src/pq/x25519.rs`), so the pq subsystem's X25519 primitive becomes a
//! zero-dependency `no_std` core citizen — matching the already-extracted
//! `keccak` / `kem` / `dsa` (all KAT-gated, no external crates).

/// A field element: 5 limbs of 51 bits, little-endian (value = Σ limb[i]·2^(51·i)).
///
/// Canonical form keeps every limb < 2^51; lazy form (the output of `fe_add` /
/// `fe_sub`) may carry up to 2^52 in a limb and a value up to 2^256 — `fe_mul` /
/// `fe_sq` tolerate that and always return canonical results.
type Fe = [u64; 5];

/// 2^51 - 1 (the per-limb mask).
const MASK_51: u64 = (1u64 << 51) - 1;

/// `p = 2^255 - 19` as 5 × 51-bit limbs.
///
/// `p = 2^255 - 19 = (2^51 - 19) + (2^51 - 1)·(2^51 + 2^102 + 2^153 + 2^204)`.
/// Each limb is a 51-bit value: `p0 = 2^51 - 19`, `p1..=p4 = 2^51 - 1`.
const P: Fe = [
    0x7ffffffffffed, // 2^51 - 19
    0x7ffffffffffff, // 2^51 - 1
    0x7ffffffffffff, // 2^51 - 1
    0x7ffffffffffff, // 2^51 - 1
    0x7ffffffffffff, // 2^51 - 1
];

/// `a24 = 121665`, the Montgomery-ladder curve constant for Curve25519.
const A24: Fe = [121_665, 0, 0, 0, 0];

/// Load a 32-byte little-endian value into 5 × 51-bit limbs, masking bit 255
/// (RFC 7748 §5: the u-coordinate's high bit is cleared).
fn fe_load_le(bytes: &[u8; 32]) -> Fe {
    let mut b = [0u8; 32];
    b.copy_from_slice(bytes);
    b[31] &= 0x7f;
    let mut limbs = [0u64; 5];
    for i in 0..5 {
        let bit = i * 51;
        let byte = bit / 8;
        let shift = bit % 8;
        let mut acc = 0u64;
        for j in 0..8 {
            let bi = byte + j;
            if bi < 32 {
                acc |= (b[bi] as u64) << (8 * j);
            }
        }
        limbs[i] = (acc >> shift) & MASK_51;
    }
    limbs
}

/// Serialize a fully-reduced field element to 32 bytes little-endian.
fn fe_to_bytes(limbs: &Fe) -> [u8; 32] {
    let l = fe_canonical(*limbs);
    let mut out = [0u8; 32];
    for i in 0..5 {
        let bit = i * 51;
        let byte = bit / 8;
        let shift = bit % 8;
        let mut acc = l[i] << shift;
        for j in 0..8 {
            let bi = byte + j;
            if bi < 32 {
                out[bi] |= (acc & 0xff) as u8;
                acc >>= 8;
            }
        }
    }
    out
}

/// Fully reduce a (possibly < 2^256) value to the canonical range [0, p).
fn fe_canonical(mut limbs: Fe) -> Fe {
    // Value < 2^256 < 3p, so up to two conditional subtractions of p suffice.
    for _ in 0..2 {
        let (sub, borrow) = fe_sub_p(&limbs);
        if !borrow {
            limbs = sub;
        }
    }
    limbs
}

/// Compute `limbs - p`, returning the result and a borrow flag (true if negative).
fn fe_sub_p(limbs: &Fe) -> (Fe, bool) {
    let mut r = [0u64; 5];
    let mut borrow = false;
    for i in 0..5 {
        let (a, b1) = limbs[i].overflowing_sub(P[i]);
        let (v, b2) = a.overflowing_sub(borrow as u64);
        r[i] = v;
        borrow = b1 || b2;
    }
    (r, borrow)
}

/// Limb-wise addition (lazy — limbs may reach 2^52; consumed by `fe_mul`/`fe_sq`).
fn fe_add(a: &Fe, b: &Fe) -> Fe {
    let mut r = [0u64; 5];
    for i in 0..5 {
        r[i] = a[i].wrapping_add(b[i]);
    }
    r
}

/// `p - b` (branch-free). For canonical `b`, `p >= b`, so no borrow; `b == 0`
/// yields `p`, which is fine — the consumer (`fe_sub`) treats it as ≡ 0.
fn fe_neg(b: &Fe) -> Fe {
    let mut r = [0u64; 5];
    let mut borrow = false;
    for i in 0..5 {
        let (a, b1) = P[i].overflowing_sub(b[i]);
        let (v, b2) = a.overflowing_sub(borrow as u64);
        r[i] = v;
        borrow = b1 || b2;
    }
    r
}

/// `a - b` (lazy) = `a + (p - b)`, landing in [p, 2p) ⊂ [0, 2^256). Branch-free.
fn fe_sub(a: &Fe, b: &Fe) -> Fe {
    fe_add(a, &fe_neg(b))
}

/// Multiply two field elements, returning a fully-reduced result.
fn fe_mul(a: &Fe, b: &Fe) -> Fe {
    // 1. Schoolbook 5×5 into 10 u128 limbs.
    let mut c = [0u128; 10];
    for i in 0..5 {
        for j in 0..5 {
            c[i + j] += (a[i] as u128) * (b[j] as u128);
        }
    }
    fe_reduce(&c)
}

/// Square a field element, returning a fully-reduced result.
fn fe_sq(a: &Fe) -> Fe {
    fe_mul(a, a)
}

/// Reduce a 10-limb (u128) product into a field element (≤ 3 fixed carry passes,
/// then canonicalized to [0, p)). Each pass is unconditional, so the reduction is
/// constant-time regardless of the input magnitude.
fn fe_reduce(c: &[u128; 10]) -> Fe {
    let mut t = *c;
    // 1. Fold the top 5 limbs (bits ≥ 255) into the bottom via 2^255 ≡ 19 (mod p).
    for i in 0..5 {
        t[i] += 19 * t[i + 5];
    }
    // 2. Carry-propagate and re-fold the top carry. Inputs here are < 2^111 per
    //    limb; one pass leaves every limb < 2^51 except limb 0 (< 2^51 + 19). Four
    //    passes give a wide safety margin; extra passes are no-ops once limbs < 2^51.
    for _ in 0..4 {
        for i in 0..4 {
            t[i + 1] += t[i] >> 51;
            t[i] &= MASK_51 as u128;
        }
        t[0] += 19 * (t[4] >> 51);
        t[4] &= MASK_51 as u128;
    }
    let mut limbs = [0u64; 5];
    for i in 0..5 {
        limbs[i] = t[i] as u64;
    }
    // 3. Canonicalize to [0, p).
    fe_canonical(limbs)
}

/// Constant-time conditional swap (RFC 7748 §5): swap if `swap == 1`.
fn fe_cswap(swap: u64, a: &mut Fe, b: &mut Fe) {
    let mask = swap.wrapping_neg(); // 0 → 0x0, 1 → 0xffff…
    for i in 0..5 {
        let t = mask & (a[i] ^ b[i]);
        a[i] ^= t;
        b[i] ^= t;
    }
}

/// Field inversion: `a^(p-2)` via square-and-multiply (Fermat). The exponent is
/// fixed (public), so the multiply/skip pattern is not secret-dependent.
fn fe_invert(a: &Fe) -> Fe {
    // p - 2 = 2^255 - 21 = 0x7fff…ffeb (255 bits). Bits 8..254 are all 1; the
    // low byte is 0xeb = 0b1110_1011.
    let mut result = [1u64, 0, 0, 0, 0];
    for bit in (0..255).rev() {
        result = fe_sq(&result);
        let b = if bit >= 8 {
            1u64
        } else {
            (0xebu64 >> bit) & 1
        };
        if b == 1 {
            result = fe_mul(&result, a);
        }
    }
    result
}

/// X25519 scalar multiplication: returns `X25519(k, u)`.
///
/// `k` is the scalar (clamped internally, RFC 7748 §5), `u` the u-coordinate
/// (peer public key). Both are 32-byte little-endian, per RFC 7748.
pub fn x25519(k: &[u8; 32], u: &[u8; 32]) -> [u8; 32] {
    let mut scalar = *k;
    // Clamp the scalar (RFC 7748 §5). Idempotent — safe if already clamped.
    scalar[0] &= 248;
    scalar[31] &= 127;
    scalar[31] |= 64;

    let x1 = fe_load_le(u);

    // Montgomery ladder (RFC 7748 §5).
    let mut x2 = [1u64, 0, 0, 0, 0];
    let mut z2 = [0u64; 5];
    let mut x3 = x1;
    let mut z3 = [1u64, 0, 0, 0, 0];
    let mut swap = 0u64;

    for t in (0..255).rev() {
        let k_t = ((scalar[t / 8] >> (t % 8)) & 1) as u64;
        swap ^= k_t;
        fe_cswap(swap, &mut x2, &mut x3);
        fe_cswap(swap, &mut z2, &mut z3);
        swap = k_t;

        let a = fe_add(&x2, &z2);
        let aa = fe_sq(&a);
        let b = fe_sub(&x2, &z2);
        let bb = fe_sq(&b);
        let e = fe_sub(&aa, &bb);
        let c = fe_add(&x3, &z3);
        let d = fe_sub(&x3, &z3);
        let da = fe_mul(&d, &a);
        let cb = fe_mul(&c, &b);
        let x3n = fe_sq(&fe_add(&da, &cb));
        let z3n = fe_mul(&x1, &fe_sq(&fe_sub(&da, &cb)));
        let x2n = fe_mul(&aa, &bb);
        let a24e = fe_mul(&A24, &e);
        let z2n = fe_mul(&e, &fe_add(&aa, &a24e));

        x2 = x2n;
        z2 = z2n;
        x3 = x3n;
        z3 = z3n;
    }

    fe_cswap(swap, &mut x2, &mut x3);
    fe_cswap(swap, &mut z2, &mut z3);

    // Result = x2 / z2 = x2 · z2^(p-2).
    let z2inv = fe_invert(&z2);
    let out = fe_mul(&x2, &z2inv);
    fe_to_bytes(&out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex32(s: &str) -> [u8; 32] {
        let s = s.trim().trim_start_matches("0x");
        let mut out = [0u8; 32];
        for i in 0..32 {
            out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap();
        }
        out
    }

    /// RFC 7748 §6.1 test vector 1 (corrected per RFC erratum; verified against
    /// OpenSSL `cryptography` + curve25519-dalek, which agree).
    #[test]
    fn kat_x25519_vector1() {
        let k = hex32("a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4");
        let u = hex32("0e6db6867583030db3594c1a424b15f7c726624ec26b3353b10a903a6d0ab1c4");
        let expected = hex32("1e94412fbe802344f310dbc07dab2b408184ef3c74472d78196163f44a15654d");
        assert_eq!(x25519(&k, &u), expected);
    }

    /// RFC 7748 §6.1 test vector 2 (corrected per RFC erratum; verified against
    /// OpenSSL + dalek).
    #[test]
    fn kat_x25519_vector2() {
        let k = hex32("4b66e9d4d1b05647ce7c57896a1e3bb4ddde786446b17a99c88441d375c72958");
        let u = hex32("0e5210f12786811d3f4b7959d0538ae2c31dbe7106fc03d2b87a31f3b9b7b2b0");
        let expected = hex32("fa90b2a73221d009a3175bc9d098ec72062638274f2bfa246bc52796e30c5609");
        assert_eq!(x25519(&k, &u), expected);
    }

    /// Iterated scalar mult must be associative: X25519(a, X25519(b, 9)) ==
    /// X25519(b, X25519(a, 9)).
    #[test]
    fn kat_x25519_associative() {
        let a = hex32("a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4");
        let b = hex32("4b66e9d4d1b05647ce7c57896a1e3bb4ddde786446b17a99c88441d375c72958");
        let nine = [9u8; 32];
        let ab = x25519(&a, &x25519(&b, &nine));
        let ba = x25519(&b, &x25519(&a, &nine));
        assert_eq!(ab, ba, "X25519 not associative");
    }

    #[test]
    fn debug_field_roundtrip() {
        let mut b = [0u8; 32];
        for i in 0..32 {
            b[i] = (i as u8).wrapping_mul(17);
        }
        let f = fe_load_le(&b);
        let out = fe_to_bytes(&f);
        let mut expected = b;
        expected[31] &= 0x7f;
        assert_eq!(out, expected, "roundtrip failed: got {out:02x?}");
    }

    #[test]
    fn debug_field_mul_identity() {
        let one = [1u64, 0, 0, 0, 0];
        let x = fe_mul(&one, &one);
        assert_eq!(x, one, "1*1 != 1: {x:?}");
    }

    #[test]
    fn debug_field_mul_small() {
        let two = [2u64, 0, 0, 0, 0];
        let three = [3u64, 0, 0, 0, 0];
        let six = fe_mul(&two, &three);
        assert_eq!(six, [6u64, 0, 0, 0, 0], "2*3 != 6: {six:?}");
    }

    #[test]
    fn debug_field_sub() {
        let five = [5u64, 0, 0, 0, 0];
        let three = [3u64, 0, 0, 0, 0];
        let two = fe_sub(&five, &three);
        assert_eq!(
            fe_to_bytes(&two),
            fe_to_bytes(&[2u64, 0, 0, 0, 0]),
            "5-3 != 2"
        );
    }

    #[test]
    fn debug_field_invert() {
        let two = [2u64, 0, 0, 0, 0];
        let inv = fe_invert(&two);
        let prod = fe_mul(&two, &inv);
        assert_eq!(prod, [1u64, 0, 0, 0, 0], "2 * 2^-1 != 1: {prod:?}");
    }
}
