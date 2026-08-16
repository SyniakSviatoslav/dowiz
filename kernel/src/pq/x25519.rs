//! X25519 (RFC 7748) — Curve25519 scalar multiplication: `no_std` shim.
//!
//! The primitive is the hand-rolled, `no_std`, zero-dependency Montgomery-ladder
//! implementation in `dowiz_core::pq::x25519` (field arithmetic over `p = 2^255 - 19`
//! in 5 × 51-bit limbs; branch-free ladder). Correctness is KAT-gated in core against
//! RFC 7748 §6.1 (two vectors) + the iterated scalar-mult associativity check. This
//! shim re-exports it so `crate::pq::x25519::x25519` call sites stay unchanged.
//!
//! A differential test cross-checks the hand-rolled core against the incumbent
//! `curve25519-dalek` (`mul_clamped`) on a fixed deterministic vector set plus edge
//! cases — the bootstrap validation that retired the dalek *runtime* dependency from
//! the `pq` feature. dalek now lives only as a `[dev-dependencies]` guard, so
//! `cargo tree -e no-dev --features pq` is dalek-free.

pub use dowiz_core::pq::x25519::x25519;

#[cfg(test)]
mod tests {
    use super::x25519;

    /// Differential bootstrap: the hand-rolled core vs `curve25519-dalek`
    /// `mul_clamped` on a deterministic vector set + edge cases. This is the
    /// regression guard that pins the zero-dep reimplementation to the audited
    /// incumbent; both clamp the scalar and mask the u-coordinate identically
    /// (RFC 7748 §5).
    #[test]
    fn kat_x25519_differential_vs_dalek() {
        use curve25519_dalek::montgomery::MontgomeryPoint;

        let mut cases: Vec<([u8; 32], [u8; 32])> = Vec::with_capacity(20);
        for i in 0..16u8 {
            let mut k = [0u8; 32];
            let mut u = [0u8; 32];
            for j in 0..32 {
                k[j] = i.wrapping_mul(37).wrapping_add(j as u8);
                u[j] = i.wrapping_mul(91).wrapping_add(31u8.wrapping_sub(j as u8));
            }
            cases.push((k, u));
        }
        // Edge cases: zero scalar, all-ones scalar, zero u-coordinate (→ all-zero
        // output), the RFC basepoint u = 9, and the high-bit-set u (bit 255 must be
        // masked by both sides).
        cases.push(([0u8; 32], [9u8; 32]));
        cases.push(([0xff; 32], [9u8; 32]));
        cases.push(([1u8; 32], [0u8; 32]));
        cases.push(([0u8; 32], [0u8; 32]));
        let mut high_bit_u = [9u8; 32];
        high_bit_u[31] |= 0x80;
        cases.push(([7u8; 32], high_bit_u));

        for (k, u) in &cases {
            let core = x25519(k, u);
            let dalek = MontgomeryPoint(*u).mul_clamped(*k).0;
            assert_eq!(core, dalek, "mismatch k={k:02x?} u={u:02x?}");
        }
    }
}
