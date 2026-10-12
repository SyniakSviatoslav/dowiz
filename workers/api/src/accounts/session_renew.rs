//! Renewal for the two roles that have no refresh family (W-SESSION, TZ 7).
//!
//! The owner rotates a refresh token (`accounts::owner_refresh`). A courier's
//! session is a ROW with a hashed secret the login handed back and the app
//! never used -- `COURIER_TTL_MS` is a day and every courier signed in again
//! every morning. A member of staff has only the token. Both are renewed here,
//! each by the proof it already holds:
//!
//!   * `POST /api/courier/auth/refresh` -- the `<session_id>.<secret>` the
//!     login returned; the row is slid by `auth::slide` and a new day-long
//!     jwt is minted on the SAME `jti`, so the owner's revocation of that row
//!     still ends it.
//!   * `POST /api/staff/refresh` -- the still-valid bearer. The guard proves
//!     it the way every staff call is proved (membership and session row read
//!     now), then the row slides by the shift window up to a week.
//!
//! NOTHING HERE WEAKENS REVOCATION. The guard on every request still reads
//! the session row and the roster; renewal only moves `expires_at_ms`.

use serde::Deserialize;
use serde_json::json;
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

use crate::auth::{self, Claims, Principal, COURIER_TTL_MS};
use crate::identity_store as ids;
use crate::services::identity::staff_rules as sr;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CourierRefreshIn {
    /// `<session_id>.<secret>`, as `courier_login` returned it (camelCase, the
    /// courier API's spelling).
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
}

/// `POST /api/courier/auth/refresh`
pub async fn courier_refresh(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: CourierRefreshIn = match crate::body::parse(&mut req).await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let now = ctx.data.now_ms;
    // EVERY refusal below is the same 401: a missing row, a wrong secret and a
    // revoked session must not be told apart by a caller guessing session ids.
    let Some((sid, secret)) = body.refresh_token.split_once('.') else {
        return Response::error("invalid refresh token", 401);
    };
    let (sess, crew) = futures_util::future::join(ids::sessions(&ctx.env), ids::couriers(&ctx.env)).await;
    let (sess, crew) = (sess?, crew?);
    let Some(row) = ids::rec(&sess, ids::K_CSESSION, sid) else {
        return Response::error("invalid refresh token", 401);
    };
    if !auth::verify_opaque(secret, &ids::s_of(&row, "token_hash")) {
        return Response::error("invalid refresh token", 401);
    }
    if row.get("revoked_at_ms").is_some_and(|v| !v.is_null()) {
        return Response::error("courier session revoked", 401);
    }
    let courier_id = ids::s_of(&row, "courier_id");
    let venue = ids::s_of(&row, "active_location_id");
    // The roster is read NOW, as the guard reads it: a courier the owner removed
    // this morning gets no new token this afternoon.
    if crew.get(ids::K_ROSTER, &ids::roster_id(&venue, &courier_id)).is_none() {
        return Response::error("courier no longer at this location", 401);
    }
    let until = match auth::slide(
        ids::i_of(&row, "issued_at_ms"),
        ids::i_of(&row, "expires_at_ms"),
        auth::SESSION_IDLE_MS,
        auth::SESSION_ABSOLUTE_MS,
        now,
    ) {
        Ok(u) => u,
        Err(why) => return Response::error(format!("courier {why}"), 401),
    };
    let (sid_w, mut row_w) = (sid.to_string(), row);
    ids::with_sessions(&ctx.env, move |t| {
        row_w["expires_at_ms"] = json!(until);
        t.put(ids::K_CSESSION, &sid_w, &row_w.to_string(), &[], &[])
            .map_err(|e| Error::RustError(format!("courier session: {e}")))
    })
    .await?;
    let claims = Claims::Courier {
        sub: courier_id.clone(),
        active_location_id: venue.clone(),
        jti: sid.to_string(),
        iat: now,
        exp: now + COURIER_TTL_MS,
    };
    let jwt = match auth::sign(&ctx.env, &claims) {
        Ok(t) => t,
        Err(e) => return e.into_response(),
    };
    Response::from_json(&json!({
        "jwt": jwt,
        "courier": { "id": courier_id, "locationId": venue },
        "sessionUntilMs": until,
    }))
}

/// `POST /api/staff/refresh` -- the bearer is the proof; the row slides.
pub async fn staff_refresh(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let now = ctx.data.now_ms;
    let p = match auth::authenticate(&req, &ctx.env, now).await {
        Ok(p) => p,
        Err(e) => return e.into_response(),
    };
    let Principal::Staff { person_id, active_location_id, session_id, caps } = p else {
        return Response::error("not a staff session", 401);
    };
    let (sid, uid, loc) = (session_id.clone(), person_id.clone(), active_location_id.clone());
    let slid: std::result::Result<i64, &'static str> = ids::with_sessions(&ctx.env, move |t| {
        let Some(mut r) = ids::rec(t, sr::K_SSESSION, &sid) else {
            return Ok(Err("no such staff session"));
        };
        let until = match auth::slide(
            ids::i_of(&r, "issued_at_ms"),
            ids::i_of(&r, "expires_at_ms"),
            sr::STAFF_TTL_MS,
            auth::STAFF_ABSOLUTE_MS,
            now,
        ) {
            Ok(u) => u,
            Err(why) => return Ok(Err(why)),
        };
        r["expires_at_ms"] = json!(until);
        let index = vec![(sr::ssession_of(&loc, &uid, &sid), sid.clone())];
        t.put(sr::K_SSESSION, &sid, &r.to_string(), &index, &[])
            .map_err(|e| Error::RustError(format!("staff session: {e}")))?;
        Ok(Ok(until))
    })
    .await?;
    let until = match slid {
        Ok(u) => u,
        Err(why) => return Response::error(format!("staff {why}"), 401),
    };
    // The capabilities are the ones that SURVIVED the roster, not the minted
    // ones: a token can only lose rights on renewal, never regain them.
    let claims = Claims::Staff {
        sub: person_id.clone(),
        active_location_id: active_location_id.clone(),
        jti: session_id,
        caps: caps.to_string(),
        iat: now,
        exp: now + sr::STAFF_TTL_MS,
    };
    let jwt = match auth::sign(&ctx.env, &claims) {
        Ok(t) => t,
        Err(e) => return e.into_response(),
    };
    Response::from_json(&json!({
        "jwt": jwt,
        "staff": { "id": person_id, "locationId": active_location_id, "caps": caps.to_string(),
                   "expiresMs": (now + sr::STAFF_TTL_MS).min(until) },
        "sessionUntilMs": until,
    }))
}

#[cfg(test)]
mod tests;
