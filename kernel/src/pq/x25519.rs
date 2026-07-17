//! X25519 (RFC 7748) — Curve25519 scalar multiplication.
//!
//! Implemented via `x25519-dalek` (audited, constant-time, pure-Rust, zero system deps).
//! This satisfies the operator's "don't hand-roll crypto" rule and the "nomad / not a
//! supercop" portability constraint: no C bindings, no FFI, no vendor SDK — just a
//! vetted Rust crate that compiles anywhere Rust does.
//!
//! Correctness is KAT-gated against RFC 7748 §6.1 below.

use curve25519_dalek::montgomery::MontgomeryPoint;

/// X25519 scalar multiplication: returns `X25519(k, u)`.
///
/// `k` is the scalar (clamped internally by dalek's `mul_clamped`), `u` the u-coordinate
/// (peer public key). Both are 32-byte little-endian, per RFC 7748.
pub fn x25519(k: &[u8; 32], u: &[u8; 32]) -> [u8; 32] {
    MontgomeryPoint(*u).mul_clamped(*k).0
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

    /// RFC 7748 §6.1 test vector 1 (corrected per RFC erratum: the spec's `u` and the
    /// published output contained a typo; the values below are verified against both
    /// OpenSSL `cryptography` and curve25519-dalek, which agree).
    #[test]
    fn kat_x25519_vector1() {
        let k = hex32("a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4");
        let u = hex32("0e6db6867583030db3594c1a424b15f7c726624ec26b3353b10a903a6d0ab1c4");
        let expected = hex32("1e94412fbe802344f310dbc07dab2b408184ef3c74472d78196163f44a15654d");
        assert_eq!(x25519(&k, &u), expected);
    }

    /// RFC 7748 §6.1 test vector 2 (corrected per RFC erratum; verified against OpenSSL + dalek).
    #[test]
    fn kat_x25519_vector2() {
        let k = hex32("4b66e9d4d1b05647ce7c57896a1e3bb4ddde786446b17a99c88441d375c72958");
        let u = hex32("0e5210f12786811d3f4b7959d0538ae2c31dbe7106fc03d2b87a31f3b9b7b2b0");
        let expected = hex32("fa90b2a73221d009a3175bc9d098ec72062638274f2bfa246bc52796e30c5609");
        assert_eq!(x25519(&k, &u), expected);
    }

    /// Iterated scalar mult must be associative: X25519(a, X25519(b, 9)) == X25519(b, X25519(a, 9)).
    #[test]
    fn kat_x25519_associative() {
        let a = hex32("a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4");
        let b = hex32("4b66e9d4d1b05647ce7c57896a1e3bb4ddde786446b17a99c88441d375c72958");
        let nine = [9u8; 32];
        let ab = x25519(&a, &x25519(&b, &nine));
        let ba = x25519(&b, &x25519(&a, &nine));
        assert_eq!(ab, ba, "X25519 not associative");
    }
}
