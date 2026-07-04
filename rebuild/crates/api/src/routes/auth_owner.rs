//! AUTH-01/02/03/09 owner flows: local login, Google OAuth (start/callback/exchange), Telegram
//! (start/poll), refresh rotation (ADR-0004), logout. Ports `auth/local.ts` + `auth.ts`.

use axum::Json;
use axum::extract::{Extension, Query};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use domain::ErrorCode;

use crate::auth::AuthState;
use crate::auth::claims::{Claims, OwnerClaims};
use crate::auth::crypto;
use crate::auth::dto::{
    ExchangeRequest, OwnerLoginRequest, OwnerLoginResponse, OwnerRefreshRequest,
    TelegramAuthenticated, TelegramPollQuery, TelegramStartResponse, TokenPairResponse,
};
use crate::auth::error::{AuthEnvelopeError, ConcurrentRefreshResponse};
use crate::auth::extractors::OwnerClaimsExt;
use crate::auth::jwt::ttl;
use crate::auth::service::{
    OwnerRefreshDisposition, OwnerRefreshInputs, owner_refresh_disposition,
};

/// Correlation id for envelope errors — the main router's request-id layer stamps
/// `x-correlation-id`; fall back to a fresh uuid if a handler is invoked without it.
fn corr(headers: &HeaderMap) -> String {
    headers
        .get("x-correlation-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
        .unwrap_or_else(|| crypto::random_uuid().to_string())
}

fn envelope(code: ErrorCode, message: &str, headers: &HeaderMap) -> Response {
    // Q-VAL-400: VALIDATION_FAILED is 400, not 422 (auth.ts:72,77,99 all reply.sendError(400, ...)).
    if code == ErrorCode::ValidationFailed {
        return AuthEnvelopeError::validation_failed(message, corr(headers)).into_response();
    }
    AuthEnvelopeError::new(code, message, corr(headers)).into_response()
}

/// `POST /api/auth/local/login` (AUTH-01) — dev bypass (ADR-0003) then real argon2 (local.ts:36).
#[utoipa::path(
    post, path = "/api/auth/local/login", tag = "auth",
    request_body = OwnerLoginRequest,
    responses(
        (status = 200, description = "Logged in", body = OwnerLoginResponse),
        (status = 401, description = "INVALID_CREDENTIALS / WRONG_AUTH_METHOD"),
    )
)]
pub async fn owner_local_login(
    Extension(state): Extension<AuthState>,
    headers: HeaderMap,
    Json(req): Json<OwnerLoginRequest>,
) -> Response {
    let email_lower = req.email.to_lowercase();

    // ── Path 1: flag-gated dev bypass (inert on prod; local.ts:51-71) ──
    #[cfg(feature = "dev-routes")]
    if state.config.dev_login_allowed() {
        if let (Some(dev_email), Some(dev_password)) = dev_login_creds() {
            if crypto::timing_safe_eq(&email_lower, &dev_email.to_lowercase())
                && crypto::timing_safe_eq(&req.password, &dev_password)
            {
                let Ok(Some((uid, _))) = state.repo.user_by_email(&email_lower).await else {
                    return envelope(
                        ErrorCode::InvalidCredentials,
                        "Invalid credentials",
                        &headers,
                    );
                };
                let (_, loc) = state
                    .repo
                    .resolve_owner_membership(uid)
                    .await
                    .unwrap_or(("owner".to_string(), None));
                let claims = Claims::Owner(OwnerClaims::new(uid, loc));
                // Dev path: signDevToken 7d, NO refresh (local.ts:69-70).
                match state.verifier.mint_dev(claims, ttl::OWNER_DEV_ACCESS) {
                    Ok(token) => {
                        return Json(OwnerLoginResponse {
                            access_token: token,
                            refresh_token: None,
                            user_id: uid,
                            active_location_id: loc,
                        })
                        .into_response();
                    }
                    Err(_) => {
                        return envelope(
                            ErrorCode::Internal,
                            "Authentication unavailable",
                            &headers,
                        );
                    }
                }
            }
        }
    }

    // ── Path 2: real argon2 (local.ts:86-168) ──
    let user = match state.repo.user_by_email(&email_lower).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return envelope(
                ErrorCode::InvalidCredentials,
                "Invalid email or password",
                &headers,
            );
        }
        Err(_) => {
            return envelope(
                ErrorCode::ServiceUnavailable,
                "Service temporarily unavailable, please try again",
                &headers,
            );
        }
    };
    let (user_id, password_hash) = user;
    let Some(hash) = password_hash else {
        // No password_hash → Google/Telegram-only account (local.ts:95-97).
        return envelope(
            ErrorCode::WrongAuthMethod,
            "Account uses another sign-in method",
            &headers,
        );
    };
    if !crypto::argon2_verify(&hash, &req.password) {
        return envelope(
            ErrorCode::InvalidCredentials,
            "Invalid email or password",
            &headers,
        );
    }

    let (role, active_location_id) = state
        .repo
        .resolve_owner_membership(user_id)
        .await
        .unwrap_or(("customer".to_string(), None));

    // Q-ROLE-DEGRADE (local.ts:113,136-139): the resolver degrades to `role:'customer'` on error /
    // no membership. In Node that degraded token signs `{role:'customer', userId}` WITHOUT
    // orderId/locationId — which its own strict `AuthToken.parse` then REJECTS on the next verify
    // (a latent broken-token quirk). The Rust `CustomerClaims` requires orderId/locationId, so it
    // cannot even represent that malformed shape. The port therefore mints the owner-shaped token
    // with the resolved (possibly absent) location — observably the "empty dashboard" state — and
    // never emits an unverifiable customer token. `role` is read only to document this decision.
    let _resolved_role = role;
    let claims = Claims::Owner(OwnerClaims::new(user_id, active_location_id));

    let access_token = match state.verifier.mint(claims, ttl::OWNER_ACCESS) {
        Ok(t) => t,
        Err(_) => return envelope(ErrorCode::Internal, "Authentication unavailable", &headers),
    };

    // Rotating 7d refresh family; conditionally omitted if the INSERT fails (Q-REFRESH-OMIT).
    let refresh_plain = crypto::random_hex_32();
    let refresh_hash = crypto::sha256_hex(&refresh_plain);
    let family_id = crypto::random_uuid();
    let persisted = state
        .repo
        .insert_owner_refresh(user_id, family_id, &refresh_hash)
        .await
        .unwrap_or(false);

    Json(OwnerLoginResponse {
        access_token,
        refresh_token: persisted.then_some(refresh_plain),
        user_id,
        active_location_id,
    })
    .into_response()
}

#[cfg(feature = "dev-routes")]
fn dev_login_creds() -> (Option<String>, Option<String>) {
    (
        std::env::var("DEV_LOGIN_EMAIL").ok(),
        std::env::var("DEV_LOGIN_PASSWORD").ok(),
    )
}

/// `GET /api/auth/google` (AUTH-02) — 302 to Google, or 404 when the flag is off (auth.ts:34).
#[utoipa::path(get, path = "/api/auth/google", tag = "auth",
    responses((status = 302, description = "Redirect to Google"), (status = 404, description = "Flag off")))]
pub async fn google_oauth_start(
    Extension(state): Extension<AuthState>,
    headers: HeaderMap,
) -> Response {
    if !state.config.google_oauth_enabled {
        return envelope(ErrorCode::NotFound, "Not found", &headers);
    }
    let oauth_state = crypto::random_uuid().to_string();
    let nonce = crypto::random_uuid().to_string();
    // The PKCE verifier + nonce live in the ephemeral store, 600s (auth.ts:46).
    state
        .store
        .setex(
            &format!("auth:state:{oauth_state}"),
            600,
            serde_json::json!({ "codeVerifier": crypto::random_hex_32(), "nonce": nonce }),
        )
        .await;
    let redirect_uri = format!("{}/api/auth/google/callback", state.config.app_base_url);
    let url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?response_type=code&scope=openid%20email%20profile&state={oauth_state}&nonce={nonce}&redirect_uri={redirect_uri}"
    );
    (StatusCode::FOUND, [(header::LOCATION, url)]).into_response()
}

/// `GET /api/auth/google/callback` (AUTH-02) — validate state (single-use), exchange the code via
/// the Google seam, upsert the owner, mint a pair, stash under a one-time code, 302 with `#code=`.
#[utoipa::path(get, path = "/api/auth/google/callback", tag = "auth",
    responses((status = 302, description = "Redirect with #code="), (status = 400, description = "VALIDATION_FAILED"), (status = 404, description = "Flag off")))]
pub async fn google_oauth_callback(
    Extension(state): Extension<AuthState>,
    headers: HeaderMap,
    Query(q): Query<OauthCallbackQuery>,
) -> Response {
    if !state.config.google_oauth_enabled {
        return envelope(ErrorCode::NotFound, "Not found", &headers);
    }
    let state_key = format!("auth:state:{}", q.state);
    let Some(state_data) = state.store.get(&state_key).await else {
        return envelope(
            ErrorCode::ValidationFailed,
            "Invalid or expired state",
            &headers,
        );
    };
    state.store.del(&state_key).await; // single-use
    let code_verifier = state_data
        .get("codeVerifier")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let expected_nonce = state_data
        .get("nonce")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let redirect_uri = format!("{}/api/auth/google/callback", state.config.app_base_url);
    let identity = match state
        .google
        .exchange_code(&q.code, &code_verifier, &redirect_uri)
        .await
    {
        Ok(id) => id,
        Err(_) => {
            return envelope(
                ErrorCode::ValidationFailed,
                "Failed to exchange token",
                &headers,
            );
        }
    };
    if identity.nonce != expected_nonce {
        return envelope(ErrorCode::ValidationFailed, "Nonce mismatch", &headers);
    }

    let user_id = match state
        .repo
        .upsert_google_user(&identity.email, &identity.sub, identity.name.as_deref())
        .await
    {
        Ok(id) => id,
        Err(_) => {
            return envelope(
                ErrorCode::ValidationFailed,
                "Failed to upsert user",
                &headers,
            );
        }
    };

    let access_token = match state.verifier.mint(
        Claims::Owner(OwnerClaims::new(user_id, None)),
        ttl::OWNER_ACCESS,
    ) {
        Ok(t) => t,
        Err(_) => return envelope(ErrorCode::Internal, "Internal server error", &headers),
    };
    let refresh_plain = crypto::random_hex_32();
    let refresh_hash = crypto::sha256_hex(&refresh_plain);
    let _ignored = state
        .repo
        .insert_owner_refresh(user_id, crypto::random_uuid(), &refresh_hash)
        .await;

    let opaque = crypto::random_uuid().to_string();
    state
        .store
        .setex(
            &format!("auth:code:{opaque}"),
            60,
            serde_json::json!({ "access_token": access_token, "refresh_token": refresh_plain }),
        )
        .await;
    // Fragment carriage (#code=) — anti-Referer-leak (auth.ts:166).
    let location = format!("{}/auth/callback#code={opaque}", state.config.app_base_url);
    (StatusCode::FOUND, [(header::LOCATION, location)]).into_response()
}

#[derive(Debug, serde::Deserialize)]
pub struct OauthCallbackQuery {
    pub code: String,
    pub state: String,
}

/// `POST /api/auth/exchange` (AUTH-02) — one-shot GET+DEL of the stashed pair (auth.ts:173).
#[utoipa::path(post, path = "/api/auth/exchange", tag = "auth",
    request_body = ExchangeRequest,
    responses((status = 200, body = TokenPairResponse), (status = 400, description = "VALIDATION_FAILED")))]
pub async fn exchange_oauth_code(
    Extension(state): Extension<AuthState>,
    headers: HeaderMap,
    Json(req): Json<ExchangeRequest>,
) -> Response {
    let key = format!("auth:code:{}", req.code);
    let Some(pair) = state.store.getdel(&key).await else {
        return envelope(
            ErrorCode::ValidationFailed,
            "Invalid or expired code",
            &headers,
        );
    };
    let access_token = pair
        .get("access_token")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let refresh_token = pair
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    Json(TokenPairResponse {
        access_token: access_token.to_string(),
        refresh_token: refresh_token.to_string(),
    })
    .into_response()
}

/// `POST /api/auth/telegram/start` (AUTH-03) — mint a 5-min deep-link token (auth.ts:191).
#[utoipa::path(post, path = "/api/auth/telegram/start", tag = "auth",
    responses((status = 200, body = TelegramStartResponse)))]
pub async fn telegram_login_start(
    Extension(state): Extension<AuthState>,
    headers: HeaderMap,
) -> Response {
    match state.repo.telegram_create_login_token().await {
        Ok(token) => {
            let bot = &state.config.telegram_bot_username;
            Json(TelegramStartResponse {
                token,
                bot_username: bot.clone(),
                deep_link: format!("https://t.me/{bot}?start=login_{token}"),
            })
            .into_response()
        }
        Err(_) => envelope(ErrorCode::Internal, "Internal server error", &headers),
    }
}

/// `GET /api/auth/telegram/poll` (AUTH-03) — poll + atomic single-use consume (auth.ts:202).
/// Non-envelope `{status:...}` bodies on 404/410 (Q-TG-POLLSHAPE) carried verbatim.
#[utoipa::path(get, path = "/api/auth/telegram/poll", tag = "auth",
    params(("token" = uuid::Uuid, Query, description = "login token")),
    responses((status = 200, description = "pending/consumed/authenticated"), (status = 404, description = "{status:'unknown'}"), (status = 410, description = "{status:'expired'|'consumed'}")))]
pub async fn telegram_login_poll(
    Extension(state): Extension<AuthState>,
    headers: HeaderMap,
    Query(q): Query<TelegramPollQuery>,
) -> Response {
    let poll = match state.repo.telegram_poll_token(q.token).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            // Q-TG-POLLSHAPE: non-envelope {status:'unknown'} at 404.
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "status": "unknown" })),
            )
                .into_response();
        }
        Err(_) => return envelope(ErrorCode::Internal, "Internal server error", &headers),
    };
    if poll.expired {
        return (
            StatusCode::GONE,
            Json(serde_json::json!({ "status": "expired" })),
        )
            .into_response();
    }
    if poll.status != "authenticated" || poll.user_id.is_none() {
        let status = if poll.status == "consumed" {
            "consumed"
        } else {
            "pending"
        };
        return Json(serde_json::json!({ "status": status })).into_response();
    }

    // Atomic single-use consume; the race loser gets 410 {status:'consumed'} (auth.ts:221).
    let Ok(Some(user_id)) = state.repo.telegram_consume_token(q.token).await else {
        return (
            StatusCode::GONE,
            Json(serde_json::json!({ "status": "consumed" })),
        )
            .into_response();
    };

    let access_token = match state.verifier.mint(
        Claims::Owner(OwnerClaims::new(user_id, None)),
        ttl::OWNER_ACCESS,
    ) {
        Ok(t) => t,
        Err(_) => return envelope(ErrorCode::Internal, "Internal server error", &headers),
    };
    let refresh_plain = crypto::random_hex_32();
    let refresh_hash = crypto::sha256_hex(&refresh_plain);
    let _ignored = state
        .repo
        .insert_owner_refresh(user_id, crypto::random_uuid(), &refresh_hash)
        .await;
    Json(TelegramAuthenticated {
        status: "authenticated".to_string(),
        access_token,
        refresh_token: refresh_plain,
    })
    .into_response()
}

/// `POST /api/auth/refresh` (AUTH-09) — the ADR-0004 rotation state machine (auth.ts:235).
#[utoipa::path(post, path = "/api/auth/refresh", tag = "auth",
    request_body = OwnerRefreshRequest,
    responses(
        (status = 200, body = TokenPairResponse),
        (status = 401, description = "reuse (family revoked) / OWNER_REVOKED / invalid"),
        (status = 409, description = "concurrent_refresh (non-envelope)"),
    ))]
pub async fn owner_refresh(
    Extension(state): Extension<AuthState>,
    headers: HeaderMap,
    Json(req): Json<OwnerRefreshRequest>,
) -> Response {
    let token_hash = crypto::sha256_hex(&req.refresh_token);
    let row = match state.repo.owner_refresh_by_hash(&token_hash).await {
        Ok(r) => r,
        Err(_) => return envelope(ErrorCode::Internal, "Internal server error", &headers),
    };
    let Some(row) = row else {
        return envelope(ErrorCode::Unauthorized, "Invalid refresh token", &headers);
    };

    // Atomic single-use claim (auth.ts:265). If lost, probe the recent-rotation window (5s).
    let claimed = state
        .repo
        .claim_owner_refresh(row.id)
        .await
        .unwrap_or(false);
    let recent = if claimed {
        false
    } else {
        state
            .repo
            .family_rotated_within_5s(row.family_id)
            .await
            .unwrap_or(false)
    };
    let active_owner_locations = if claimed {
        state
            .repo
            .active_owner_locations(row.user_id)
            .await
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let disposition = owner_refresh_disposition(&OwnerRefreshInputs {
        token_found: true,
        token_expired: row.expired,
        claimed,
        recent_family_rotation: recent,
        active_owner_locations: &active_owner_locations,
        requested_location: req.active_location_id,
    });

    match disposition {
        OwnerRefreshDisposition::Invalid => {
            envelope(ErrorCode::Unauthorized, "Invalid refresh token", &headers)
        }
        OwnerRefreshDisposition::ConcurrentRefresh => ConcurrentRefreshResponse.into_response(),
        OwnerRefreshDisposition::ReuseRevokeFamily => {
            let _ignored = state.repo.delete_owner_family(row.family_id).await;
            envelope(
                ErrorCode::Unauthorized,
                "Token reuse detected. Family revoked.",
                &headers,
            )
        }
        OwnerRefreshDisposition::OwnerRevoked => envelope(
            ErrorCode::OwnerRevoked,
            "No active owner membership",
            &headers,
        ),
        OwnerRefreshDisposition::Rotate { active_location_id } => {
            let claims = Claims::Owner(OwnerClaims::new(row.user_id, Some(active_location_id)));
            let access_token = match state.verifier.mint(claims, ttl::OWNER_ACCESS) {
                Ok(t) => t,
                Err(_) => return envelope(ErrorCode::Internal, "Internal server error", &headers),
            };
            let refresh_plain = crypto::random_hex_32();
            let refresh_hash = crypto::sha256_hex(&refresh_plain);
            let _ignored = state
                .repo
                .insert_owner_refresh(row.user_id, row.family_id, &refresh_hash)
                .await;
            Json(TokenPairResponse {
                access_token,
                refresh_token: refresh_plain,
            })
            .into_response()
        }
    }
}

/// `POST /api/auth/logout` (AUTH-09, ADR-0004 P-b) — user-wide refresh-family delete (auth.ts:325).
/// Bound to `OwnerClaimsExt` so a non-owner bearer 401s structurally (Q-LOGOUT).
#[utoipa::path(post, path = "/api/auth/logout", tag = "auth", security(("bearerAuth" = [])),
    responses((status = 204, description = "All families revoked"), (status = 401, description = "Missing/invalid bearer")))]
pub async fn owner_logout(
    Extension(state): Extension<AuthState>,
    owner: OwnerClaimsExt,
) -> Response {
    let _ignored = state
        .repo
        .delete_owner_families_for_user(owner.0.user_id)
        .await;
    StatusCode::NO_CONTENT.into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::repo::OwnerRefreshRow;
    use crate::auth::repo::fake::FakeAuthRepo;
    use axum::body::to_bytes;
    use std::sync::Arc;
    use uuid::Uuid;

    async fn json_body(resp: Response) -> (StatusCode, serde_json::Value) {
        let status = resp.status();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json = if bytes.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        };
        (status, json)
    }

    #[tokio::test]
    async fn owner_login_mints_access_and_refresh_for_valid_password() {
        let repo = FakeAuthRepo::default();
        let uid = Uuid::new_v4();
        // Seed a user with a known argon2 hash.
        use argon2::Argon2;
        use argon2::password_hash::{PasswordHasher, SaltString};
        let salt = SaltString::from_b64("YWJjZGVmZ2hpamtsbW5vcA").unwrap();
        let hash = Argon2::default()
            .hash_password(b"pw123456", &salt)
            .unwrap()
            .to_string();
        repo.users_by_email
            .lock()
            .unwrap()
            .insert("o@x.com".to_string(), (uid, Some(hash)));
        repo.owner_membership
            .lock()
            .unwrap()
            .insert(uid, ("owner".to_string(), Some(Uuid::new_v4())));
        let state = AuthState::test_state(Arc::new(repo));

        let resp = owner_local_login(
            Extension(state),
            HeaderMap::new(),
            Json(OwnerLoginRequest {
                email: "O@x.com".to_string(),
                password: "pw123456".to_string(),
            }),
        )
        .await;
        let (status, json) = json_body(resp).await;
        assert_eq!(status, StatusCode::OK);
        assert!(json.get("access_token").is_some());
        assert!(json.get("refresh_token").is_some());
    }

    #[tokio::test]
    async fn owner_login_wrong_password_is_401_invalid_credentials() {
        let repo = FakeAuthRepo::default();
        let uid = Uuid::new_v4();
        use argon2::Argon2;
        use argon2::password_hash::{PasswordHasher, SaltString};
        let salt = SaltString::from_b64("YWJjZGVmZ2hpamtsbW5vcA").unwrap();
        let hash = Argon2::default()
            .hash_password(b"correct-pw", &salt)
            .unwrap()
            .to_string();
        repo.users_by_email
            .lock()
            .unwrap()
            .insert("o@x.com".to_string(), (uid, Some(hash)));
        let state = AuthState::test_state(Arc::new(repo));
        let resp = owner_local_login(
            Extension(state),
            HeaderMap::new(),
            Json(OwnerLoginRequest {
                email: "o@x.com".to_string(),
                password: "wrong".to_string(),
            }),
        )
        .await;
        let (status, json) = json_body(resp).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(json["code"], "INVALID_CREDENTIALS");
    }

    #[tokio::test]
    async fn owner_login_no_password_hash_is_wrong_auth_method() {
        let repo = FakeAuthRepo::default();
        let uid = Uuid::new_v4();
        repo.users_by_email
            .lock()
            .unwrap()
            .insert("g@x.com".to_string(), (uid, None));
        let state = AuthState::test_state(Arc::new(repo));
        let resp = owner_local_login(
            Extension(state),
            HeaderMap::new(),
            Json(OwnerLoginRequest {
                email: "g@x.com".to_string(),
                password: "x".to_string(),
            }),
        )
        .await;
        let (status, json) = json_body(resp).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(json["code"], "WRONG_AUTH_METHOD");
    }

    #[tokio::test]
    async fn owner_refresh_reuse_revokes_family_401() {
        // T-1: a stale token (claim lost, no recent rotation) → family revoked, 401.
        let repo = FakeAuthRepo::default();
        let id = Uuid::new_v4();
        let family = Uuid::new_v4();
        repo.owner_refresh_by_hash.lock().unwrap().insert(
            crypto::sha256_hex("stale-token"),
            OwnerRefreshRow {
                id,
                user_id: Uuid::new_v4(),
                family_id: family,
                expired: false,
            },
        );
        repo.claimed_ids.lock().unwrap().insert(id, false); // claim lost
        repo.recent_family.lock().unwrap().insert(family, false); // no recent rotation
        let state = AuthState::test_state(Arc::new(repo));
        let resp = owner_refresh(
            Extension(state),
            HeaderMap::new(),
            Json(OwnerRefreshRequest {
                refresh_token: "stale-token".to_string(),
                active_location_id: None,
            }),
        )
        .await;
        let (status, json) = json_body(resp).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(json["message"], "Token reuse detected. Family revoked.");
    }

    #[tokio::test]
    async fn owner_refresh_concurrent_is_409_non_envelope() {
        // T-2: claim lost BUT a family token rotated <5s ago → benign 409 concurrent_refresh.
        let repo = FakeAuthRepo::default();
        let id = Uuid::new_v4();
        let family = Uuid::new_v4();
        repo.owner_refresh_by_hash.lock().unwrap().insert(
            crypto::sha256_hex("t"),
            OwnerRefreshRow {
                id,
                user_id: Uuid::new_v4(),
                family_id: family,
                expired: false,
            },
        );
        repo.claimed_ids.lock().unwrap().insert(id, false);
        repo.recent_family.lock().unwrap().insert(family, true);
        let state = AuthState::test_state(Arc::new(repo));
        let resp = owner_refresh(
            Extension(state),
            HeaderMap::new(),
            Json(OwnerRefreshRequest {
                refresh_token: "t".to_string(),
                active_location_id: None,
            }),
        )
        .await;
        let (status, json) = json_body(resp).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(json, serde_json::json!({ "error": "concurrent_refresh" }));
    }

    #[tokio::test]
    async fn owner_refresh_demoted_owner_is_owner_revoked() {
        // T-4: claimed but no active owner membership → 401 OWNER_REVOKED.
        let repo = FakeAuthRepo::default();
        let id = Uuid::new_v4();
        let user = Uuid::new_v4();
        repo.owner_refresh_by_hash.lock().unwrap().insert(
            crypto::sha256_hex("t"),
            OwnerRefreshRow {
                id,
                user_id: user,
                family_id: Uuid::new_v4(),
                expired: false,
            },
        );
        repo.claimed_ids.lock().unwrap().insert(id, true);
        // No active_owner_locations entry → empty → OwnerRevoked.
        let state = AuthState::test_state(Arc::new(repo));
        let resp = owner_refresh(
            Extension(state),
            HeaderMap::new(),
            Json(OwnerRefreshRequest {
                refresh_token: "t".to_string(),
                active_location_id: None,
            }),
        )
        .await;
        let (status, json) = json_body(resp).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(json["code"], "OWNER_REVOKED");
    }

    #[tokio::test]
    async fn owner_refresh_rotates_and_preserves_requested_location() {
        let repo = FakeAuthRepo::default();
        let id = Uuid::new_v4();
        let user = Uuid::new_v4();
        let loc = Uuid::new_v4();
        repo.owner_refresh_by_hash.lock().unwrap().insert(
            crypto::sha256_hex("t"),
            OwnerRefreshRow {
                id,
                user_id: user,
                family_id: Uuid::new_v4(),
                expired: false,
            },
        );
        repo.claimed_ids.lock().unwrap().insert(id, true);
        repo.active_owner_locations
            .lock()
            .unwrap()
            .insert(user, vec![loc, Uuid::new_v4()]);
        let state = AuthState::test_state(Arc::new(repo));
        let resp = owner_refresh(
            Extension(state.clone()),
            HeaderMap::new(),
            Json(OwnerRefreshRequest {
                refresh_token: "t".to_string(),
                active_location_id: Some(loc),
            }),
        )
        .await;
        let (status, json) = json_body(resp).await;
        assert_eq!(status, StatusCode::OK);
        // Verify the new access token carries the preserved location.
        let access = json["access_token"].as_str().unwrap();
        let claims = state.verifier.verify(access).unwrap();
        assert_eq!(claims.as_owner().unwrap().active_location_id, Some(loc));
    }
}
