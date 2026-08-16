//! AES-256-GCM (NIST SP 800-38D): `no_std` shim.
//!
//! The primitive is the hand-rolled, `no_std`, zero-dependency implementation in
//! `dowiz_core::pq::aes_gcm` (AES-256 table S-box + GHASH in GF(2^128)). This shim
//! re-exports it so `crate::pq::aes_gcm::Aes256Gcm` call sites stay unchanged.
//!
//! A differential test cross-checks the hand-rolled core against the RustCrypto
//! `aes-gcm` crate on a deterministic vector set (empty-AAD, variable lengths) — the
//! bootstrap validation that retires the `aes-gcm` *runtime* dependency from the `pq`
//! feature (it then lives only as a `[dev-dependencies]` guard, so
//! `cargo tree -e no-dev --features pq` is aes-gcm-free).

pub use dowiz_core::pq::aes_gcm::{AeadError, Aes256Gcm};

#[cfg(test)]
mod tests {
    use super::Aes256Gcm;

    /// Differential bootstrap: the hand-rolled core vs RustCrypto `aes-gcm` on a
    /// deterministic vector set + edge cases (empty plaintext, non-block-aligned
    /// lengths, zero key/nonce).
    #[test]
    fn kat_aes256gcm_differential_vs_aesgcm() {
        use aes_gcm::aead::{Aead, KeyInit};
        use aes_gcm::{Aes256Gcm as RefGcm, Nonce};

        let mut cases: Vec<([u8; 32], [u8; 12], Vec<u8>)> = Vec::new();
        for i in 0..16u8 {
            let mut key = [0u8; 32];
            let mut nonce = [0u8; 12];
            let len = (i as usize) * 7 + 3; // mix of non-block-aligned lengths
            let mut pt = vec![0u8; len];
            for j in 0..32 {
                key[j] = i.wrapping_mul(31).wrapping_add(j as u8);
            }
            for j in 0..12 {
                nonce[j] = i.wrapping_mul(17).wrapping_add(j as u8 + 1);
            }
            for (j, b) in pt.iter_mut().enumerate() {
                *b = i.wrapping_mul(53).wrapping_add(j as u8);
            }
            cases.push((key, nonce, pt));
        }
        // Edge cases: empty plaintext, zero key, zero nonce, full 16-byte block.
        cases.push(([0u8; 32], [0u8; 12], Vec::new()));
        cases.push(([0u8; 32], [0u8; 12], vec![0u8; 16]));
        cases.push(([0xff; 32], [0xff; 12], vec![0xaa; 64]));

        for (key, nonce, pt) in &cases {
            let mine = Aes256Gcm::new(key);
            let sealed = mine.encrypt(nonce, pt);

            let refc = RefGcm::new_from_slice(key).unwrap();
            let ref_sealed = refc.encrypt(Nonce::from_slice(nonce), pt.as_ref()).unwrap();
            assert_eq!(sealed, ref_sealed, "encrypt mismatch key={key:02x?} nonce={nonce:02x?}");

            let opened = mine.decrypt(nonce, &sealed).expect("decrypt must succeed");
            assert_eq!(&opened, pt);

            // Tamper the tag → core must reject.
            let mut bad = sealed.clone();
            let n = bad.len();
            bad[n - 1] ^= 0x01;
            assert!(mine.decrypt(nonce, &bad).is_err(), "tampered tag accepted");
        }
    }
}
