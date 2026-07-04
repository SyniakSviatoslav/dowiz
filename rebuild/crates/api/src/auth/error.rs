//! Auth response shapes — the ADR-0010 envelope PLUS the four divergent non-envelope shapes the
//! S2 surface carries VERBATIM (Q4 carry; proposal §10 quirk register). The council's rule:
//! CARRY the exact wire shape today's FE branches on; migrate-to-envelope is a deferred FE-lockstep
//! pass, never a silent port fix. Each shape is a concrete response type so a route returns the
//! exact bytes the openapi contract documents.
//!
//! The four divergent shapes (each an openapi `components.responses`/`schemas` quirk entry):
//!   1. `ClaimBareError`      — `{error: CODE}` (claim surface, Q-CLAIM-BARE).
//!   2. `CourierManualZod400` — `{error:'Validation failed', details:{...}}` (Q-COURIER-ZOD).
//!   3. `ConcurrentRefresh`   — `{error:'concurrent_refresh'}` (409, Q-CONCURRENT).
//!   4. `TrackLinkExpired`    — `{error:'TRACK_LINK_EXPIRED', message}` (410, Q-TRACK-SHAPE).
//!      plus the pre-route `GlobalBearerGate401` (`{error:'Unauthorized'}`) and the telegram-poll
//!      `{status:...}` non-envelope bodies, and the dev-gate `{error:'Not found'}` 404.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

use domain::{ErrorCode, ErrorEnvelope};

/// The standard ADR-0010 envelope response (reused from `crate::error::ApiError` semantics but
/// carrying the S2 code set). Built via `domain::ErrorEnvelope` so `status`/`error` are always
/// present. This is the DEFAULT shape; the four below are the documented exceptions.
#[derive(Debug)]
pub struct AuthEnvelopeError {
    pub status: StatusCode,
    pub envelope: ErrorEnvelope,
}

impl AuthEnvelopeError {
    pub fn new(
        code: ErrorCode,
        message: impl Into<String>,
        correlation_id: impl Into<String>,
    ) -> Self {
        let envelope = ErrorEnvelope::new(code, message, correlation_id);
        // The envelope's own http_status is authoritative for status parity (crate::error).
        let status =
            StatusCode::from_u16(envelope.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        AuthEnvelopeError { status, envelope }
    }

    /// S2 VALIDATION_FAILED is deliberately **400**, not 422 (Q-VAL-400 / ADR-0010 code-preserving:
    /// ~10 E2E assert 400; server.ts:479 `reply.status(400).send(...'VALIDATION_FAILED'...)`). The
    /// S1-established `ErrorCode::ValidationFailed.http_status()` is 422 for the S1 surface and must
    /// stay untouched, so S2 forces 400 here (both the HTTP status AND the legacy `status` field).
    pub fn validation_failed(
        message: impl Into<String>,
        correlation_id: impl Into<String>,
    ) -> Self {
        let mut envelope = ErrorEnvelope::new(ErrorCode::ValidationFailed, message, correlation_id);
        envelope.status = 400;
        AuthEnvelopeError {
            status: StatusCode::BAD_REQUEST,
            envelope,
        }
    }
}

impl IntoResponse for AuthEnvelopeError {
    fn into_response(self) -> Response {
        (self.status, Json(self.envelope)).into_response()
    }
}

/// Quirk 1 — `ClaimBareError` (Q-CLAIM-BARE, `public/claim.ts`): bare `{error: <CODE>}` where the
/// `error` slot carries the MACHINE CODE (inverse of the envelope). No message/correlationId/status.
#[derive(Debug, Serialize, ToSchema)]
pub struct ClaimBareError {
    /// The `ClaimError` code (e.g. `VALIDATION_FAILED`, `CONTACT_MISMATCH`).
    pub error: String,
}

/// A `(status, ClaimBareError)` responder — the claim routes map each code to its status
/// (`service::claim_accept_status`) then emit this bare shape.
pub struct ClaimBareResponse {
    pub status: StatusCode,
    pub code: String,
}

impl IntoResponse for ClaimBareResponse {
    fn into_response(self) -> Response {
        (self.status, Json(ClaimBareError { error: self.code })).into_response()
    }
}

/// Quirk 2 — `CourierManualZod400` (Q-COURIER-ZOD, courier/auth.ts:44): manual-Zod 400
/// `{error:'Validation failed', details:<tree>}`. `details` is the ZodError.format() tree; the
/// Rust port carries the SHAPE (a nested paths+messages object) — never submitted values (B4).
#[derive(Debug, Serialize, ToSchema)]
pub struct CourierManualZod400 {
    /// Always the literal string `"Validation failed"`.
    pub error: String,
    /// The validation-issue tree (paths + messages, no submitted values).
    pub details: serde_json::Value,
}

pub struct CourierZodResponse {
    pub details: serde_json::Value,
}

impl IntoResponse for CourierZodResponse {
    fn into_response(self) -> Response {
        (
            StatusCode::BAD_REQUEST,
            Json(CourierManualZod400 {
                error: "Validation failed".to_string(),
                details: self.details,
            }),
        )
            .into_response()
    }
}

/// Quirk 3 — `ConcurrentRefresh` (Q-CONCURRENT, auth.ts:281): lowercase, non-envelope
/// `{error:'concurrent_refresh'}` at 409. The FE single-flight depends on this EXACT shape.
#[derive(Debug, Serialize, ToSchema)]
pub struct ConcurrentRefreshBody {
    /// Always the literal string `"concurrent_refresh"`.
    pub error: String,
}

pub struct ConcurrentRefreshResponse;

impl IntoResponse for ConcurrentRefreshResponse {
    fn into_response(self) -> Response {
        (
            StatusCode::CONFLICT,
            Json(ConcurrentRefreshBody {
                error: "concurrent_refresh".to_string(),
            }),
        )
            .into_response()
    }
}

/// Quirk 4 — `TrackLinkExpired` (Q-TRACK-SHAPE, track.ts:58-61): `{error:'TRACK_LINK_EXPIRED',
/// message}` at 410 — `error` carries the CODE (inverse of the envelope), no code/correlationId.
#[derive(Debug, Serialize, ToSchema)]
pub struct TrackLinkExpiredBody {
    /// Always the literal `"TRACK_LINK_EXPIRED"`.
    pub error: String,
    pub message: String,
}

pub struct TrackLinkExpiredResponse;

impl IntoResponse for TrackLinkExpiredResponse {
    fn into_response(self) -> Response {
        (
            StatusCode::GONE,
            Json(TrackLinkExpiredBody {
                error: "TRACK_LINK_EXPIRED".to_string(),
                message: "This tracking link is no longer valid. Please reopen the menu."
                    .to_string(),
            }),
        )
            .into_response()
    }
}

/// The pre-route `GlobalBearerGate401` (Q-BEARERGATE, server.ts:417-426): bare `{error:'Unauthorized'}`.
#[derive(Debug, Serialize, ToSchema)]
pub struct GlobalBearerGate401 {
    /// Always the literal `"Unauthorized"`.
    pub error: String,
}

/// The dev-gate closed 404 (Q-DEV-404, server.ts:414): bare `{error:'Not found'}` — existence-hiding.
#[derive(Debug, Serialize, ToSchema)]
pub struct DevGate404 {
    /// Always the literal `"Not found"`.
    pub error: String,
}

/// The mock-auth synthetic-missing 409 (Q-DEV-409, mock-auth.ts:38-41): `{error:<prose>,
/// code:'SYNTHETIC_COURIER_MISSING'}` — `error` carries prose, `code` the machine token (swapped
/// vs the envelope).
#[derive(Debug, Serialize, ToSchema)]
pub struct SyntheticCourierMissing {
    pub error: String,
    /// Always the literal `"SYNTHETIC_COURIER_MISSING"`.
    pub code: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    async fn body_json(resp: Response) -> (StatusCode, serde_json::Value) {
        let status = resp.status();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn claim_bare_error_is_bare_error_code() {
        let (status, json) = body_json(
            ClaimBareResponse {
                status: StatusCode::CONFLICT,
                code: "ALREADY_CLAIMED".to_string(),
            }
            .into_response(),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(json, serde_json::json!({ "error": "ALREADY_CLAIMED" }));
    }

    #[tokio::test]
    async fn concurrent_refresh_is_lowercase_non_envelope_409() {
        // Q-CONCURRENT: FE single-flight branches on this EXACT body.
        let (status, json) = body_json(ConcurrentRefreshResponse.into_response()).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(json, serde_json::json!({ "error": "concurrent_refresh" }));
        assert!(json.get("code").is_none(), "no envelope code");
        assert!(json.get("correlationId").is_none());
    }

    #[tokio::test]
    async fn track_link_expired_carries_code_in_error_slot_at_410() {
        let (status, json) = body_json(TrackLinkExpiredResponse.into_response()).await;
        assert_eq!(status, StatusCode::GONE);
        assert_eq!(json["error"], "TRACK_LINK_EXPIRED");
        assert!(json.get("message").is_some());
        assert!(json.get("code").is_none(), "inverse of the envelope");
    }

    #[tokio::test]
    async fn courier_zod_400_carries_prose_error_and_details_tree() {
        let (status, json) = body_json(
            CourierZodResponse {
                details: serde_json::json!({ "email": { "_errors": ["Invalid email"] } }),
            }
            .into_response(),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(json["error"], "Validation failed");
        assert!(json.get("details").is_some());
    }

    #[test]
    fn envelope_error_uses_domain_status() {
        let e = AuthEnvelopeError::new(ErrorCode::Unauthorized, "Invalid refresh token", "corr-1");
        assert_eq!(e.status, StatusCode::UNAUTHORIZED);
        assert_eq!(e.envelope.code, ErrorCode::Unauthorized);
    }
}
