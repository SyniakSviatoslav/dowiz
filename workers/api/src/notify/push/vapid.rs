//! RFC 8292 VAPID: the application server says who it is to a push service.
//!
//! Every push request carries `Authorization: vapid t=<JWT>, k=<public key>`.
//! The JWT is ES256 (ECDSA P-256 over SHA-256, the signature as raw r||s, RFC
//! 7518 §3.4), its `aud` is the ORIGIN of the push endpoint, its `exp` at most
//! 24 hours ahead, and its `sub` a contact the push service can reach.
//!
//! THE KEY PAIR. The browser was given [`PUBLIC_KEY`] when it subscribed
//! (`applicationServerKey`), and a push service refuses a message signed by
//! any other key with 401/403. So the private half, the Worker secret
//! [`SECRET_NAME`], must be THE pair of this constant -- and [`Signer::new`]
//! derives the public key from the secret and REFUSES when they differ, so a
//! wrongly set secret is one loud line, not every message silently refused.

use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use base64::Engine;
use p256::ecdsa::signature::Signer as _;
use p256::ecdsa::{Signature, SigningKey};

/// The application server's public key (uncompressed P-256, base64url).
/// Generated on the box 2026-10-03; its private half is in `/root/.dowiz_vapid`
/// and, once set, in the Worker secret [`SECRET_NAME`].
pub const PUBLIC_KEY: &str = "BPzZDsH_vwvJJ0KZ0kPL-fxyDsw_vDpNu2TTGNsAnKDglChuA_Ywvz81UGVE3lw9ikZvYUNyYhj7v5PujRmU11s";
/// A Worker VAR that may replace [`PUBLIC_KEY`] (a test site, a rotated key);
/// unset is the constant. The browser and the signer read the same one.
pub const PUBLIC_VAR: &str = "VAPID_PUBLIC_KEY";

/// The public key in force: [`PUBLIC_VAR`] when set, else [`PUBLIC_KEY`].
pub fn public_key(env: &crate::edge::Env) -> String {
    env.var(PUBLIC_VAR).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty()).unwrap_or_else(|| PUBLIC_KEY.to_string())
}

/// The Worker secret holding the private scalar (32 octets, base64url).
pub const SECRET_NAME: &str = "VAPID_PRIVATE_KEY";
/// RFC 8292 §2.1: a contact for the push service's operator.
pub const SUBJECT: &str = "https://dowiz.org";
/// How long a token is good for. RFC 8292 caps it at 24 h; 12 h leaves room
/// for a push service whose clock is ahead of ours.
pub const TOKEN_SECS: i64 = 12 * 3600;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VapidError {
    /// The secret is missing, not base64url, or not a P-256 scalar.
    BadSecret,
    /// The secret is a valid key, but not the pair of [`PUBLIC_KEY`].
    NotOurKey { derived: String },
    /// The endpoint is not an absolute https URL.
    BadEndpoint,
}

/// A signing key that has been checked against the public key browsers hold.
pub struct Signer {
    key: SigningKey,
    public: String,
}

impl Signer {
    /// From the secret's text; refuses a key that is not the pair of `expected`.
    pub fn new(secret_b64: &str, expected: &str) -> Result<Self, VapidError> {
        let raw = B64.decode(secret_b64.trim()).map_err(|_| VapidError::BadSecret)?;
        let key = SigningKey::from_slice(&raw).map_err(|_| VapidError::BadSecret)?;
        let public = B64.encode(key.verifying_key().to_encoded_point(false).as_bytes());
        if public != expected {
            return Err(VapidError::NotOurKey { derived: public });
        }
        Ok(Signer { key, public })
    }

    /// The compact JWS for one audience.
    pub fn jwt(&self, aud: &str, exp_s: i64) -> String {
        let head = B64.encode(br#"{"typ":"JWT","alg":"ES256"}"#);
        let claims = serde_json::json!({ "aud": aud, "exp": exp_s, "sub": SUBJECT });
        let body = B64.encode(claims.to_string());
        let input = format!("{head}.{body}");
        let sig: Signature = self.key.sign(input.as_bytes());
        format!("{input}.{}", B64.encode(sig.to_bytes()))
    }

    /// The `Authorization` header value for a message to `endpoint` at `now_ms`.
    pub fn authorization(&self, endpoint: &str, now_ms: i64) -> Result<String, VapidError> {
        let aud = audience(endpoint).ok_or(VapidError::BadEndpoint)?;
        let exp = now_ms.div_euclid(1000) + TOKEN_SECS;
        Ok(format!("vapid t={}, k={}", self.jwt(&aud, exp), self.public))
    }
}

/// The origin of an https URL: `https://host[:port]`, lower-cased host.
pub fn audience(endpoint: &str) -> Option<String> {
    let rest = endpoint.strip_prefix("https://")?;
    let authority = rest.split(['/', '?', '#']).next()?;
    if authority.is_empty() || authority.contains('@') {
        return None;
    }
    Some(format!("https://{}", authority.to_ascii_lowercase()))
}

#[cfg(test)]
mod tests;
