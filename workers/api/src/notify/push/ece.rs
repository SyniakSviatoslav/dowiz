//! RFC 8291 MESSAGE ENCRYPTION FOR WEB PUSH, `aes128gcm` (RFC 8188), PURE.
//!
//! A push service (FCM, Mozilla autopush, Apple) carries the message and must
//! not be able to read it: the browser gave us its public key (`p256dh`) and a
//! 16-byte secret (`auth`) when it subscribed, and only that browser can open
//! what this makes. The order number and the words "your order is ready" are
//! all that is ever put inside, and even that is sealed.
//!
//! THE STEPS, as RFC 8291 §3.3-3.4 and §4 write them:
//!
//!   ecdh_secret = ECDH(as_private, ua_public)                    (P-256, 32 B)
//!   PRK_key     = HKDF-Extract(salt = auth_secret, ecdh_secret)
//!   key_info    = "WebPush: info" 0x00 ua_public as_public
//!   IKM         = HKDF-Expand(PRK_key, key_info, 32)
//!   PRK         = HKDF-Extract(salt, IKM)
//!   CEK         = HKDF-Expand(PRK, "Content-Encoding: aes128gcm" 0x00, 16)
//!   NONCE       = HKDF-Expand(PRK, "Content-Encoding: nonce" 0x00, 12)
//!   body        = salt(16) rs(u32 BE) idlen(1) as_public(65)
//!                 AES-128-GCM(CEK, NONCE, plaintext 0x02)
//!
//! ONE RECORD, so the padding delimiter is 0x02 and the plaintext is bounded
//! well under the 4096-octet record size every push service accepts.
//!
//! THE EPHEMERAL KEY AND THE SALT ARE ARGUMENTS of [`encrypt_with`], which is
//! how the RFC 8291 §5 example is checked byte for byte in `ece/tests.rs`;
//! [`encrypt`] draws both from the platform CSPRNG and is the one the rail calls.
//! A key or a salt reused across messages would be a real weakness, so nothing
//! outside the tests can choose them.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes128Gcm, Nonce};
use hkdf::Hkdf;
use p256::elliptic_curve::sec1::ToEncodedPoint;
use p256::{PublicKey, SecretKey};
use sha2::Sha256;

/// The record size written into the header (RFC 8188 §2.1).
pub const RECORD_SIZE: u32 = 4096;
/// An uncompressed P-256 point: 0x04, x, y.
pub const POINT_LEN: usize = 65;
/// The browser's authentication secret (RFC 8291 §3.2).
pub const AUTH_LEN: usize = 16;
/// salt + rs + idlen + keyid.
pub const HEADER_LEN: usize = 16 + 4 + 1 + POINT_LEN;
/// AES-GCM's tag.
const TAG_LEN: usize = 16;
/// The most a message may carry. One record holds RECORD_SIZE - 16 (tag) - 1
/// (delimiter) = 4079 octets, but FCM refuses a BODY over 4096 and the header is
/// 86 more, so the honest bound is the body's: 4096 - 86 - 16 - 1.
pub const PLAINTEXT_MAX: usize = 4096 - HEADER_LEN - TAG_LEN - 1;

/// Why a message could not be sealed. Every one is the subscription's fault
/// or the message's, never a transient: the rail drops the entry on these.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EceError {
    /// `p256dh` is not a P-256 point.
    BadPublicKey,
    /// `auth` is not 16 octets.
    BadAuth,
    /// The message is longer than one record may be.
    TooLong(usize),
    /// The ephemeral secret is not a valid scalar (tests only reach this).
    BadSecret,
    /// No entropy from the platform.
    NoEntropy,
}

/// Seal `plain` for the browser that owns `ua_public` and `auth`, with a
/// fresh ephemeral key and salt.
pub fn encrypt(plain: &[u8], ua_public: &[u8], auth: &[u8]) -> Result<Vec<u8>, EceError> {
    let mut salt = [0u8; 16];
    getrandom::getrandom(&mut salt).map_err(|_| EceError::NoEntropy)?;
    // A uniformly random 32-byte string is a valid scalar with probability
    // 1 - 2^-32; the loop is for that remainder, and is bounded.
    for _ in 0..4 {
        let mut sk = [0u8; 32];
        getrandom::getrandom(&mut sk).map_err(|_| EceError::NoEntropy)?;
        match encrypt_with(plain, ua_public, auth, &sk, &salt) {
            Err(EceError::BadSecret) => continue,
            other => return other,
        }
    }
    Err(EceError::NoEntropy)
}

/// The same with the ephemeral secret and the salt given. PUBLIC FOR THE
/// TESTS ONLY in spirit: a caller that reuses either breaks the scheme.
pub fn encrypt_with(plain: &[u8], ua_public: &[u8], auth: &[u8], as_secret: &[u8; 32], salt: &[u8; 16]) -> Result<Vec<u8>, EceError> {
    if plain.len() > PLAINTEXT_MAX {
        return Err(EceError::TooLong(plain.len()));
    }
    if auth.len() != AUTH_LEN {
        return Err(EceError::BadAuth);
    }
    if ua_public.len() != POINT_LEN || ua_public[0] != 0x04 {
        return Err(EceError::BadPublicKey);
    }
    let ua = PublicKey::from_sec1_bytes(ua_public).map_err(|_| EceError::BadPublicKey)?;
    let sk = SecretKey::from_slice(as_secret).map_err(|_| EceError::BadSecret)?;
    let as_public = sk.public_key().to_encoded_point(false);
    let as_public = as_public.as_bytes();
    let shared = p256::ecdh::diffie_hellman(sk.to_nonzero_scalar(), ua.as_affine());

    let (cek, nonce) = derive(shared.raw_secret_bytes().as_slice(), ua_public, as_public, auth, salt);

    let mut record = Vec::with_capacity(plain.len() + 1);
    record.extend_from_slice(plain);
    record.push(0x02);
    let cipher = Aes128Gcm::new_from_slice(&cek).map_err(|_| EceError::BadSecret)?;
    let sealed = cipher.encrypt(Nonce::from_slice(&nonce), record.as_slice()).map_err(|_| EceError::TooLong(plain.len()))?;

    let mut out = Vec::with_capacity(HEADER_LEN + sealed.len());
    out.extend_from_slice(salt);
    out.extend_from_slice(&RECORD_SIZE.to_be_bytes());
    out.push(POINT_LEN as u8);
    out.extend_from_slice(as_public);
    out.extend_from_slice(&sealed);
    Ok(out)
}

/// The content-encryption key and nonce from the shared secret (RFC 8291 §3.4).
pub fn derive(ecdh_secret: &[u8], ua_public: &[u8], as_public: &[u8], auth: &[u8], salt: &[u8]) -> ([u8; 16], [u8; 12]) {
    let mut key_info = Vec::with_capacity(14 + 2 * POINT_LEN);
    key_info.extend_from_slice(b"WebPush: info\0");
    key_info.extend_from_slice(ua_public);
    key_info.extend_from_slice(as_public);
    let mut ikm = [0u8; 32];
    Hkdf::<Sha256>::new(Some(auth), ecdh_secret)
        .expand(&key_info, &mut ikm)
        .expect("32 octets is a valid HKDF-SHA256 length");
    let prk = Hkdf::<Sha256>::new(Some(salt), &ikm);
    let mut cek = [0u8; 16];
    let mut nonce = [0u8; 12];
    prk.expand(b"Content-Encoding: aes128gcm\0", &mut cek).expect("16 octets is a valid HKDF-SHA256 length");
    prk.expand(b"Content-Encoding: nonce\0", &mut nonce).expect("12 octets is a valid HKDF-SHA256 length");
    (cek, nonce)
}

/// The receiving side, from RFC 8291 §3.4 and RFC 8188 §2: what a browser
/// does with what [`encrypt`] made. TESTS ONLY -- the Worker never opens a message.
#[cfg(test)]
pub fn open(body: &[u8], ua_private: &[u8], auth: &[u8]) -> Vec<u8> {
    let salt = &body[..16];
    assert_eq!(u32::from_be_bytes(body[16..20].try_into().unwrap()), RECORD_SIZE);
    assert_eq!(body[20] as usize, POINT_LEN);
    let as_public = &body[21..HEADER_LEN];
    let sk = SecretKey::from_slice(ua_private).unwrap();
    let ua_public = sk.public_key().to_encoded_point(false);
    let peer = PublicKey::from_sec1_bytes(as_public).unwrap();
    let shared = p256::ecdh::diffie_hellman(sk.to_nonzero_scalar(), peer.as_affine());
    let (cek, nonce) = derive(shared.raw_secret_bytes().as_slice(), ua_public.as_bytes(), as_public, auth, salt);
    let mut plain = Aes128Gcm::new_from_slice(&cek).unwrap().decrypt(Nonce::from_slice(&nonce), &body[HEADER_LEN..]).unwrap();
    assert_eq!(plain.pop(), Some(0x02), "one record ends in the 0x02 delimiter");
    plain
}

#[cfg(test)]
mod tests;
