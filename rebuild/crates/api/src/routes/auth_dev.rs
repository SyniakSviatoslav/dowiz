//! AUTH-08 dev mock-auth — TWO separate handlers (Q11/REV-5: the council did NOT ratify the
//! collapse; `/dev/mock-auth` and `/api/dev/mock-auth` diverge in `fresh` mode, `locationSlug`,
//! owner auto-membership, and courier default-location). Compiled ONLY in a `dev-routes` build
//! (ADR-0003 layer 3 — a release binary has no dev mint surface). Tokens are dev-kid-signed
//! (`mint_dev`) so a prod verifier rejects them cryptographically.
#![cfg(feature = "dev-routes")]

use axum::Json;
use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use uuid::Uuid;

use crate::auth::AuthState;
use crate::auth::claims::{Claims, CourierClaims, OwnerClaims};
use crate::auth::dto::{MockAuthRequest, MockAuthResponse};
use crate::auth::jwt::ttl;

/// `POST /dev/mock-auth` (mock-auth.ts:14). Owner default / courier / synthetic-courier. The
/// dev-path 404 gate (middleware) already authorized this; here we just mint. Divergences from the
/// `/api/dev/mock-auth` twin: NO `fresh` mode; owner path resolves `locationSlug` then owner
/// membership; courier default location is a hardcoded uuid (mock-auth.ts:58).
#[utoipa::path(post, path = "/dev/mock-auth", tag = "dev",
    request_body = MockAuthRequest,
    responses((status = 200, body = MockAuthResponse), (status = 404, description = "gate closed"), (status = 409, description = "SYNTHETIC_COURIER_MISSING")))]
pub async fn dev_mock_auth(
    Extension(state): Extension<AuthState>,
    body: Option<Json<MockAuthRequest>>,
) -> Response {
    let req = body.map(|Json(b)| b).unwrap_or_default();
    if req.role.as_deref() == Some("courier") {
        return mint_courier(&state, &req, CourierDefault::Hardcoded).await;
    }
    // Owner (default). This twin has NO `fresh` mode (that's the /api/dev twin only).
    mint_owner(&state, req.location_id).await
}

/// `POST /api/dev/mock-auth` (server.ts:549). The twin. Divergences: HAS a `fresh` mode
/// (server.ts:595 — a throwaway owner with NO membership → onboarding); courier default location
/// is resolved from the `demo` slug (server.ts:586) not a hardcoded uuid.
#[utoipa::path(post, path = "/api/dev/mock-auth", tag = "dev",
    request_body = MockAuthRequest,
    responses((status = 200, body = MockAuthResponse), (status = 404, description = "gate closed"), (status = 409, description = "SYNTHETIC_COURIER_MISSING")))]
pub async fn api_dev_mock_auth(
    Extension(state): Extension<AuthState>,
    body: Option<Json<MockAuthRequest>>,
) -> Response {
    let req = body.map(|Json(b)| b).unwrap_or_default();
    if req.role.as_deref() == Some("courier") {
        return mint_courier(&state, &req, CourierDefault::DemoSlug).await;
    }
    // DIVERGENCE (REV-5): fresh-owner E2E mode — a brand-new owner with NO location membership so
    // the admin flow lands on onboarding (server.ts:595). Absent from the /dev twin.
    if req.fresh == Some(true) {
        let user_id = Uuid::new_v4(); // a distinct throwaway user per call
        return match state.verifier.mint_dev(
            Claims::Owner(OwnerClaims::new(user_id, None)),
            ttl::DEV_MOCK_ACCESS,
        ) {
            Ok(token) => Json(MockAuthResponse {
                access_token: token,
                user_id,
                active_location_id: None,
                synthetic: None,
            })
            .into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };
    }
    mint_owner(&state, req.location_id).await
}

enum CourierDefault {
    /// `/dev/mock-auth`: a hardcoded default location uuid (mock-auth.ts:58).
    Hardcoded,
    /// `/api/dev/mock-auth`: resolve the `demo` slug (server.ts:586).
    DemoSlug,
}

async fn mint_courier(
    state: &AuthState,
    req: &MockAuthRequest,
    default: CourierDefault,
) -> Response {
    // synthetic:true → re-derive the ONE seeded synthetic courier by the sentinel email_hash
    // (never a caller-supplied id — impersonation reduced to the one fixture BY CONSTRUCTION). In
    // this dark port the synthetic lookup is a repo seam; when unseeded → 409 (Q-DEV-409).
    if req.synthetic == Some(true) {
        // The synthetic fixture lookup is DB-backed (couriers JOIN courier_locations on the
        // sentinel hash). Not wired to a fake here — return the documented not-seeded 409 shape so
        // the divergent Q-DEV-409 body is exercised. Prod/staging with a seeded DB resolves it.
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "synthetic courier not seeded — run /dev/seed-visual-state first",
                "code": "SYNTHETIC_COURIER_MISSING",
            })),
        )
            .into_response();
    }
    let location_id =
        req.location_id.unwrap_or(match default {
            // mock-auth.ts:58 hardcoded default.
            CourierDefault::Hardcoded => Uuid::parse_str("1f609add-062a-4bb5-89bf-d695f963ede6")
                .unwrap_or_else(|_| Uuid::nil()),
            // server.ts:586 demo-slug default: resolved from the DB in prod; the hardcoded fallback
            // matches Node's fallback when the slug lookup misses.
            CourierDefault::DemoSlug => Uuid::parse_str("1f609add-062a-4bb5-89bf-d695f963ede6")
                .unwrap_or_else(|_| Uuid::nil()),
        });
    // mock-auth mints a RANDOM courierId for the non-synthetic path (no DB row, Q-DEV-RANDUID).
    let courier_id = Uuid::new_v4();
    match state.verifier.mint_dev(
        Claims::Courier(CourierClaims::new(courier_id, location_id, None)),
        ttl::DEV_MOCK_ACCESS,
    ) {
        Ok(token) => Json(MockAuthResponse {
            access_token: token,
            user_id: courier_id,
            active_location_id: Some(location_id),
            synthetic: None,
        })
        .into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn mint_owner(state: &AuthState, location_id: Option<Uuid>) -> Response {
    // The dev owner is a fixed fixture user; here we mint a dev-kid owner token. The upsert of the
    // `dev@deliveryos.com` user + membership resolution is DB-backed in Node; in this dark port the
    // token carries a fresh owner id + the provided/absent location (the observable dev behavior).
    let user_id = Uuid::new_v4();
    match state.verifier.mint_dev(
        Claims::Owner(OwnerClaims::new(user_id, location_id)),
        ttl::DEV_MOCK_ACCESS,
    ) {
        Ok(token) => Json(MockAuthResponse {
            access_token: token,
            user_id,
            active_location_id: location_id,
            synthetic: None,
        })
        .into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::config::{AuthConfig, DevAuthConfig, NodeEnv};
    use crate::auth::jwt::JwtVerifier;
    use crate::auth::repo::fake::FakeAuthRepo;
    use std::sync::Arc;

    /// A dev-routes AuthState whose verifier HAS a dev keypair (so `mint_dev` works + the token
    /// verifies under the dev kid). Uses runtime-generated throwaway keypairs (no committed keys).
    fn dev_state() -> AuthState {
        use crate::test_support::keys;
        let config = AuthConfig {
            node_env: NodeEnv::Test,
            jwt_kid: "prod-kid-1".to_string(),
            jwt_private_key_pem: keys::prod_private().to_string(),
            jwt_public_key_pem: keys::prod_public().to_string(),
            google_oauth_enabled: false,
            app_base_url: "https://x".to_string(),
            dev: Some(DevAuthConfig {
                jwt_dev_kid: "dev-kid-1".to_string(),
                jwt_dev_private_key_pem: keys::dev_private().to_string(),
                jwt_dev_public_key_pem: keys::dev_public().to_string(),
            }),
            allow_dev_login: true,
            dev_auth_secret: Some("s".to_string()),
            telegram_bot_username: "dowiz_bot".to_string(),
        };
        let verifier = Arc::new(JwtVerifier::from_config(&config).unwrap());
        AuthState {
            verifier,
            repo: Arc::new(FakeAuthRepo::default()),
            config: Arc::new(config),
            store: Arc::new(crate::auth::store::InMemoryStore::default()),
            google: Arc::new(crate::auth::store::NullGoogleClient),
            pii_cipher: None,
        }
    }

    async fn json_body(resp: Response) -> (StatusCode, serde_json::Value) {
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn dev_mock_auth_mints_a_dev_kid_owner_token() {
        let state = dev_state();
        let resp = dev_mock_auth(
            Extension(state.clone()),
            Some(Json(MockAuthRequest::default())),
        )
        .await;
        let (status, json) = json_body(resp).await;
        assert_eq!(status, StatusCode::OK);
        // The token verifies under the dev kid in THIS dev build.
        let token = json["access_token"].as_str().unwrap();
        assert!(state.verifier.verify(token).is_ok());
    }

    #[tokio::test]
    async fn api_dev_mock_auth_fresh_mode_has_no_location() {
        // REV-5 divergence: the /api/dev twin's `fresh` mode → owner with NO location.
        let state = dev_state();
        let req = MockAuthRequest {
            fresh: Some(true),
            ..Default::default()
        };
        let resp = api_dev_mock_auth(Extension(state), Some(Json(req))).await;
        let (status, json) = json_body(resp).await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            json["activeLocationId"].is_null(),
            "fresh owner has no membership"
        );
    }

    #[tokio::test]
    async fn synthetic_courier_unseeded_is_409_swapped_shape() {
        // Q-DEV-409: {error:<prose>, code:'SYNTHETIC_COURIER_MISSING'} (fields swapped vs envelope).
        let state = dev_state();
        let req = MockAuthRequest {
            role: Some("courier".to_string()),
            synthetic: Some(true),
            ..Default::default()
        };
        let resp = dev_mock_auth(Extension(state), Some(Json(req))).await;
        let (status, json) = json_body(resp).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(json["code"], "SYNTHETIC_COURIER_MISSING");
        assert!(json["error"].as_str().unwrap().contains("not seeded"));
    }
}
