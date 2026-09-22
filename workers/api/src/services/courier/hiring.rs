//! Hiring: minting an invite code, withdrawing one, and turning a courier off.
//!
//! A CODE IS STORED HASHED, exactly like a password, because that is what it
//! is: until it is claimed, whoever holds it can become this courier. The
//! rules that decide what a code may be made from are `roster`, beside this,
//! where they are tested.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use crate::owner::{now_ms, owner_and_venue};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InviteIn {
    phone: String,
    name: String,
}

/// `POST /api/owner/couriers/invite` — mint a code, shown ONCE.
///
/// The code is stored HASHED, exactly like a password, because that is what it
/// is: until it is claimed, whoever holds it can become this courier. A copied
/// database would otherwise hand over every pending account.
pub async fn invite_courier(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    use crate::services::courier::roster;

    /// A week. Long enough for a courier who starts next Monday, short enough
    /// that a code found in an old message no longer opens anything.
    const TTL_MS: i64 = 7 * 24 * 60 * 60 * 1000;

    let body: InviteIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let db = ctx.d1("DB")?;
    let (owner, loc) = match owner_and_venue(&req, &ctx, &db).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    // The owner's words back to the owner: `invite_fields` carries the refusal
    // text, so the rule and the sentence it produces are tested together.
    let (phone, name) = match roster::invite_fields(&body.phone, &body.name) {
        Ok(v) => v,
        Err(why) => return Response::error(why, 400),
    };
    let phone_hash = crate::auth::sha256_hex(&phone);

    // THE BYTES ARE THE PLATFORM'S, the alphabet is the hub's. Two UUIDs from
    // the platform CSPRNG are 32 hex-encoded bytes; `code_from_entropy` renders
    // sixteen of them and hands back the digest that is all we keep.
    let Some(minted) = crate::edge_id()
        .zip(crate::edge_id())
        .map(|(a, b)| format!("{a}{b}").replace('-', ""))
        .and_then(|hex| roster::code_from_entropy(&hex, crate::auth::sha256_hex))
    else {
        return Response::error("no platform CSPRNG", 500);
    };
    let code = minted.code;
    let Some(id) = crate::edge_id() else {
        return Response::error("no platform CSPRNG", 500);
    };
    let now = now_ms();
    // ── THE CHECK, THE REVOKE AND THE MINT, IN ONE TURN ──
    //
    // Three statements before: does this phone already have an account, revoke
    // any pending invite for it, write the new one. Inviting the same phone
    // twice REPLACES the pending invite rather than leaving two codes alive for
    // one person -- the first would keep working after the owner believed they
    // had replaced it -- and that only holds if nothing runs in between.
    let (ph, l2, own, nm, ch) = (
        phone_hash.clone(),
        loc.clone(),
        owner.clone(),
        name.clone(),
        // Hashed with the same one-way function the phone uses. A 16-character
        // code from a 32-symbol alphabet is 80 bits, so a plain digest is not
        // brute-forceable the way a human password would be.
        minted.hash,
    );
    let taken = crate::identity_store::with_couriers(&ctx.env, move |t| {
        if crate::identity_store::courier_id_for_phone(t, &ph).is_some() {
            return Ok(true);
        }
        if let Some(pending) = t.lookup(&crate::identity_store::invite_by_phone(&ph)) {
            if let Some(mut i) =
                crate::identity_store::rec(t, crate::identity_store::K_INVITE, &pending)
            {
                i["revoked_at_ms"] = serde_json::json!(now);
                let loc = crate::identity_store::s_of(&i, "location_id");
                // The phone index goes with the revocation: a revoked invite
                // must stop being findable by the number it was sent to.
                let index =
                    vec![(crate::identity_store::invite_at(&loc, &pending), pending.clone())];
                t.put(
                    crate::identity_store::K_INVITE,
                    &pending,
                    &i.to_string(),
                    &index,
                    &[],
                )
                .map_err(|e| Error::RustError(format!("invite: {e}")))?;
            }
        }
        let rec = serde_json::json!({
            "id": id, "location_id": l2, "created_by_owner_id": own, "role": "courier",
            "invited_email_hash": ph, "invited_phone_hash": ph, "invited_name": nm,
            "code_hash": ch, "expires_at_ms": now + TTL_MS, "created_at_ms": now,
            "used_at_ms": serde_json::Value::Null,
            "revoked_at_ms": serde_json::Value::Null,
        })
        .to_string();
        t.put(
            crate::identity_store::K_INVITE,
            &id,
            &rec,
            &[
                (crate::identity_store::invite_by_phone(&ph), id.clone()),
                (crate::identity_store::invite_at(&l2, &id), id.clone()),
            ],
            &[],
        )
        .map_err(|e| Error::RustError(format!("invite: {e}")))?;
        Ok(false)
    })
    .await?;
    if taken {
        return Response::error("that phone already has an account", 409);
    }

    Response::from_json(&json!({ "code": code, "expiresMs": now + TTL_MS }))
}

/// `POST /api/owner/couriers/:id/uninvite` — withdraw a pending code.
pub async fn uninvite_courier(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing invite", 400);
    };
    let now = now_ms();
    let (iid, l2) = (id.clone(), loc.clone());
    let revoked = crate::identity_store::with_couriers(&ctx.env, move |t| {
        // THE VENUE IS IN THE CHECK, not only in the WHERE clause: an invite of
        // another restaurant is refused because its record does not say this
        // venue, and the answer is the same 404 either way -- a 403 would tell
        // the caller the invite exists.
        let Some(mut i) = crate::identity_store::rec(t, crate::identity_store::K_INVITE, &iid)
        else {
            return Ok(false);
        };
        if crate::identity_store::s_of(&i, "location_id") != l2
            || i.get("used_at_ms").map_or(false, |v| !v.is_null())
            || i.get("revoked_at_ms").map_or(false, |v| !v.is_null())
        {
            return Ok(false);
        }
        i["revoked_at_ms"] = serde_json::json!(now);
        let index = vec![(crate::identity_store::invite_at(&l2, &iid), iid.clone())];
        t.put(crate::identity_store::K_INVITE, &iid, &i.to_string(), &index, &[])
            .map_err(|e| Error::RustError(format!("invite: {e}")))?;
        Ok(true)
    })
    .await?;
    if !revoked {
        return Response::error("not found", 404);
    }
    Response::from_json(&json!({ "ok": true }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ActiveIn {
    active: bool,
}

/// `POST /api/owner/couriers/:id/active` — a courier who has left.
///
/// Their record STAYS -- an order they delivered still names them -- and every
/// session they hold dies, because somebody who has left must not keep a working
/// app in their pocket.
pub async fn set_courier_active(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: ActiveIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let db = ctx.d1("DB")?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let Some(ident) = ctx.param("id").cloned() else {
        return Response::error("missing courier", 400);
    };
    // The console addresses a courier by the phone it displays, which is the
    // only handle it has; the id is internal.
    let hash = crate::auth::sha256_hex(&ident);
    struct C {
        id: String,
    }
    let crew = crate::identity_store::couriers(&ctx.env).await?;
    let row: Option<C> = if crew
        .get(crate::identity_store::K_ROSTER, &crate::identity_store::roster_id(&loc, &ident))
        .is_some()
    {
        Some(C { id: ident.clone() })
    } else {
        crate::identity_store::courier_id_for_phone(&crew, &hash)
            .filter(|cid| {
                crew.get(
                    crate::identity_store::K_ROSTER,
                    &crate::identity_store::roster_id(&loc, cid),
                )
                .is_some()
            })
            .map(|id| C { id })
    };
    let Some(row) = row else {
        return Response::error("not found", 404);
    };
    let status = if body.active { "active" } else { "deactivated" };
    let cid = row.id.clone();
    let st = status.to_string();
    crate::identity_store::with_couriers(&ctx.env, move |t| {
        if let Some(mut r) = crate::identity_store::rec(t, crate::identity_store::K_COURIER, &cid) {
            r["status"] = serde_json::json!(st);
            let index = crate::identity_store::courier_index(&cid, &r);
            t.put(crate::identity_store::K_COURIER, &cid, &r.to_string(), &index, &[])
                .map_err(|e| Error::RustError(format!("courier: {e}")))?;
        }
        Ok(())
    })
    .await?;
    let mut revoked = 0usize;
    if !body.active {
        // EVERY SESSION THEY HOLD DIES. Somebody who has left must not keep a
        // working app in their pocket -- and the sessions are records, so this
        // is a walk rather than an UPDATE whose `changes` count was the answer.
        let cid = row.id.clone();
        let now = now_ms();
        revoked = crate::identity_store::with_sessions(&ctx.env, move |t| {
            let mut n = 0usize;
            let live: Vec<(String, Value)> = t
                .all(crate::identity_store::K_CSESSION)
                .into_iter()
                .filter_map(|(id, j)| serde_json::from_str::<Value>(&j).ok().map(|v| (id, v)))
                .filter(|(_, v)| crate::identity_store::s_of(v, "courier_id") == cid)
                .filter(|(_, v)| v.get("revoked_at_ms").map_or(true, |x| x.is_null()))
                .collect();
            for (id, mut v) in live {
                v["revoked_at_ms"] = serde_json::json!(now);
                t.put(crate::identity_store::K_CSESSION, &id, &v.to_string(), &[], &[])
                    .map_err(|e| Error::RustError(format!("session: {e}")))?;
                n += 1;
            }
            Ok(n)
        })
        .await?;
    }
    Response::from_json(&json!({ "ok": true, "active": body.active, "sessionsRevoked": revoked }))
}
