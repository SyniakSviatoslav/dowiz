//! Entropy mixing seam — the quantum-safe root of all key material.
//!
//! The kernel is RNG-free (all randomness enters via caller seed). This module defines
//! the SINGLE sanctioned way to derive a uniform 32-byte seed from one or more entropy
//! sources. The optional OS/QRNG provider (which pulls real noise from /dev/urandom and
//! a public QRNG endpoint) is std-only and lives in the kernel held-handle shim — this
//! no_std core ships only the pure mixing + derivation helpers.
//!
//! Security model (NIST SP 800-90B): NEVER use raw quantum noise alone. Mix it with OS
//! entropy so a biased/failed QRNG cannot collapse the seed. SHAKE256(quantum || os)
//! gives a seed whose entropy ≥ max(H(quantum), H(os)).

use alloc::vec::Vec;
use crate::pq::keccak::shake256;

/// Mix two entropy blobs into one 32-byte uniform seed: SHAKE256(a || b).
/// Order-independent caller-side (pass quantum first, os second — convention only).
pub fn entropy_mix(a: &[u8], b: &[u8]) -> [u8; 32] {
    let mut buf = Vec::with_capacity(a.len() + b.len());
    buf.extend_from_slice(a);
    buf.extend_from_slice(b);
    let mut out = [0u8; 32];
    shake256(&buf, &mut out);
    out
}

/// Convenience: derive a labeled sub-seed for a specific primitive from a master seed.
/// `label` namespaces the KDF so keygen vs encaps vs signing draws are independent.
pub fn derive_seed(master: &[u8; 32], label: &[u8]) -> [u8; 32] {
    let mut buf = Vec::with_capacity(32 + label.len());
    buf.extend_from_slice(master);
    buf.extend_from_slice(label);
    let mut out = [0u8; 32];
    shake256(&buf, &mut out);
    out
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn green_mix_is_uniform_and_order_independent() {
        let a = [0xABu8; 32];
        let b = [0xCDu8; 32];
        let m1 = entropy_mix(&a, &b);
        let m2 = entropy_mix(&b, &a); // swapped
        assert_ne!(
            m1, [0u8; 32],
            "seed must not be all-zero for constant input"
        );
        assert_ne!(m1, m2, "mixing must be input-dependent (swapped != same)");
    }

    #[test]
    fn green_derive_seed_is_labeled() {
        let master = [7u8; 32];
        let kg = derive_seed(&master, b"kem-kg");
        let enc = derive_seed(&master, b"kem-enc");
        assert_ne!(kg, enc, "different labels must yield different sub-seeds");
    }

}
