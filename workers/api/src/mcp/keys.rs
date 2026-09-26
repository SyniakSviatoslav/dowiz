//! The I/O around `rolekey`: a member of staff or a courier mints, lists and
//! revokes THEIR OWN agent keys from their own app; the owner lists and
//! revokes any of the venue's. And the exchange at the MCP door.
//!
//! WHY SELF-MINTED, not minted by the owner for a person. A key minted by the
//! person is bound to the principal that minted it, so "a key never exceeds
//! its person's rights" holds by construction: there is no person selector to
//! get wrong, and no owner route that could name somebody at another venue.
//! The owner keeps the other half — seeing every key and ending any of them —
//! and suspending the person ends all of theirs with their sessions.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use super::rolekey::{self as rk, Holder};
use crate::auth::{self, AuthError, Principal};
use crate::identity_store as ids;
use crate::services::identity::staff_rules as sr;

/// `dowizs_…` / `dowizc_…` -> the principal it stands for, and the
/// five-minute token the tools will carry. The token is authenticated like
/// any other, so the live roster has the last word.
pub async fn exchange(env: &Env, h: Holder, id: &str, secret: &str, now: i64) -> std::result::Result<(Principal, String), AuthError> {
    let sess = ids::sessions(env).await.map_err(|e| AuthError::Db(e.to_string()))?;
    let claims = rk::verdict(h, id, ids::rec(&sess, h.kind(), id).as_ref(), secret, now).map_err(AuthError::Revoked)?;
    let token = auth::sign(env, &claims)?;
    let p = auth::authenticate_token(&token, env, now).await?;
    Ok((p, token))
}

/// Who is asking, for the self-service routes: the holder, the person, the
/// venue. An owner is sent to the console's own keys.
async fn me(req: &Request, ctx: &RouteContext<crate::Req>, want: Holder) -> std::result::Result<(String, String), Response> {
    let p = auth::authenticate(req, &ctx.env, ctx.data.now_ms).await.map_err(|e| e.into_response().unwrap())?;
    match (want, p) {
        (Holder::Staff, Principal::Staff { person_id, active_location_id, .. }) => Ok((person_id, active_location_id)),
        (Holder::Courier, Principal::Courier { courier_id, active_location_id, .. }) => Ok((courier_id, active_location_id)),
        _ => Err(Response::error("forbidden role", 403).unwrap()),
    }
}

/// The index a staff key row is written with, so suspending the person
/// (`staff_admin`) finds it by the same prefix as their sign-ins.
fn index_of(h: Holder, row: &Value, id: &str) -> Vec<(String, String)> {
    match h {
        Holder::Staff => {
            let (person, venue) = rk::owner_of(h, row);
            vec![(sr::ssession_of(&venue, &person, id), id.to_string())]
        }
        Holder::Courier => Vec::new(),
    }
}

/// The venue's live keys of one holder, optionally one person's.
fn keys_in(sess: &dowiz_hub::table::Table, h: Holder, venue: &str, person: Option<&str>, now: i64) -> Vec<Value> {
    let rows: Vec<(String, Value)> = match h {
        Holder::Staff => {
            let prefix = match person {
                Some(p) => sr::ssessions_prefix(venue, p),
                None => format!("ssession.person/{venue}/"),
            };
            sess.scan(&prefix).into_iter().filter_map(|(_, id)| ids::rec(sess, h.kind(), &id).map(|r| (id, r))).collect()
        }
        Holder::Courier => sess.all(h.kind()).into_iter().filter_map(|(id, j)| serde_json::from_str(&j).ok().map(|v| (id, v))).collect(),
    };
    rows.iter()
        .filter(|(_, r)| {
            let (who, at) = rk::owner_of(h, r);
            at == venue && person.is_none_or(|p| p == who)
        })
        .filter_map(|(id, r)| rk::shown(h, id, r, now))
        .collect()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MintIn {
    label: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RevokeIn {
    id: String,
    #[serde(default)]
    holder: Option<String>,
}

async fn mint(mut req: Request, ctx: RouteContext<crate::Req>, h: Holder) -> Result<Response> {
    let body: MintIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let (person, venue) = match me(&req, &ctx, h).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let label = match rk::label_of(&body.label) {
        Ok(l) => l,
        Err(why) => return Response::error(why, 400),
    };
    // THE ROLE WORD FROM THE ROSTER, read now, never from the request.
    let role = match h {
        Holder::Staff => {
            let ident = ids::identity(&ctx.env).await?;
            ids::membership(&ident, &venue, &person).map(|m| ids::s_of(&m, "role")).unwrap_or_default()
        }
        Holder::Courier => String::new(),
    };
    let (Some(id), Some(secret)) = (crate::edge_id(), crate::edge_id()) else {
        return Response::error("no platform CSPRNG", 500);
    };
    let secret = secret.replace('-', "");
    let hash = auth::hash_opaque(&secret);
    let now = ctx.data.now_ms;
    let row = match h {
        Holder::Staff => rk::staff_row(&person, &venue, &role, &label, &hash, now),
        Holder::Courier => rk::courier_row(&person, &venue, &id, &label, &hash, now),
    };
    let (rid, index) = (id.clone(), index_of(h, &row, &id));
    let text = row.to_string();
    ids::with_sessions(&ctx.env, move |t| {
        t.put(h.kind(), &rid, &text, &index, &[]).map_err(|e| Error::RustError(format!("agent key: {e}")))
    })
    .await?;
    // Shown ONCE: the row holds a hash and cannot show it again.
    Response::from_json(&json!({ "key": rk::spell(h, &id, &secret), "id": id, "label": label, "expiresMs": now + rk::KEY_TTL_MS }))
}

async fn list(req: Request, ctx: RouteContext<crate::Req>, h: Holder) -> Result<Response> {
    let (person, venue) = match me(&req, &ctx, h).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let sess = ids::sessions(&ctx.env).await?;
    Response::from_json(&json!({ "keys": keys_in(&sess, h, &venue, Some(&person), ctx.data.now_ms) }))
}

/// Revoke one row, if `allowed` says so; the same 404 for "not yours" as for
/// "not there", as the owner's keys answer.
async fn end(env: &Env, h: Holder, id: String, now: i64, allowed: impl Fn(Option<&Value>) -> bool + 'static) -> Result<Response> {
    let done = ids::with_sessions(env, move |t| {
        let row = ids::rec(t, h.kind(), &id);
        if !allowed(row.as_ref()) {
            return Ok(false);
        }
        let Some(r) = row else { return Ok(false) };
        let index = index_of(h, &r, &id);
        t.put(h.kind(), &id, &sr::revoked(r, now).to_string(), &index, &[])
            .map_err(|e| Error::RustError(format!("agent key: {e}")))?;
        Ok(true)
    })
    .await?;
    if !done {
        return Response::error("not found", 404);
    }
    Response::from_json(&json!({ "ok": true }))
}

async fn revoke(mut req: Request, ctx: RouteContext<crate::Req>, h: Holder) -> Result<Response> {
    let body: RevokeIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let (person, venue) = match me(&req, &ctx, h).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    end(&ctx.env, h, body.id, ctx.data.now_ms, move |r| rk::may_revoke_own(h, r, &person, &venue)).await
}

/// `POST /api/staff/mcp/keys` · `GET` lists · `POST …/revoke {id}`.
pub async fn staff_mint(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> { mint(req, ctx, Holder::Staff).await }
pub async fn staff_list(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> { list(req, ctx, Holder::Staff).await }
pub async fn staff_revoke(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> { revoke(req, ctx, Holder::Staff).await }
/// `POST /api/courier/mcp/keys` · `GET` lists · `POST …/revoke {id}`.
pub async fn courier_mint(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> { mint(req, ctx, Holder::Courier).await }
pub async fn courier_list(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> { list(req, ctx, Holder::Courier).await }
pub async fn courier_revoke(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> { revoke(req, ctx, Holder::Courier).await }

/// `GET /api/owner/mcp/keys` — every person's agent key at this venue.
pub async fn owner_list(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let venue = match crate::owner::owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let now = ctx.data.now_ms;
    let sess = ids::sessions(&ctx.env).await?;
    let mut keys = keys_in(&sess, Holder::Staff, &venue, None, now);
    keys.extend(keys_in(&sess, Holder::Courier, &venue, None, now));
    Response::from_json(&json!({ "keys": keys }))
}

/// `POST /api/owner/mcp/keys/revoke {id, holder}` — end any key of this venue.
pub async fn owner_revoke(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: RevokeIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let venue = match crate::owner::owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let Some(h) = body.holder.as_deref().and_then(Holder::from_str) else {
        return Response::error("holder is staff or courier", 400);
    };
    end(&ctx.env, h, body.id, ctx.data.now_ms, move |r| rk::may_revoke_at(h, r, &venue)).await
}

#[cfg(test)]
mod tests;
