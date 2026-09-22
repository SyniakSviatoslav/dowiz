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

/// The raw signing key, for the few places that mint something this module's
/// `Claims` enum does not describe -- a voice proposal, for instance, which is
/// scoped to an action rather than to a role.
pub fn signing_key(env: &Env) -> Vec<u8> {
    secret(env).map(|(_, k)| k).unwrap_or_default()
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

/// argon2id, at parameters a Worker isolate can actually afford.
///
/// THE DEFAULT DID NOT FIT. `Argon2::default()` is m=19456 — 19 MiB allocated
/// and touched for a single verify — and a login that did that answered `503
/// error 1102` under any load at all. Slowness is the feature here, but a cost
/// the platform refuses to pay is not slowness, it is an outage, and an outage
/// authenticates nobody.
///
/// m=8192, t=3 keeps the same product of work at a third of the memory: three
/// passes over 8 MiB instead of two over 19. OWASP's second recommended profile
/// is m=9216/t=4, so this sits inside the range they consider current rather
/// than below it. What it gives up is real and worth naming: an attacker with
/// the hash file gets a cheaper guess than the default would have cost them.
/// What it buys is that the login works.
///
/// The parameters live in the PHC string, so a hash written at the old cost is
/// still VERIFIED at the old cost — which is why `needs_rehash` exists and why
/// the login path uses it. Without that, every legacy account would keep
/// spending 19 MiB forever and the outage would never actually end.
fn argon2() -> argon2::Argon2<'static> {
    use argon2::{Algorithm, Argon2, Params, Version};
    let params = Params::new(8 * 1024, 3, 1, None).expect("argon2 parameters are constant");
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

pub fn hash_password(password: &str) -> std::result::Result<String, AuthError> {
    use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
    let salt = SaltString::generate(&mut OsRng);
    argon2()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|_| AuthError::Config("password hashing failed"))
}

pub fn verify_password(password: &str, stored: &str) -> bool {
    use argon2::password_hash::{PasswordHash, PasswordVerifier};
    match PasswordHash::new(stored) {
        // VERIFIED WITH THE STORED HASH'S OWN PARAMETERS, not with ours. This
        // is what makes the change above safe to deploy against live accounts:
        // the m and t that produced the hash are written in it.
        Ok(parsed) => argon2().verify_password(password.as_bytes(), &parsed).is_ok(),
        Err(_) => false,
    }
}

/// Was this hash written at a cost this Worker can no longer afford?
///
/// Read from the hash rather than from a stored flag or a version column: the
/// PHC string is the only thing that cannot disagree with the hash it describes.
pub fn needs_rehash(stored: &str) -> bool {
    use argon2::password_hash::PasswordHash;
    let Ok(parsed) = PasswordHash::new(stored) else {
        // Not a PHC string at all, so not something we can reason about. Leave
        // it alone; `verify_password` will refuse it on its own terms.
        return false;
    };
    let Ok(params) = argon2::Params::try_from(&parsed) else {
        return false;
    };
    params.m_cost() > 8 * 1024
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

/// The hash a login MISS is checked against — see `verify_password_constant_work`.
// A REAL HASH, produced by `hash_password` and pasted here, not a plausible
// looking string. The hand-written one that stood here did not parse: its
// final base64 character carried non-zero padding bits, so `PasswordHash`
// refused it, `verify_password` returned false without running argon2, and
// the miss cost NOTHING while the hit cost a full derivation. The timing
// difference this constant exists to erase was therefore present for the
// whole life of the function, and looked exactly like working code.
// `the_dummy_hash_actually_costs_something` is what now says otherwise.
const DUMMY: &str = "$argon2id$v=19$m=8192,t=3,p=1$ZG93aXpkdW1teXNhbHQ$\
                     cwMAH3VmqnqbDRZKJyROeGXCWQuBkRLbvmPp9z6HzXc";

pub fn verify_password_constant_work(password: &str, stored: Option<&str>) -> bool {
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

/// Does this principal belong to this venue?
///
/// THE CLAIM DECIDES, not a membership table. An owner's token carries the hub
/// they opened (`active_location_id`); a platform-admin token carries NONE, so
/// it is not an owner here -- which is what `accounts.rs` says a platform
/// admin is, and what three routes did not enforce. An owner of two venues
/// acting on the other one re-opens it and gets a token that says so; that is
/// the same narrowing `/api/order/:id` already applies.
pub fn belongs_to(p: &Principal, venue: &str) -> bool {
    match p {
        Principal::Owner { active_location_id, .. } => {
            active_location_id.as_deref() == Some(venue)
        }
        Principal::Courier { active_location_id, .. } => active_location_id == venue,
        Principal::Customer { location_id, .. } => location_id == venue,
    }
}

/// Authenticate, and require the principal to belong to THIS venue.
///
/// WHY THIS EXISTS. Three whole route families -- reservations, threads and the
/// wallet journal -- were mounted under `/api/public/` with no authentication
/// at all: a reservation's detail returned the guest's name and phone to
/// anyone with the id, its `action` route drove the state machine and wrote a
/// caller-supplied `actor` into the audit trail, `issue_pass` minted a signed
/// entry pass, and `wallet/topup` wrote balanced ledger rows for any amount
/// anybody asked for. They had no caller in `public/`; they were surface.
///
/// This is the guard they now share, and it asks the question every hub route
/// asks: is the caller authenticated, and is the venue they belong to THIS
/// one. A principal from another venue is answered 404 -- a 403 would confirm
/// the venue exists.
///
/// It is deliberately NOT a role check. Which roles a route accepts is the
/// route's business; this one only settles identity and tenancy.
pub async fn principal_at(
    req: &Request,
    env: &Env,
    db: &D1Database,
    venue: &str,
    now_ms: i64,
) -> std::result::Result<Principal, Response> {
    let p = match authenticate(req, env, db, now_ms).await {
        Ok(p) => p,
        Err(e) => return Err(e.into_response().unwrap()),
    };
    if belongs_to(&p, venue) {
        Ok(p)
    } else {
        Err(Response::error("not found", 404).unwrap())
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
    authenticate_token(&raw, env, db, now_ms).await
}

/// The same, for a caller that holds the token rather than the request.
///
/// A BROWSER CANNOT SET A HEADER ON A WEBSOCKET. The handshake carries the
/// token as a subprotocol instead (`Sec-WebSocket-Protocol: bearer, <token>`),
/// which is the standard way round it and keeps the token out of the URL --
/// out of logs, out of history, out of anything that keeps URLs. This is the
/// door that path comes in by; everything it does afterwards is what
/// `authenticate` does.
pub async fn authenticate_token(
    raw: &str,
    env: &Env,
    _db: &D1Database,
    now_ms: i64,
) -> std::result::Result<Principal, AuthError> {
    let raw = raw.to_string();

    // ── AN API KEY IS NOT A JWT AND IS NOT VERIFIED LIKE ONE ──
    //
    // The prefix is what tells them apart, and it is deliberate rather than
    // decorative: a key that looked like a token would be handed to `verify`,
    // fail as malformed, and the owner would be told their session expired
    // instead of that their key is wrong. `dowiz_` is also what lets a secret
    // scanner recognise one in a repository.
    if let Some(rest) = raw.strip_prefix("dowiz_") {
        return api_key_principal(env, rest, now_ms).await;
    }

    let claims = verify(env, &raw, now_ms)?;
    match claims {
        Claims::Owner { user_id, active_location_id, .. } => {
            // Authority is re-derived, never trusted from the token.
            //
            // AND IT IS RE-DERIVED FOR THE VENUE THE CLAIM NAMES, not for
            // "somewhere". The question this used to ask was "is this user an
            // owner of anything", which is the wrong question for an owner of
            // two venues: remove them from venue A and, while they still own
            // B, every route that trusts `belongs_to` -- which compares the
            // claim and nothing else -- kept letting them into A for the whole
            // 24-hour life of the access token. Those are exactly the routes
            // moved behind `principal_at` in the red-team pass: a reservation's
            // guest name and phone, its FSM, its signed entry pass, a thread,
            // and the wallet.
            //
            // A claim that names NO venue is a platform-admin token and is
            // handled below as it always was; it owns no restaurant by design.
            let t = crate::identity_store::identity(env)
                .await
                .map_err(|e| AuthError::Db(e.to_string()))?;
            let still = match active_location_id.as_deref() {
                Some(venue) => crate::identity_store::membership(&t, venue, &user_id)
                    .filter(|m| crate::identity_store::s_of(m, "role") == "owner")
                    .is_some(),
                // A claim that names NO venue is a platform-admin token and is
                // handled below as it always was; it owns no restaurant by
                // design. "Owner of anything" is only asked when no venue was
                // claimed -- asking it when one WAS claimed is the defect that
                // kept a removed owner in venue A for the life of their token.
                None => crate::identity_store::memberships_of(&t, &user_id)
                    .iter()
                    .any(|(_, role)| role == "owner"),
            };
            let still = if still { Some(String::new()) } else { None };
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
            // FOUR CONDITIONS, TWO READS. The session says whether it is live;
            // the roster says whether this courier is on THIS venue's. They are
            // in two images because they are two facts with different lifetimes
            // -- a session turns over, a roster does not -- and the join that
            // used to decide both at once is now two lookups by key.
            let (sess, crew) = futures_util::future::join(
                crate::identity_store::sessions(env),
                crate::identity_store::couriers(env),
            )
            .await;
            let sess = sess.map_err(|e| AuthError::Db(e.to_string()))?;
            let crew = crew.map_err(|e| AuthError::Db(e.to_string()))?;
            let row = crate::identity_store::rec(&sess, crate::identity_store::K_CSESSION, &jti)
                .filter(|r| crate::identity_store::s_of(r, "courier_id") == sub)
                .map(|r| Row {
                    revoked_at_ms: r
                        .get("revoked_at_ms")
                        .and_then(serde_json::Value::as_i64),
                    expires_at_ms: crate::identity_store::i_of(&r, "expires_at_ms"),
                    has_location: crew
                        .get(
                            crate::identity_store::K_ROSTER,
                            &crate::identity_store::roster_id(&active_location_id, &sub),
                        )
                        .is_some() as i64,
                });
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
    env: &Env,
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
    let sess = crate::identity_store::sessions(env)
        .await
        .map_err(|e| AuthError::Db(e.to_string()))?;
    let row = crate::identity_store::rec(&sess, crate::identity_store::K_APIKEY, id).map(|r| Row {
        owner_id: crate::identity_store::s_of(&r, "owner_id"),
        location_id: crate::identity_store::s_of(&r, "location_id"),
        key_hash: crate::identity_store::s_of(&r, "key_hash"),
        expires_at_ms: crate::identity_store::i_of(&r, "expires_at_ms"),
        revoked_at_ms: r.get("revoked_at_ms").and_then(serde_json::Value::as_i64),
    });
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
    //
    // AND NOT ALLOWED TO FAIL THE CALL. It was `let _ = ...` before and it
    // stays that way: a key that works must not stop working because a
    // bookkeeping write lost a generation guard.
    let key_id = id.to_string();
    let _ = crate::identity_store::with_sessions(env, move |t| {
        if let Some(mut r) =
            crate::identity_store::rec(t, crate::identity_store::K_APIKEY, &key_id)
        {
            r["last_used_ms"] = serde_json::json!(now_ms);
            let loc = crate::identity_store::s_of(&r, "location_id");
            let index = vec![(
                crate::identity_store::apikey_at(&loc, &key_id),
                key_id.clone(),
            )];
            t.put(
                crate::identity_store::K_APIKEY,
                &key_id,
                &r.to_string(),
                &index,
                &[],
            )
            .map_err(|e| Error::RustError(format!("api key: {e}")))?;
        }
        Ok(())
    })
    .await;
    Ok(Principal::Owner {
        user_id: row.owner_id,
        active_location_id: Some(row.location_id),
    })
}

// `require_location` WAS HERE AND IT ENCODED THE HOLE. For an owner whose
// claim carried no location -- a platform admin -- it answered "fine, any
// venue", because it was written when a role was the whole question. Nothing
// called it, and `belongs_to` above is the rule that replaced it: a claim that
// names no hub belongs to no hub. Deleted rather than left for the next
// caller to reach for.

#[cfg(test)]
mod tenancy_tests {
    use super::*;

    fn owner(at: Option<&str>) -> Principal {
        Principal::Owner { user_id: "u1".into(), active_location_id: at.map(str::to_string) }
    }

    /// THE TENANCY TEST, WHICH THREE ROUTE FAMILIES DID NOT HAVE. A token is
    /// proof of who, never of where; the venue comes from the URL, and these
    /// two have to be compared or one venue's owner reads another's.
    #[test]
    fn a_principal_belongs_only_to_the_venue_its_claim_names() {
        assert!(belongs_to(&owner(Some("sushi-durres")), "sushi-durres"));
        assert!(!belongs_to(&owner(Some("sushi-durres")), "dubin-durres"));

        // A PLATFORM ADMIN BELONGS TO NO HUB. Its token carries no location at
        // all, and `accounts.rs` states plainly that it "cannot be used to
        // read a hub" -- which was true of every route that asked for a
        // membership and false of the three that only asked for a role.
        assert!(!belongs_to(&owner(None), "sushi-durres"));
        assert!(!belongs_to(&owner(None), ""));

        let courier = Principal::Courier {
            courier_id: "c1".into(),
            active_location_id: "sushi-durres".into(),
            session_id: "s1".into(),
        };
        assert!(belongs_to(&courier, "sushi-durres"));
        assert!(!belongs_to(&courier, "dubin-durres"));

        let customer = Principal::Customer {
            customer_id: "cu1".into(),
            order_id: "ord_1".into(),
            location_id: "sushi-durres".into(),
        };
        assert!(belongs_to(&customer, "sushi-durres"));
        assert!(!belongs_to(&customer, "dubin-durres"));
    }
}

#[cfg(test)]
mod password_tests {
    use super::*;

    #[test]
    fn a_password_verifies_against_its_own_hash() {
        let h = hash_password("correct horse battery staple").expect("hash");
        assert!(verify_password("correct horse battery staple", &h));
        assert!(!verify_password("correct horse battery stapl", &h));
        assert!(!verify_password("", &h));
    }

    /// The parameters must actually be the cheaper ones. A `Params::new` that
    /// silently fell back to the default would leave the 503 in place while
    /// every test here still passed.
    #[test]
    fn hashes_are_written_at_the_affordable_cost() {
        let h = hash_password("x").expect("hash");
        assert!(h.contains("m=8192"), "wrong memory cost: {h}");
        assert!(h.contains("t=3"), "wrong time cost: {h}");
        assert!(!needs_rehash(&h), "a hash we just wrote must not need rewriting");
    }

    /// A hash written by the old default is still accepted -- deploying this
    /// must not lock out the accounts that exist -- and is flagged for rewrite.
    #[test]
    fn the_old_expensive_hashes_still_work_and_are_flagged() {
        use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
        let salt = SaltString::generate(&mut OsRng);
        let old = argon2::Argon2::default()
            .hash_password(b"legacy", &salt)
            .expect("legacy hash")
            .to_string();
        assert!(old.contains("m=19456"), "this must be the OLD cost: {old}");

        assert!(verify_password("legacy", &old), "an existing account must still sign in");
        assert!(needs_rehash(&old), "and must be marked for rewrite");
    }

    /// THE LATENT ONE. `verify_password_constant_work` spends work on a miss so
    /// that "no such account" and "wrong password" take the same time. If the
    /// dummy hash does not PARSE, `verify_password` returns false immediately,
    /// no argon2 runs, and the miss becomes measurably faster than the hit --
    /// which is exactly the leak the dummy exists to close, passing silently.
    #[test]
    fn the_dummy_hash_actually_costs_something() {
        use argon2::password_hash::PasswordHash;
        let parsed = PasswordHash::new(DUMMY).expect("the dummy must parse or it costs nothing");
        let params = argon2::Params::try_from(&parsed).expect("and carry usable parameters");
        assert_eq!(params.m_cost(), 8192, "the miss must cost what the hit costs");
        assert_eq!(params.t_cost(), 3);

        // And the function itself answers false without panicking.
        assert!(!verify_password_constant_work("anything", None));
    }
}
