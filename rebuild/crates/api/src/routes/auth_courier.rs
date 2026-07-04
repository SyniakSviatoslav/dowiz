//! AUTH-05/06 courier flows: invite details, redeem, login, refresh (reuse-detection), logout.
//! Ports `apps/api/src/routes/courier/auth.ts`. Courier auth uses the MANUAL-Zod 400 shape
//! (Q-COURIER-ZOD) and the courier-specific envelope codes.

use axum::Json;
use axum::extract::{Extension, Path};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use domain::ErrorCode;
use uuid::Uuid;

use crate::auth::AuthState;
use crate::auth::claims::{Claims, CourierClaims};
use crate::auth::crypto;
use crate::auth::dto::{
    CourierBrief, CourierInviteResponse, CourierLocationBrief, CourierLoginRequest,
    CourierLoginResponse, CourierLogoutRequest, CourierLogoutResponse, CourierRedeemRequest,
    CourierRedeemResponse, CourierRefreshRequest, CourierRefreshResponse,
};
use crate::auth::error::{AuthEnvelopeError, CourierZodResponse};
use crate::auth::jwt::ttl;
use crate::auth::pii::mask_str;

fn corr(headers: &HeaderMap) -> String {
    headers
        .get("x-correlation-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
        .unwrap_or_else(|| crypto::random_uuid().to_string())
}

fn envelope(code: ErrorCode, message: &str, headers: &HeaderMap) -> Response {
    AuthEnvelopeError::new(code, message, corr(headers)).into_response()
}

/// `GET /api/courier/auth/invites/{inviteId}` (AUTH-05) — public invite details (courier/auth.ts:159).
#[utoipa::path(get, path = "/api/courier/auth/invites/{inviteId}", tag = "auth",
    params(("inviteId" = Uuid, Path, description = "invite id")),
    responses((status = 200, body = CourierInviteResponse), (status = 404, description = "Unknown invite")))]
pub async fn get_courier_invite(
    Extension(state): Extension<AuthState>,
    headers: HeaderMap,
    Path(invite_id): Path<Uuid>,
) -> Response {
    match state.repo.courier_invite_details(invite_id).await {
        Ok(Some(d)) => {
            let is_valid = !d.is_used && !d.is_revoked && !d.is_expired;
            Json(CourierInviteResponse {
                id: invite_id,
                role: d.role,
                location_name: d.location_name,
                is_valid,
                is_expired: d.is_expired,
                is_used: d.is_used,
                is_revoked: d.is_revoked,
            })
            .into_response()
        }
        Ok(None) => envelope(ErrorCode::NotFound, "Invite not found", &headers),
        Err(_) => envelope(ErrorCode::Internal, "Internal server error", &headers),
    }
}

/// `POST /api/courier/auth/invites/{inviteId}/redeem` (AUTH-05) — create courier (PII encrypted),
/// mint 14d JWT + 30d session (courier/auth.ts:23). Manual-Zod 400 (Q-COURIER-ZOD).
#[utoipa::path(post, path = "/api/courier/auth/invites/{inviteId}/redeem", tag = "auth",
    params(("inviteId" = Uuid, Path)), request_body = CourierRedeemRequest,
    responses((status = 200, body = CourierRedeemResponse), (status = 400, description = "manual-Zod"), (status = 401, description = "INVALID_CODE"), (status = 410, description = "INVITE_INVALID")))]
pub async fn courier_redeem_invite(
    Extension(state): Extension<AuthState>,
    headers: HeaderMap,
    Path(invite_id): Path<Uuid>,
    Json(req): Json<CourierRedeemRequest>,
) -> Response {
    // Manual-Zod parity (courier/auth.ts:34-45): password ≥12, email/full_name non-empty.
    if req.password.len() < 12 || req.email.is_empty() || req.full_name.is_empty() {
        return CourierZodResponse {
            details: serde_json::json!({ "_errors": ["Validation failed"] }),
        }
        .into_response();
    }
    let email = req.email.to_lowercase();
    let email = email.trim();

    let invite = match state.repo.courier_invite_for_redeem(invite_id).await {
        Ok(Some(i)) => i,
        Ok(None) => {
            return envelope(
                ErrorCode::InviteInvalid,
                "Invite invalid, expired, or already used",
                &headers,
            );
        }
        Err(_) => return envelope(ErrorCode::Internal, "Internal server error", &headers),
    };

    // argon2 verify the invite code (app-layer; courier/auth.ts:68).
    if !crypto::argon2_verify(&invite.code_hash, &req.code) {
        return envelope(ErrorCode::InvalidCode, "Invalid code", &headers);
    }

    let Some(cipher) = state.pii_cipher.as_ref() else {
        return envelope(ErrorCode::Internal, "PII encryption unavailable", &headers);
    };
    let password_hash = match hash_password(&req.password) {
        Ok(h) => h,
        Err(_) => return envelope(ErrorCode::Internal, "Internal server error", &headers),
    };
    let email_hash = crypto::sha256_hex(email);
    let (email_enc, full_name_enc) = match (cipher.encrypt(email), cipher.encrypt(&req.full_name)) {
        (Ok(e), Ok(f)) => (e, f),
        _ => return envelope(ErrorCode::Internal, "Internal server error", &headers),
    };
    let (phone_hash, phone_enc) = match req.phone.as_deref().filter(|p| !p.is_empty()) {
        Some(p) => match cipher.encrypt(p) {
            Ok(enc) => (Some(crypto::sha256_hex(p)), Some(enc)),
            Err(_) => return envelope(ErrorCode::Internal, "Internal server error", &headers),
        },
        None => (None, None),
    };

    let courier_id = match state
        .repo
        .redeem_courier_write(
            invite_id,
            invite.location_id,
            &invite.role,
            invite.created_by_owner_id,
            &email_hash,
            &email_enc,
            phone_hash.as_deref(),
            phone_enc.as_deref(),
            &full_name_enc,
            &password_hash,
        )
        .await
    {
        Ok(id) => id,
        Err(_) => return envelope(ErrorCode::Internal, "Internal server error", &headers),
    };

    // 30d session + 14d JWT (AUTH-GAP-3). refreshToken = "sessionId.tokenPlain".
    let token_plain = crypto::random_hex_32();
    let token_hash = match hash_password(&token_plain) {
        Ok(h) => h,
        Err(_) => return envelope(ErrorCode::Internal, "Internal server error", &headers),
    };
    let family_id = crypto::random_uuid();
    let session_id = match state
        .repo
        .create_courier_session(courier_id, family_id, &token_hash, invite.location_id)
        .await
    {
        Ok(id) => id,
        Err(_) => return envelope(ErrorCode::Internal, "Internal server error", &headers),
    };
    let jwt = match state.verifier.mint(
        Claims::Courier(CourierClaims::new(
            courier_id,
            invite.location_id,
            Some(session_id),
        )),
        ttl::COURIER_REDEEM_ACCESS,
    ) {
        Ok(t) => t,
        Err(_) => return envelope(ErrorCode::Internal, "Internal server error", &headers),
    };

    Json(CourierRedeemResponse {
        jwt,
        refresh_token: format!("{session_id}.{token_plain}"),
        courier: CourierBrief {
            id: courier_id,
            masked_email: mask_str(email),
            full_name: req.full_name,
            locations: vec![CourierLocationBrief {
                id: invite.location_id,
                role: invite.role,
            }],
        },
    })
    .into_response()
}

/// `POST /api/courier/auth/login` (AUTH-06) — email OR phone + password → 24h JWT + 30d session
/// (courier/auth.ts:219). Timing-safe: a dummy argon2 verify on unknown identity (T-11).
#[utoipa::path(post, path = "/api/courier/auth/login", tag = "auth",
    request_body = CourierLoginRequest,
    responses((status = 200, body = CourierLoginResponse), (status = 401, description = "INVALID_CREDENTIALS"), (status = 403, description = "COURIER_DEACTIVATED / location")))]
pub async fn courier_login(
    Extension(state): Extension<AuthState>,
    headers: HeaderMap,
    Json(req): Json<CourierLoginRequest>,
) -> Response {
    if req.email.is_empty() || req.password.is_empty() {
        return CourierZodResponse {
            details: serde_json::json!({ "_errors": ["Validation failed"] }),
        }
        .into_response();
    }
    let identity = req.email.to_lowercase();
    let identity = identity.trim();
    let identity_hash = crypto::sha256_hex(identity);

    let courier = match state.repo.courier_by_identity_hash(&identity_hash).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            // T-11: real dummy hash+verify to equalize timing for an unknown identity, matching
            // Node's `argon2.verify(await argon2.hash('dummy'), password)` (courier/auth.ts:255).
            // A real hash+verify (not a cheap parse-fail) is what actually equalizes the timing.
            if let Ok(dummy) = hash_password("dummy") {
                let _ = crypto::argon2_verify(&dummy, &req.password);
            }
            return envelope(
                ErrorCode::InvalidCredentials,
                "Invalid credentials",
                &headers,
            );
        }
        Err(_) => return envelope(ErrorCode::Internal, "Internal server error", &headers),
    };

    if !crypto::argon2_verify(&courier.password_hash, &req.password) {
        return envelope(
            ErrorCode::InvalidCredentials,
            "Invalid credentials",
            &headers,
        );
    }
    if courier.status == "deactivated" {
        return envelope(
            ErrorCode::CourierDeactivated,
            "Courier deactivated",
            &headers,
        );
    }

    // Resolve location: explicit must be a membership, else first assigned.
    let (location_id, role) = match req.location_id {
        Some(loc) => match state.repo.courier_location_role(courier.id, loc).await {
            Ok(Some(role)) => (loc, role),
            Ok(None) => {
                return envelope(
                    ErrorCode::NotAuthorizedForLocation,
                    "Not authorized for this location",
                    &headers,
                );
            }
            Err(_) => return envelope(ErrorCode::Internal, "Internal server error", &headers),
        },
        None => match state.repo.courier_first_location(courier.id).await {
            Ok(Some((loc, role))) => (loc, role),
            Ok(None) => {
                return envelope(
                    ErrorCode::NoLocationAssigned,
                    "No location assigned",
                    &headers,
                );
            }
            Err(_) => return envelope(ErrorCode::Internal, "Internal server error", &headers),
        },
    };

    let token_plain = crypto::random_hex_32();
    let token_hash = match hash_password(&token_plain) {
        Ok(h) => h,
        Err(_) => return envelope(ErrorCode::Internal, "Internal server error", &headers),
    };
    let family_id = crypto::random_uuid();
    let session_id = match state
        .repo
        .create_courier_session(courier.id, family_id, &token_hash, location_id)
        .await
    {
        Ok(id) => id,
        Err(_) => return envelope(ErrorCode::Internal, "Internal server error", &headers),
    };
    let jwt = match state.verifier.mint(
        Claims::Courier(CourierClaims::new(
            courier.id,
            location_id,
            Some(session_id),
        )),
        ttl::COURIER_LOGIN_ACCESS,
    ) {
        Ok(t) => t,
        Err(_) => return envelope(ErrorCode::Internal, "Internal server error", &headers),
    };
    Json(CourierLoginResponse {
        jwt,
        refresh_token: format!("{session_id}.{token_plain}"),
        active_location_id: location_id,
        role,
    })
    .into_response()
}

/// `POST /api/courier/auth/refresh` (AUTH-06) — reuse-detection family-revoke + status re-check
/// (courier/auth.ts:354). Format `sessionId.tokenPlain`.
#[utoipa::path(post, path = "/api/courier/auth/refresh", tag = "auth",
    request_body = CourierRefreshRequest,
    responses((status = 200, body = CourierRefreshResponse), (status = 401, description = "INVALID_REFRESH_TOKEN / SESSION_NOT_FOUND / REFRESH_REUSED / REFRESH_EXPIRED / COURIER_DEACTIVATED")))]
pub async fn courier_refresh(
    Extension(state): Extension<AuthState>,
    headers: HeaderMap,
    Json(req): Json<CourierRefreshRequest>,
) -> Response {
    let Some((session_id_str, token_plain)) = req.refresh_token.split_once('.') else {
        return envelope(
            ErrorCode::InvalidRefreshToken,
            "Invalid refresh token format",
            &headers,
        );
    };
    let Ok(session_id) = Uuid::parse_str(session_id_str) else {
        return envelope(
            ErrorCode::InvalidRefreshToken,
            "Invalid refresh token format",
            &headers,
        );
    };

    let session = match state.repo.courier_session_by_id(session_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return envelope(ErrorCode::SessionNotFound, "Session not found", &headers),
        Err(_) => return envelope(ErrorCode::Internal, "Internal server error", &headers),
    };

    if !crypto::argon2_verify(&session.token_hash, token_plain) {
        return envelope(
            ErrorCode::InvalidRefreshToken,
            "Invalid refresh token",
            &headers,
        );
    }
    if session.revoked {
        // Reuse detected → revoke the whole family (committed), 401 REFRESH_REUSED (auth.ts:418).
        let _ignored = state.repo.revoke_courier_family(session.family_id).await;
        return envelope(ErrorCode::RefreshReused, "Refresh token reused", &headers);
    }
    if session.expired {
        return envelope(ErrorCode::RefreshExpired, "Refresh token expired", &headers);
    }
    // Status re-check (Q-COURIER-NORECHECK: only status, no per-location re-check — carried).
    match state.repo.courier_status(session.courier_id).await {
        Ok(Some(s)) if s == "active" => {}
        Ok(_) => {
            return envelope(
                ErrorCode::CourierDeactivated,
                "Courier deactivated",
                &headers,
            );
        }
        Err(_) => return envelope(ErrorCode::Internal, "Internal server error", &headers),
    }

    let new_token_plain = crypto::random_hex_32();
    let new_token_hash = match hash_password(&new_token_plain) {
        Ok(h) => h,
        Err(_) => return envelope(ErrorCode::Internal, "Internal server error", &headers),
    };
    let new_session_id = match state
        .repo
        .rotate_courier_session(
            session.id,
            session.courier_id,
            session.family_id,
            &new_token_hash,
            session.active_location_id,
        )
        .await
    {
        Ok(id) => id,
        Err(_) => return envelope(ErrorCode::Internal, "Internal server error", &headers),
    };
    let jwt = match state.verifier.mint(
        Claims::Courier(CourierClaims::new(
            session.courier_id,
            session.active_location_id,
            Some(new_session_id),
        )),
        ttl::COURIER_LOGIN_ACCESS,
    ) {
        Ok(t) => t,
        Err(_) => return envelope(ErrorCode::Internal, "Internal server error", &headers),
    };
    Json(CourierRefreshResponse {
        jwt,
        refresh_token: format!("{new_session_id}.{new_token_plain}"),
    })
    .into_response()
}

/// `POST /api/courier/auth/logout` (AUTH-06) — best-effort, ALWAYS 200 (Q-COURIER-LOGOUT).
#[utoipa::path(post, path = "/api/courier/auth/logout", tag = "auth",
    request_body = CourierLogoutRequest,
    responses((status = 200, body = CourierLogoutResponse)))]
pub async fn courier_logout(
    Extension(state): Extension<AuthState>,
    Json(req): Json<CourierLogoutRequest>,
) -> Response {
    // Silently ignore malformed input (logout must never error the client).
    if let Some((session_id_str, _)) = req.refresh_token.split_once('.') {
        if let Ok(session_id) = Uuid::parse_str(session_id_str) {
            let _ignored = state.repo.revoke_courier_session(session_id).await;
        }
    }
    Json(CourierLogoutResponse { success: true }).into_response()
}

/// argon2id hash (matching Node's `argon2.hash(x, {type:argon2id, m:65536, t:3, p:4})`).
fn hash_password(password: &str) -> Result<String, ()> {
    use argon2::password_hash::{PasswordHasher, SaltString};
    use argon2::{Argon2, Params};
    let salt = SaltString::generate(&mut rand::thread_rng());
    let params = Params::new(65536, 3, 4, None).map_err(|_e| ())?;
    let argon = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    argon
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|_e| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::repo::fake::FakeAuthRepo;
    use crate::auth::repo::{CourierAuthRow, CourierSessionRefreshRow};
    use axum::body::to_bytes;
    use axum::http::StatusCode;
    use std::sync::Arc;

    async fn json_body(resp: Response) -> (StatusCode, serde_json::Value) {
        let status = resp.status();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn courier_login_unknown_identity_is_401_invalid_credentials() {
        // T-11: unknown identity → 401 (indistinguishable from bad password).
        let state = AuthState::test_state(Arc::new(FakeAuthRepo::default()));
        let resp = courier_login(
            Extension(state),
            HeaderMap::new(),
            Json(CourierLoginRequest {
                email: "ghost@x.com".to_string(),
                password: "whatever".to_string(),
                location_id: None,
            }),
        )
        .await;
        let (status, json) = json_body(resp).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(json["code"], "INVALID_CREDENTIALS");
    }

    #[tokio::test]
    async fn courier_login_mints_jwt_and_session_for_valid_courier() {
        let repo = FakeAuthRepo::default();
        let courier_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let pw_hash = hash_password("delivery-pw").unwrap();
        repo.couriers_by_hash.lock().unwrap().insert(
            crypto::sha256_hex("rider@x.com"),
            CourierAuthRow {
                id: courier_id,
                password_hash: pw_hash,
                status: "active".to_string(),
            },
        );
        repo.courier_first_location
            .lock()
            .unwrap()
            .insert(courier_id, (loc, "courier".to_string()));
        let state = AuthState::test_state(Arc::new(repo));
        let resp = courier_login(
            Extension(state.clone()),
            HeaderMap::new(),
            Json(CourierLoginRequest {
                email: "rider@x.com".to_string(),
                password: "delivery-pw".to_string(),
                location_id: None,
            }),
        )
        .await;
        let (status, json) = json_body(resp).await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            json["refreshToken"].as_str().unwrap().contains('.'),
            "sessionId.tokenPlain"
        );
        // JWT carries the jti (session-bound).
        let jwt = json["jwt"].as_str().unwrap();
        let claims = state.verifier.verify(jwt).unwrap();
        assert!(claims.as_courier().unwrap().jti.is_some());
    }

    #[tokio::test]
    async fn courier_refresh_reuse_revokes_family_401() {
        // A revoked session presented again → REFRESH_REUSED (family revoked).
        let repo = FakeAuthRepo::default();
        let session_id = Uuid::new_v4();
        let token_plain = "plainsecret";
        let token_hash = hash_password(token_plain).unwrap();
        repo.courier_sessions.lock().unwrap().insert(
            session_id,
            CourierSessionRefreshRow {
                id: session_id,
                courier_id: Uuid::new_v4(),
                family_id: Uuid::new_v4(),
                token_hash,
                active_location_id: Uuid::new_v4(),
                revoked: true, // already revoked → reuse
                expired: false,
            },
        );
        let state = AuthState::test_state(Arc::new(repo));
        let resp = courier_refresh(
            Extension(state),
            HeaderMap::new(),
            Json(CourierRefreshRequest {
                refresh_token: format!("{session_id}.{token_plain}"),
            }),
        )
        .await;
        let (status, json) = json_body(resp).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(json["code"], "REFRESH_REUSED");
    }

    #[tokio::test]
    async fn courier_logout_always_200() {
        let state = AuthState::test_state(Arc::new(FakeAuthRepo::default()));
        let resp = courier_logout(
            Extension(state),
            Json(CourierLogoutRequest {
                refresh_token: "garbage".to_string(),
            }),
        )
        .await;
        let (status, json) = json_body(resp).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json, serde_json::json!({ "success": true }));
    }
}
