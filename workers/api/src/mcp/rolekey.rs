//! PURE. A key minted for ONE person who is not the owner: a waiter, the
//! kitchen, a counter-manager, a courier.
//!
//! **A KEY IS A SESSION ROW.** Not a new record kind: the key lives in the
//! same `ssession` (staff) or `csession` (courier) row a sign-in writes, with
//! two extra fields, `mcp_key_hash` and `label`. That one choice is what makes
//! the four promises hold without a line of new enforcement:
//!
//!   * **never more than the person.** At the MCP door the key becomes a
//!     five-minute token of the person's own kind (`Claims::Staff` /
//!     `Claims::Courier`) bound to this row by `jti`, and that token goes
//!     through `auth::authenticate_token` like any other: the staff roster word
//!     is re-read and the signed caps narrowed to it; the courier's roster row
//!     is re-read. A waiter demoted to kitchen is a kitchen key on the next call.
//!   * **revocable.** Revoking the row ends it; so does everything that already
//!     ends a person's sessions: suspending a member of staff
//!     (`staff_admin::set_staff`), deactivating a courier (`hiring`).
//!   * **useless anywhere but `/api/mcp`.** The prefixes `dowizs_` / `dowizc_`
//!     are neither a JWT nor the owner's `dowiz_`, so every other route refuses
//!     the raw key; and `api_key_principal` looks in the `apikey` kind, where no
//!     row of these ids exists, so re-spelling one as `dowiz_…` finds nothing.
//!   * **not a refresh token.** A courier row's `token_hash` is a string no
//!     secret hashes to, so `/api/courier/auth/refresh` can never turn a key
//!     into a day-long courier session.
//!
//! Every function here is a function of its arguments; `keys.rs` does the I/O.

use dowiz_hub::caps::Preset;
use serde_json::{json, Value};

use crate::auth::{verify_opaque, Claims};
use crate::identity_store::{i_of, s_of};

/// A person's key lives ninety days. Shorter than the owner's year: it is a
/// credential of somebody who may leave, and a person re-minting a key every
/// quarter is cheaper than a key outliving the job.
pub const KEY_TTL_MS: i64 = 90 * 24 * 60 * 60 * 1000;
/// A tool's token lives this long; a call takes seconds.
pub const TOOL_TOKEN_TTL_MS: i64 = 5 * 60 * 1000;
/// What a courier key row carries where a sign-in row carries its refresh
/// hash. Not `sha256:`-prefixed and not a PHC string, so `verify_opaque`
/// answers false for every secret: a key is never a refresh token.
pub const NOT_A_REFRESH: &str = "mcp-key:not-a-refresh-token";
/// A label is read months later, by the person deciding whether to revoke.
pub const LABEL_MAX_CHARS: usize = 80;

/// Whose key: which session kind it lives in, and how it is spelled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Holder {
    Staff,
    Courier,
}

impl Holder {
    pub fn prefix(self) -> &'static str {
        match self {
            Holder::Staff => "dowizs_",
            Holder::Courier => "dowizc_",
        }
    }
    /// The session kind the row is in.
    pub fn kind(self) -> &'static str {
        match self {
            Holder::Staff => crate::services::identity::staff_rules::K_SSESSION,
            Holder::Courier => crate::identity_store::K_CSESSION,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Holder::Staff => "staff",
            Holder::Courier => "courier",
        }
    }
    pub fn from_str(s: &str) -> Option<Holder> {
        [Holder::Staff, Holder::Courier].into_iter().find(|h| h.as_str() == s)
    }
}

/// `dowizs_<id>.<secret>` -> (Staff, id, secret). Anything else is `None`,
/// including the owner's `dowiz_` keys and every JWT.
pub fn parse(raw: &str) -> Option<(Holder, &str, &str)> {
    let (h, rest) = [Holder::Staff, Holder::Courier]
        .into_iter()
        .find_map(|h| raw.strip_prefix(h.prefix()).map(|r| (h, r)))?;
    let (id, secret) = rest.split_once('.')?;
    if id.is_empty() || secret.is_empty() {
        return None;
    }
    Some((h, id, secret))
}

pub fn spell(h: Holder, id: &str, secret: &str) -> String {
    format!("{}{id}.{secret}", h.prefix())
}

/// The label, trimmed, or the owner's-key refusal in the same words.
pub fn label_of(raw: &str) -> Result<String, &'static str> {
    let l = raw.trim().to_string();
    if l.is_empty() || l.chars().count() > LABEL_MAX_CHARS {
        return Err("say what this key is for");
    }
    Ok(l)
}

/// The staff session row a key lives in. `session_verdict` reads the same
/// five fields a sign-in row has; the rest is the key's.
pub fn staff_row(person: &str, venue: &str, role: &str, label: &str, hash: &str, now: i64) -> Value {
    json!({
        "person_id": person, "location_id": venue, "role": role,
        "issued_at_ms": now, "expires_at_ms": now + KEY_TTL_MS, "revoked_at_ms": Value::Null,
        "label": label, "mcp_key_hash": hash,
    })
}

/// The courier session row a key lives in. `family_id` is the row itself: a
/// key has no rotation family, and a family of one is revoked with it.
pub fn courier_row(courier: &str, venue: &str, id: &str, label: &str, hash: &str, now: i64) -> Value {
    json!({
        "courier_id": courier, "family_id": id, "token_hash": NOT_A_REFRESH,
        "active_location_id": venue,
        "issued_at_ms": now, "expires_at_ms": now + KEY_TTL_MS, "revoked_at_ms": Value::Null,
        "label": label, "mcp_key_hash": hash,
    })
}

/// Is this row a key at all? A sign-in row has no `mcp_key_hash`, so a
/// session id learned from anywhere is not a key id.
pub fn is_key(row: &Value) -> bool {
    !s_of(row, "mcp_key_hash").is_empty()
}

fn revoked(row: &Value) -> bool {
    row.get("revoked_at_ms").is_some_and(|v| !v.is_null())
}

/// The person a row belongs to, and the venue it is at, by holder.
pub fn owner_of(h: Holder, row: &Value) -> (String, String) {
    match h {
        Holder::Staff => (s_of(row, "person_id"), s_of(row, "location_id")),
        Holder::Courier => (s_of(row, "courier_id"), s_of(row, "active_location_id")),
    }
}

/// The key presented, checked against its row, as the claims of a token that
/// lives five minutes (never past the key's own expiry). The person and venue
/// come from the ROW, never from anything the caller said.
pub fn verdict(h: Holder, id: &str, row: Option<&Value>, secret: &str, now: i64) -> Result<Claims, &'static str> {
    let Some(row) = row.filter(|r| is_key(r)) else { return Err("no such key") };
    if !verify_opaque(secret, &s_of(row, "mcp_key_hash")) {
        return Err("no such key");
    }
    if revoked(row) {
        return Err("that key was revoked");
    }
    let until = i_of(row, "expires_at_ms");
    if until <= now {
        return Err("that key has expired");
    }
    let exp = (now + TOOL_TOKEN_TTL_MS).min(until);
    let (sub, venue) = owner_of(h, row);
    if sub.is_empty() || venue.is_empty() {
        return Err("no such key");
    }
    Ok(match h {
        Holder::Staff => {
            // The word the key was minted under; the guard narrows it to the
            // roster's word NOW, so a demotion since then wins.
            let preset = Preset::from_str(&s_of(row, "role")).ok_or("that key names no staff role")?;
            Claims::Staff { sub, active_location_id: venue, jti: id.to_string(), caps: preset.caps().to_string(), iat: now, exp }
        }
        Holder::Courier => Claims::Courier { sub, active_location_id: venue, jti: id.to_string(), iat: now, exp },
    })
}

/// A key that still works, as the screens show it. Never the hash.
pub fn shown(h: Holder, id: &str, row: &Value, now: i64) -> Option<Value> {
    if !is_key(row) || revoked(row) || i_of(row, "expires_at_ms") <= now {
        return None;
    }
    let (person, _) = owner_of(h, row);
    Some(json!({
        "id": id, "holder": h.as_str(), "person": person, "label": s_of(row, "label"),
        "role": if h == Holder::Courier { "courier".to_string() } else { s_of(row, "role") },
        "createdMs": i_of(row, "issued_at_ms"), "expiresMs": i_of(row, "expires_at_ms"),
    }))
}

/// May this person end this key? Only a key, only their own, only here. The
/// answer to anyone else's is the same "not found" as to no key at all.
pub fn may_revoke_own(h: Holder, row: Option<&Value>, person: &str, venue: &str) -> bool {
    row.is_some_and(|r| is_key(r) && !revoked(r) && owner_of(h, r) == (person.to_string(), venue.to_string()))
}

/// May the OWNER of `venue` end this key? Any key of their venue.
pub fn may_revoke_at(h: Holder, row: Option<&Value>, venue: &str) -> bool {
    row.is_some_and(|r| is_key(r) && !revoked(r) && owner_of(h, r).1 == venue)
}
