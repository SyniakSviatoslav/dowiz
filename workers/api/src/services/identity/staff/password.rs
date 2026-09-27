//! A member of staff's password: changed by the person, or reset by the owner
//! of their venue (operator, 2026-09-27: production had no way to change one --
//! a hash was written only by `/api/staff/claim`, and a second claim of an
//! existing account refused without the old password).
//!
//!   * `POST /api/staff/password` `{email, old_password, new_password}` -- the
//!     person (`staff::staff_password`, main 282d2c45). The new one obeys
//!     `MIN_PASSWORD_CHARS`; the old one is checked with the SAME work on a
//!     miss as on a hit (`verify_password_constant_work`); every open staff
//!     session of theirs ends, and the console signs in again with the new one.
//!   * `POST /api/owner/staff/:id/password` `{new_password}` -- the owner of
//!     the venue, for a member of staff of THAT venue only. An account that is
//!     also a member anywhere else (an owner of another venue, staff there) is
//!     REFUSED: a password is the person's across every venue, and one owner
//!     setting it would hand them the other venue's door.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use super::super::staff_rules as sr;
use crate::auth::{self, verify_password_constant_work};
use crate::identity_store as ids;

/// The new password is long enough -- the claim's rule, the one rule.
pub fn new_password_ok(new: &str) -> std::result::Result<(), (u16, &'static str)> {
    if new.chars().count() < sr::MIN_PASSWORD_CHARS {
        return Err((400, "choose a password of at least 8 characters"));
    }
    Ok(())
}

/// The person's own change: the new one's length first (no derivation is
/// spent on a request that cannot succeed), then the old password, always
/// paying the derivation, so a missing account costs what a present one does.
pub fn self_change(stored: Option<&str>, old: &str, new: &str) -> std::result::Result<(), (u16, &'static str)> {
    new_password_ok(new)?;
    if !verify_password_constant_work(old, stored) {
        return Err((401, "invalid credentials"));
    }
    Ok(())
}

/// May the owner of `venue` set this person's password? `here` is their
/// membership row at `venue` (active or not); `everywhere` every venue they are
/// an active member of, with the word.
pub fn owner_may_reset(here: Option<&Value>, everywhere: &[(String, String)], venue: &str) -> std::result::Result<(), (u16, &'static str)> {
    if !here.is_some_and(sr::is_staff_member) {
        return Err((403, "not a member of staff at your venue"));
    }
    if everywhere.iter().any(|(loc, _)| loc != venue) {
        return Err((403, "this account is also used at another venue: its owner changes the password themselves"));
    }
    Ok(())
}

/// The user record with a new hash, and when it changed.
pub fn with_hash(mut user: Value, hash: &str, now_ms: i64) -> Value {
    user["password_hash"] = json!(hash);
    user["password_changed_at_ms"] = json!(now_ms);
    user
}

/// Store `hash` on `uid`'s record, in the identity object's own turn.
pub(crate) async fn store_hash(env: &Env, uid: &str, hash: String, now: i64) -> Result<bool> {
    let uid = uid.to_string();
    ids::with_identity(env, move |t| {
        let Some(u) = ids::rec(t, ids::K_USER, &uid) else { return Ok(false) };
        let email = ids::s_of(&u, "email");
        let index = vec![(ids::user_by_email(&email), uid.clone())];
        t.put(ids::K_USER, &uid, &with_hash(u, &hash, now).to_string(), &index, &[])
            .map_err(|e| Error::RustError(format!("user: {e}")))?;
        Ok(true)
    })
    .await
}

/// End every open session of `uid` at `venue`; how many ended. The suspend
/// path (`staff_admin::set_staff`) and both password paths end them here.
pub(crate) async fn end_sessions(env: &Env, venue: &str, uid: &str, now: i64) -> Result<usize> {
    let (loc, uid) = (venue.to_string(), uid.to_string());
    ids::with_sessions(env, move |t| {
        let mut n = 0usize;
        for (_, sid) in t.scan(&sr::ssessions_prefix(&loc, &uid)) {
            let Some(r) = ids::rec(t, sr::K_SSESSION, &sid) else { continue };
            if r.get("revoked_at_ms").is_some_and(|v| !v.is_null()) {
                continue;
            }
            let index = [(sr::ssession_of(&loc, &uid, &sid), sid.clone())];
            t.put(sr::K_SSESSION, &sid, &sr::revoked(r, now).to_string(), &index, &[])
                .map_err(|e| Error::RustError(format!("staff session: {e}")))?;
            n += 1;
        }
        Ok(n)
    })
    .await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResetIn {
    new_password: String,
    #[serde(default)]
    location_id: Option<String>,
}

/// `POST /api/owner/staff/:id/password` -- the owner sets a new password for a
/// member of staff of their venue, and every session of that person there ends.
/// Routed by the `lib.rs` line handed back to the main session.
pub async fn owner_reset(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: ResetIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let loc = match crate::owner::owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let Some(uid) = ctx.param("id").cloned() else {
        return Response::error("missing staff id", 400);
    };
    if let Err((s, why)) = new_password_ok(&body.new_password) {
        return Response::error(why, s);
    }
    let t = ids::identity(&ctx.env).await?;
    let here = ids::rec(&t, ids::K_MEMBER, &ids::member_id(&loc, &uid));
    // EVERY venue the account belongs to, suspended or not: a suspended owner
    // row elsewhere is still an account this password would open there.
    let everywhere: Vec<(String, String)> = t
        .scan(&format!("member.user/{uid}/"))
        .into_iter()
        .filter_map(|(key, _)| {
            let l = key.rsplit('/').next()?.to_string();
            let m = ids::rec(&t, ids::K_MEMBER, &ids::member_id(&l, &uid))?;
            Some((l, ids::s_of(&m, "role")))
        })
        .collect();
    if let Err((s, why)) = owner_may_reset(here.as_ref(), &everywhere, &loc) {
        return Response::error(why, s);
    }
    let hash = match auth::hash_password(&body.new_password) {
        Ok(h) => h,
        Err(e) => return e.into_response(),
    };
    let now = ctx.data.now_ms;
    if !store_hash(&ctx.env, &uid, hash, now).await? {
        return Response::error("not found", 404);
    }
    let ended = end_sessions(&ctx.env, &loc, &uid, now).await?;
    let _ = body.location_id;
    Response::from_json(&json!({ "ok": true, "sessionsEnded": ended }))
}

#[cfg(test)]
#[path = "password/tests.rs"]
mod tests;
