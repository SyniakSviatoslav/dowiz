//! AUTH-10 customer track exchange: trade the opaque `?t=` grant code for a 7d customer JWT.
//! Ports `apps/api/src/routes/customer/track.ts`. Pre-auth (NO_AUTH_PATHS). The minted token
//! carries NO phone/PII (P0-PII, T-8) and is scoped to `(orderId, locationId, sub)` (REV-3).

use axum::Json;
use axum::extract::Extension;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use domain::ErrorCode;

use crate::auth::AuthState;
use crate::auth::claims::{Claims, CustomerClaims};
use crate::auth::crypto;
use crate::auth::dto::{TrackExchangeRequest, TrackExchangeResponse};
use crate::auth::error::{AuthEnvelopeError, TrackLinkExpiredResponse};
use crate::auth::jwt::ttl;

/// `POST /api/customer/track/exchange` (AUTH-10). Grant lookup by sha256(code); reusable to expiry
/// (Q-TRACK-REUSE). Mints `issueCustomerToken` — role:customer, orderId/locationId/sub, NO phone.
#[utoipa::path(post, path = "/api/customer/track/exchange", tag = "auth",
    request_body = TrackExchangeRequest,
    responses(
        (status = 200, body = TrackExchangeResponse),
        (status = 400, description = "VALIDATION_FAILED"),
        (status = 410, description = "TRACK_LINK_EXPIRED (non-envelope)"),
        (status = 500, description = "token-issue failure"),
    ))]
pub async fn customer_track_exchange(
    Extension(state): Extension<AuthState>,
    headers: HeaderMap,
    Json(req): Json<TrackExchangeRequest>,
) -> Response {
    // .strict() code validation (track.ts:18-23): 20..64 chars (base64url(32 bytes)). Q-VAL-400: 400.
    if req.code.len() < 20 || req.code.len() > 64 {
        return AuthEnvelopeError::validation_failed("Invalid request", corr(&headers))
            .into_response();
    }
    // Never log the raw code — hash and look up the grant (track.ts:41).
    let token_hash = crypto::sha256_hex(&req.code);

    let grant = match state.repo.track_grant_by_hash(&token_hash).await {
        Ok(Some(g)) => g,
        // Unknown/expired/order-gone — SINGLE uniform 410, non-envelope (Q-TRACK-SHAPE, no case leak).
        Ok(None) => return TrackLinkExpiredResponse.into_response(),
        Err(_) => {
            return AuthEnvelopeError::new(
                ErrorCode::Internal,
                "Internal server error",
                corr(&headers),
            )
            .into_response();
        }
    };
    let (grant_id, order_id, location_id, customer_id) = grant;

    // Reusable until expiry — use_count is observability, not a single-use gate (Q-TRACK-REUSE).
    let _ignored = state.repo.bump_track_use_count(grant_id).await; // best-effort observability bump

    // issueCustomerToken: 7d, {role:customer, orderId, locationId, sub=customerId}, NO phone (T-8).
    let claims = Claims::Customer(CustomerClaims::new(customer_id, order_id, location_id));
    match state.verifier.mint(claims, ttl::CUSTOMER_ACCESS) {
        Ok(token) => Json(TrackExchangeResponse { token }).into_response(),
        Err(_) => {
            AuthEnvelopeError::new(ErrorCode::Internal, "Internal server error", corr(&headers))
                .into_response()
        }
    }
}

fn corr(headers: &HeaderMap) -> String {
    headers
        .get("x-correlation-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
        .unwrap_or_else(|| crypto::random_uuid().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::repo::fake::FakeAuthRepo;
    use axum::body::to_bytes;
    use axum::http::StatusCode;
    use std::sync::Arc;
    use uuid::Uuid;

    async fn json_body(resp: Response) -> (StatusCode, serde_json::Value) {
        let status = resp.status();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn track_exchange_mints_a_customer_token_without_phone() {
        // T-8 + REV-3: the minted token has NO phone claim and is scoped to its (order, location, sub).
        let repo = FakeAuthRepo::default();
        let code = "a".repeat(32);
        let order = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let cust = Uuid::new_v4();
        repo.track_grants.lock().unwrap().insert(
            crypto::sha256_hex(&code),
            (Uuid::new_v4(), order, loc, cust),
        );
        let state = AuthState::test_state(Arc::new(repo));
        let resp = customer_track_exchange(
            Extension(state.clone()),
            HeaderMap::new(),
            Json(TrackExchangeRequest { code }),
        )
        .await;
        let (status, json) = json_body(resp).await;
        assert_eq!(status, StatusCode::OK);
        let token = json["token"].as_str().unwrap();
        let claims = state.verifier.verify(token).unwrap();
        let customer = claims.as_customer().unwrap();
        assert_eq!(customer.order_id, order);
        assert_eq!(customer.location_id, loc);
        assert_eq!(customer.sub, cust);
        // P0-PII: no phone anywhere in the serialized claims.
        let serialized = serde_json::to_value(&claims).unwrap();
        assert!(serialized.get("phone").is_none());
    }

    #[tokio::test]
    async fn track_exchange_unknown_code_is_uniform_410_non_envelope() {
        let state = AuthState::test_state(Arc::new(FakeAuthRepo::default()));
        let resp = customer_track_exchange(
            Extension(state),
            HeaderMap::new(),
            Json(TrackExchangeRequest {
                code: "z".repeat(32),
            }),
        )
        .await;
        let (status, json) = json_body(resp).await;
        assert_eq!(status, StatusCode::GONE);
        assert_eq!(json["error"], "TRACK_LINK_EXPIRED");
        assert!(json.get("code").is_none(), "non-envelope (Q-TRACK-SHAPE)");
    }

    #[tokio::test]
    async fn track_exchange_short_code_is_400() {
        let state = AuthState::test_state(Arc::new(FakeAuthRepo::default()));
        let resp = customer_track_exchange(
            Extension(state),
            HeaderMap::new(),
            Json(TrackExchangeRequest {
                code: "short".to_string(),
            }),
        )
        .await;
        // Q-VAL-400: validation is 400, not 422.
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
