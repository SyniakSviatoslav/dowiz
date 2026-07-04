//! Error taxonomy — ports `packages/domain/src/errors.ts` (`IllegalTransitionError`,
//! `ScaffoldDisabledError`, `SameStatusError`, `ConflictError`) as `DomainError`, and models the
//! ADR-0010 wire envelope shape (`docs/adr/0010-error-contract-envelope.md`):
//! `{ code: SCREAMING_SNAKE string, message, fields?, correlationId, retryAfterMs?, status? }`.
//!
//! This is a skeleton, not the full ~50-code A2 matrix (ADR-0010 §4) — Phase A only needs enough
//! codes to cover the domain errors that exist today plus the generic HTTP-adjacent codes the
//! `api` crate's health/menu-stub routes need. The full matrix lands with the OpenAPI contract
//! lane (REBUILD-MAP §4).

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
}

/// The ADR-0010 wire envelope. `code` is the string machine code; `status` is the legacy numeric
/// HTTP code the ADR keeps for code-preserving rollout — optional here since this skeleton has no
/// FE consumer yet to preserve compatibility with.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorEnvelope {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,
    pub correlation_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
}

impl ErrorEnvelope {
    pub fn new(
        code: ErrorCode,
        message: impl Into<String>,
        correlation_id: impl Into<String>,
    ) -> Self {
        ErrorEnvelope {
            code,
            message: message.into(),
            fields: None,
            correlation_id: correlation_id.into(),
            retry_after_ms: None,
            status: None,
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

    #[test]
    fn envelope_omits_absent_optional_fields() {
        let envelope = ErrorEnvelope::new(ErrorCode::NotImplemented, "not built yet", "corr-123");
        let json = serde_json::to_value(&envelope).unwrap();
        assert!(
            json.get("fields").is_none(),
            "absent fields must not serialize"
        );
        assert!(json.get("retryAfterMs").is_none());
        assert!(json.get("status").is_none());
        assert_eq!(json["code"], "NOT_IMPLEMENTED");
        assert_eq!(json["correlationId"], "corr-123");
    }
}
