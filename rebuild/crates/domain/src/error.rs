//! Error taxonomy — ports `packages/domain/src/errors.ts` (`IllegalTransitionError`,
//! `ScaffoldDisabledError`, `SameStatusError`, `ConflictError`) as `DomainError`, and models the
//! ADR-0010 wire envelope shape (`docs/adr/0010-error-contract-envelope.md`):
//! `{ code: SCREAMING_SNAKE string, message, fields?, correlationId, retryAfterMs?, status, error }`.
//!
//! This is a skeleton, not the full ~50-code A2 matrix (ADR-0010 §4) — Phase A only needs enough
//! codes to cover the domain errors that exist today plus the generic HTTP-adjacent codes the
//! `api` crate's health/menu-stub routes need. The full matrix lands with the OpenAPI contract
//! lane (REBUILD-MAP §4).
//!
//! S1 storefront-read addition (`openapi-s1-storefront-read.yaml` `ErrorEnvelope`
//! `required: [code, message, correlationId, status, error]`, `CONVENTIONS.md` "Error envelope"):
//! `status` and `error` are ALWAYS present on the wire (not Phase-A-optional as the skeleton
//! doc above once had it) — `error` is `buildErrorEnvelope`'s legacy alias
//! (`apps/api/src/lib/api-error.ts:70`, always `== message`, the un-migrated FE reads
//! `message || error`) and `status` is the numeric HTTP status the ADR keeps for
//! code-preserving rollout. Both are now derived from `ErrorCode` itself
//! (`ErrorCode::http_status`) at `ErrorEnvelope::new` time, so every envelope this crate builds
//! is contract-complete by construction — no call site can forget to set them.

use crate::order_status::OrderStatus;
use serde::{Deserialize, Serialize};

/// The order-status machine's error classes, ported 1:1 from `errors.ts`. `Conflict` mirrors
/// `ConflictError` (a plain-message class in Node with no structured payload).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    #[error("Illegal transition: {from:?} -> {to:?}")]
    IllegalTransition { from: OrderStatus, to: OrderStatus },
    #[error("Scaffold transition disabled: {from:?} -> {to:?}")]
    ScaffoldDisabled { from: OrderStatus, to: OrderStatus },
    #[error("Cannot transition to same status: {0:?}")]
    SameStatus(OrderStatus),
}

impl DomainError {
    pub const fn code(self) -> ErrorCode {
        match self {
            DomainError::IllegalTransition { .. } => ErrorCode::IllegalTransition,
            DomainError::ScaffoldDisabled { .. } => ErrorCode::ScaffoldDisabled,
            DomainError::SameStatus(_) => ErrorCode::SameStatus,
        }
    }
}

/// SCREAMING_SNAKE machine codes — ADR-0010 §4b: this namespace (`envelope.code`) is
/// SCREAMING_SNAKE-stable; it is distinct from business-outcome `reasons[].code` tokens (which
/// stay lowercase and out of this enum entirely — never normalize those here).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    IllegalTransition,
    ScaffoldDisabled,
    SameStatus,
    Conflict,
    NotFound,
    Unauthorized,
    Forbidden,
    ValidationFailed,
    RateLimit,
    Internal,
    NotImplemented,
    /// S1 storefront-read (`openapi-s1-storefront-read.yaml` `getPublicLocationInfo` 503):
    /// DB unavailable AND no usable stale-cache row — typed so the FE renders "couldn't
    /// load" instead of a raw 500 (`apps/api/src/routes/public/menu.ts:328`).
    ServiceUnavailable,
    /// S1 storefront-read (`getImage`/`getMediaObject` 400): the traversal-guard rejection
    /// code for a `..`/NUL/backslash-shaped object key
    /// (`apps/api/src/routes/spa-proxy.ts:165,190` `INVALID_KEY`).
    InvalidKey,
}

impl ErrorCode {
    /// The numeric HTTP status for this code — pure, framework-free (no `axum`/`http` crate
    /// dependency in `domain`, see the crate doc on why this crate stays IO/framework-free).
    /// This is the ONE table (`crates/api/src/error.rs`'s `status_for_code` delegates here) so
    /// the code -> status mapping cannot drift between the envelope this crate builds and the
    /// `axum::http::StatusCode` the `api` crate sends.
    pub const fn http_status(self) -> u16 {
        match self {
            ErrorCode::NotFound => 404,
            ErrorCode::Unauthorized => 401,
            ErrorCode::Forbidden => 403,
            ErrorCode::ValidationFailed => 422,
            ErrorCode::RateLimit => 429,
            ErrorCode::Conflict
            | ErrorCode::IllegalTransition
            | ErrorCode::ScaffoldDisabled
            | ErrorCode::SameStatus => 409,
            ErrorCode::NotImplemented => 501,
            ErrorCode::Internal => 500,
            ErrorCode::ServiceUnavailable => 503,
            ErrorCode::InvalidKey => 400,
        }
    }
}

/// The ADR-0010 wire envelope. `code` is the string machine code; `status`/`error` are ALWAYS
/// present (contract-required, see module doc) — populated from `code` at construction, so no
/// call site can build a contract-incomplete envelope.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ErrorEnvelope {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,
    pub correlation_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_ms: Option<u64>,
    /// Numeric HTTP status (legacy field — CONVENTIONS.md "keep until FE re-audit").
    pub status: u16,
    /// Legacy alias, always equal to `message` (`apiClient.ts:211` reads `message || error`).
    pub error: String,
}

impl ErrorEnvelope {
    pub fn new(
        code: ErrorCode,
        message: impl Into<String>,
        correlation_id: impl Into<String>,
    ) -> Self {
        let message = message.into();
        ErrorEnvelope {
            code,
            error: message.clone(),
            message,
            fields: None,
            correlation_id: correlation_id.into(),
            retry_after_ms: None,
            status: code.http_status(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::order_status::OrderStatus::*;

    #[test]
    fn domain_error_code_mapping() {
        assert_eq!(
            DomainError::IllegalTransition {
                from: Pending,
                to: Delivered
            }
            .code(),
            ErrorCode::IllegalTransition
        );
        assert_eq!(
            DomainError::ScaffoldDisabled {
                from: Pending,
                to: Scheduled
            }
            .code(),
            ErrorCode::ScaffoldDisabled
        );
        assert_eq!(
            DomainError::SameStatus(Pending).code(),
            ErrorCode::SameStatus
        );
    }

    #[test]
    fn error_code_serializes_screaming_snake() {
        assert_eq!(
            serde_json::to_string(&ErrorCode::IllegalTransition).unwrap(),
            "\"ILLEGAL_TRANSITION\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::NotImplemented).unwrap(),
            "\"NOT_IMPLEMENTED\""
        );
    }

    /// S1 contract codes (`openapi-s1-storefront-read.yaml` ErrorEnvelope description: "the
    /// S1 set: NOT_FOUND, SERVICE_UNAVAILABLE, INVALID_KEY, VALIDATION_FAILED, RATE_LIMIT,
    /// INTERNAL") — pins the two codes this crate lacked before the S1 port.
    #[test]
    fn s1_error_codes_serialize_screaming_snake() {
        assert_eq!(
            serde_json::to_string(&ErrorCode::ServiceUnavailable).unwrap(),
            "\"SERVICE_UNAVAILABLE\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::InvalidKey).unwrap(),
            "\"INVALID_KEY\""
        );
    }

    #[test]
    fn envelope_omits_absent_optional_fields_but_always_sends_status_and_error() {
        let envelope = ErrorEnvelope::new(ErrorCode::NotImplemented, "not built yet", "corr-123");
        let json = serde_json::to_value(&envelope).unwrap();
        assert!(
            json.get("fields").is_none(),
            "absent fields must not serialize"
        );
        assert!(json.get("retryAfterMs").is_none());
        assert_eq!(json["code"], "NOT_IMPLEMENTED");
        assert_eq!(json["correlationId"], "corr-123");
        // Contract-required (openapi-s1-storefront-read.yaml ErrorEnvelope `required`): status
        // and error are ALWAYS present, never Option-skipped.
        assert_eq!(json["status"], 501);
        assert_eq!(json["error"], "not built yet");
    }

    #[test]
    fn http_status_matches_the_s1_contract_table() {
        // openapi-s1-storefront-read.yaml's S1 code set: NOT_FOUND, SERVICE_UNAVAILABLE,
        // INVALID_KEY, VALIDATION_FAILED, RATE_LIMIT, INTERNAL.
        assert_eq!(ErrorCode::NotFound.http_status(), 404);
        assert_eq!(ErrorCode::ServiceUnavailable.http_status(), 503);
        assert_eq!(ErrorCode::InvalidKey.http_status(), 400);
        assert_eq!(ErrorCode::ValidationFailed.http_status(), 422);
        assert_eq!(ErrorCode::RateLimit.http_status(), 429);
        assert_eq!(ErrorCode::Internal.http_status(), 500);
    }

    #[test]
    fn error_alias_always_equals_message() {
        let envelope = ErrorEnvelope::new(ErrorCode::NotFound, "Location not found", "corr-1");
        assert_eq!(envelope.error, envelope.message);
    }
}
