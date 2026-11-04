//! THE WORKER'S SIDE OF `POST /fold/verdict` (W-OWNER-CPU2, `hubdo/verdict.rs` has the why):
//! the few records a sign-in decision reads, asked of the platform object as bytes instead of
//! the whole identity / sessions / couriers image fetched and `Table::load`ed here.
//!
//! THE DECISION STAYS HERE. `auth::authenticate_token`, `owner::owner_at` and
//! `owner::owner_and_venue` run the same checks over the same records in the same order; only
//! the read moved. In process (`Stub::Own` inside the platform object, `Stub::Mem` in tests)
//! the ask is a function call, and the records a test compares are the records the Worker
//! decided on.

use crate::edge::Env;
use crate::wire::Call;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use worker::{Error, Method, Result};

/// What one decision needs to read. A closed body (`crate::body`): a field this does not name
/// is a refusal, as every command's is.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "q", rename_all = "snake_case", deny_unknown_fields)]
pub enum Ask {
    /// The membership of `user` at `venue`, if it is active (`identity_store::membership`).
    Member { venue: String, user: String },
    /// Every venue `user` is an active member of, with the role (`memberships_of`).
    Owned { user: String },
    /// A member of staff: their membership here and the session row their token names.
    Staff { venue: String, user: String, jti: String },
    /// A courier: the session row their token names (theirs, or nothing) and whether this
    /// venue's roster still holds them.
    Courier { venue: String, courier: String, jti: String },
    /// An API key, judged and stamped in the object (see `hubdo/verdict.rs`).
    ApiKey { id: String, secret: String, now_ms: i64 },
}

/// The records, each present only when the ask reads it. `#[serde(default)]` so a field the
/// object did not send is `None`, never a parse failure.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Answer {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owned: Option<Vec<(String, String)>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rostered: Option<bool>,
    /// `{"owner_id", "location_id"}` of a key that holds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<Value>,
    /// Why a key does not hold, in the words `judge_api_key` uses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refused: Option<String>,
}

/// One ask of the platform object; an unreachable object or an unreadable answer is an `Err`
/// (the caller's 503, `AuthError::Db`), never a refusal.
pub async fn ask(env: &Env, ask: &Ask) -> Result<Answer> {
    let stub = crate::platform_store::stub(env)?;
    // `Call::with_json`/`with_body` are #[cfg(test)]; the release build passes a body through
    // `new_with_init`, as `command::call` does (a test-only builder here broke the wasm build).
    let body = serde_json::to_vec(ask).map_err(|e| Error::RustError(format!("verdict: unencodable ask: {e}")))?;
    let mut call = Call::new_with_init(
        "https://hub/fold/verdict",
        crate::wire::RequestInit::new().with_method(Method::Post).with_body(Some(body)),
    )?;
    call.headers_mut()?.set("content-type", "application/json")?;
    let mut res = stub.fetch_with_request(call).await?;
    let status = res.status_code();
    let text = res.text().await?;
    if status != 200 {
        return Err(Error::RustError(format!("verdict: the platform object answered {status}: {text}")));
    }
    serde_json::from_str(&text).map_err(|e| Error::RustError(format!("verdict: unreadable answer: {e}")))
}

/// The active membership of `user` at `venue`, or nothing.
pub async fn member(env: &Env, venue: &str, user: &str) -> Result<Option<Value>> {
    Ok(ask(env, &Ask::Member { venue: venue.into(), user: user.into() }).await?.member)
}

/// Every venue `user` is an active member of, with the role.
pub async fn owned(env: &Env, user: &str) -> Result<Vec<(String, String)>> {
    Ok(ask(env, &Ask::Owned { user: user.into() }).await?.owned.unwrap_or_default())
}

/// `(membership here, session row)` of a member of staff.
pub async fn staff(env: &Env, venue: &str, user: &str, jti: &str) -> Result<(Option<Value>, Option<Value>)> {
    let a = ask(env, &Ask::Staff { venue: venue.into(), user: user.into(), jti: jti.into() }).await?;
    Ok((a.member, a.session))
}

/// `(the courier's own session row, still on this venue's roster)`.
pub async fn courier(env: &Env, venue: &str, courier: &str, jti: &str) -> Result<(Option<Value>, bool)> {
    let a = ask(env, &Ask::Courier { venue: venue.into(), courier: courier.into(), jti: jti.into() }).await?;
    Ok((a.session, a.rostered.unwrap_or(false)))
}

/// An API key's `(owner_id, location_id)`, or why it does not hold.
pub async fn api_key(env: &Env, id: &str, secret: &str, now_ms: i64) -> Result<std::result::Result<(String, String), &'static str>> {
    let a = ask(env, &Ask::ApiKey { id: id.into(), secret: secret.into(), now_ms }).await?;
    if let Some(k) = a.key {
        let s = |f: &str| k.get(f).and_then(Value::as_str).unwrap_or("").to_string();
        return Ok(Ok((s("owner_id"), s("location_id"))));
    }
    Ok(Err(reason(a.refused.as_deref().unwrap_or(""))))
}

// ── the API key's judgement, pure, shared by the object and the tests ─────────

pub const KEY_MALFORMED: &str = "that key is malformed";
pub const KEY_UNKNOWN: &str = "no such key";
pub const KEY_REVOKED: &str = "that key was revoked";
pub const KEY_EXPIRED: &str = "that key has expired";

/// The refusals `judge_api_key` can give, as the `&'static str` `AuthError::Revoked` carries.
/// An unknown word (an object of another build) is "no such key": fail closed, say little.
pub fn reason(s: &str) -> &'static str {
    [KEY_MALFORMED, KEY_UNKNOWN, KEY_REVOKED, KEY_EXPIRED].into_iter().find(|w| *w == s).unwrap_or(KEY_UNKNOWN)
}

/// `api_key_principal`'s four refusals, in its order, over the stored record: unknown id,
/// revoked, expired, wrong secret -- the last three say the same to a caller who holds a wrong
/// secret as to one who holds a wrong id, as before.
pub fn judge_api_key(rec: Option<&Value>, secret: &str, now_ms: i64) -> std::result::Result<(String, String), &'static str> {
    use crate::identity_store::{i_of, s_of};
    let Some(r) = rec else { return Err(KEY_UNKNOWN) };
    if r.get("revoked_at_ms").and_then(Value::as_i64).is_some() {
        return Err(KEY_REVOKED);
    }
    if now_ms >= i_of(r, "expires_at_ms") {
        return Err(KEY_EXPIRED);
    }
    if !super::verify_opaque(secret, &s_of(r, "key_hash")) {
        return Err(KEY_UNKNOWN);
    }
    Ok((s_of(r, "owner_id"), s_of(r, "location_id")))
}
