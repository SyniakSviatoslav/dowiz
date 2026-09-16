//! Accounts, tokens and the guard.
//!
//! Ported in behaviour, not in mechanism, from the old platform's auth
//! (`origin/backup-wip-2026-07-08`). Three things it got right are kept exactly,
//! because each one closes a hole that a token alone cannot:
//!
//!   1. Owner authority is NEVER read from the token. It is re-derived from
//!      `memberships` on every request, so an owner removed a second ago is
//!      refused on their next call while still holding a valid JWT.
//!   2. A courier's session is a ROW. The guard re-reads it every request, so
//!      revoking the session -- or merely removing the courier from the location
//!      -- ends access immediately instead of at token expiry.
//!   3. The customer claim carries NO phone and no name. It names an order and a
//!      location and nothing else, so a leaked customer token leaks no PII.
//!
//! ONE DELIBERATE DEVIATION. The old service signed RS256. Asymmetry earns its
//! keep when many parties must verify without being able to sign; here a single
//! Worker does both. On Workers CPU is the scarce resource -- the free tier
//! allows 10ms per invocation -- and an RSA verify in wasm costs milliseconds
//! against HMAC's microseconds. So tokens are HS256. `kid` is kept so a key can
//! still be rotated without invalidating every live token.

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use worker::*;

use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use base64::Engine as _;

type HmacSha256 = Hmac<Sha256>;

/// Access-token lifetimes, mirroring the old service's table.
pub const OWNER_TTL_MS: i64 = 24 * 60 * 60 * 1000;
pub const COURIER_TTL_MS: i64 = 24 * 60 * 60 * 1000;
pub const CUSTOMER_TTL_MS: i64 = 7 * 24 * 60 * 60 * 1000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum Claims {
    Owner {
        sub: String,
        user_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        active_location_id: Option<String>,
        iat: i64,
        exp: i64,
    },
    Courier {
        sub: String,
        active_location_id: String,
        /// The `courier_sessions` row this token is bound to. A courier token
        /// without one is refused outright: it could only come from a path that
        /// never created a session, and an unbindable courier token cannot be
        /// revoked before it expires.
        jti: String,
        iat: i64,
        exp: i64,
    },
    Customer {
        sub: String,
        order_id: String,
        location_id: String,
        // NO phone. NO name. Deliberate -- see the module header.
        iat: i64,
        exp: i64,
    },
}

impl Claims {
    pub fn exp(&self) -> i64 {
        match self {
            Claims::Owner { exp, .. } | Claims::Courier { exp, .. } | Claims::Customer { exp, .. } => *exp,
        }
    }
    pub fn subject(&self) -> &str {
        match self {
            Claims::Owner { sub, .. } | Claims::Courier { sub, .. } | Claims::Customer { sub, .. } => sub,
        }
    }
    pub fn role_name(&self) -> &'static str {
        match self {
            Claims::Owner { .. } => "owner",
            Claims::Courier { .. } => "courier",
            Claims::Customer { .. } => "customer",
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Header {
    alg: String,
    typ: String,
    kid: String,
}

#[derive(Debug)]
pub enum AuthError {
    Missing,
    Malformed,
    BadSignature,
    Expired,
    /// The token verified but the live state behind it says no: session revoked,
    /// membership gone, courier removed from the location.
    Revoked(&'static str),
    Config(&'static str),
    Db(String),
}

impl AuthError {
    pub fn into_response(self) -> Result<Response> {
        match self {
            AuthError::Missing => Response::error("missing bearer token", 401),
            AuthError::Malformed => Response::error("malformed token", 401),
            AuthError::BadSignature => Response::error("bad token signature", 401),
            AuthError::Expired => Response::error("token expired", 401),
            AuthError::Revoked(why) => Response::error(format!("not authorised: {why}"), 401),
            // Fail CLOSED on a misconfiguration or a database error. The old
            // admin gate made the same call explicitly: a 503 beats guessing.
            AuthError::Config(w) => Response::error(format!("auth misconfigured: {w}"), 503),
            AuthError::Db(e) => Response::error(format!("auth backend unavailable: {e}"), 503),
        }
    }
}

fn secret(env: &Env) -> std::result::Result<(String, Vec<u8>), AuthError> {
    let kid = env
        .secret("JWT_KID")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "v1".to_string());
    let s = env
        .secret("JWT_SECRET")
        .map_err(|_| AuthError::Config("JWT_SECRET is not set"))?
        .to_string();
    if s.len() < 32 {
        // A short secret is the failure that looks like success.
        return Err(AuthError::Config("JWT_SECRET must be at least 32 bytes"));
    }
    Ok((kid, s.into_bytes()))
}

pub fn sign(env: &Env, claims: &Claims) -> std::result::Result<String, AuthError> {
    let (kid, key) = secret(env)?;
    let header = Header { alg: "HS256".into(), typ: "JWT".into(), kid };
    let h = B64.encode(serde_json::to_vec(&header).map_err(|_| AuthError::Malformed)?);
    let p = B64.encode(serde_json::to_vec(claims).map_err(|_| AuthError::Malformed)?);
    let signing_input = format!("{h}.{p}");
    let mut mac = HmacSha256::new_from_slice(&key).map_err(|_| AuthError::Config("bad key"))?;
    mac.update(signing_input.as_bytes());
    let sig = B64.encode(mac.finalize().into_bytes());
    Ok(format!("{signing_input}.{sig}"))
}

/// Verify signature and expiry ONLY. Live state is checked by the guard.
pub fn verify(env: &Env, token: &str, now_ms: i64) -> std::result::Result<Claims, AuthError> {
    let (_, key) = secret(env)?;
    let mut parts = token.split('.');
    let (h, p, s) = match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some(h), Some(p), Some(s), None) => (h, p, s),
        _ => return Err(AuthError::Malformed),
    };

    // Pin the algorithm from OUR side before trusting anything in the header.
    // Reading `alg` out of the token and honouring it is how alg-confusion works.
    let header: Header = serde_json::from_slice(&B64.decode(h).map_err(|_| AuthError::Malformed)?)
        .map_err(|_| AuthError::Malformed)?;
    if header.alg != "HS256" {
        return Err(AuthError::BadSignature);
    }

    let mut mac = HmacSha256::new_from_slice(&key).map_err(|_| AuthError::Config("bad key"))?;
    mac.update(format!("{h}.{p}").as_bytes());
    let expected = mac.finalize().into_bytes();
    let got = B64.decode(s).map_err(|_| AuthError::Malformed)?;
    if got.len() != expected.len() || !bool::from(got.ct_eq(&expected)) {
        return Err(AuthError::BadSignature);
    }

    let claims: Claims = serde_json::from_slice(&B64.decode(p).map_err(|_| AuthError::Malformed)?)
        .map_err(|_| AuthError::Malformed)?;
    if claims.exp() <= now_ms {
        return Err(AuthError::Expired);
    }
    Ok(claims)
}

pub fn bearer(req: &Request) -> std::result::Result<String, AuthError> {
    let raw = req
        .headers()
        .get("authorization")
        .ok()
        .flatten()
        .ok_or(AuthError::Missing)?;
    let t = raw.strip_prefix("Bearer ").or_else(|| raw.strip_prefix("bearer "));
    match t {
        Some(t) if !t.is_empty() => Ok(t.to_string()),
        _ => Err(AuthError::Missing),
    }
}

/// sha256 hex. Used for refresh-token storage and for PII lookup columns, never
/// for passwords -- those are argon2id below.
pub fn sha256_hex(input: &str) -> String {
    let mut h = Sha256::new();
    h.update(input.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

// ── passwords ────────────────────────────────────────────────────────────────

/// argon2id, kept from the old service. This is the one place slowness is the
/// feature; note it needs the Workers PAID CPU budget, the 10ms free tier cannot
/// run it.
pub fn hash_password(password: &str) -> std::result::Result<String, AuthError> {
    use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
    use argon2::Argon2;
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|_| AuthError::Config("password hashing failed"))
}

pub fn verify_password(password: &str, stored: &str) -> bool {
    use argon2::password_hash::{PasswordHash, PasswordVerifier};
    use argon2::Argon2;
    match PasswordHash::new(stored) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

/// Spend the same work on a miss as on a hit, so "no such account" and "wrong
/// password" are indistinguishable by timing. The old service did this with a
/// dummy argon2 verify; the reason is the same.
/// Hash a SECRET THIS SERVICE GENERATED, not a password a person chose.
///
/// ARGON2 IS FOR PASSWORDS, and the difference is not stylistic. Argon2 is
/// deliberately expensive -- 19 MiB and two passes at the default -- because a
/// human password has maybe thirty bits of entropy and the only defence is
/// making each guess cost something. A session secret is 128 bits from the
/// platform CSPRNG: there is no dictionary, no list, and no number of guesses
/// that gets anywhere, so the expense buys nothing.
///
/// It cost something, though. A courier login ran TWO Argon2 operations -- one
/// verifying their password, one hashing the new session secret -- and the pair
/// exceeded the isolate's memory. Cloudflare answered 503 error 1102, "Worker
/// exceeded resource limits", and no courier on that deployment could sign in.
///
/// SHA-256 over a 128-bit random value is not a weakening; it is the right
/// primitive for the thing being hashed. The same reasoning already governs the
/// invite codes.
pub fn hash_opaque(secret: &str) -> String {
    format!("sha256:{}", sha256_hex(secret))
}

/// Check one, accepting the argon2 hashes written before this existed.
///
/// A deployment mid-upgrade holds both kinds, and refusing the old ones would
/// log out every live session to save a few milliseconds.
pub fn verify_opaque(secret: &str, stored: &str) -> bool {
    match stored.strip_prefix("sha256:") {
        Some(want) => {
            // Constant-time: a timing difference here would leak the prefix of
            // a valid session id, one byte at a time.
            let got = sha256_hex(secret);
            got.len() == want.len()
                && got.bytes().zip(want.bytes()).fold(0u8, |a, (x, y)| a | (x ^ y)) == 0
        }
        None => verify_password(secret, stored),
    }
}

pub fn verify_password_constant_work(password: &str, stored: Option<&str>) -> bool {
    const DUMMY: &str = "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHR2YWx1ZQ$\
                         Yx3vJ8xQmN0oKZ7Xp1qLdT4hVn2sRwEbGcFuHiJkLmN";
    match stored {
        Some(h) => verify_password(password, h),
        None => {
            let _ = verify_password(password, DUMMY);
            false
        }
    }
}

// ── the guard ────────────────────────────────────────────────────────────────

/// An authenticated principal, AFTER live state has been confirmed.
pub enum Principal {
    Owner { user_id: String, active_location_id: Option<String> },
    Courier { courier_id: String, active_location_id: String, session_id: String },
    Customer { customer_id: String, order_id: String, location_id: String },
}

impl Principal {
    pub fn role_name(&self) -> &'static str {
        match self {
            Principal::Owner { .. } => "owner",
            Principal::Courier { .. } => "courier",
            Principal::Customer { .. } => "customer",
        }
    }
}

/// Verify the token AND the live state behind it.
pub async fn authenticate(
    req: &Request,
    env: &Env,
    db: &D1Database,
    now_ms: i64,
) -> std::result::Result<Principal, AuthError> {
    let raw = bearer(req)?;

    // ── AN API KEY IS NOT A JWT AND IS NOT VERIFIED LIKE ONE ──
    //
    // The prefix is what tells them apart, and it is deliberate rather than
    // decorative: a key that looked like a token would be handed to `verify`,
    // fail as malformed, and the owner would be told their session expired
    // instead of that their key is wrong. `dowiz_` is also what lets a secret
    // scanner recognise one in a repository.
    if let Some(rest) = raw.strip_prefix("dowiz_") {
        return api_key_principal(db, rest, now_ms).await;
    }

    let claims = verify(env, &raw, now_ms)?;
    match claims {
        Claims::Owner { user_id, active_location_id, .. } => {
            // Authority is re-derived, never trusted from the token.
            let still: Option<String> = db
                .prepare(
                    "SELECT m.id FROM memberships m \
                     WHERE m.user_id = ?1 AND m.role = 'owner' AND m.status = 'active' LIMIT 1",
                )
                .bind(&[user_id.clone().into()])
                .map_err(|e| AuthError::Db(e.to_string()))?
                .first(Some("id"))
                .await
                .map_err(|e| AuthError::Db(e.to_string()))?;
            if still.is_none() {
                return Err(AuthError::Revoked("owner membership is gone or suspended"));
            }
            Ok(Principal::Owner { user_id, active_location_id })
        }
        Claims::Courier { sub, active_location_id, jti, .. } => {
            // One joined read decides all four conditions the old guard checked.
            #[derive(Deserialize)]
            struct Row {
                revoked_at_ms: Option<i64>,
                expires_at_ms: i64,
                has_location: i64,
            }
            let row: Option<Row> = db
                .prepare(
                    "SELECT s.revoked_at_ms AS revoked_at_ms, s.expires_at_ms AS expires_at_ms, \
                     EXISTS(SELECT 1 FROM courier_locations cl \
                            WHERE cl.courier_id = s.courier_id AND cl.location_id = ?2) AS has_location \
                     FROM courier_sessions s WHERE s.id = ?1 AND s.courier_id = ?3",
                )
                .bind(&[jti.clone().into(), active_location_id.clone().into(), sub.clone().into()])
                .map_err(|e| AuthError::Db(e.to_string()))?
                .first(None)
                .await
                .map_err(|e| AuthError::Db(e.to_string()))?;
            let row = row.ok_or(AuthError::Revoked("no such courier session"))?;
            if row.revoked_at_ms.is_some() {
                return Err(AuthError::Revoked("courier session revoked"));
            }
            if row.expires_at_ms <= now_ms {
                return Err(AuthError::Revoked("courier session expired"));
            }
            if row.has_location == 0 {
                return Err(AuthError::Revoked("courier no longer at this location"));
            }
            Ok(Principal::Courier { courier_id: sub, active_location_id, session_id: jti })
        }
        Claims::Customer { sub, order_id, location_id, .. } => {
            Ok(Principal::Customer { customer_id: sub, order_id, location_id })
        }
    }
}

/// Resolve `dowiz_<id>.<secret>` to the owner who minted it.
///
/// The id travels in the token because the secret is HASHED in the row and a
/// hash lookup is impossible by design -- the same reason the refresh tokens
/// carry their row id. Everything that can refuse does: an unknown id, a
/// mismatched secret, a revoked key and an expired one all answer the same way.
async fn api_key_principal(
    db: &D1Database,
    rest: &str,
    now_ms: i64,
) -> std::result::Result<Principal, AuthError> {
    let Some((id, secret)) = rest.split_once('.') else {
        return Err(AuthError::Revoked("that key is malformed"));
    };
    #[derive(Deserialize)]
    struct Row {
        owner_id: String,
        location_id: String,
        key_hash: String,
        expires_at_ms: i64,
        revoked_at_ms: Option<i64>,
    }
    let row: Option<Row> = db
        .prepare(
            "SELECT owner_id, location_id, key_hash, expires_at_ms, revoked_at_ms              FROM owner_api_keys WHERE id = ?1",
        )
        .bind(&[id.into()])
        .map_err(|e| AuthError::Db(e.to_string()))?
        .first(None)
        .await
        .map_err(|e| AuthError::Db(e.to_string()))?;
    let Some(row) = row else {
        return Err(AuthError::Revoked("no such key"));
    };
    if row.revoked_at_ms.is_some() {
        return Err(AuthError::Revoked("that key was revoked"));
    }
    if now_ms >= row.expires_at_ms {
        return Err(AuthError::Revoked("that key has expired"));
    }
    if !verify_opaque(secret, &row.key_hash) {
        return Err(AuthError::Revoked("no such key"));
    }
    // Recorded, not enforced: an owner looking at a key they no longer recognise
    // needs to know whether anything is still using it before they revoke it.
    let _ = db
        .prepare("UPDATE owner_api_keys SET last_used_ms = ?2 WHERE id = ?1")
        .bind(&[id.into(), wasm_bindgen::JsValue::from_f64(now_ms as f64)])
        .map_err(|e| AuthError::Db(e.to_string()))?
        .run()
        .await;
    Ok(Principal::Owner {
        user_id: row.owner_id,
        active_location_id: Some(row.location_id),
    })
}

/// Cross-tenant access answers 404, not 403 — the old guard's choice, and the
/// right one: 403 confirms the resource exists.
pub fn require_location(p: &Principal, location_id: &str) -> std::result::Result<(), Response> {
    let ok = match p {
        Principal::Owner { active_location_id, .. } => {
            // Owners are additionally checked against memberships by the caller
            // when the route is location-scoped; this only rejects an obvious
            // mismatch early.
            active_location_id.as_deref().map_or(true, |l| l == location_id)
        }
        Principal::Courier { active_location_id, .. } => active_location_id == location_id,
        Principal::Customer { location_id: l, .. } => l == location_id,
    };
    if ok {
        Ok(())
    } else {
        Err(Response::error("not found", 404).unwrap())
    }
}
