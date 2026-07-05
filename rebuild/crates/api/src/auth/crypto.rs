//! Small crypto helpers — the ported equivalents of the `crypto.*` one-liners scattered through
//! the Node auth files. Kept pure/synchronous and unit-tested so the hashing/format decisions are
//! pinned without a live DB.
//!
//! ## sha256 over hex/base64url STRINGS, never raw bytes (cutover gate ii, REV / Q10-ii)
//! Node computes `crypto.createHash('sha256').update(x).digest('hex')` where `x` is the opaque
//! token STRING (the 64-hex refresh token, the base64url track code). The hash input is the
//! string's UTF-8 bytes, and the output is a lowercase hex string. `sha256_hex` reproduces that
//! EXACTLY — a Node-minted refresh token must rotate on Rust and vice-versa, which only holds if
//! both stacks hash the identical string bytes to the identical hex digest.

use argon2::{Argon2, PasswordHash, PasswordVerifier};
use rand::RngCore;
use sha2::{Digest, Sha256};

/// `crypto.createHash('sha256').update(input).digest('hex')` — sha256 of the input STRING's UTF-8
/// bytes, lowercase-hex encoded (Q10-ii hash-format parity).
pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

/// `crypto.randomBytes(32).toString('hex')` — a 64-char lowercase-hex opaque token (owner refresh
/// token, OAuth handoff material). CSPRNG.
pub fn random_hex_32() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// `crypto.randomUUID()` — a fresh v4 UUID (family ids, opaque OAuth codes, telegram tokens).
pub fn random_uuid() -> uuid::Uuid {
    uuid::Uuid::new_v4()
}

/// argon2id verify against a stored PHC string (`argon2.verify(hash, password)`,
/// courier/auth.ts:261, local uses argon2 too). Returns `true` iff the password matches. The Rust
/// `argon2` crate reads the same `$argon2id$...` PHC format Node's `argon2` lib writes, so existing
/// courier/owner hashes verify unchanged (cutover gate — cross-stack hash compatibility).
pub fn argon2_verify(phc_hash: &str, password: &str) -> bool {
    match PasswordHash::new(phc_hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

/// Constant-time string compare (`crypto.timingSafeEqual`, dev-guard.ts:35-41,
/// local.ts:11-16). False on any length mismatch; never short-circuits on content.
pub fn timing_safe_eq(a: &str, b: &str) -> bool {
    let ab = a.as_bytes();
    let bb = b.as_bytes();
    if ab.len() != bb.len() {
        return false;
    }
    // Bitwise-OR the XOR of every byte pair; the result is 0 iff every pair matched. Reading both
    // buffers fully (no early return) keeps the timing independent of WHERE a mismatch is.
    let mut diff: u8 = 0;
    for (x, y) in ab.iter().zip(bb.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_hex_matches_a_known_vector() {
        // echo -n "abc" | sha256sum → ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
        assert_eq!(
            sha256_hex("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn sha256_hex_of_a_64hex_token_is_stable_and_hex() {
        // Q10-ii: a Node-minted 64-hex refresh token hashes to the same digest on both stacks.
        let token = "a".repeat(64);
        let digest = sha256_hex(&token);
        assert_eq!(digest.len(), 64);
        assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
        // Determinism: same string → same digest, every time.
        assert_eq!(digest, sha256_hex(&token));
    }

    #[test]
    fn random_hex_32_is_64_chars_and_unique() {
        let a = random_hex_32();
        let b = random_hex_32();
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b, "CSPRNG output must differ");
    }

    #[test]
    fn timing_safe_eq_matches_only_on_equal_strings() {
        assert!(timing_safe_eq("secret", "secret"));
        assert!(!timing_safe_eq("secret", "secreT"));
        assert!(!timing_safe_eq("secret", "secret-longer"));
        assert!(!timing_safe_eq("", "x"));
        assert!(timing_safe_eq("", ""));
    }

    #[test]
    fn argon2_verify_round_trips_a_known_hash() {
        use argon2::Argon2;
        use argon2::password_hash::{PasswordHasher, SaltString};
        let salt = SaltString::from_b64("YWJjZGVmZ2hpamtsbW5vcA").unwrap();
        let hash = Argon2::default()
            .hash_password(b"correct-horse", &salt)
            .unwrap()
            .to_string();
        assert!(argon2_verify(&hash, "correct-horse"));
        assert!(!argon2_verify(&hash, "wrong-password"));
        assert!(!argon2_verify("not-a-phc-string", "anything"));
    }
}
