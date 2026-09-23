//! pq/backup_seal — seal a backup to a PUBLIC key: hybrid X25519 + ML-KEM-768
//! KEM, AES-256-GCM DEM. Pure (no RNG, no I/O): the caller supplies entropy.
//!
//! The nightly off-site copy (`workers/api/src/cloud.rs`) seals with this; the
//! opener (`tools/seal-open`) opens with it. The Worker holds only the public
//! key; the secret key never exists there. No new primitive is written here:
//! the KEM is `pq::hybrid` (both legs mandatory, `hybrid_encaps_to` refuses a
//! degraded recipient), the DEM is `pq::aes_gcm`, the KDF is `pq::keccak`.
//!
//! SEALED FILE LAYOUT (version 1), all offsets in bytes:
//!
//! | offset | len  | field                                              |
//! |--------|------|----------------------------------------------------|
//! | 0      | 7    | magic `DWZSEAL`                                    |
//! | 7      | 1    | version = 1                                        |
//! | 8      | 1    | content kind (0 = as-is, 1 = gzip) — opaque here    |
//! | 9      | 1088 | ML-KEM-768 ciphertext                              |
//! | 1097   | 32   | X25519 ephemeral public key                        |
//! | 1129   | 32   | hybrid key-confirmation tag                        |
//! | 1161   | 12   | AES-GCM nonce                                      |
//! | 1173   | n+16 | AES-256-GCM(plaintext) ‖ 16-byte tag               |
//!
//! The AES key is `SHAKE256("dowiz/backup-seal/v1" ‖ ss ‖ header[0..1173])`,
//! so every header byte is bound into the key: changing any of them (magic,
//! version, kind, KEM ct, eph key, tag, nonce) makes the GCM tag fail, and the
//! KEM fields additionally fail the hybrid confirmation.
//!
//! PUBLIC KEY TEXT: `dwzseal-pk1:` + hex(x25519_pk[32] ‖ mlkem768_ek[1184]).
//! SECRET KEY FILE: `DWZSSK1\n` + x25519 seed[32] + ML-KEM seed[32] (72 bytes);
//! the keypair is re-derived from the seeds with `hybrid::hybrid_keygen`.

use alloc::string::String;
use alloc::vec::Vec;

use crate::pq::aes_gcm::Aes256Gcm;
use crate::pq::hybrid::{hybrid_decaps, hybrid_encaps_to, hybrid_keygen, HybridCiphertext, HybridKeypair};
use crate::pq::keccak::shake256;
use crate::pq::kem::{self, KEM768_CT_LEN, KEM768_EK_LEN};

pub const MAGIC: &[u8; 7] = b"DWZSEAL";
pub const VERSION: u8 = 1;
pub const HEADER_LEN: usize = 7 + 1 + 1 + KEM768_CT_LEN + 32 + 32 + 12;
const TAG_LEN: usize = 16;
pub const PK_PREFIX: &str = "dwzseal-pk1:";
pub const SK_MAGIC: &[u8; 8] = b"DWZSSK1\n";
pub const SK_FILE_LEN: usize = 8 + 32 + 32;
const KDF_LABEL: &[u8] = b"dowiz/backup-seal/v1";

/// Why a seal or open was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SealError {
    /// The public-key text is not `dwzseal-pk1:<hex>` of the right length.
    PublicKeyFormat,
    /// The public key would degrade a leg (ek_check failed / low-order X25519).
    PublicKeyRefused(&'static str),
    /// The secret-key file is not `DWZSSK1\n` + 64 bytes.
    SecretKeyFormat,
    /// Shorter than a header plus a GCM tag.
    Truncated(usize),
    /// Wrong magic or unknown version.
    NotASeal,
    /// The hybrid KEM refused (tampered KEM fields, wrong secret key).
    KemRejected(&'static str),
    /// AES-GCM tag mismatch (tampered header / nonce / body / tag).
    Decrypt,
}

/// The recipient's public half, as the Worker holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealPublic {
    pub x_pk: [u8; 32],
    pub kem_pk: Vec<u8>,
}

/// The per-seal entropy (caller supplies it: `getrandom` in the Worker).
pub struct SealEntropy {
    pub m: [u8; 32],
    pub eph: [u8; 32],
    pub nonce: [u8; 12],
}

impl SealPublic {
    /// Parse `dwzseal-pk1:<hex>` and run both leg checks, so a malformed key is
    /// refused where it is read rather than producing a file nobody can open.
    pub fn parse(text: &str) -> Result<Self, SealError> {
        let hex = text.trim().strip_prefix(PK_PREFIX).ok_or(SealError::PublicKeyFormat)?;
        let raw = unhex(hex).ok_or(SealError::PublicKeyFormat)?;
        if raw.len() != 32 + KEM768_EK_LEN {
            return Err(SealError::PublicKeyFormat);
        }
        let pk = SealPublic { x_pk: raw[..32].try_into().unwrap(), kem_pk: raw[32..].to_vec() };
        // Trial encapsulation with fixed entropy: the same checks `seal` runs.
        hybrid_encaps_to(&pk.x_pk, &pk.kem_pk, &[0u8; 32], &[1u8; 32]).map_err(SealError::PublicKeyRefused)?;
        Ok(pk)
    }

    pub fn encode(&self) -> String {
        let mut s = String::from(PK_PREFIX);
        for b in self.x_pk.iter().chain(self.kem_pk.iter()) {
            s.push(char::from_digit((b >> 4) as u32, 16).unwrap());
            s.push(char::from_digit((b & 15) as u32, 16).unwrap());
        }
        s
    }
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    let b = s.as_bytes();
    if b.len() % 2 != 0 {
        return None;
    }
    let nib = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
    (0..b.len()).step_by(2).map(|i| Some(nib(b[i])? << 4 | nib(b[i + 1])?)).collect()
}

/// A keypair from two 32-byte seeds: (secret-key file bytes, public key).
pub fn keygen(x_seed: &[u8; 32], kem_seed: &[u8; 32]) -> (Vec<u8>, SealPublic) {
    let kp = hybrid_keygen(x_seed, kem_seed);
    let mut file = Vec::with_capacity(SK_FILE_LEN);
    file.extend_from_slice(SK_MAGIC);
    file.extend_from_slice(x_seed);
    file.extend_from_slice(kem_seed);
    (file, SealPublic { x_pk: kp.x_pk, kem_pk: kp.kem_pk })
}

/// Re-derive the hybrid keypair from a secret-key file.
pub fn secret_from_file(file: &[u8]) -> Result<HybridKeypair, SealError> {
    if file.len() != SK_FILE_LEN || &file[..8] != SK_MAGIC {
        return Err(SealError::SecretKeyFormat);
    }
    Ok(hybrid_keygen(file[8..40].try_into().unwrap(), file[40..72].try_into().unwrap()))
}

fn aes_key(ss: &[u8; 32], header: &[u8]) -> [u8; 32] {
    let mut input = Vec::with_capacity(KDF_LABEL.len() + 32 + header.len());
    input.extend_from_slice(KDF_LABEL);
    input.extend_from_slice(ss);
    input.extend_from_slice(header);
    let mut k = [0u8; 32];
    shake256(&input, &mut k);
    k
}

/// Seal `plaintext` to `pk`. `kind` is recorded in the header for the opener.
pub fn seal(pk: &SealPublic, kind: u8, plaintext: &[u8], e: &SealEntropy) -> Result<Vec<u8>, SealError> {
    let (ct, ss) = hybrid_encaps_to(&pk.x_pk, &pk.kem_pk, &e.m, &e.eph).map_err(SealError::PublicKeyRefused)?;
    let mut out = Vec::with_capacity(HEADER_LEN + plaintext.len() + TAG_LEN);
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    out.push(kind);
    out.extend_from_slice(&ct.kem_ct);
    out.extend_from_slice(&ct.x_ephemeral);
    out.extend_from_slice(&ct.confirm);
    out.extend_from_slice(&e.nonce);
    debug_assert_eq!(out.len(), HEADER_LEN);
    let body = Aes256Gcm::new(&aes_key(&ss, &out)).encrypt(&e.nonce, plaintext);
    out.extend_from_slice(&body);
    Ok(out)
}

/// Open a sealed file with the recipient's keypair. Returns (kind, plaintext).
/// Every malformed or tampered input is an `Err`; nothing here panics.
pub fn open(sk: &HybridKeypair, sealed: &[u8]) -> Result<(u8, Vec<u8>), SealError> {
    if sealed.len() < HEADER_LEN + TAG_LEN {
        return Err(SealError::Truncated(sealed.len()));
    }
    if &sealed[..7] != MAGIC || sealed[7] != VERSION {
        return Err(SealError::NotASeal);
    }
    let (header, body) = sealed.split_at(HEADER_LEN);
    let at = 9 + KEM768_CT_LEN;
    let ct = HybridCiphertext {
        kem_ct: header[9..at].to_vec(),
        x_ephemeral: header[at..at + 32].try_into().unwrap(),
        confirm: header[at + 32..at + 64].try_into().unwrap(),
    };
    let nonce: [u8; 12] = header[at + 64..at + 76].try_into().unwrap();
    let ss = hybrid_decaps(sk, &ct).map_err(SealError::KemRejected)?;
    let plain = Aes256Gcm::new(&aes_key(&ss, header)).decrypt(&nonce, body).map_err(|_| SealError::Decrypt)?;
    Ok((header[8], plain))
}

// `kem` is named so the layout's lengths are visibly the KEM's own constants.
const _: () = assert!(HEADER_LEN == 1173 && kem::KEM768_CT_LEN == 1088);

#[cfg(test)]
mod tests;
