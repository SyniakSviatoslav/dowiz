//! Login, refresh, logout — for owners and couriers.
//!
//! The refresh design is the old platform's, kept because it solves a problem
//! naive rotation gets wrong. Tokens rotate inside a FAMILY. Replaying a spent
//! one is theft and revokes the whole family. But two tabs refreshing at the
//! same instant is NOT theft, and the old code learned that the hard way: it
//! distinguishes the two by whether a sibling rotation happened seconds ago, and
//! answers 409 instead of logging every device out.

use serde::{Deserialize, Serialize};
use serde_json::json;
use worker::*;

use crate::auth::{
    self, hash_password, sha256_hex, verify_password_constant_work, Claims, COURIER_TTL_MS,
    OWNER_TTL_MS,
};

const REFRESH_TTL_MS: i64 = 7 * 24 * 60 * 60 * 1000;
const COURIER_REFRESH_TTL_MS: i64 = 30 * 24 * 60 * 60 * 1000;
/// Two tabs refreshing together is not an attack. The old service picked five
/// seconds after a bug where honest double-fires logged people out everywhere.
const CONCURRENT_REFRESH_GRACE_MS: i64 = 5_000;

#[derive(Deserialize)]
pub struct LoginIn {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct RefreshIn {
    pub refresh_token: String,
}

#[derive(Serialize)]
struct TokenPair {
    access_token: String,
    refresh_token: String,
}

fn now_ms() -> i64 {
    Date::now().as_millis() as i64
}

fn opaque_token() -> Option<String> {
    // Two UUIDs of platform CSPRNG. Fail closed if it is unreachable rather than
    // reach for a weaker source.
    Some(format!("{}{}", crate::edge_id()?, crate::edge_id()?).replace('-', ""))
}

async fn issue_owner_refresh(
    db: &D1Database,
    user_id: &str,
    family_id: &str,
) -> Result<Option<String>> {
    let Some(tok) = opaque_token() else { return Ok(None) };
    let Some(id) = crate::edge_id() else { return Ok(None) };
    let now = now_ms();
    db.prepare(
        "INSERT INTO auth_refresh_tokens (id,user_id,family_id,token_hash,used,expires_at_ms,created_at_ms) \
         VALUES (?1,?2,?3,?4,0,?5,?6)",
    )
    .bind(&[
        id.into(),
        user_id.into(),
        family_id.into(),
        sha256_hex(&tok).into(),
        wasm_bindgen::JsValue::from_f64((now + REFRESH_TTL_MS) as f64),
        wasm_bindgen::JsValue::from_f64(now as f64),
    ])?
    .run()
    .await?;
    Ok(Some(tok))
}

/// `POST /api/auth/login` — owner, email + password.
pub async fn owner_login(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: LoginIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let db = ctx.d1("DB")?;
    let email = body.email.trim().to_lowercase();

    #[derive(Deserialize)]
    struct U {
        id: String,
        password_hash: Option<String>,
        display_name: Option<String>,
    }
    let user: Option<U> = db
        .prepare("SELECT id, password_hash, display_name FROM users WHERE email = ?1")
        .bind(&[email.into()])?
        .first(None)
        .await?;

    // Same work on a miss as on a hit: "no such account" and "wrong password"
    // must not be distinguishable by how long the answer takes.
    let stored = user.as_ref().and_then(|u| u.password_hash.as_deref());
    if !verify_password_constant_work(&body.password, stored) {
        return Response::error("invalid credentials", 401);
    }
    let user = user.expect("verified above");

    // Authority comes from memberships, not from the request.
    #[derive(Deserialize)]
    struct M {
        location_id: String,
    }
    let m: Option<M> = db
        .prepare(
            "SELECT location_id FROM memberships \
             WHERE user_id = ?1 AND role = 'owner' AND status = 'active' \
             ORDER BY created_at_ms LIMIT 1",
        )
        .bind(&[user.id.clone().into()])?
        .first(None)
        .await?;
    let Some(m) = m else {
        return Response::error("no active owner membership", 403);
    };

    let now = now_ms();
    let claims = Claims::Owner {
        sub: user.id.clone(),
        user_id: user.id.clone(),
        active_location_id: Some(m.location_id.clone()),
        iat: now,
        exp: now + OWNER_TTL_MS,
    };
    let access = match auth::sign(&ctx.env, &claims) {
        Ok(t) => t,
        Err(e) => return e.into_response(),
    };
    let Some(family) = crate::edge_id() else {
        return Response::error("no platform CSPRNG", 500);
    };
    let refresh = issue_owner_refresh(&db, &user.id, &family).await?;

    Response::from_json(&json!({
        "access_token": access,
        "refresh_token": refresh,
        "user": { "id": user.id, "name": user.display_name, "locationId": m.location_id }
    }))
}

/// `POST /api/auth/refresh` — rotate within the family, detect reuse.
pub async fn owner_refresh(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: RefreshIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let db = ctx.d1("DB")?;
    let hash = sha256_hex(&body.refresh_token);
    let now = now_ms();

    #[derive(Deserialize)]
    struct R {
        id: String,
        user_id: String,
        family_id: String,
        used: i64,
        expires_at_ms: i64,
    }
    let row: Option<R> = db
        .prepare("SELECT id,user_id,family_id,used,expires_at_ms FROM auth_refresh_tokens WHERE token_hash = ?1")
        .bind(&[hash.into()])?
        .first(None)
        .await?;
    let Some(row) = row else {
        return Response::error("invalid refresh token", 401);
    };
    if row.expires_at_ms <= now {
        return Response::error("refresh token expired", 401);
    }

    if row.used == 1 {
        // Spent already. Either an honest race or a stolen token; the two are
        // told apart by whether the family rotated moments ago.
        #[derive(Deserialize)]
        struct C {
            n: i64,
        }
        let recent: Option<C> = db
            .prepare(
                "SELECT COUNT(*) AS n FROM auth_refresh_tokens \
                 WHERE family_id = ?1 AND created_at_ms > ?2",
            )
            .bind(&[
                row.family_id.clone().into(),
                wasm_bindgen::JsValue::from_f64((now - CONCURRENT_REFRESH_GRACE_MS) as f64),
            ])?
            .first(None)
            .await?;
        if recent.map(|c| c.n).unwrap_or(0) > 0 {
            return Response::error("concurrent refresh", 409);
        }
        db.prepare("DELETE FROM auth_refresh_tokens WHERE family_id = ?1")
            .bind(&[row.family_id.into()])?
            .run()
            .await?;
        return Response::error("token reuse detected; family revoked", 401);
    }

    // Claim it atomically: whoever flips `used` first owns the rotation.
    let claimed = db
        .prepare("UPDATE auth_refresh_tokens SET used = 1 WHERE id = ?1 AND used = 0")
        .bind(&[row.id.clone().into()])?
        .run()
        .await?;
    if claimed.meta()?.and_then(|m| m.changes).unwrap_or(0) == 0 {
        return Response::error("concurrent refresh", 409);
    }

    // Re-derive authority. A revoked owner does not roll forward on a refresh.
    #[derive(Deserialize)]
    struct M {
        location_id: String,
    }
    let m: Option<M> = db
        .prepare(
            "SELECT location_id FROM memberships \
             WHERE user_id = ?1 AND role = 'owner' AND status = 'active' \
             ORDER BY created_at_ms LIMIT 1",
        )
        .bind(&[row.user_id.clone().into()])?
        .first(None)
        .await?;
    let Some(m) = m else {
        return Response::error("owner access revoked", 401);
    };

    let claims = Claims::Owner {
        sub: row.user_id.clone(),
        user_id: row.user_id.clone(),
        active_location_id: Some(m.location_id),
        iat: now,
        exp: now + OWNER_TTL_MS,
    };
    let access = match auth::sign(&ctx.env, &claims) {
        Ok(t) => t,
        Err(e) => return e.into_response(),
    };
    let refresh = issue_owner_refresh(&db, &row.user_id, &row.family_id).await?;
    Response::from_json(&TokenPair {
        access_token: access,
        refresh_token: refresh.unwrap_or_default(),
    })
}

/// `POST /api/auth/logout` — every device, like the old service.
pub async fn owner_logout(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let p = match auth::authenticate(&req, &ctx.env, &db, now_ms()).await {
        Ok(p) => p,
        Err(e) => return e.into_response(),
    };
    let auth::Principal::Owner { user_id, .. } = p else {
        return Response::error("forbidden role", 403);
    };
    db.prepare("DELETE FROM auth_refresh_tokens WHERE user_id = ?1")
        .bind(&[user_id.into()])?
        .run()
        .await?;
    // The access token itself outlives this, up to its 24h exp. The old service
    // recorded that as an accepted risk rather than pretending otherwise.
    Response::from_json(&json!({ "ok": true }))
}

/// `POST /api/courier/auth/login`
pub async fn courier_login(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    #[derive(Deserialize)]
    struct In {
        #[serde(default)]
        email: Option<String>,
        #[serde(default)]
        phone: Option<String>,
        password: String,
        #[serde(default)]
        location_id: Option<String>,
    }
    let body: In = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let db = ctx.d1("DB")?;

    // Email and phone are looked up in SEPARATE columns. The old schema matched
    // both against one shared hash space, where a phone could in principle
    // resolve an email row.
    let (sql, key) = match (&body.email, &body.phone) {
        (Some(e), _) if !e.trim().is_empty() => (
            "SELECT id,password_hash,status FROM couriers WHERE email_hash = ?1",
            sha256_hex(&e.trim().to_lowercase()),
        ),
        (_, Some(p)) if !p.trim().is_empty() => (
            "SELECT id,password_hash,status FROM couriers WHERE phone_hash = ?1",
            sha256_hex(p.trim()),
        ),
        _ => return Response::error("email or phone required", 400),
    };

    #[derive(Deserialize)]
    struct C {
        id: String,
        password_hash: String,
        status: String,
    }
    let c: Option<C> = db.prepare(sql).bind(&[key.into()])?.first(None).await?;
    let stored = c.as_ref().map(|c| c.password_hash.as_str());
    if !verify_password_constant_work(&body.password, stored) {
        return Response::error("invalid credentials", 401);
    }
    let c = c.expect("verified above");
    if c.status != "active" {
        return Response::error("courier account is not active", 403);
    }

    #[derive(Deserialize)]
    struct L {
        location_id: String,
    }
    let loc: Option<L> = match &body.location_id {
        Some(want) => {
            db.prepare("SELECT location_id FROM courier_locations WHERE courier_id = ?1 AND location_id = ?2")
                .bind(&[c.id.clone().into(), want.clone().into()])?
                .first(None)
                .await?
        }
        None => {
            db.prepare("SELECT location_id FROM courier_locations WHERE courier_id = ?1 ORDER BY added_at_ms LIMIT 1")
                .bind(&[c.id.clone().into()])?
                .first(None)
                .await?
        }
    };
    let Some(loc) = loc else {
        return Response::error("not assigned to this location", 403);
    };

    let now = now_ms();
    let (Some(session_id), Some(family_id), Some(secret)) =
        (crate::edge_id(), crate::edge_id(), opaque_token())
    else {
        return Response::error("no platform CSPRNG", 500);
    };
    // The session secret is argon2-hashed, which is why the refresh token has to
    // carry the row id as a prefix: a hash lookup is impossible by design.
    let token_hash = match hash_password(&secret) {
        Ok(h) => h,
        Err(e) => return e.into_response(),
    };
    db.prepare(
        "INSERT INTO courier_sessions (id,courier_id,family_id,token_hash,active_location_id,\
         issued_at_ms,expires_at_ms) VALUES (?1,?2,?3,?4,?5,?6,?7)",
    )
    .bind(&[
        session_id.clone().into(),
        c.id.clone().into(),
        family_id.into(),
        token_hash.into(),
        loc.location_id.clone().into(),
        wasm_bindgen::JsValue::from_f64(now as f64),
        wasm_bindgen::JsValue::from_f64((now + COURIER_REFRESH_TTL_MS) as f64),
    ])?
    .run()
    .await?;

    let _ = db
        .prepare("UPDATE couriers SET last_login_at_ms = ?2 WHERE id = ?1")
        .bind(&[c.id.clone().into(), wasm_bindgen::JsValue::from_f64(now as f64)])?
        .run()
        .await;
    let _ = db
        .prepare(
            "INSERT INTO courier_audit_log (courier_id,location_id,action,actor_kind,actor_id,created_at_ms) \
             VALUES (?1,?2,'login.success','courier',?1,?3)",
        )
        .bind(&[
            c.id.clone().into(),
            loc.location_id.clone().into(),
            wasm_bindgen::JsValue::from_f64(now as f64),
        ])?
        .run()
        .await;

    let claims = Claims::Courier {
        sub: c.id.clone(),
        active_location_id: loc.location_id.clone(),
        jti: session_id.clone(),
        iat: now,
        exp: now + COURIER_TTL_MS,
    };
    let jwt = match auth::sign(&ctx.env, &claims) {
        Ok(t) => t,
        Err(e) => return e.into_response(),
    };
    Response::from_json(&json!({
        "jwt": jwt,
        "refreshToken": format!("{session_id}.{secret}"),
        "courier": { "id": c.id, "locationId": loc.location_id }
    }))
}

use worker::wasm_bindgen;
