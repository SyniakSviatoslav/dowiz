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
    /// WHICH VENUE, when the caller owns more than one.
    ///
    /// The token carries `active_location_id`, and `Place::of_any` trusts that
    /// claim BEFORE the host: an owner of two venues signing in without saying
    /// which one got a token for the OLDEST membership, and every write they
    /// then made -- a photograph, a logo, an address -- landed on that venue
    /// however the request was addressed. Two hundred writes went to the wrong
    /// restaurant that way, each answering 200.
    ///
    /// It is checked against this caller's OWN memberships, so naming a venue
    /// grants nothing: an id they do not own finds no row and the login is
    /// refused exactly as if they had no membership at all.
    #[serde(default)]
    pub location_id: Option<String>,
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

/// Rewrite a password hash that was stored at a cost this Worker can no longer
/// afford — see `auth::needs_rehash`.
///
/// ON LOGIN, because that is the only moment the plaintext exists. A migration
/// cannot do this: the whole point of the stored value is that the password is
/// not recoverable from it, so the rehash has to ride along with someone
/// actually signing in.
///
/// IT NEVER FAILS THE LOGIN. The caller has already been authenticated; if the
/// write does not land they simply pay the old cost again on their next visit
/// and we try again then. Turning a successful authentication into a 500
/// because of an optimisation would be the worse trade by a wide margin.
///
/// The table name is a `&'static str` chosen at the two call sites rather than
/// anything derived from a request, because it is interpolated into SQL.
async fn upgrade_hash(
    db: &D1Database,
    table: &'static str,
    id: &str,
    password: &str,
    stored: Option<&str>,
) {
    let Some(stored) = stored else { return };
    if !auth::needs_rehash(stored) {
        return;
    }
    let Ok(fresh) = hash_password(password) else { return };
    let sql = format!("UPDATE {table} SET password_hash = ?2 WHERE id = ?1");
    if let Ok(stmt) = db.prepare(&sql).bind(&[id.into(), fresh.into()]) {
        let _ = stmt.run().await;
    }
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
    upgrade_hash(&db, "users", &user.id, &body.password, user.password_hash.as_deref()).await;

    // Authority comes from memberships, not from the request.
    #[derive(Deserialize)]
    struct M {
        location_id: String,
    }
    let wanted = body.location_id.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let m: Option<M> = match wanted {
        Some(l) => {
            db.prepare(
                "SELECT location_id FROM memberships \
                 WHERE user_id = ?1 AND location_id = ?2 AND role = 'owner' \
                 AND status = 'active' LIMIT 1",
            )
            .bind(&[user.id.clone().into(), l.into()])?
            .first(None)
            .await?
        }
        // Unchanged for everyone who owns one venue: the oldest membership.
        None => {
            db.prepare(
                "SELECT location_id FROM memberships \
                 WHERE user_id = ?1 AND role = 'owner' AND status = 'active' \
                 ORDER BY created_at_ms LIMIT 1",
            )
            .bind(&[user.id.clone().into()])?
            .first(None)
            .await?
        }
    };
    // A PLATFORM ADMINISTRATOR OWNS NO RESTAURANT, and that is the point of
    // them. Authority still comes from a table and not from the request -- it
    // just comes from a different one. Without this, the only way to sign in to
    // the main hub was to first make its administrator the owner of somebody's
    // venue, which would put a platform account inside a tenant's data.
    //
    // THE TOKEN NAMES NO VENUE. `active_location_id` stays `None`, so
    // `claimed_venue` finds nothing and this token cannot be used to read a
    // hub: every venue route resolves its venue from the claim first. A
    // platform admin can create hubs and cannot read one.
    let admin: Option<M> = if m.is_none() {
        #[derive(Deserialize)]
        struct P {
            user_id: String,
        }
        let p: Option<P> = db
            .prepare("SELECT user_id FROM platform_admins WHERE user_id = ?1 LIMIT 1")
            .bind(&[user.id.clone().into()])?
            .first(None)
            .await?;
        p.map(|_| M { location_id: String::new() })
    } else {
        None
    };
    if m.is_none() && admin.is_none() {
        return Response::error("no active owner membership", 403);
    }
    let venue = m.as_ref().map(|m| m.location_id.clone());

    let now = now_ms();
    let claims = Claims::Owner {
        sub: user.id.clone(),
        user_id: user.id.clone(),
        active_location_id: venue.clone(),
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
        "user": { "id": user.id, "name": user.display_name, "locationId": venue,
                  "platformAdmin": m.is_none() }
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
/// The body named a venue and the host named a different one.
pub struct VenueContradiction;

/// WHICH VENUE A COURIER'S SESSION IS FOR, as a rule that can be read.
///
/// `None` means "the caller has named no venue and the host names none
/// either" -- the single-membership fallback is then honest. Everything else
/// is a name the membership lookup must confirm.
pub fn venue_for_courier_login(
    host_venue: Option<&str>,
    body_location: Option<&str>,
) -> std::result::Result<Option<String>, VenueContradiction> {
    match (host_venue, body_location) {
        (Some(h), Some(b)) if h != b => Err(VenueContradiction),
        (Some(h), _) => Ok(Some(h.to_string())),
        (None, Some(b)) => Ok(Some(b.to_string())),
        (None, None) => Ok(None),
    }
}

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
    upgrade_hash(&db, "couriers", &c.id, &body.password, Some(c.password_hash.as_str())).await;
    if c.status != "active" {
        return Response::error("courier account is not active", 403);
    }

    #[derive(Deserialize)]
    struct L {
        location_id: String,
    }
    // ── WHICH VENUE, AND WHY THE HOST DECIDES IT ──
    //
    // This was the courier half of the bug that `owner_and_venue` had: with no
    // `location_id` in the body it fell to `ORDER BY added_at_ms LIMIT 1`, the
    // courier's FIRST venue, whatever host they were standing on. The courier
    // app never sends the field, so that branch is the one every real login
    // takes -- and a courier of `dubin-durres` signing in at
    // `sushi-durres.dowiz.org` was handed a dubin session on the sushi domain.
    // It looked like it worked: the login succeeded, the shift opened, and the
    // app then showed the OTHER venue's pool for ever while `/api/live`, the
    // one place that does compare the principal to the host, answered 404 to
    // every handshake. A courier waiting for orders that are being placed two
    // streets away has no way to tell that from a quiet evening.
    //
    // So the host is asked FIRST, exactly as it is for an anonymous read in
    // `Place::of_any`: a venue's own subdomain names its venue, and a session
    // minted there is for that venue or it is refused. A body `location_id`
    // that disagrees with the host is a contradiction, not a preference, and
    // is refused rather than silently resolved one way -- fail closed, the
    // same stance the socket takes. The old guess survives only where it was
    // ever true: a host that names no venue (the apex, `*.workers.dev`), where
    // the caller genuinely has not said, and a single membership is an answer
    // rather than a coin toss.
    let host_venue: Option<String> = match crate::hubstore::Place::slug_of_host(&req, &ctx) {
        Some(slug) => {
            let venue = crate::hubstore::Place::of_slug(&ctx, &slug).await?.venue;
            (venue != crate::hubstore::UNNAMED_VENUE).then_some(venue)
        }
        None => None,
    };
    let want = match venue_for_courier_login(host_venue.as_deref(), body.location_id.as_deref()) {
        Ok(v) => v,
        Err(VenueContradiction) => {
            return Response::error("that location is not this venue", 403)
        }
    };
    let loc: Option<L> = match &want {
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
    let token_hash = auth::hash_opaque(&secret);
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

#[derive(Deserialize)]
pub struct ClaimIn {
    pub phone: String,
    pub code: String,
    pub password: String,
}

/// `POST /api/courier/auth/claim` — turn an invite into an account.
///
/// PUBLIC BY NECESSITY: the courier has no credentials yet, which is the whole
/// point. What stands in for authentication is the code, and the code is stored
/// hashed and compared in constant time, so this route cannot be used to
/// discover which phone numbers a venue has invited.
///
/// "No invite for this phone" and "wrong code" are ONE answer, reached after the
/// same lookup. Expiry is told apart in the answer, because a courier whose code
/// ran out needs a new one rather than another attempt.
///
/// The courier chooses their own password. An owner who set it for them would
/// know it.
pub async fn courier_claim(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: ClaimIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    // Eight characters. Short enough to type at a door, long enough not to be
    // the four-digit PIN everyone would otherwise pick. The cost of a weak one
    // here is somebody else's shift.
    if body.password.chars().count() < 8 {
        return Response::error("choose a password of at least 8 characters", 400);
    }
    let db = ctx.d1("DB")?;
    let phone = body.phone.trim().to_string();
    let phone_hash = auth::sha256_hex(&phone);
    let code_hash = auth::sha256_hex(body.code.trim());
    let now = now_ms();

    #[derive(Deserialize)]
    struct Inv {
        id: String,
        location_id: String,
        invited_name: Option<String>,
        expires_at_ms: i64,
    }
    let inv: Option<Inv> = db
        .prepare(
            "SELECT id, location_id, invited_name, expires_at_ms FROM courier_invites \
             WHERE invited_phone_hash = ?1 AND code_hash = ?2 \
             AND used_at_ms IS NULL AND revoked_at_ms IS NULL",
        )
        .bind(&[phone_hash.clone().into(), code_hash.into()])?
        .first(None)
        .await?;
    let Some(inv) = inv else {
        return Response::error("that code does not match", 400);
    };
    if now >= inv.expires_at_ms {
        return Response::error("that code has expired -- ask for a new one", 400);
    }

    #[derive(Deserialize)]
    struct Row {
        id: String,
    }
    let taken: Option<Row> = db
        .prepare("SELECT id FROM couriers WHERE phone_hash = ?1")
        .bind(&[phone_hash.clone().into()])?
        .first(None)
        .await?;
    if taken.is_some() {
        return Response::error("this person already has an account", 409);
    }

    let (Some(cid), Some(session_id), Some(family_id), Some(secret)) =
        (crate::edge_id(), crate::edge_id(), crate::edge_id(), opaque_token())
    else {
        return Response::error("no platform CSPRNG", 500);
    };
    let pw_hash = match hash_password(&body.password) {
        Ok(h) => h,
        Err(e) => return e.into_response(),
    };
    let name = inv.invited_name.clone().unwrap_or_default();
    // The email column is NOT NULL and unique, and a courier who signs in by
    // phone has no address. A derived placeholder keeps the constraint honest
    // without inventing one that might reach somebody.
    let email = format!("{}@courier.invalid", phone.replace(['+', ' '], ""));
    db.prepare(
        "INSERT INTO couriers (id,email_encrypted,email_hash,phone_encrypted,phone_hash,\
         full_name_encrypted,password_hash,status,created_at_ms) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,'active',?8)",
    )
    .bind(&[
        cid.clone().into(),
        email.clone().into(),
        auth::sha256_hex(&email).into(),
        phone.clone().into(),
        phone_hash.into(),
        name.into(),
        pw_hash.into(),
        wasm_bindgen::JsValue::from_f64(now as f64),
    ])?
    .run()
    .await?;
    db.prepare(
        "INSERT INTO courier_locations (courier_id,location_id,role,added_at_ms) \
         VALUES (?1,?2,'courier',?3)",
    )
    .bind(&[
        cid.clone().into(),
        inv.location_id.clone().into(),
        wasm_bindgen::JsValue::from_f64(now as f64),
    ])?
    .run()
    .await?;
    // ONE SHOT. A code that survived its own use would be a second key to
    // somebody else's account. Marked used only after the account exists, so a
    // failure above leaves the invite still claimable.
    db.prepare("UPDATE courier_invites SET used_at_ms = ?2, used_by_courier_id = ?3 WHERE id = ?1")
        .bind(&[
            inv.id.into(),
            wasm_bindgen::JsValue::from_f64(now as f64),
            cid.clone().into(),
        ])?
        .run()
        .await?;

    let token_hash = auth::hash_opaque(&secret);
    db.prepare(
        "INSERT INTO courier_sessions (id,courier_id,family_id,token_hash,active_location_id,\
         issued_at_ms,expires_at_ms) VALUES (?1,?2,?3,?4,?5,?6,?7)",
    )
    .bind(&[
        session_id.clone().into(),
        cid.clone().into(),
        family_id.into(),
        token_hash.into(),
        inv.location_id.clone().into(),
        wasm_bindgen::JsValue::from_f64(now as f64),
        wasm_bindgen::JsValue::from_f64((now + COURIER_REFRESH_TTL_MS) as f64),
    ])?
    .run()
    .await?;

    let claims = Claims::Courier {
        sub: cid.clone(),
        active_location_id: inv.location_id.clone(),
        jti: session_id.clone(),
        iat: now,
        exp: now + COURIER_TTL_MS,
    };
    let jwt = match auth::sign(&ctx.env, &claims) {
        Ok(t) => t,
        Err(e) => return e.into_response(),
    };
    // Signed in on the spot: making them claim the code and then type the
    // password they set ten seconds ago is a step that exists only because the
    // two things were written separately.
    Response::from_json(&json!({
        "jwt": jwt,
        "refreshToken": format!("{session_id}.{secret}"),
        "courier": { "id": cid, "locationId": inv.location_id }
    }))
}

#[cfg(test)]
mod tests {
    use super::{venue_for_courier_login as venue, VenueContradiction};

    /// THE BUG THIS ENCODES. A courier of `dubin-durres` signed in at
    /// `sushi-durres.dowiz.org` and was given a dubin session, because the app
    /// sends no `location_id` and the fallback was the courier's FIRST venue.
    /// The host is the one thing that request did say.
    #[test]
    fn the_host_names_the_venue_when_the_body_does_not() {
        assert_eq!(venue(Some("sushi-durres"), None).ok().flatten().as_deref(), Some("sushi-durres"));
    }

    /// A body that agrees with the host is not a conflict.
    #[test]
    fn agreement_is_allowed() {
        assert_eq!(
            venue(Some("sushi-durres"), Some("sushi-durres")).ok().flatten().as_deref(),
            Some("sushi-durres")
        );
    }

    /// FAIL CLOSED. Resolving a contradiction either way mints a session for a
    /// venue the caller did not ask for on a domain that is not it.
    #[test]
    fn a_body_that_contradicts_the_host_is_refused() {
        assert!(matches!(venue(Some("sushi-durres"), Some("dubin-durres")), Err(VenueContradiction)));
    }

    /// Where the host names no venue -- the apex, `*.workers.dev` -- the
    /// caller genuinely has not said, and the old single-membership guess is
    /// still the only answer available.
    #[test]
    fn a_host_that_names_no_venue_leaves_the_choice_open() {
        assert_eq!(venue(None, None).ok().flatten(), None);
        assert_eq!(venue(None, Some("dubin-durres")).ok().flatten().as_deref(), Some("dubin-durres"));
    }
}
