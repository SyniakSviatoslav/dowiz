//! Courier PII-at-rest encryption — ports `apps/api/src/lib/pii-cipher.ts` (threat-model asset A7).
//! AES-256-GCM, output layout `[IV(12)] || [ciphertext] || [authTag(16)]`, stored directly in the
//! `bytea` columns. The layout is byte-identical to Node's `encryptPII`, so a courier row written
//! by either stack decrypts on the other (cutover cross-stack readability).
//!
//! The 32-byte key is `COURIER_PII_ENCRYPTION_KEY` base64-decoded (pii-cipher.ts:13-16). Held in
//! the repo/handler layer, never in `domain`. Encryption is the only direction the S2 port needs
//! (redeem WRITES encrypted PII; the masked-email response is derived from the plaintext before
//! encryption), so `decrypt` is provided for completeness + the round-trip test but isn't wired
//! into a read path here.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use rand::RngCore;

const IV_LEN: usize = 12;
const TAG_LEN: usize = 16;

#[derive(Debug, thiserror::Error)]
pub enum PiiError {
    #[error("COURIER_PII_ENCRYPTION_KEY must decode to exactly 32 bytes")]
    BadKey,
    #[error("encryption failed")]
    Encrypt,
    #[error("invalid ciphertext")]
    Decrypt,
}

/// A loaded 32-byte AES-256-GCM key. Constructed once from the base64 env value.
#[derive(Clone)]
pub struct PiiCipher {
    key: [u8; 32],
}

impl PiiCipher {
    /// Load from the base64 `COURIER_PII_ENCRYPTION_KEY` (pii-cipher.ts:13).
    pub fn from_base64(b64: &str) -> Result<Self, PiiError> {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|_e| PiiError::BadKey)?;
        let key: [u8; 32] = bytes.try_into().map_err(|_e| PiiError::BadKey)?;
        Ok(PiiCipher { key })
    }

    /// `encryptPII` (pii-cipher.ts:42-54): `[IV][ct][tag]`. Empty plaintext → empty buffer.
    pub fn encrypt(&self, plaintext: &str) -> Result<Vec<u8>, PiiError> {
        if plaintext.is_empty() {
            return Ok(Vec::new());
        }
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.key));
        let mut iv = [0u8; IV_LEN];
        rand::thread_rng().fill_bytes(&mut iv);
        let nonce = Nonce::from_slice(&iv);
        // aes-gcm appends the 16-byte tag to the ciphertext, matching Node's `ct || authTag`.
        let ct_and_tag = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|_e| PiiError::Encrypt)?;
        let mut out = Vec::with_capacity(IV_LEN + ct_and_tag.len());
        out.extend_from_slice(&iv);
        out.extend_from_slice(&ct_and_tag);
        Ok(out)
    }

    /// `decryptPII` (pii-cipher.ts:63-94) for the raw-binary (non-legacy) format.
    pub fn decrypt(&self, buf: &[u8]) -> Result<String, PiiError> {
        if buf.len() < IV_LEN + TAG_LEN {
            return Err(PiiError::Decrypt);
        }
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.key));
        let (iv, ct_and_tag) = buf.split_at(IV_LEN);
        let nonce = Nonce::from_slice(iv);
        let plaintext = cipher
            .decrypt(nonce, ct_and_tag)
            .map_err(|_e| PiiError::Decrypt)?;
        String::from_utf8(plaintext).map_err(|_e| PiiError::Decrypt)
    }
}

/// `maskStr` (pii-mask.ts) — mask an email/identity for a response body (courier/auth.ts:145).
/// Keeps the first char + domain, masks the local-part middle: `a***@x.com`.
pub fn mask_str(s: &str) -> String {
    match s.split_once('@') {
        Some((local, domain)) if !local.is_empty() => {
            let first = &local[..1];
            format!("{first}***@{domain}")
        }
        _ => "***".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_cipher() -> PiiCipher {
        // A fixed 32-byte test key (never a real key).
        PiiCipher { key: [7u8; 32] }
    }

    #[test]
    fn encrypt_decrypt_round_trips() {
        let c = test_cipher();
        let ct = c.encrypt("courier@example.com").unwrap();
        // Layout: at least IV + tag + some ciphertext.
        assert!(ct.len() > IV_LEN + TAG_LEN);
        assert_eq!(c.decrypt(&ct).unwrap(), "courier@example.com");
    }

    #[test]
    fn empty_plaintext_is_empty_buffer() {
        let c = test_cipher();
        assert!(c.encrypt("").unwrap().is_empty());
    }

    #[test]
    fn ciphertext_is_nondeterministic_random_iv() {
        let c = test_cipher();
        let a = c.encrypt("x").unwrap();
        let b = c.encrypt("x").unwrap();
        assert_ne!(a, b, "random IV per encryption");
    }

    #[test]
    fn tampered_ciphertext_fails_auth() {
        let c = test_cipher();
        let mut ct = c.encrypt("secret").unwrap();
        let last = ct.len() - 1;
        ct[last] ^= 0xff;
        assert!(
            c.decrypt(&ct).is_err(),
            "GCM auth tag must reject tampering"
        );
    }

    #[test]
    fn from_base64_rejects_wrong_length_key() {
        use base64::Engine;
        let short = base64::engine::general_purpose::STANDARD.encode([1u8; 16]);
        assert!(matches!(
            PiiCipher::from_base64(&short),
            Err(PiiError::BadKey)
        ));
        let ok = base64::engine::general_purpose::STANDARD.encode([1u8; 32]);
        assert!(PiiCipher::from_base64(&ok).is_ok());
    }

    #[test]
    fn mask_str_masks_local_part() {
        assert_eq!(mask_str("courier@example.com"), "c***@example.com");
        assert_eq!(mask_str("no-at-sign"), "***");
    }
}
