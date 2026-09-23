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
    let body: InviteIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let (owner, loc) = match owner_and_venue(&req, &ctx).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
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
    // THE REVOKE AND THE MINT IN ONE TURN: a second invite to one address
    // replaces the first, and only holds if nothing runs in between.
    ids::with_identity(&ctx.env, move |t| {
        if let Some(pending) = t.lookup(&sr::sinvite_by_email(&em)) {
            if let Some(mut i) = ids::rec(t, sr::K_SINVITE, &pending) {
                i["revoked_at_ms"] = json!(now);
                let at = sr::sinvite_at(&ids::s_of(&i, "location_id"), &pending);
                t.put(sr::K_SINVITE, &pending, &i.to_string(), &[(at, pending.clone())], &[])
                    .map_err(|e| Error::RustError(format!("invite: {e}")))?;
            }
        }
        let rec = json!({
            "id": iid, "location_id": l2, "created_by_owner_id": own, "role": preset.as_str(),
            "invited_email": em, "invited_name": nm, "code_hash": ch,
            "expires_at_ms": now + sr::STAFF_INVITE_TTL_MS, "created_at_ms": now,
            "used_at_ms": null, "revoked_at_ms": null,
        })
        .to_string();
        let index = [(sr::sinvite_by_email(&em), iid.clone()), (sr::sinvite_at(&l2, &iid), iid.clone())];
        t.put(sr::K_SINVITE, &iid, &rec, &index, &[])
            .map_err(|e| Error::RustError(format!("invite: {e}")))
    })
    .await?;
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
    let body: ChangeIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
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
        let now = ctx.data.now_ms;
        let (l3, u3) = (loc.clone(), uid.clone());
        ids::with_sessions(&ctx.env, move |t| {
            let mut n = 0usize;
            for (_, sid) in t.scan(&sr::ssessions_prefix(&l3, &u3)) {
                let Some(r) = ids::rec(t, sr::K_SSESSION, &sid) else { continue };
                if r.get("revoked_at_ms").is_some_and(|v| !v.is_null()) {
                    continue;
                }
                let index = [(sr::ssession_of(&l3, &u3, &sid), sid.clone())];
                t.put(sr::K_SSESSION, &sid, &sr::revoked(r, now).to_string(), &index, &[])
                    .map_err(|e| Error::RustError(format!("staff session: {e}")))?;
                n += 1;
            }
            Ok(n)
        })
        .await?
    } else {
        0
    };
    let _ = body.location_id;
    Response::from_json(&json!({ "ok": true, "role": m["role"], "active": m["status"] == "active", "sessionsEnded": ended }))
}
