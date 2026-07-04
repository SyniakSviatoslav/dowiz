//! Upload token — REV-S4-2 (breaker H1/M4, counsel Q2 RESOLVED): "token-proxy-PUT preserving the
//! FE contract shape". The old `product-media.ts` presign route signed a real SigV4 query-string
//! URL straight to R2 (`@aws-sdk/s3-request-presigner`); the shipped Rust R2 client only has
//! `aws-sign-v4` (header-form SigV4, breaker H1: NOT extendable to the query-string presign
//! variant "for ~1 function" — that claim was priced wrong, see breaker-findings.md H1). Instead
//! of hand-rolling that crypto (rejected outright), the presign route now returns a URL pointing
//! at OUR OWN proxy-PUT endpoint, authorized by this opaque, stateless, HMAC-SHA256-signed token
//! — scoped to exactly `{key, content_type, max_bytes}`, TTL 300 s (parity with
//! `PRESIGN_TTL_SECONDS`, `product-media.ts:29`).
//!
//! ## Honest single-use caveat (the resolution requires this be stated, not hidden)
//! Fly runs MULTIPLE machines and this token carries NO server-side state (no in-memory table,
//! no DB row — the DB schema is FROZEN for this port) — so it is **stateless and therefore
//! REPLAYABLE within its 300 s TTL**: nothing here stops the SAME token being used to PUT twice.
//! This is bounded, not ignored: every `product-media` key is **content-addressed**
//! (`${locId}/${productId}/${subKind}/${sha256.slice(0,12)}.${ext}`, carried verbatim from
//! `product-media.ts:165` — Q-KEY-DERIVE/Q-SHA-DECLARED) and the object store's `put` is a full
//! overwrite, so replaying the same token just re-writes the SAME key with (at worst) the SAME
//! declared-shape bytes — an idempotent no-op, not a privilege escalation. A replay that
//! substitutes DIFFERENT same-type bytes under that key is the exact "same-type substitution"
//! residual risk the original packet already priced for the real presigned-URL design (§5:
//! "a same-type substitution is not [caught]") — this port does not make that residual risk any
//! wider, it only relocates who signs the URL.
//!
//! ## What the token is NOT
//! It is not a bearer credential for the API — the proxy-PUT route this token authorizes is
//! deliberately mounted OUTSIDE the `bearer_and_dev_gate`/`OwnerClaimsExt` stack (see
//! `routes/media_upload.rs` module doc): exactly like a real presigned URL, POSSESSION of a
//! valid, unexpired token is the entire authorization model for that one PUT, independent of
//! whether the owner's own JWT session is still valid. It is scoped narrowly enough (one key,
//! one declared content-type, one byte ceiling, 300 s) that this is an acceptable trade — the
//! same trade the original SigV4 presigned-URL design already made.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

/// `PRESIGN_TTL_SECONDS` parity (`product-media.ts:29`) — "≤5 min — short-lived PUT window
/// (contract)."
pub const TOKEN_TTL_SECONDS: u64 = 300;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadTokenClaims {
    pub key: String,
    pub content_type: String,
    pub max_bytes: u64,
    pub expires_at: u64,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TokenError {
    #[error("malformed upload token")]
    Malformed,
    #[error("upload token signature is invalid")]
    BadSignature,
    #[error("upload token has expired")]
    Expired,
}

/// HMAC-SHA256 signer/verifier over a `\0`-joined canonical claims string. The secret comes from
/// `config::MediaConfig::upload_token_secret_hex` (boot-guard-validated hex, ≥32 bytes) — never a
/// raw `std::env::var` at this layer.
#[derive(Clone)]
pub struct UploadTokenSigner {
    secret: Vec<u8>,
}

impl UploadTokenSigner {
    pub fn new(secret: Vec<u8>) -> Self {
        UploadTokenSigner { secret }
    }

    pub fn from_hex(hex_secret: &str) -> Result<Self, hex::FromHexError> {
        Ok(UploadTokenSigner::new(hex::decode(hex_secret)?))
    }

    /// Mints a token scoped to `key`/`content_type`/`max_bytes`, expiring `TOKEN_TTL_SECONDS`
    /// from `now` (unix seconds) — `now` is an explicit parameter (not read internally) so tests
    /// never depend on wall-clock timing.
    pub fn mint(&self, key: &str, content_type: &str, max_bytes: u64, now: u64) -> String {
        let claims = UploadTokenClaims {
            key: key.to_string(),
            content_type: content_type.to_string(),
            max_bytes,
            expires_at: now.saturating_add(TOKEN_TTL_SECONDS),
        };
        self.encode(&claims)
    }

    /// Convenience wrapper over [`Self::mint`] using the real wall clock — the one call site route
    /// handlers should use; `mint` itself stays clock-free for tests.
    pub fn mint_now(&self, key: &str, content_type: &str, max_bytes: u64) -> String {
        self.mint(key, content_type, max_bytes, unix_now())
    }

    fn encode(&self, claims: &UploadTokenClaims) -> String {
        let payload = canonical_payload(
            &claims.key,
            &claims.content_type,
            claims.max_bytes,
            claims.expires_at,
        );
        let signature = self.sign(payload.as_bytes());
        format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(payload.as_bytes()),
            URL_SAFE_NO_PAD.encode(signature)
        )
    }

    /// Verifies signature + TTL against `now` (unix seconds, explicit for the same reason as
    /// `mint`) and returns the scoped claims on success. The CALLER (the proxy-PUT handler) is
    /// responsible for checking the actual request against `key`/`content_type`/`max_bytes` —
    /// this function only proves the token itself is authentic and unexpired.
    pub fn verify(&self, token: &str, now: u64) -> Result<UploadTokenClaims, TokenError> {
        let (payload_b64, sig_b64) = token.split_once('.').ok_or(TokenError::Malformed)?;
        let payload_bytes = URL_SAFE_NO_PAD
            .decode(payload_b64)
            .map_err(|_base64_err| TokenError::Malformed)?;
        let sig_bytes = URL_SAFE_NO_PAD
            .decode(sig_b64)
            .map_err(|_base64_err| TokenError::Malformed)?;

        self.verify_signature(&payload_bytes, &sig_bytes)?;

        let payload_str =
            std::str::from_utf8(&payload_bytes).map_err(|_utf8_err| TokenError::Malformed)?;
        let mut parts = payload_str.split('\0');
        let key = parts.next().ok_or(TokenError::Malformed)?.to_string();
        let content_type = parts.next().ok_or(TokenError::Malformed)?.to_string();
        let max_bytes: u64 = parts
            .next()
            .ok_or(TokenError::Malformed)?
            .parse()
            .map_err(|_parse_err| TokenError::Malformed)?;
        let expires_at: u64 = parts
            .next()
            .ok_or(TokenError::Malformed)?
            .parse()
            .map_err(|_parse_err| TokenError::Malformed)?;
        if parts.next().is_some() {
            return Err(TokenError::Malformed);
        }

        if expires_at < now {
            return Err(TokenError::Expired);
        }

        Ok(UploadTokenClaims {
            key,
            content_type,
            max_bytes,
            expires_at,
        })
    }

    /// Convenience wrapper over [`Self::verify`] using the real wall clock.
    pub fn verify_now(&self, token: &str) -> Result<UploadTokenClaims, TokenError> {
        self.verify(token, unix_now())
    }

    fn sign(&self, payload: &[u8]) -> Vec<u8> {
        #[allow(
            clippy::expect_used,
            reason = "HMAC accepts a key of any length (it's hashed down if oversized) — \
                      `new_from_slice` only fails for a fixed-output MAC needing an EXACT key \
                      size, which HMAC never requires"
        )]
        let mut mac =
            HmacSha256::new_from_slice(&self.secret).expect("HMAC accepts any key length");
        mac.update(payload);
        mac.finalize().into_bytes().to_vec()
    }

    /// Constant-time signature check (`Mac::verify_slice`, NOT a manual `==` byte compare — a
    /// timing side-channel on a signature check is exactly the class of bug a token-based
    /// authorization scheme must not reintroduce).
    fn verify_signature(&self, payload: &[u8], signature: &[u8]) -> Result<(), TokenError> {
        #[allow(clippy::expect_used, reason = "see sign()'s identical justification")]
        let mut mac =
            HmacSha256::new_from_slice(&self.secret).expect("HMAC accepts any key length");
        mac.update(payload);
        mac.verify_slice(signature)
            .map_err(|_mac_err| TokenError::BadSignature)
    }
}

fn canonical_payload(key: &str, content_type: &str, max_bytes: u64, expires_at: u64) -> String {
    format!("{key}\0{content_type}\0{max_bytes}\0{expires_at}")
}

#[allow(
    clippy::expect_used,
    reason = "the real wall clock is never before the unix epoch on any machine this runs on"
)]
fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after the unix epoch")
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signer() -> UploadTokenSigner {
        UploadTokenSigner::new(vec![7u8; 32])
    }

    #[test]
    fn mint_then_verify_round_trips_the_exact_scope() {
        let signer = signer();
        let token = signer.mint("loc/prod/image/abc123.webp", "image/webp", 8_000_000, 1_000);
        let claims = signer.verify(&token, 1_000).unwrap();
        assert_eq!(claims.key, "loc/prod/image/abc123.webp");
        assert_eq!(claims.content_type, "image/webp");
        assert_eq!(claims.max_bytes, 8_000_000);
        assert_eq!(claims.expires_at, 1_000 + TOKEN_TTL_SECONDS);
    }

    #[test]
    fn ttl_is_300_seconds_matching_the_old_presign_contract() {
        assert_eq!(TOKEN_TTL_SECONDS, 300);
    }

    #[test]
    fn expired_token_is_rejected() {
        let signer = signer();
        let token = signer.mint("k", "image/webp", 100, 1_000);
        // Verifying at exactly `expires_at` + 1s (300s TTL elapsed) must fail.
        let err = signer
            .verify(&token, 1_000 + TOKEN_TTL_SECONDS + 1)
            .unwrap_err();
        assert_eq!(err, TokenError::Expired);
    }

    #[test]
    fn token_still_valid_at_the_exact_expiry_instant() {
        // Boundary check: `expires_at == now` must still verify (the TTL window is inclusive of
        // its own last second) — only strictly-past-expiry fails.
        let signer = signer();
        let token = signer.mint("k", "image/webp", 100, 1_000);
        assert!(signer.verify(&token, 1_000 + TOKEN_TTL_SECONDS).is_ok());
    }

    #[test]
    fn tampered_payload_fails_signature_check() {
        let signer = signer();
        let token = signer.mint("k1", "image/webp", 100, 1_000);
        let (payload_b64, sig_b64) = token.split_once('.').unwrap();
        let mut payload_bytes = URL_SAFE_NO_PAD.decode(payload_b64).unwrap();
        // Flip the key inside the signed payload without re-signing — a classic "same-shape,
        // different content" forgery attempt.
        payload_bytes[0] ^= 0xFF;
        let tampered = format!("{}.{}", URL_SAFE_NO_PAD.encode(payload_bytes), sig_b64);
        let err = signer.verify(&tampered, 1_000).unwrap_err();
        assert_eq!(err, TokenError::BadSignature);
    }

    #[test]
    fn wrong_secret_cannot_verify_a_token_it_did_not_mint() {
        let signer_a = UploadTokenSigner::new(vec![1u8; 32]);
        let signer_b = UploadTokenSigner::new(vec![2u8; 32]);
        let token = signer_a.mint("k", "image/webp", 100, 1_000);
        let err = signer_b.verify(&token, 1_000).unwrap_err();
        assert_eq!(err, TokenError::BadSignature);
    }

    #[test]
    fn malformed_token_shapes_are_rejected_not_panicking() {
        let signer = signer();
        for bad in [
            "",
            "no-dot-at-all",
            "not-base64!.also-not-base64!",
            "..",
            "abc.",
        ] {
            assert!(
                signer.verify(bad, 1_000).is_err(),
                "input {bad:?} must not verify"
            );
        }
    }

    #[test]
    fn from_hex_round_trips_a_valid_secret() {
        let hex_secret = "ab".repeat(32);
        let signer = UploadTokenSigner::from_hex(&hex_secret).unwrap();
        let token = signer.mint("k", "image/webp", 100, 1_000);
        assert!(signer.verify(&token, 1_000).is_ok());
    }

    #[test]
    fn same_key_replay_within_ttl_is_permitted_by_design() {
        // Documents the honest caveat from the module doc as an executable assertion: a token
        // used twice within its TTL verifies BOTH times (no consumed/single-use state exists) —
        // the safety property this system relies on instead is content-addressed idempotent
        // keys at the route/storage layer, not single-use tokens.
        let signer = signer();
        let token = signer.mint("loc/prod/image/hash.webp", "image/webp", 100, 1_000);
        assert!(signer.verify(&token, 1_050).is_ok(), "first use");
        assert!(
            signer.verify(&token, 1_100).is_ok(),
            "replay within TTL still verifies"
        );
    }
}
