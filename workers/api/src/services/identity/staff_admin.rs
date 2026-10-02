//! The owner's side of the staff roster: who is on it, inviting somebody, and
//! changing or suspending them.
//!
//! THE COURIER'S HIRING, for the room (`services::courier::hiring`). A code is
//! minted from the platform's randomness, shown ONCE and kept only as a digest;
//! inviting the same address twice replaces the pending code rather than
//! leaving two alive. What an owner may hand out is `staff_rules::invitable`:
//! the ruled staff words, never `owner`.

use serde::Deserialize;
use serde_json::json;
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

use super::staff_rules as sr;
use crate::identity_store as ids;
use crate::owner::owner_and_venue;

/// `GET /api/owner/staff` — the venue's staff and its outstanding invites.
pub async fn list_staff(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let t = ids::identity(&ctx.env).await?;
    let staff: Vec<_> = t
        .scan(&format!("member.venue/{loc}/"))
        .into_iter()
        .filter_map(|(_, uid)| {
            let m = ids::rec(&t, ids::K_MEMBER, &ids::member_id(&loc, &uid))?;
            if !sr::is_staff_member(&m) {
                return None;
            }
            let name = ids::rec(&t, ids::K_USER, &uid).map(|u| ids::s_of(&u, "display_name")).unwrap_or_default();
            Some(sr::staff_row(&uid, &m, &name))
        })
        .collect();
    let invites: Vec<_> = t
        .scan(&format!("sinvite.loc/{loc}/"))
        .into_iter()
        .filter_map(|(_, id)| sr::invite_row(&id, &ids::rec(&t, sr::K_SINVITE, &id)?, ctx.data.now_ms))
        .collect();
    Response::from_json(&json!({ "staff": staff, "invites": invites }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InviteIn {
    email: String,
    name: String,
    role: String,
    #[serde(default)]
    location_id: Option<String>,
}

/// `POST /api/owner/staff/invite` — mint a code for one address, shown ONCE.
pub async fn invite_staff(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let (owner, loc) = match owner_and_venue(&req, &ctx).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    // AUTHORITY BEFORE THE BODY (W-FIX O9): nobody's JSON is parsed before the door.
    let body: InviteIn = match crate::body::parse(&mut req).await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let (email, name) = match sr::invite_fields(&body.email, &body.name) {
        Ok(v) => v,
        Err(why) => return Response::error(why, 400),
    };
    let preset = match sr::invitable(&body.role) {
        Ok(p) => p,
        Err(why) => return Response::error(why, 400),
    };
    let Some(minted) = crate::edge_id()
        .zip(crate::edge_id())
        .map(|(a, b)| format!("{a}{b}").replace('-', ""))
        .and_then(|hex| crate::services::courier::roster::code_from_entropy(&hex, crate::auth::sha256_hex))
    else {
        return Response::error("no platform CSPRNG", 500);
    };
    let Some(id) = crate::edge_id() else {
        return Response::error("no platform CSPRNG", 500);
    };
    let now = ctx.data.now_ms;
    let (em, l2, own, nm, ch, iid) = (email.clone(), loc.clone(), owner.clone(), name.clone(), minted.hash, id.clone());
    // THE REVOKE AND THE MINT IN ONE TURN (`invite_turn`): a second invite to
    // one address replaces THIS venue's first, and only holds if nothing runs
    // in between.
    let rec = json!({
        "id": iid, "location_id": l2, "created_by_owner_id": own, "role": preset.as_str(),
        "invited_email": em, "invited_name": nm, "code_hash": ch,
        "expires_at_ms": now + sr::STAFF_INVITE_TTL_MS, "created_at_ms": now,
        "used_at_ms": null, "revoked_at_ms": null,
    })
    .to_string();
    let taken = ids::with_identity(&ctx.env, move |t| invite_turn(t, &em, &l2, &iid, &rec, now)).await?;
    if taken {
        return Response::error("that address has an open invitation from another venue", 409);
    }
    let _ = body.location_id;
    Response::from_json(&json!({ "code": minted.code, "expiresMs": now + sr::STAFF_INVITE_TTL_MS }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChangeIn {
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    active: Option<bool>,
    #[serde(default)]
    location_id: Option<String>,
}

/// `POST /api/owner/staff/:id` — `{role?, active?}`: re-word or suspend.
///
/// SUSPENDING ALSO ENDS EVERY SESSION of that person at this venue. The
/// membership read already refuses them on the next call; ending the sessions
/// as well means a later restore does not quietly revive a tablet that was
/// left somewhere during the suspension.
pub async fn set_staff(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // AUTHORITY BEFORE THE BODY (W-FIX O9): nobody's JSON is parsed before the door.
    let body: ChangeIn = match crate::body::parse(&mut req).await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let Some(uid) = ctx.param("id").cloned() else {
        return Response::error("missing staff id", 400);
    };
    let (l2, u2, role, active) = (loc.clone(), uid.clone(), body.role.clone(), body.active);
    let changed = ids::with_identity(&ctx.env, move |t| {
        let Some(m) = ids::rec(t, ids::K_MEMBER, &ids::member_id(&l2, &u2)) else {
            return Ok(Err("not found"));
        };
        let m = match sr::member_changed(&m, role.as_deref(), active) {
            Ok(m) => m,
            Err(why) => return Ok(Err(why)),
        };
        let index = [(ids::member_by_venue(&l2, &u2), u2.clone()), (ids::member_by_user(&u2, &l2), l2.clone())];
        t.put(ids::K_MEMBER, &ids::member_id(&l2, &u2), &m.to_string(), &index, &[])
            .map_err(|e| Error::RustError(format!("member: {e}")))?;
        Ok(Ok(m))
    })
    .await?;
    let m = match changed {
        Ok(m) => m,
        Err("not found") => return Response::error("not found", 404),
        Err(why) => return Response::error(why, 400),
    };
    let ended = if body.active == Some(false) {
        super::staff::password::end_sessions(&ctx.env, &loc, &uid, ctx.data.now_ms).await?
    } else {
        0
    };
    let _ = body.location_id;
    Response::from_json(&json!({ "ok": true, "role": m["role"], "active": m["status"] == "active", "sessionsEnded": ended }))
}

/// THE CHECK, THE REVOKE AND THE MINT, as one table turn over the identity
/// image. `Ok(true)` means another venue's invitation to this address is still
/// open and NOTHING was written; `Ok(false)` means `rec` (invitation `iid` of
/// `venue`) now stands and holds the address index.
///
/// ONLY THIS VENUE'S OWN PENDING INVITATION IS REVOKED (W-FIX H2 / W-AUDIT O6,
/// 2026-09-27). The address index is platform-wide and this turn used to revoke
/// whatever it found there: venue B inviting an address venue A had invited
/// revoked A's code -- a cross-tenant write, the same one `hiring::invite_turn`
/// closed for couriers (S8). Skipping the revoke alone is not enough: the index
/// holds ONE invitation and `staff_claim` finds the code through it, so taking
/// the index would strand A's code just the same. Another venue's OPEN
/// invitation therefore refuses; a spent, revoked or expired one of another
/// venue is left exactly as it is and the address index moves to this one --
/// a person may work at two venues, one after the other.
pub(crate) fn invite_turn(
    t: &mut dowiz_hub::table::Table,
    em: &str,
    venue: &str,
    iid: &str,
    rec: &str,
    now: i64,
) -> Result<bool> {
    if let Some(pending) = t.lookup(&sr::sinvite_by_email(em)) {
        if let Some(mut i) = ids::rec(t, sr::K_SINVITE, &pending) {
            if ids::s_of(&i, "location_id") != venue {
                let open = |k: &str| i.get(k).map_or(true, serde_json::Value::is_null);
                if open("used_at_ms") && open("revoked_at_ms") && now < ids::i_of(&i, "expires_at_ms") {
                    return Ok(true);
                }
            } else {
                i["revoked_at_ms"] = json!(now);
                let at = sr::sinvite_at(venue, &pending);
                t.put(sr::K_SINVITE, &pending, &i.to_string(), &[(at, pending.clone())], &[])
                    .map_err(|e| Error::RustError(format!("invite: {e}")))?;
            }
        }
    }
    let index = [(sr::sinvite_by_email(em), iid.to_string()), (sr::sinvite_at(venue, iid), iid.to_string())];
    t.put(sr::K_SINVITE, iid, rec, &index, &[]).map_err(|e| Error::RustError(format!("invite: {e}")))?;
    Ok(false)
}

#[cfg(test)]
mod tests;
