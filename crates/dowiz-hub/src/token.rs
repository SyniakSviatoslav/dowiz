//! Bearer tokens the hub mints and checks.
//!
//! NOT JWT, deliberately. JWT's `alg` header is a field the ATTACKER supplies,
//! and the family of bugs that follows from honouring it (`alg: none`, HS256
//! verified against an RS256 public key) is entirely self-inflicted: it exists
//! only because the format asks the verifier to be told which algorithm to use.
//! These tokens have one algorithm, fixed in code, with nothing negotiable on
//! the wire. The front-ends treat the token as opaque — they send it back as
//! `Authorization: Bearer …` and never parse it — so there was no reason to
//! adopt a format whose only advantage is being parseable by software that is
//! not here.
//!
//! The wire form is `<base64url(payload)>.<base64url(mac)>` where the MAC is
//! HMAC-SHA256 over the FIRST PART AS SENT — the encoded bytes, not the decoded
//! ones. Signing the re-encoded payload instead would let two spellings of one
//! token both verify.

use crate::crypto::{b64url_decode, b64url_encode, constant_time_eq, hmac_sha256};
use crate::minijson::{esc as esc_json, int_field as num_field, str_field};

/// Who a token speaks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Owner,
    Courier,
    /// A customer following one order. Scoped to that order and nothing else.
    Customer,
    /// Exchangeable for a new access token, and for nothing else. A refresh
    /// token that could also read orders would make short access-token
    /// lifetimes pointless.
    Refresh,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Owner => "owner",
            Role::Courier => "courier",
            Role::Customer => "customer",
            Role::Refresh => "refresh",
        }
    }
    pub fn from_str(s: &str) -> Option<Role> {
        match s {
            "owner" => Some(Role::Owner),
            "courier" => Some(Role::Courier),
            "customer" => Some(Role::Customer),
            "refresh" => Some(Role::Refresh),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claims {
    pub role: Role,
    /// The person this token speaks for.
    pub subject: String,
    /// The session this token belongs to. Revoking the session kills every
    /// token issued under it without waiting for expiry.
    pub session: String,
    /// Extra scope: the order id, for a customer token. Empty otherwise.
    pub scope: String,
    pub issued_ms: i64,
    pub expires_ms: i64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TokenError {
    /// Not two dot-separated base64url parts.
    Malformed,
    /// The MAC did not match. Either the token was altered or it was signed
    /// with a different key.
    BadSignature,
    Expired,
    /// Structurally fine, but not the role this endpoint needs.
    WrongRole,
}

/// Access tokens are short-lived because revocation is checked at the roster,
/// and between checks a stolen token is live. 15 minutes bounds that window
/// without making the front-end's refresh loop constant work.
pub const ACCESS_TTL_MS: i64 = 15 * 60 * 1000;
/// A refresh token is bound to a session that can be revoked, so it may live
/// long enough that an owner is not logged out mid-shift.
pub const REFRESH_TTL_MS: i64 = 30 * 24 * 60 * 60 * 1000;

/// Serialise claims. A fixed field order, hand-written rather than via serde,
/// so the signed bytes cannot change because a derive's output changed.
fn encode_payload(c: &Claims) -> String {
    format!(
        r#"{{"r":"{}","s":"{}","j":"{}","p":"{}","i":{},"e":{}}}"#,
        c.role.as_str(),
        esc_json(&c.subject),
        esc_json(&c.session),
        esc_json(&c.scope),
        c.issued_ms,
        c.expires_ms
    )
}

/// Mint a token.
pub fn mint(key: &[u8], claims: &Claims) -> String {
    let payload = b64url_encode(encode_payload(claims).as_bytes());
    let mac = hmac_sha256(key, payload.as_bytes());
    format!("{payload}.{}", b64url_encode(&mac))
}

/// Verify a token and return its claims.
///
/// ORDER MATTERS: the signature is checked BEFORE anything in the payload is
/// believed, including the expiry. Reading claims out of an unverified token
/// and acting on them — even to reject — is how parsers become the attack
/// surface.
pub fn verify(key: &[u8], token: &str, now_ms: i64) -> Result<Claims, TokenError> {
    let (payload_b64, mac_b64) = token.split_once('.').ok_or(TokenError::Malformed)?;
    if payload_b64.is_empty() || mac_b64.contains('.') {
        return Err(TokenError::Malformed);
    }
    let given = b64url_decode(mac_b64).ok_or(TokenError::Malformed)?;
    let want = hmac_sha256(key, payload_b64.as_bytes());
    if !constant_time_eq(&given, &want) {
        return Err(TokenError::BadSignature);
    }

    let raw = b64url_decode(payload_b64).ok_or(TokenError::Malformed)?;
    let json = String::from_utf8(raw).map_err(|_| TokenError::Malformed)?;
    let claims = Claims {
        role: str_field(&json, "r")
            .and_then(|r| Role::from_str(&r))
            .ok_or(TokenError::Malformed)?,
        subject: str_field(&json, "s").ok_or(TokenError::Malformed)?,
        session: str_field(&json, "j").ok_or(TokenError::Malformed)?,
        scope: str_field(&json, "p").unwrap_or_default(),
        issued_ms: num_field(&json, "i").ok_or(TokenError::Malformed)?,
        expires_ms: num_field(&json, "e").ok_or(TokenError::Malformed)?,
    };
    if now_ms >= claims.expires_ms {
        return Err(TokenError::Expired);
    }
    Ok(claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &[u8] = b"a hub signing key, 32 bytes long";

    fn claims(role: Role, now: i64) -> Claims {
        Claims {
            role,
            subject: "person_1".into(),
            session: "sess_abc".into(),
            scope: String::new(),
            issued_ms: now,
            expires_ms: now + ACCESS_TTL_MS,
        }
    }

    #[test]
    fn a_minted_token_verifies_and_round_trips() {
        let now = 1_700_000_000_000;
        let c = claims(Role::Owner, now);
        let t = mint(KEY, &c);
        assert_eq!(verify(KEY, &t, now + 1000).expect("verify"), c);
    }

    /// The single most important test in this file: a token signed by someone
    /// else must not be accepted.
    #[test]
    fn a_token_signed_with_another_key_is_refused() {
        let now = 1_700_000_000_000;
        let t = mint(b"a different hub's signing key!!!", &claims(Role::Owner, now));
        assert_eq!(verify(KEY, &t, now), Err(TokenError::BadSignature));
    }

    /// Flipping ANY byte of the payload must invalidate the token -- this is
    /// what stops a courier token being edited into an owner token.
    #[test]
    fn every_payload_byte_is_covered_by_the_mac() {
        let now = 1_700_000_000_000;
        let t = mint(KEY, &claims(Role::Courier, now));
        let (payload, mac) = t.split_once('.').unwrap();
        for i in 0..payload.len() {
            let mut bytes = payload.as_bytes().to_vec();
            // Move to a different character in the alphabet.
            bytes[i] = if bytes[i] == b'A' { b'B' } else { b'A' };
            let altered = format!("{}.{}", String::from_utf8_lossy(&bytes), mac);
            assert!(
                verify(KEY, &altered, now).is_err(),
                "byte {i} changed and the token still verified"
            );
        }
    }

    #[test]
    fn an_expired_token_is_refused() {
        let now = 1_700_000_000_000;
        let t = mint(KEY, &claims(Role::Owner, now));
        assert!(verify(KEY, &t, now + ACCESS_TTL_MS - 1).is_ok());
        // Exactly at expiry is already too late; a token valid "until" a moment
        // must not be valid AT it.
        assert_eq!(verify(KEY, &t, now + ACCESS_TTL_MS), Err(TokenError::Expired));
    }

    /// An expired token must fail on its SIGNATURE first if the signature is
    /// also wrong -- the error must never reveal that a forged token would
    /// otherwise have been in date.
    #[test]
    fn signature_is_checked_before_expiry() {
        let now = 1_700_000_000_000;
        let t = mint(b"wrong key wrong key wrong key!!!", &claims(Role::Owner, now));
        assert_eq!(verify(KEY, &t, now + ACCESS_TTL_MS * 10), Err(TokenError::BadSignature));
    }

    /// A quote in the scope must not be able to inject claim fields into the
    /// payload BEFORE it is signed. The MAC would be perfectly valid over the
    /// forged claims, which is what makes this the dangerous case.
    #[test]
    fn a_quote_in_the_scope_cannot_forge_claims() {
        let now = 1_700_000_000_000;
        let mut c = claims(Role::Customer, now);
        c.scope = r#"ord_1","r":"owner","x":"#.into();
        let t = mint(KEY, &c);
        let got = verify(KEY, &t, now).expect("verify");
        assert_eq!(got.role, Role::Customer, "the role must not have been overwritten");
        assert_eq!(got.scope, c.scope, "the scope must survive intact");
    }

    #[test]
    fn junk_is_refused_rather_than_panicking() {
        let now = 1_700_000_000_000;
        for junk in ["", ".", "a", "a.b.c", "....", "!!!.???", "Zm9v."] {
            assert!(verify(KEY, junk, now).is_err(), "accepted {junk:?}");
        }
    }

    #[test]
    fn roles_round_trip_through_their_strings() {
        for r in [Role::Owner, Role::Courier, Role::Customer, Role::Refresh] {
            assert_eq!(Role::from_str(r.as_str()), Some(r));
        }
        assert_eq!(Role::from_str("admin"), None);
    }
}
