//! PURE. What a staff session row, a staff invite and a staff roster row say,
//! and which of them still opens a door.
//!
//! THE COURIER'S MACHINERY, REUSED RATHER THAN RE-INVENTED. A courier is
//! invited with a code stored as a digest, claims it with a password they chose
//! themselves, and is then held by a SESSION ROW that the token names by `jti`
//! — so the owner can end the session before the token expires. A member of
//! staff gets the same three things. What differs is only where the person
//! lives: a courier is a record in the couriers image, a member of staff is a
//! `user` with a `member` row at the venue whose `role` is a staff word
//! (`dowiz_hub::caps::Preset`). That membership row is what `auth.rs` already
//! re-reads on every call; the session row is what this adds.
//!
//! WHY A SESSION ROW AT ALL, when the membership is re-read anyway. Suspending
//! the membership ends EVERY device of that person at once. Ending one device
//! — the tablet left at the pass, the phone that was stolen — is a session,
//! and a stateless token has no handle to end one by.
//!
//! Every function here is a function of its arguments; the handlers that call
//! them are in `staff.rs` and `staff_admin.rs`.

use dowiz_hub::caps::Preset;
use serde_json::{json, Value};

use crate::identity_store::{i_of, s_of};

/// A staff access token lives for one SHIFT. Twelve hours covers a double at a
/// restaurant; past that the person signs in again, which is also the moment
/// the roster's current word for them is re-read into the token.
pub const STAFF_TTL_MS: i64 = 12 * 60 * 60 * 1000;

/// A staff invite lives a week, as a courier's does (`hiring.rs`).
pub const STAFF_INVITE_TTL_MS: i64 = 7 * 24 * 60 * 60 * 1000;

/// A password a member of staff chooses is at least this long, the courier's
/// rule (`accounts::courier_claim`).
pub const MIN_PASSWORD_CHARS: usize = 8;

/// The record kinds, in the SESSIONS image and the IDENTITY image. Composed
/// here, once, by every reader and writer — the rule `identity_store` states.
pub const K_SSESSION: &str = "ssession";
pub const K_SINVITE: &str = "sinvite";

/// Every session of one person at one venue, so "end all of them" is a prefix
/// scan rather than a walk of every session on the platform.
pub fn ssession_of(location_id: &str, person_id: &str, session_id: &str) -> String {
    format!("ssession.person/{location_id}/{person_id}/{session_id}")
}

pub fn ssessions_prefix(location_id: &str, person_id: &str) -> String {
    format!("ssession.person/{location_id}/{person_id}/")
}

/// A staff invite, found by the address it was sent to and listable per venue.
pub fn sinvite_by_email(email: &str) -> String {
    format!("sinvite.email/{}", email.trim().to_ascii_lowercase())
}

pub fn sinvite_at(location_id: &str, id: &str) -> String {
    format!("sinvite.loc/{location_id}/{id}")
}

/// Which staff words an OWNER may hand out from the console.
///
/// NOT `owner`. Making somebody an owner is a different act with a different
/// blast radius — they could then remove the person who invited them — and it
/// is not something a staff invite should be able to do by a typo in a select.
/// The four presets are Kitchen, Waiter, Counter-Manager and Owner
/// (BLUEPRINT-POS-THE-ROOM §2.8). This function refuses Owner and accepts the rest.
pub fn invitable(word: &str) -> Result<Preset, &'static str> {
    match Preset::from_str(word.trim()) {
        Some(Preset::Owner) => Err("an owner is not invited as staff"),
        Some(p) => Ok(p),
        None => Err("unknown staff role"),
    }
}

/// The fields an invite is minted from, trimmed and checked. The owner's own
/// words come back as the refusal, as `roster::invite_fields` does it.
pub fn invite_fields(email: &str, name: &str) -> Result<(String, String), &'static str> {
    let email = email.trim().to_ascii_lowercase();
    if !email.contains('@') || email.len() < 3 {
        return Err("that does not look like an email address");
    }
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("who is this code for?");
    }
    Ok((email, name))
}

/// Does this stored session still stand for this person at this venue now?
///
/// FIVE REFUSALS, each its own sentence, because "revoked" and "expired" need
/// different things from the person at the till: one needs the owner, the
/// other needs to sign in again. A session belonging to somebody else, or to
/// another venue, is answered exactly like a missing one.
pub fn session_verdict(
    rec: Option<&Value>,
    person_id: &str,
    venue: &str,
    now_ms: i64,
) -> Result<(), &'static str> {
    let Some(r) = rec else { return Err("no such staff session") };
    if s_of(r, "person_id") != person_id || s_of(r, "location_id") != venue {
        return Err("no such staff session");
    }
    if r.get("revoked_at_ms").is_some_and(|v| !v.is_null()) {
        return Err("staff session revoked");
    }
    if i_of(r, "expires_at_ms") <= now_ms {
        return Err("staff session expired");
    }
    Ok(())
}

/// The session row a login writes.
pub fn session_record(person_id: &str, venue: &str, preset: Preset, now_ms: i64) -> Value {
    json!({
        "person_id": person_id, "location_id": venue, "role": preset.as_str(),
        "issued_at_ms": now_ms, "expires_at_ms": now_ms + STAFF_TTL_MS,
        "revoked_at_ms": Value::Null,
    })
}

/// A session, ended. The row stays — it is the record that it was ended.
pub fn revoked(mut rec: Value, now_ms: i64) -> Value {
    rec["revoked_at_ms"] = json!(now_ms);
    rec
}

/// A membership row that makes somebody staff at a venue.
pub fn member_record(user_id: &str, venue: &str, preset: Preset, now_ms: i64) -> Value {
    json!({
        "user_id": user_id, "location_id": venue, "role": preset.as_str(),
        "status": "active", "created_at_ms": now_ms,
    })
}

/// Is this membership a STAFF membership, as opposed to an owner's?
///
/// The list the console shows, and the rows an owner may change from it. An
/// owner's own row is never on it, so the console cannot demote or suspend the
/// person using it.
pub fn is_staff_member(member: &Value) -> bool {
    matches!(Preset::from_str(&s_of(member, "role")), Some(Preset::Kitchen | Preset::Waiter | Preset::CounterManager))
}

/// One member of staff as the console lists them.
pub fn staff_row(user_id: &str, member: &Value, name: &str) -> Value {
    json!({
        "id": user_id,
        "name": name,
        "role": s_of(member, "role"),
        "active": s_of(member, "status") == "active",
    })
}

/// An outstanding invite, or `None` once it has been spent or withdrawn.
pub fn invite_row(id: &str, rec: &Value, now_ms: i64) -> Option<Value> {
    let spent = |k: &str| rec.get(k).is_some_and(|v| !v.is_null());
    if spent("used_at_ms") || spent("revoked_at_ms") {
        return None;
    }
    let until = i_of(rec, "expires_at_ms");
    Some(json!({
        "id": id,
        "name": s_of(rec, "invited_name"),
        "role": s_of(rec, "role"),
        "untilMs": until,
        "expired": now_ms >= until,
    }))
}

/// Can this invite be claimed with this code, now?
///
/// "No such invite" and "wrong code" are ONE answer, so the route cannot be
/// used to find out which addresses a venue has invited. Expiry is told apart,
/// because a person whose code ran out needs a new one, not another attempt.
pub fn claimable(rec: Option<&Value>, code_hash: &str, now_ms: i64) -> Result<(), &'static str> {
    let Some(r) = rec else { return Err("that code does not match") };
    let spent = |k: &str| r.get(k).is_some_and(|v| !v.is_null());
    if spent("used_at_ms") || spent("revoked_at_ms") || s_of(r, "code_hash") != code_hash {
        return Err("that code does not match");
    }
    if now_ms >= i_of(r, "expires_at_ms") {
        return Err("that code has expired -- ask for a new one");
    }
    Ok(())
}

/// An owner's change to one member of staff: a new staff word, suspended or
/// restored. `None` for a row the console may not touch — an owner's own, a
/// courier's, anything that is not staff — which the handler answers as 404.
///
/// Suspending is the whole-person door: `auth.rs` reads `status` on every
/// call, so every device of theirs stops at once.
pub fn member_changed(member: &Value, role: Option<&str>, active: Option<bool>) -> Result<Value, &'static str> {
    if !is_staff_member(member) {
        return Err("not found");
    }
    let mut m = member.clone();
    if let Some(word) = role {
        m["role"] = json!(invitable(word)?.as_str());
    }
    if let Some(on) = active {
        m["status"] = json!(if on { "active" } else { "suspended" });
    }
    Ok(m)
}

#[cfg(test)]
mod tests;
