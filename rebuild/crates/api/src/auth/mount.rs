//! `AuthState` + the S2 auth router assembly. `AuthState` is the cloneable bundle (verifier +
//! repo + config) every extractor and handler reads; it's inserted into request extensions by a
//! layer so `FromRequestParts` can pull it. `auth_router` wires the 19 live S2 operations onto an
//! axum `Router`, applies the REV-4 middleware tower in the load-bearing order, and — per the
//! council RETIRE decision — does NOT register `/api/auth/courier/activate` (it 404s as absent).

use std::sync::Arc;

use axum::Router;
use axum::routing::{get, post};

use super::config::AuthConfig;
use super::jwt::JwtVerifier;
use super::repo::AuthRepo;
use super::store::{EphemeralStore, GoogleOAuthClient};

/// The cloneable auth context. `Arc` everywhere so cloning into request extensions is cheap.
#[derive(Clone)]
pub struct AuthState {
    pub verifier: Arc<JwtVerifier>,
    pub repo: Arc<dyn AuthRepo>,
    pub config: Arc<AuthConfig>,
    /// Ephemeral KV for OAuth state + the one-time handoff code (Redis/Pg per A19 in prod).
    pub store: Arc<dyn EphemeralStore>,
    /// Google token-exchange seam (real HTTPS client in prod; NullGoogleClient when OAuth is dark).
    pub google: Arc<dyn GoogleOAuthClient>,
    /// Courier PII-at-rest cipher (asset A7). `None` when `COURIER_PII_ENCRYPTION_KEY` is unset —
    /// courier redeem then returns a typed 500 rather than writing plaintext PII.
    pub pii_cipher: Option<Arc<super::pii::PiiCipher>>,
}

impl AuthState {
    pub fn new(
        verifier: Arc<JwtVerifier>,
        repo: Arc<dyn AuthRepo>,
        config: Arc<AuthConfig>,
        store: Arc<dyn EphemeralStore>,
        google: Arc<dyn GoogleOAuthClient>,
        pii_cipher: Option<Arc<super::pii::PiiCipher>>,
    ) -> Self {
        AuthState {
            verifier,
            repo,
            config,
            store,
            google,
            pii_cipher,
        }
    }

    /// Test constructor: a verifier built from the checked-in throwaway test keypair + a supplied
    /// (fake) repo + a permissive dev-enabled config. Used by the extractor/route unit tests so
    /// they can mint + verify real RS256 tokens without env or a DB.
    #[cfg(test)]
    pub fn test_state(repo: Arc<dyn AuthRepo>) -> Self {
        use super::config::NodeEnv;
        let config = AuthConfig {
            node_env: NodeEnv::Test,
            jwt_kid: "prod-kid-1".to_string(),
            jwt_private_key_pem: crate::test_support::keys::prod_private().to_string(),
            jwt_public_key_pem: crate::test_support::keys::prod_public().to_string(),
            google_oauth_enabled: false,
            app_base_url: "https://dowiz.fly.dev".to_string(),
            #[cfg(feature = "dev-routes")]
            dev: None,
            allow_dev_login: true,
            dev_auth_secret: Some("test-secret".to_string()),
            telegram_bot_username: "dowiz_bot".to_string(),
        };
        let verifier = Arc::new(JwtVerifier::from_config(&config).unwrap());
        AuthState {
            verifier,
            repo,
            config: Arc::new(config),
            store: Arc::new(super::store::InMemoryStore::default()),
            google: Arc::new(super::store::NullGoogleClient),
            pii_cipher: Some(Arc::new(
                super::pii::PiiCipher::from_base64(&{
                    use base64::Engine;
                    base64::engine::general_purpose::STANDARD.encode([7u8; 32])
                })
                .unwrap(),
            )),
        }
    }
}

/// Assemble the S2 auth surface. The routes are mounted at the SAME paths the Node API serves
/// (server.ts registers `authRoutes` under `/api`, courier-auth under `/api/courier/auth`, claim
/// under `/api`, track under `/api/customer/track`, dev at `/dev` + `/api/dev`).
///
/// ## RETIRE `/api/auth/courier/activate` (council Q2, UNANIMOUS)
/// The dead courier-activate flow is DELIBERATELY NOT registered here. A request to it falls
/// through to axum's default 404 — matching the "documented 404/absent" disposition. Registering
/// it would carry the live-reachable privilege-escalation (courier refresh in the owner table) +
/// the unverifiable-CourierClaims token into Rust; the council ruled carry-verbatim is strictly
/// more dangerous than deletion here. See `resolution.md` convergence item 1.
pub fn auth_router(state: AuthState) -> Router {
    let router = Router::new()
        // ── AUTH-01 owner local login ──
        .route(
            "/api/auth/local/login",
            post(crate::routes::auth_owner::owner_local_login),
        )
        // ── AUTH-02 Google OAuth ──
        .route(
            "/api/auth/google",
            get(crate::routes::auth_owner::google_oauth_start),
        )
        .route(
            "/api/auth/google/callback",
            get(crate::routes::auth_owner::google_oauth_callback),
        )
        .route(
            "/api/auth/exchange",
            post(crate::routes::auth_owner::exchange_oauth_code),
        )
        // ── AUTH-03 Telegram ──
        .route(
            "/api/auth/telegram/start",
            post(crate::routes::auth_owner::telegram_login_start),
        )
        .route(
            "/api/auth/telegram/poll",
            get(crate::routes::auth_owner::telegram_login_poll),
        )
        // ── AUTH-09 owner refresh + logout ──
        .route(
            "/api/auth/refresh",
            post(crate::routes::auth_owner::owner_refresh),
        )
        .route(
            "/api/auth/logout",
            post(crate::routes::auth_owner::owner_logout),
        )
        // ── AUTH-GAP-2 courier-activate: RETIRED (unregistered → 404). See fn doc. ──
        // ── AUTH-05 courier invite ──
        .route(
            "/api/courier/auth/invites/{inviteId}",
            get(crate::routes::auth_courier::get_courier_invite),
        )
        .route(
            "/api/courier/auth/invites/{inviteId}/redeem",
            post(crate::routes::auth_courier::courier_redeem_invite),
        )
        // ── AUTH-06 courier login/refresh/logout ──
        .route(
            "/api/courier/auth/login",
            post(crate::routes::auth_courier::courier_login),
        )
        .route(
            "/api/courier/auth/refresh",
            post(crate::routes::auth_courier::courier_refresh),
        )
        .route(
            "/api/courier/auth/logout",
            post(crate::routes::auth_courier::courier_logout),
        )
        // ── AUTH-07 claim (web side) ──
        .route(
            "/api/claim/accept",
            post(crate::routes::auth_claim::claim_accept),
        )
        .route(
            "/api/claim/request",
            post(crate::routes::auth_claim::claim_request),
        )
        .route(
            "/api/claim/decline",
            post(crate::routes::auth_claim::claim_decline),
        )
        // ── AUTH-10 customer track exchange ──
        .route(
            "/api/customer/track/exchange",
            post(crate::routes::auth_customer::customer_track_exchange),
        );

    // ── AUTH-08 dev mock-auth (dev-routes builds only) — TWO separate handlers (Q11/REV-5) ──
    // The council did NOT ratify the Q11 collapse: /dev/mock-auth and /api/dev/mock-auth diverge in
    // 4 concrete ways (fresh mode, locationSlug, owner auto-membership, courier default location).
    // Ported as TWO handlers, not one registered at two paths. Only compiled in a dev-routes build.
    #[cfg(feature = "dev-routes")]
    let router = router
        .route(
            "/dev/mock-auth",
            post(crate::routes::auth_dev::dev_mock_auth),
        )
        .route(
            "/api/dev/mock-auth",
            post(crate::routes::auth_dev::api_dev_mock_auth),
        );

    router
        // REV-4 middleware tower — tower applies layers OUTSIDE-IN, so the LAST `.layer` runs
        // FIRST. Order (outer→inner): [rate-limit would sit here, ahead of everything → 429
        // precedes 401] → bearer/dev pre-gate → extractor (per-route). The bearer/dev gate is the
        // outermost auth layer so its bare 401/404 fire before any handler/extractor.
        .layer(axum::middleware::from_fn(
            super::middleware::bearer_and_dev_gate,
        ))
        // Insert AuthState into every request's extensions so the extractors can read it.
        .layer(axum::Extension(state))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::repo::fake::FakeAuthRepo;

    #[test]
    fn auth_router_builds_without_panicking() {
        // Router::route panics at construction on an invalid path pattern; this proves all S2
        // paths register cleanly (parity with S1's build_router panic-freedom test).
        let state = AuthState::test_state(Arc::new(FakeAuthRepo::default()));
        let _router = auth_router(state);
    }

    #[tokio::test]
    async fn retired_courier_activate_is_absent_404() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let state = AuthState::test_state(Arc::new(FakeAuthRepo::default()));
        let app = auth_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/courier/activate")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        // RETIRE (Q2): the route is not registered → axum default 404. Proof-of-absence.
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
