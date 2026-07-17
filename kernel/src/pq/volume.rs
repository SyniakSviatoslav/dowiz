//! At-rest AES-256-GCM volume crypto — P2 (hybrid KEM wrap + ML-DSA envelope).
//!
//! Threat model (MANIFESTO / D4): data stored on disk / at rest in a node's
//! custody must be confidential against a future quantum adversary. We therefore
//! do NOT store a raw AES volume key. Instead we use a KEM-DEM construction:
//!
//!   1. The caller supplies a fresh 32-byte volume key `vk` (RNG-free core, C10).
//!   2. The volume key is WRAPPED (encrypted) to the recipient node's *hybrid*
//!      public key. Concretely: `hybrid_encaps` yields a shared secret `ss`
//!      (X25519 + ML-KEM-768, BOTH mandatory); the data-encryption key is
//!      `dk = KDF(ss)`; the volume key is sealed as `AES-256-GCM(vk, dk)`.
//!      Only the holder of the recipient's hybrid secret key can recover `ss`,
//!      hence only they can unwrap `vk`.
//!   3. The plaintext blob is AES-256-GCM-encrypted under `vk`.
//!   4. The whole `VolumeSeal` (hybrid wrap + wrapped-vk + blob ciphertext +
//!      nonces) is ML-DSA-65 signed via [`crate::pq::envelope::seal`] so the
//!      at-rest artifact's integrity/authorship is provable (reuses the L1 seam).
//!
//! Opening requires (a) the recipient's hybrid secret key to recover `vk`
//! (both KEM legs must verify — the RED gate), and (b) the ML-DSA signature.
//!
//! ponytail: AES-256-GCM is the `aes-gcm` crate (pure-Rust, zero system deps,
//! feature `std` enabled). The volume key, nonces, and KEM entropy are all
//! caller-supplied — no OS RNG is used anywhere. We do NOT touch money
//! constants or the D0–D9 decision surface.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use serde::{Deserialize, Serialize};

use crate::pq::envelope::{self, SignedEnvelope};
use crate::pq::hybrid::{hybrid_decaps, hybrid_encaps, HybridCiphertext, HybridKeypair};
use crate::pq::keccak::shake256;

/// AES-256-GCM requires a 32-byte (256-bit) content-encryption key.
pub const VOLUME_KEY_LEN: usize = 32;
/// AES-GCM standard 96-bit (12-byte) nonce.
pub const NONCE_LEN: usize = 12;
/// Caller-supplied entropy length for the hybrid encaps legs.
pub const ENC_ENTROPY_LEN: usize = 32;

/// Errors for the at-rest volume operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VolumeError {
    /// Hybrid decapsulation failed: wrong recipient / tampered wrap / degraded leg.
    WrapRejected,
    /// AES-256-GCM decrypt failed: wrong key or tampered ciphertext.
    DecryptFailed,
    /// ML-DSA envelope verification failed (hash mismatch or bad signature).
    Envelope(envelope::EnvelopeError),
}

/// The signed, at-rest artifact: a hybrid volume-key wrap + AES-GCM blob.
///
/// The volume key `vk` itself never appears in the clear — it is wrapped to the
/// recipient via the hybrid KEM (field `wrapped`, which carries the hybrid
/// shared-secret encaps) and then sealed under the derived DEM key (field
/// `wrapped_vk`). `nonce` + `ciphertext` are the AES-256-GCM output of the
/// plaintext under `vk`. The entire structure is ML-DSA-65 signed (field `seal`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VolumeSeal {
    /// Hybrid encaps of the shared secret `ss` to the recipient's hybrid pubkey.
    pub wrapped: HybridCiphertext,
    /// Nonce for the volume-key wrap (`AES-256-GCM(vk, dk)`).
    pub wrap_nonce: [u8; NONCE_LEN],
    /// Volume key `vk` sealed under the DEM key `dk = KDF(ss)`.
    pub wrapped_vk: Vec<u8>,
    /// AES-256-GCM nonce (12 bytes) for the blob encryption under `vk`.
    pub nonce: [u8; NONCE_LEN],
    /// AES-256-GCM ciphertext (includes the 16-byte GCM tag) of the plaintext.
    pub ciphertext: Vec<u8>,
    /// ML-DSA-65 envelope over the canonical wire bytes (`wire_bytes`).
    pub seal: SignedEnvelope,
}

impl VolumeSeal {
    /// Canonical wire bytes signed by the ML-DSA envelope. Stable, length-prefixed
    /// serialization of every integrity-relevant field (so sign/verify agree).
    fn wire_bytes(&self) -> Vec<u8> {
        let mut v = Vec::new();
        let kem_ct = &self.wrapped.kem_ct;
        v.extend_from_slice(&(kem_ct.len() as u32).to_le_bytes());
        v.extend_from_slice(kem_ct);
        v.extend_from_slice(&self.wrapped.x_ephemeral);
        v.extend_from_slice(&self.wrapped.confirm);
        v.extend_from_slice(&self.wrap_nonce);
        v.extend_from_slice(&(self.wrapped_vk.len() as u32).to_le_bytes());
        v.extend_from_slice(&self.wrapped_vk);
        v.extend_from_slice(&self.nonce);
        v.extend_from_slice(&(self.ciphertext.len() as u32).to_le_bytes());
        v.extend_from_slice(&self.ciphertext);
        v
    }
}

/// Derive the 32-byte data-encryption key from the hybrid shared secret.
/// `dk = SHAKE256(ss)` — a one-way KDF binding the data key to the KEM output.
fn derive_dk(ss: &[u8; 32]) -> [u8; 32] {
    let mut dk = [0u8; 32];
    shake256(ss, &mut dk);
    dk
}

/// Seal a plaintext blob at rest for `recipient`.
///
/// * `vk`        — caller-supplied 32-byte volume key (RNG-free, C10).
/// * `blob_nonce`— caller-supplied 12-byte AES-GCM nonce for the blob (NEVER reuse with `vk`).
/// * `wrap_nonce`— caller-supplied 12-byte AES-GCM nonce for the volume-key wrap.
/// * `eph_seed`  — caller entropy for the ephemeral X25519 leg.
/// * `m`         — caller entropy for the ML-KEM encaps leg.
/// * `signer_sk` — ML-DSA-65 signer secret key (seals integrity/authorship).
/// * `sign_rnd`  — caller entropy for the ML-DSA signature.
#[allow(clippy::too_many_arguments)]
pub fn seal_volume(
    plaintext: &[u8],
    recipient: &HybridKeypair,
    vk: &[u8; VOLUME_KEY_LEN],
    blob_nonce: &[u8; NONCE_LEN],
    wrap_nonce: &[u8; NONCE_LEN],
    eph_seed: &[u8; ENC_ENTROPY_LEN],
    m: &[u8; ENC_ENTROPY_LEN],
    signer_sk: &[u8],
    sign_rnd: &[u8; envelope::ENTROPY_LEN],
) -> VolumeSeal {
    // 1. Hybrid-encapsulate to the recipient; derive the DEM wrapping key from the ss.
    let (wrapped, ss) = hybrid_encaps(recipient, m, eph_seed);
    let dk = derive_dk(&ss);

    // 2. Wrap the volume key under the DEM key (the actual KEM "wrap").
    let wrap_cipher = Aes256Gcm::new_from_slice(&dk).expect("dk is exactly 32 bytes");
    let wrapped_vk = wrap_cipher
        .encrypt(Nonce::from_slice(wrap_nonce), vk.as_slice())
        .expect("aes-gcm wrap cannot fail for valid key+nonce");

    // 3. AES-256-GCM-encrypt the plaintext under the volume key.
    let blob_cipher = Aes256Gcm::new_from_slice(vk).expect("vk is exactly 32 bytes");
    let ciphertext = blob_cipher
        .encrypt(Nonce::from_slice(blob_nonce), plaintext)
        .expect("aes-gcm encrypt cannot fail for valid key+nonce");

    // 4. Build the wire bytes and ML-DSA-sign the artifact.
    let mut seal = VolumeSeal {
        wrapped,
        wrap_nonce: *wrap_nonce,
        wrapped_vk,
        nonce: *blob_nonce,
        ciphertext,
        seal: SignedEnvelope {
            payload: Vec::new(),
            content_hash: [0u8; envelope::HASH_LEN],
            sig: Vec::new(),
        },
    };
    let wire = seal.wire_bytes();
    seal.seal = envelope::seal(&wire, signer_sk, sign_rnd);
    seal
}

/// Open a [`VolumeSeal`] given the recipient's hybrid secret key and the
/// signer's ML-DSA public key. Returns the recovered plaintext only if BOTH the
/// hybrid wrap opens (RED gate) AND the ML-DSA envelope verifies.
pub fn open_volume(
    seal: &VolumeSeal,
    recipient_sk: &HybridKeypair,
    signer_pk: &[u8],
) -> Result<Vec<u8>, VolumeError> {
    // (a) RED gate: recover the hybrid shared secret — BOTH KEM legs must verify;
    //     the wrong recipient / tampered wrap / degraded-leg is rejected here.
    let ss = hybrid_decaps(recipient_sk, &seal.wrapped).map_err(|_| VolumeError::WrapRejected)?;
    let dk = derive_dk(&ss);

    // (b) Unwrap the volume key under the DEM key. Fails if the wrap was not for us.
    let wrap_cipher = Aes256Gcm::new_from_slice(&dk).map_err(|_| VolumeError::DecryptFailed)?;
    let vk = wrap_cipher
        .decrypt(
            Nonce::from_slice(&seal.wrap_nonce),
            seal.wrapped_vk.as_slice(),
        )
        .map_err(|_| VolumeError::DecryptFailed)?;

    // (c) Verify the ML-DSA envelope over the canonical wire bytes. envelope::open
    //     checks the content_hash AND the ML-DSA signature, and we assert the
    //     recovered wire equals our locally derived wire (defends against a
    //     substituted envelope payload post-sign, belt-and-suspenders).
    let wire = seal.wire_bytes();
    let recovered_wire = envelope::open(&seal.seal, signer_pk).map_err(VolumeError::Envelope)?;
    if recovered_wire != wire {
        return Err(VolumeError::Envelope(envelope::EnvelopeError::HashMismatch));
    }

    // (d) AES-256-GCM-decrypt the blob under the recovered volume key.
    let blob_cipher = Aes256Gcm::new_from_slice(&vk).map_err(|_| VolumeError::DecryptFailed)?;
    blob_cipher
        .decrypt(Nonce::from_slice(&seal.nonce), seal.ciphertext.as_slice())
        .map_err(|_| VolumeError::DecryptFailed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pq::envelope;

    /// Build a deterministic hybrid keypair from two fixed 32-byte seeds.
    fn hk(x: u8, k: u8) -> HybridKeypair {
        let xs = [x; 32];
        let ks = [k; 32];
        crate::pq::hybrid::hybrid_keygen(&xs, &ks)
    }

    #[test]
    fn green_seal_open_same_recipient() {
        let recipient = hk(1, 2);
        let (signer_pk, signer_sk) = envelope::new_identity(&[3u8; 32]);

        let plaintext = b"at-rest order manifest: zone kyiv-7 / 12 pizzas";
        let vk = [7u8; VOLUME_KEY_LEN];
        let blob_nonce = [8u8; NONCE_LEN];
        let wrap_nonce = [9u8; NONCE_LEN];

        let seal = seal_volume(
            plaintext,
            &recipient,
            &vk,
            &blob_nonce,
            &wrap_nonce,
            &[4u8; ENC_ENTROPY_LEN],
            &[5u8; ENC_ENTROPY_LEN],
            &signer_sk,
            &[6u8; envelope::ENTROPY_LEN],
        );

        // Sanity: the volume key is NOT stored in the clear.
        assert_ne!(
            seal.wrapped_vk,
            vk.to_vec(),
            "volume key must be wrapped, not plaintext"
        );
        assert_ne!(
            seal.ciphertext,
            plaintext.to_vec(),
            "blob must be ciphertext"
        );

        let recovered = open_volume(&seal, &recipient, &signer_pk)
            .expect("same recipient + valid signer must open");
        assert_eq!(recovered, plaintext.to_vec());
    }

    #[test]
    fn green_seal_uses_hybrid_wrap() {
        // Confirm the wrap went through the hybrid KEM: a recipient built from the
        // SAME seeds recovers the plaintext, while the volume key is derived end-to-end.
        let recipient = hk(11, 12);
        let (signer_pk, signer_sk) = envelope::new_identity(&[13u8; 32]);
        let plaintext = b"confidential volume blob";
        let seal = seal_volume(
            plaintext,
            &recipient,
            &[14u8; VOLUME_KEY_LEN],
            &[15u8; NONCE_LEN],
            &[15u8; NONCE_LEN],
            &[16u8; ENC_ENTROPY_LEN],
            &[17u8; ENC_ENTROPY_LEN],
            &signer_sk,
            &[18u8; envelope::ENTROPY_LEN],
        );
        let recovered = open_volume(&seal, &recipient, &signer_pk).unwrap();
        assert_eq!(recovered, plaintext.to_vec());
    }

    #[test]
    fn red_wrong_recipient_rejected() {
        // Encapsulate the volume key to peer A, then try to open with peer B's
        // hybrid key. hybrid_decaps MUST fail (both legs won't verify) → Err.
        let peer_a = hk(20, 21);
        let peer_b = hk(22, 23);
        let (signer_pk, signer_sk) = envelope::new_identity(&[24u8; 32]);

        let seal = seal_volume(
            b"secret",
            &peer_a, // wrapped to A
            &[25u8; VOLUME_KEY_LEN],
            &[26u8; NONCE_LEN],
            &[26u8; NONCE_LEN],
            &[27u8; ENC_ENTROPY_LEN],
            &[28u8; ENC_ENTROPY_LEN],
            &signer_sk,
            &[29u8; envelope::ENTROPY_LEN],
        );

        let res = open_volume(&seal, &peer_b, &signer_pk); // B tries to open
        assert_eq!(
            res,
            Err(VolumeError::WrapRejected),
            "wrong recipient must fail"
        );
    }

    #[test]
    fn red_tampered_ciphertext_rejected() {
        let recipient = hk(30, 31);
        let (signer_pk, signer_sk) = envelope::new_identity(&[32u8; 32]);
        let mut seal = seal_volume(
            b"secret-blob",
            &recipient,
            &[33u8; VOLUME_KEY_LEN],
            &[34u8; NONCE_LEN],
            &[34u8; NONCE_LEN],
            &[35u8; ENC_ENTROPY_LEN],
            &[36u8; ENC_ENTROPY_LEN],
            &signer_sk,
            &[37u8; envelope::ENTROPY_LEN],
        );
        // Flip a byte in the blob ciphertext after signing (breaks AES-GCM tag AND
        // the envelope content_hash). Opening must fail — never yield plaintext.
        seal.ciphertext[0] ^= 0xFF;
        let res = open_volume(&seal, &recipient, &signer_pk);
        assert!(
            res.is_err(),
            "tampered ciphertext must be rejected: {:?}",
            res
        );
    }

    #[test]
    fn red_wrong_signer_rejected() {
        let recipient = hk(40, 41);
        let (_real_pk, signer_sk) = envelope::new_identity(&[42u8; 32]);
        let (other_pk, _other_sk) = envelope::new_identity(&[43u8; 32]);

        let seal = seal_volume(
            b"secret-blob",
            &recipient,
            &[44u8; VOLUME_KEY_LEN],
            &[45u8; NONCE_LEN],
            &[45u8; NONCE_LEN],
            &[46u8; ENC_ENTROPY_LEN],
            &[47u8; ENC_ENTROPY_LEN],
            &signer_sk,
            &[48u8; envelope::ENTROPY_LEN],
        );
        // Verify against a different signer pubkey → envelope fails.
        let res = open_volume(&seal, &recipient, &other_pk);
        assert!(
            matches!(res, Err(VolumeError::Envelope(_))),
            "wrong signer must be rejected: {:?}",
            res
        );
    }

    #[test]
    fn green_deterministic_wrap_roundtrip() {
        // Two seals to the same recipient with identical inputs MUST be bit-identical
        // (deterministic crypto core, C10), and both open identically.
        let recipient = hk(50, 51);
        let (signer_pk, signer_sk) = envelope::new_identity(&[52u8; 32]);
        let seal1 = seal_volume(
            b"same",
            &recipient,
            &[53u8; VOLUME_KEY_LEN],
            &[54u8; NONCE_LEN],
            &[54u8; NONCE_LEN],
            &[55u8; ENC_ENTROPY_LEN],
            &[56u8; ENC_ENTROPY_LEN],
            &signer_sk,
            &[57u8; envelope::ENTROPY_LEN],
        );
        let seal2 = seal_volume(
            b"same",
            &recipient,
            &[53u8; VOLUME_KEY_LEN],
            &[54u8; NONCE_LEN],
            &[54u8; NONCE_LEN],
            &[55u8; ENC_ENTROPY_LEN],
            &[56u8; ENC_ENTROPY_LEN],
            &signer_sk,
            &[57u8; envelope::ENTROPY_LEN],
        );
        assert_eq!(seal1, seal2, "identical inputs → identical seal");
        let r1 = open_volume(&seal1, &recipient, &signer_pk).unwrap();
        let r2 = open_volume(&seal2, &recipient, &signer_pk).unwrap();
        assert_eq!(r1, r2);
    }
}
