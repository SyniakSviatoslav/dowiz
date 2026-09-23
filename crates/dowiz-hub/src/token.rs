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
    /// A member of staff, in the room. The role is only half of the answer:
    /// WHAT they may do rides on the token as `Claims::caps`, a closed set from
    /// `crate::caps`, and is narrowed at the door by the live roster. An owner
    /// is not a `Staff`; a courier is not a `Staff`; this is the person who
    /// takes the order at the table and the person in the kitchen.
    Staff,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Owner => "owner",
            Role::Courier => "courier",
            Role::Customer => "customer",
            Role::Refresh => "refresh",
            Role::Staff => "staff",
        }
    }
    pub fn from_str(s: &str) -> Option<Role> {
        match s {
            "owner" => Some(Role::Owner),
            "courier" => Some(Role::Courier),
            "customer" => Some(Role::Customer),
            "refresh" => Some(Role::Refresh),
            "staff" => Some(Role::Staff),
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
    /// What this token MAY DO, as `crate::caps::Caps` spells a set: capability
    /// names, comma-joined, in canonical order. Empty for every role but
    /// `Staff`, and empty is a real answer -- it means no capability, which is
    /// refused rather than waved through (`caps::admit`).
    ///
    /// IT IS INSIDE THE SIGNATURE, which is the only reason a capability can be
    /// trusted at all (`DECISIONS.md` OD-8). It is still not the last word: the
    /// guard intersects it with what the venue's roster grants this person NOW,
    /// so a capability taken away a second ago is gone on the next call rather
    /// than at expiry -- the same rule owner membership has always had.
    pub caps: String,
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
        r#"{{"r":"{}","s":"{}","j":"{}","p":"{}","c":"{}","i":{},"e":{}}}"#,
        c.role.as_str(),
        esc_json(&c.subject),
        esc_json(&c.session),
        esc_json(&c.scope),
        esc_json(&c.caps),
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
        // ABSENT IS THE EMPTY SET, NOT AN ERROR AND NOT EVERYTHING. A token
        // minted before this field existed is still in somebody's browser for
        // up to `REFRESH_TTL_MS`; refusing it would sign out the venue on
        // deploy, and defaulting it to anything but nothing would hand a
        // capability to a token that was never granted one.
        caps: str_field(&json, "c").unwrap_or_default(),
        issued_ms: num_field(&json, "i").ok_or(TokenError::Malformed)?,
        expires_ms: num_field(&json, "e").ok_or(TokenError::Malformed)?,
    };
    if now_ms >= claims.expires_ms {
        return Err(TokenError::Expired);
    }
    Ok(claims)
}

#[cfg(test)]
mod tests;
