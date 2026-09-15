//! Who is calling, and may they.
//!
//! The front-ends already had a login contract before the hub existed, and this
//! implements THAT contract rather than a new one: the admin pane posts
//! `{email, password}` to `/api/auth/login` and expects
//! `{access_token, refresh_token, user:{locationId}}`; the courier app posts
//! `{email|phone, password}` to `/api/courier/auth/login` and expects `{jwt}`.
//! Keeping the shapes means no front-end change was needed to put a working
//! authority behind them.
//!
//! A PERSON IS KEYED BY THEIR EMAIL OR PHONE, lowercased. For one venue that is
//! the natural identifier and there is no directory to collide with. It is also
//! what makes the roster legible to the owner, who knows their staff by phone
//! number and not by a generated id.

use std::sync::Arc;

use axum::extract::{FromRequestParts, State};
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use dowiz_hub::roster::Person;
use dowiz_hub::token::{self, Claims, Role, ACCESS_TTL_MS, REFRESH_TTL_MS};

use crate::hub::{HubHttpError, Shared};

/// An authenticated caller.
///
/// Holding a `Person` and not just claims is deliberate: the token says who the
/// bearer CLAIMS to be, and the roster says whether that person still exists,
/// is still active, and still holds a live session. A token alone cannot
/// answer those, which is why revocation has to be checked and not inferred.
#[derive(Debug, Clone)]
pub struct Caller {
    pub person: Person,
    pub session: String,
}

/// An owner-only caller.
pub struct OwnerCaller(pub Caller);
/// A courier-only caller.
pub struct CourierCaller(pub Caller);

#[derive(Debug)]
pub enum AuthError {
    /// No credential presented at all.
    Missing,
    /// A credential that does not verify, has expired, or whose session is gone.
    ///
    /// ONE variant for all three on purpose. Telling a caller *why* their token
    /// failed tells an attacker which half of a forgery worked.
    Rejected,
    /// Verified, but the wrong role for this endpoint.
    Forbidden,
    /// The hub could not read its own roster.
    Unavailable(String),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (code, msg) = match self {
            AuthError::Missing => (StatusCode::UNAUTHORIZED, "authentication required"),
            AuthError::Rejected => (StatusCode::UNAUTHORIZED, "invalid credentials"),
            AuthError::Forbidden => (StatusCode::FORBIDDEN, "not permitted"),
            AuthError::Unavailable(ref e) => {
                eprintln!("auth: roster unavailable: {e}");
                (StatusCode::SERVICE_UNAVAILABLE, "roster unavailable")
            }
        };
        (code, Json(json!({ "error": msg }))).into_response()
    }
}

/// The shared verification path. Every extractor below goes through it, so
/// there is one place where "is this caller real" is decided.
fn authenticate(state: &Shared, parts: &Parts) -> Result<Caller, AuthError> {
    let header = parts
        .headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(AuthError::Missing)?;
    let raw = header.strip_prefix("Bearer ").ok_or(AuthError::Missing)?.trim();
    if raw.is_empty() {
        return Err(AuthError::Missing);
    }

    let claims = token::verify(state.signing_key(), raw, crate::hub::now_ms())
        .map_err(|_| AuthError::Rejected)?;
    // A refresh token is exchangeable for an access token and is good for
    // nothing else. Without this check, short access lifetimes buy nothing.
    if claims.role == Role::Refresh {
        return Err(AuthError::Rejected);
    }

    let roster = state.read_roster().map_err(|e| AuthError::Unavailable(format!("{e:?}")))?;
    // The session must still be live. This is the revocation check, and it is
    // why a logout takes effect immediately instead of at token expiry.
    match roster.session_owner(&claims.session) {
        Some(owner) if owner == claims.subject => {}
        _ => return Err(AuthError::Rejected),
    }
    let person = roster.person(&claims.subject).ok_or(AuthError::Rejected)?;
    if !person.active || person.role != claims.role {
        // A role that has changed since the token was minted must not keep
        // working: demoting someone has to take effect without waiting.
        return Err(AuthError::Rejected);
    }
    Ok(Caller { person, session: claims.session })
}

impl FromRequestParts<Shared> for Caller {
    type Rejection = AuthError;
    async fn from_request_parts(parts: &mut Parts, state: &Shared) -> Result<Self, Self::Rejection> {
        authenticate(state, parts)
    }
}

impl FromRequestParts<Shared> for OwnerCaller {
    type Rejection = AuthError;
    async fn from_request_parts(parts: &mut Parts, state: &Shared) -> Result<Self, Self::Rejection> {
        let c = authenticate(state, parts)?;
        if c.person.role != Role::Owner {
            return Err(AuthError::Forbidden);
        }
        Ok(OwnerCaller(c))
    }
}

impl FromRequestParts<Shared> for CourierCaller {
    type Rejection = AuthError;
    async fn from_request_parts(parts: &mut Parts, state: &Shared) -> Result<Self, Self::Rejection> {
        let c = authenticate(state, parts)?;
        if c.person.role != Role::Courier {
            return Err(AuthError::Forbidden);
        }
        Ok(CourierCaller(c))
    }
}

// ── routes ───────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct LoginIn {
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    pub password: String,
}

impl LoginIn {
    /// The roster key. Lowercased so `Ana@` and `ana@` are one person, trimmed
    /// because a phone keyboard adds spaces.
    fn identifier(&self) -> Option<String> {
        self.email
            .as_deref()
            .or(self.phone.as_deref())
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty())
    }
}

#[derive(Deserialize)]
pub struct RefreshIn {
    pub refresh_token: String,
}

/// Mint the pair. Extracted so login and refresh cannot drift apart in what
/// they issue.
fn issue(state: &Shared, person: &Person, session: &str) -> (String, String) {
    let now = crate::hub::now_ms();
    let access = token::mint(
        state.signing_key(),
        &Claims {
            role: person.role,
            subject: person.id.clone(),
            session: session.to_string(),
            scope: String::new(),
            issued_ms: now,
            expires_ms: now + ACCESS_TTL_MS,
        },
    );
    let refresh = token::mint(
        state.signing_key(),
        &Claims {
            role: Role::Refresh,
            subject: person.id.clone(),
            session: session.to_string(),
            scope: String::new(),
            issued_ms: now,
            expires_ms: now + REFRESH_TTL_MS,
        },
    );
    (access, refresh)
}

/// `POST /api/auth/login` — the owner pane.
pub async fn login(State(st): State<Shared>, Json(body): Json<LoginIn>) -> Response {
    let Some(id) = body.identifier() else {
        return AuthError::Rejected.into_response();
    };
    login_as(&st, &id, &body.password, Role::Owner, false).await
}

/// `POST /api/courier/auth/login` — the courier app, which expects `{jwt}`.
pub async fn courier_login(State(st): State<Shared>, Json(body): Json<LoginIn>) -> Response {
    let Some(id) = body.identifier() else {
        return AuthError::Rejected.into_response();
    };
    login_as(&st, &id, &body.password, Role::Courier, true).await
}

async fn login_as(st: &Shared, id: &str, password: &str, want: Role, jwt_shape: bool) -> Response {
    // ON THE BLOCKING POOL, not the async runtime. Password verification is
    // 600k iterations of PBKDF2 -- that is the point of it -- and CPU work of
    // that size on a tokio worker thread stalls every other request sharing
    // that thread. Slow BY DESIGN for the caller is correct; slow for everyone
    // else on the box is a self-inflicted denial of service.
    let (owned_st, owned_id, owned_pw) = (st.clone(), id.to_string(), password.to_string());
    let person = match tokio::task::spawn_blocking(move || {
        let roster = owned_st.read_roster()?;
        // The password is checked BEFORE the role. Checking the role first
        // would answer instantly for a person of the wrong role, which tells a
        // caller that the account exists.
        Ok::<_, HubHttpError>(roster.authenticate(&owned_id, &owned_pw))
    })
    .await
    {
        Ok(Ok(p)) => p,
        Ok(Err(e)) => return AuthError::Unavailable(format!("{e:?}")).into_response(),
        Err(e) => return AuthError::Unavailable(format!("join: {e}")).into_response(),
    };
    let Some(person) = person.filter(|p| p.role == want) else {
        return AuthError::Rejected.into_response();
    };

    let pid = person.id.clone();
    let session = match st.with_roster(move |r| {
        r.open_session(&pid, crate::hub::now_ms())
            .map_err(|e| HubHttpError::Io(format!("{e:?}")))
    }).await {
        Ok(s) => s,
        Err(e) => return e.into_response(),
    };

    let (access, refresh) = issue(st, &person, &session);
    if jwt_shape {
        Json(json!({ "jwt": access, "refresh_token": refresh,
                     "courier": { "id": person.id, "name": person.name } }))
            .into_response()
    } else {
        Json(json!({
            "access_token": access,
            "refresh_token": refresh,
            "user": { "id": person.id, "name": person.name, "role": person.role.as_str(),
                      "locationId": st.location_id() }
        }))
        .into_response()
    }
}

/// `POST /api/auth/refresh`.
pub async fn refresh(State(st): State<Shared>, Json(body): Json<RefreshIn>) -> Response {
    let claims = match token::verify(st.signing_key(), &body.refresh_token, crate::hub::now_ms()) {
        Ok(c) if c.role == Role::Refresh => c,
        // An ACCESS token presented here is refused. Accepting one would let a
        // stolen short-lived token be laundered into a long-lived pair, which
        // removes the point of the short lifetime.
        _ => return AuthError::Rejected.into_response(),
    };

    let roster = match st.read_roster() {
        Ok(r) => r,
        Err(e) => return AuthError::Unavailable(format!("{e:?}")).into_response(),
    };
    match roster.session_owner(&claims.session) {
        Some(o) if o == claims.subject => {}
        _ => return AuthError::Rejected.into_response(),
    }
    let Some(person) = roster.person(&claims.subject).filter(|p| p.active) else {
        return AuthError::Rejected.into_response();
    };

    let (access, refresh) = issue(&st, &person, &claims.session);
    Json(json!({ "access_token": access, "refresh_token": refresh })).into_response()
}

/// `POST /api/auth/logout` — revokes the session, so every token under it dies
/// now rather than at expiry.
pub async fn logout(State(st): State<Shared>, caller: Caller) -> Result<Json<Value>, HubHttpError> {
    let sid = caller.session.clone();
    st.with_roster(move |r| {
        r.revoke_session(&sid);
        Ok(())
    })
    .await?;
    Ok(Json(json!({ "ok": true })))
}

/// `GET /api/auth/me` — who the bearer is. The front-ends use it to decide
/// whether a stored token is still worth using before drawing a logged-in UI.
pub async fn me(caller: Caller) -> Json<Value> {
    Json(json!({
        "id": caller.person.id,
        "name": caller.person.name,
        "role": caller.person.role.as_str(),
    }))
}

pub fn routes(state: Shared) -> Router {
    Router::new()
        .route("/api/auth/login", post(login))
        .route("/api/auth/refresh", post(refresh))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/me", axum::routing::get(me))
        .route("/api/courier/auth/login", post(courier_login))
        .with_state(state)
}

/// Re-exported so the route modules do not each import the hub's Arc shape.
pub type SharedState = Arc<crate::hub::HubState>;
