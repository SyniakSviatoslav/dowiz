//! axum `FromRequestParts` extractors — the type-state auth family (proposal §3). A handler that
//! binds `OwnerClaimsExt` is *unconstructible* from a courier token; a handler that binds
//! `CourierSession` has ALREADY performed the REV-1 live `has_location` bind. This is the Rust
//! answer to "identity-split × RLS-reliance" — the check is a TYPE, not a convention a handler can
//! forget.
//!
//! Extractors in order of narrowing:
//!   - `VerifiedClaims`    — RS256-verified + kid-selected + strict-parsed (any role).
//!   - `OwnerClaimsExt`    — role-narrowed to owner (Q-LOGOUT made structural).
//!   - `CustomerClaimsExt` — role-narrowed to customer (REV-3 scope helpers live on it).
//!   - `CourierSession`    — role-narrowed to courier AND live-session-bound (REV-1).
//!
//! All read `AuthState` (the JWT verifier + repo + config) from the request extensions, placed
//! there by the router's `.with_state`/layer wiring (see `auth::mod`).

use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};

use super::claims::{Claims, CourierClaims, CustomerClaims, OwnerClaims};
use super::mount::AuthState;
use super::service::{self, CourierBind, CourierSessionRow};

/// The bearer-token rejection shape used by the EXTRACTORS (not the pre-route gate): mirrors
/// `plugins/auth.ts:47,55` `{error:'Missing or invalid token'}` / `{error:'Token expired or
/// invalid'}` — a bare non-envelope 401 the FE tolerates. Distinct from `GlobalBearerGate401`
/// (`{error:'Unauthorized'}`), which is the PRE-route layer (Q-BEARERGATE) — see `middleware.rs`.
pub struct AuthRejection {
    status: StatusCode,
    message: &'static str,
}

impl AuthRejection {
    fn unauthorized(message: &'static str) -> Self {
        AuthRejection {
            status: StatusCode::UNAUTHORIZED,
            message,
        }
    }
    fn forbidden(message: &'static str) -> Self {
        AuthRejection {
            status: StatusCode::FORBIDDEN,
            message,
        }
    }
}

impl IntoResponse for AuthRejection {
    fn into_response(self) -> Response {
        (
            self.status,
            axum::Json(serde_json::json!({ "error": self.message })),
        )
            .into_response()
    }
}

/// Pull the `AuthState` from request extensions. It is a hard wiring invariant that the router
/// inserts it; a missing state is a 500 (a server bug, not a client error).
fn state_from(parts: &Parts) -> Result<AuthState, AuthRejection> {
    parts
        .extensions
        .get::<AuthState>()
        .cloned()
        .ok_or(AuthRejection {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: "auth state not wired",
        })
}

/// Extracts the raw bearer, verifies it (RS256 double-pin + kid select + strict parse + body-kid),
/// and yields the role-tagged `Claims`. The base of every narrower extractor.
#[derive(Debug, Clone)]
pub struct VerifiedClaims(pub Claims);

impl<S> FromRequestParts<S> for VerifiedClaims
where
    S: Send + Sync,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let state = state_from(parts)?;
        let token = bearer_token(parts)
            .ok_or_else(|| AuthRejection::unauthorized("Missing or invalid token"))?;
        let claims = state
            .verifier
            .verify(&token)
            .map_err(|_e| AuthRejection::unauthorized("Token expired or invalid"))?;
        Ok(VerifiedClaims(claims))
    }
}

/// Owner-narrowed claims (`Claims<Owner>` from proposal §3). A courier/customer token → 401.
#[derive(Debug, Clone)]
pub struct OwnerClaimsExt(pub OwnerClaims);

impl<S> FromRequestParts<S> for OwnerClaimsExt
where
    S: Send + Sync,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let VerifiedClaims(claims) = VerifiedClaims::from_request_parts(parts, state).await?;
        match claims {
            Claims::Owner(o) => Ok(OwnerClaimsExt(o)),
            // Q-LOGOUT made structural: a non-owner bearer cannot reach an owner-bound handler.
            _ => Err(AuthRejection::unauthorized("Authentication required")),
        }
    }
}

/// Customer-narrowed claims (`Claims<Customer>`). REV-3 scope checks are methods on this type so
/// a customer-scoped handler MUST call one — a `token(orderA)` can't authorize `orderB`.
#[derive(Debug, Clone)]
pub struct CustomerClaimsExt(pub CustomerClaims);

impl CustomerClaimsExt {
    /// REV-3/T-12: authorize this token for a specific order, else 403.
    pub fn require_order(&self, order_id: uuid::Uuid) -> Result<(), AuthRejection> {
        if service::customer_authorized_for_order(&self.0, order_id) {
            Ok(())
        } else {
            Err(AuthRejection::forbidden("Forbidden"))
        }
    }
    /// REV-3: authorize this token for a specific location, else 403.
    pub fn require_location(&self, location_id: uuid::Uuid) -> Result<(), AuthRejection> {
        if service::customer_authorized_for_location(&self.0, location_id) {
            Ok(())
        } else {
            Err(AuthRejection::forbidden("Forbidden"))
        }
    }
}

impl<S> FromRequestParts<S> for CustomerClaimsExt
where
    S: Send + Sync,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let VerifiedClaims(claims) = VerifiedClaims::from_request_parts(parts, state).await?;
        match claims {
            Claims::Customer(c) => Ok(CustomerClaimsExt(c)),
            _ => Err(AuthRejection::unauthorized("Authentication required")),
        }
    }
}

/// Courier claims that have PASSED the REV-1 live session bind — present, not revoked, not
/// expired, AND the courier still holds membership in the token's `activeLocationId`. This is the
/// `CourierSession` type from proposal §3: constructing it IS the per-request revocation check.
#[derive(Debug, Clone)]
pub struct CourierSession(pub CourierClaims);

impl<S> FromRequestParts<S> for CourierSession
where
    S: Send + Sync,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = state_from(parts)?;
        let VerifiedClaims(claims) = VerifiedClaims::from_request_parts(parts, state).await?;
        let courier = match claims {
            Claims::Courier(c) => c,
            _ => return Err(AuthRejection::unauthorized("Authentication required")),
        };

        // REV-1: a courier token WITH a jti must map to a live session row whose courier still
        // holds the token's location. A token WITHOUT a jti can only come from the dev-mock
        // minter — allowed ONLY when the dev gate is open (plugins/auth.ts:63-70).
        match courier.jti {
            None => {
                if app_state.config.dev_login_allowed() {
                    Ok(CourierSession(courier))
                } else {
                    Err(AuthRejection::unauthorized("Token expired or invalid"))
                }
            }
            Some(jti) => {
                let bind = app_state
                    .repo
                    .courier_session_bind(jti, courier.active_location_id, courier.sub)
                    .await
                    .map_err(|_e| AuthRejection {
                        status: StatusCode::INTERNAL_SERVER_ERROR,
                        message: "Internal server error",
                    })?;
                let row = bind.map(|b| CourierSessionRow {
                    revoked_at: b.revoked,
                    expired: b.expired,
                    has_location: b.has_location,
                });
                match service::courier_session_valid(row.as_ref(), true) {
                    CourierBind::Valid => Ok(CourierSession(courier)),
                    // REV-1: removed-from-location / revoked / expired / missing → 401.
                    CourierBind::Rejected | CourierBind::NoJti => Err(AuthRejection::unauthorized(
                        "Session revoked or access removed",
                    )),
                }
            }
        }
    }
}

/// Extract the `Authorization: Bearer <token>` value (`plugins/auth.ts:46,50`).
fn bearer_token(parts: &Parts) -> Option<String> {
    let header = parts.headers.get(axum::http::header::AUTHORIZATION)?;
    let value = header.to_str().ok()?;
    let rest = value.strip_prefix("Bearer ")?;
    if rest.is_empty() {
        None
    } else {
        Some(rest.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::claims::{CourierClaims, OwnerClaims};
    use crate::auth::mount::AuthState;
    use crate::auth::repo::{CourierSessionBindRow, fake::FakeAuthRepo};
    use axum::http::Request;
    use std::sync::Arc;
    use uuid::Uuid;

    fn parts_with(state: AuthState, bearer: Option<&str>) -> Parts {
        let mut builder = Request::builder().uri("/x");
        if let Some(b) = bearer {
            builder = builder.header("authorization", format!("Bearer {b}"));
        }
        let req = builder.body(()).unwrap();
        let (mut parts, ()) = req.into_parts();
        parts.extensions.insert(state);
        parts
    }

    #[tokio::test]
    async fn verified_claims_rejects_missing_bearer() {
        let state = AuthState::test_state(Arc::new(FakeAuthRepo::default()));
        let mut parts = parts_with(state, None);
        let out = VerifiedClaims::from_request_parts(&mut parts, &()).await;
        assert!(matches!(out, Err(r) if r.status == StatusCode::UNAUTHORIZED));
    }

    #[tokio::test]
    async fn owner_extractor_rejects_a_courier_token() {
        // Q-LOGOUT structural: a courier token cannot reach an owner-bound handler.
        let repo = Arc::new(FakeAuthRepo::default());
        let state = AuthState::test_state(repo);
        let courier = Claims::Courier(CourierClaims::new(Uuid::new_v4(), Uuid::new_v4(), None));
        let token = state.verifier.mint(courier, 3600).unwrap();
        let mut parts = parts_with(state, Some(&token));
        let out = OwnerClaimsExt::from_request_parts(&mut parts, &()).await;
        assert!(out.is_err(), "courier token must not narrow to owner");
    }

    #[tokio::test]
    async fn owner_extractor_accepts_an_owner_token() {
        let repo = Arc::new(FakeAuthRepo::default());
        let state = AuthState::test_state(repo);
        let owner = Claims::Owner(OwnerClaims::new(Uuid::new_v4(), None));
        let token = state.verifier.mint(owner, 3600).unwrap();
        let mut parts = parts_with(state, Some(&token));
        assert!(
            OwnerClaimsExt::from_request_parts(&mut parts, &())
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn courier_session_rejects_after_location_removed() {
        // REV-1 PROOF: a courier removed from their location gets 401 on the NEXT request even
        // though the JWT is still cryptographically valid and unexpired.
        let jti = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let courier_id = Uuid::new_v4();
        let repo = Arc::new(FakeAuthRepo::default().with_courier_bind(
            jti,
            loc,
            courier_id,
            CourierSessionBindRow {
                revoked: false,
                expired: false,
                has_location: false, // removed from the location
            },
        ));
        let state = AuthState::test_state(repo);
        let claims = Claims::Courier(CourierClaims::new(courier_id, loc, Some(jti)));
        let token = state.verifier.mint(claims, 14 * 24 * 3600).unwrap();
        let mut parts = parts_with(state, Some(&token));
        let out = CourierSession::from_request_parts(&mut parts, &()).await;
        assert!(
            matches!(out, Err(r) if r.status == StatusCode::UNAUTHORIZED),
            "removed-from-location courier must 401 on the next request (REV-1)"
        );
    }

    #[tokio::test]
    async fn courier_session_accepts_live_bound_courier() {
        let jti = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let courier_id = Uuid::new_v4();
        let repo = Arc::new(FakeAuthRepo::default().with_courier_bind(
            jti,
            loc,
            courier_id,
            CourierSessionBindRow {
                revoked: false,
                expired: false,
                has_location: true,
            },
        ));
        let state = AuthState::test_state(repo);
        let claims = Claims::Courier(CourierClaims::new(courier_id, loc, Some(jti)));
        let token = state.verifier.mint(claims, 24 * 3600).unwrap();
        let mut parts = parts_with(state, Some(&token));
        assert!(
            CourierSession::from_request_parts(&mut parts, &())
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn customer_scope_require_order_403_cross_order() {
        // REV-3/T-12: an extracted customer token authorizes ONLY its minted order.
        let order_a = Uuid::from_u128(0xA);
        let order_b = Uuid::from_u128(0xB);
        let ext = CustomerClaimsExt(CustomerClaims::new(Uuid::new_v4(), order_a, Uuid::new_v4()));
        assert!(ext.require_order(order_a).is_ok());
        let denied = ext.require_order(order_b);
        assert!(matches!(denied, Err(r) if r.status == StatusCode::FORBIDDEN));
    }
}
