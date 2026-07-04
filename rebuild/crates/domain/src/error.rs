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

    // ── S2 auth code set (openapi-s2-auth.yaml ErrorEnvelope `code` enumeration) ──
    // Each maps to a fixed HTTP status in `http_status` below; the SCREAMING_SNAKE serialization
    // is the FE-branchable contract (append-only, never renamed — ADR-0010 §4b).
    /// 401 — bad email/password OR unknown identity (indistinguishable by design; local.ts:91,
    /// courier/auth.ts:256).
    InvalidCredentials,
    /// 401 — the account has no `password_hash` (Google/Telegram-only account; local.ts:96).
    WrongAuthMethod,
    /// 401 — the ADR-0004 P-c live role re-derive found no active owner membership (auth.ts:300).
    OwnerRevoked,
    /// 410 — courier invite invalid/expired/used/revoked (courier/auth.ts:62).
    InviteInvalid,
    /// 401 — wrong courier invite code (courier/auth.ts:71).
    InvalidCode,
    /// 401 — malformed/mismatched courier refresh token (courier/auth.ts:394,415).
    InvalidRefreshToken,
    /// 401 — courier session id not found on refresh (courier/auth.ts:407).
    SessionNotFound,
    /// 401 — courier refresh token reuse detected, family revoked (courier/auth.ts:427).
    RefreshReused,
    /// 401 — courier refresh token past its expiry (courier/auth.ts:432).
    RefreshExpired,
    /// 403 — courier account deactivated (courier/auth.ts:278,439).
    CourierDeactivated,
    /// 403 — explicit `location_id` is not one of the courier's memberships (courier/auth.ts:291).
    NotAuthorizedForLocation,
    /// 403 — courier has no assigned location at all (courier/auth.ts:300).
    NoLocationAssigned,

    // ── S3 catalog/admin CRUD code set (owner-route census rows 1-89) ──
    // Each is a genuinely NEW machine-code string the owner-route TS source sends (never an
    // existing code with a different status — those go through the `ApiError`
    // status-override pattern S2 already established for VALIDATION_FAILED, see
    // `crates/api/src/error.rs`/`crates/api/src/auth/error.rs::validation_failed`).
    /// 400 — PATCH body was `{}` (`products.ts:141`, `categories.ts:129-131`): "No updates
    /// provided". Distinct from `ValidationFailed` (a different literal code string on the wire).
    NoUpdates,
    /// 400 — `PUT .../translations/:locale` targets a locale outside the location's
    /// `supported_locales` (`products.ts:228-230`).
    UnsupportedLocale,
    /// 400 — `PUT .../products/:id/modifier-groups` referenced a `group_id` that does not belong
    /// to this location (fold-in INSERT matched 0 rows, `products.ts:332-334`).
    InvalidGroup,
    /// 409 — `DELETE .../categories/:id` on a category that still has products
    /// (`categories.ts:168-180,249-255`).
    CategoryNotEmpty,

    // ── S4 media surface code set (`docs/design/rebuild-media-s4-council/`) ──
    /// 400 — invalid media `kind`, disallowed mime, invalid sha256 shape, bad bytes, missing
    /// `storageKey`, or a `storageKey` outside the tenant prefix (`product-media.ts` multiple
    /// sites, e.g. `:98,116,209`). One code covers every "the request shape/content is invalid
    /// for a media op" case, matching the TS route's own `INVALID_MEDIA` string reuse.
    InvalidMedia,
    /// 413 — a declared file size exceeds the per-mime ceiling (`product-media.ts:119`).
    FileTooLarge,

    // ── S5 orders/money surface code set (`docs/design/rebuild-orders-s5-council/`) ──
    // Each is the exact SCREAMING_SNAKE code string the Node order routes send via
    // `reply.sendError(<status>, '<CODE>', …)`; the status arms below match the Node call site
    // per-code (the route passes the status explicitly, so the code→status table here is derived
    // FROM those call sites, `orders.ts` / `customer/orders.ts`).
    /// 422 — `subtotal < min_order_value` (pickup AND delivery, `orders.ts:498`).
    MinOrderNotMet,
    /// 422 — declared cash `< total` (`orders.ts:537`).
    CashAmountTooLow,
    /// 422 — a cart product is `is_available = false` on the price snapshot (`orders.ts` §6).
    ProductUnavailable,
    /// 422 — a cart product id does not exist for this location (`orders.ts` §6).
    ProductNotFound,
    /// 422 — a modifier id is not a valid/available modifier for its product
    /// (`order-pricing.ts:99`).
    ModifierUnavailable,
    /// 422 — a required modifier group's min-select is not met (`order-pricing.ts:118`).
    ModifierMinNotMet,
    /// 422 — a modifier group's max-select is exceeded (`order-pricing.ts:124`).
    ModifierMaxExceeded,
    /// 422 — the same modifier id appears twice on one line item (`order-pricing.ts:91`).
    DuplicateModifier,
    /// 422 — the delivery pin is beyond the last configured tier (`order-pricing.ts:178`).
    NotDeliverable,
    /// 422 — no flat fee and no tiers configured for a delivery order (`order-pricing.ts:183`).
    DeliveryNotConfigured,
    /// 422 — idempotency key hit with a DIFFERENT `request_hash` (`orders.ts:403`).
    IdempotencyKeyReused,
    /// 409 — idempotency-key unique race, surfaced from `23505` (`orders.ts:719`).
    IdempotencyConflict,
    /// 403 — an owner tried to drive a SYSTEM-only CONFIRMED/PREPARING/READY→CANCELLED edge
    /// (`orderAuthz.ts:22`).
    CancelNotPermitted,
    /// 409 — PATCH→DELIVERED/PICKED_UP with an active courier binding (`orders.ts:931`).
    AssignmentActive,
    /// 409 — IN_DELIVERY without a delivered assignment on PATCH→DELIVERED/PICKED_UP
    /// (`orders.ts:945`).
    UseDeliverFlow,
    /// 409 — customer cancel attempted on a non-IN_DELIVERY order (`customer/orders.ts:296`).
    CancelNotAllowedStatus,
    /// 410 — customer cancel past the post-dispatch window (`customer/orders.ts:303`).
    CancelWindowExpired,
    /// 409 — order create against an unpublished location (`orders.ts` venue gate; NOT_PUBLISHED).
    NotPublished,
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
            // ── S2 auth codes (openapi-s2-auth.yaml per-operation response statuses) ──
            ErrorCode::InvalidCredentials
            | ErrorCode::WrongAuthMethod
            | ErrorCode::OwnerRevoked
            | ErrorCode::InvalidCode
            | ErrorCode::InvalidRefreshToken
            | ErrorCode::SessionNotFound
            | ErrorCode::RefreshReused
            | ErrorCode::RefreshExpired => 401,
            ErrorCode::CourierDeactivated
            | ErrorCode::NotAuthorizedForLocation
            | ErrorCode::NoLocationAssigned => 403,
            ErrorCode::InviteInvalid => 410,
            // ── S3 catalog/admin CRUD code set ──
            ErrorCode::NoUpdates | ErrorCode::UnsupportedLocale | ErrorCode::InvalidGroup => 400,
            ErrorCode::CategoryNotEmpty => 409,
            // ── S4 media code set ──
            ErrorCode::InvalidMedia => 400,
            ErrorCode::FileTooLarge => 413,
            // ── S5 orders/money code set (status per the Node route call site) ──
            ErrorCode::MinOrderNotMet
            | ErrorCode::CashAmountTooLow
            | ErrorCode::ProductUnavailable
            | ErrorCode::ProductNotFound
            | ErrorCode::ModifierUnavailable
            | ErrorCode::ModifierMinNotMet
            | ErrorCode::ModifierMaxExceeded
            | ErrorCode::DuplicateModifier
            | ErrorCode::NotDeliverable
            | ErrorCode::DeliveryNotConfigured
            | ErrorCode::IdempotencyKeyReused => 422,
            ErrorCode::IdempotencyConflict
            | ErrorCode::AssignmentActive
            | ErrorCode::UseDeliverFlow
            | ErrorCode::CancelNotAllowedStatus
            | ErrorCode::NotPublished => 409,
            ErrorCode::CancelNotPermitted => 403,
            ErrorCode::CancelWindowExpired => 410,
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

    /// S2 auth codes serialize SCREAMING_SNAKE and map to the openapi-s2-auth.yaml statuses.
    #[test]
    fn s2_auth_codes_serialize_and_map_status() {
        assert_eq!(
            serde_json::to_string(&ErrorCode::OwnerRevoked).unwrap(),
            "\"OWNER_REVOKED\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::InvalidRefreshToken).unwrap(),
            "\"INVALID_REFRESH_TOKEN\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::CourierDeactivated).unwrap(),
            "\"COURIER_DEACTIVATED\""
        );
        // Status parity with the contract.
        assert_eq!(ErrorCode::InvalidCredentials.http_status(), 401);
        assert_eq!(ErrorCode::WrongAuthMethod.http_status(), 401);
        assert_eq!(ErrorCode::OwnerRevoked.http_status(), 401);
        assert_eq!(ErrorCode::InviteInvalid.http_status(), 410);
        assert_eq!(ErrorCode::CourierDeactivated.http_status(), 403);
        assert_eq!(ErrorCode::NotAuthorizedForLocation.http_status(), 403);
        assert_eq!(ErrorCode::NoLocationAssigned.http_status(), 403);
    }

    /// S3 catalog/admin CRUD codes serialize SCREAMING_SNAKE and map to the census-observed
    /// (`owner-route census rows 1-89`) `sendError` statuses.
    #[test]
    fn s3_catalog_codes_serialize_and_map_status() {
        assert_eq!(
            serde_json::to_string(&ErrorCode::NoUpdates).unwrap(),
            "\"NO_UPDATES\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::UnsupportedLocale).unwrap(),
            "\"UNSUPPORTED_LOCALE\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::InvalidGroup).unwrap(),
            "\"INVALID_GROUP\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::CategoryNotEmpty).unwrap(),
            "\"CATEGORY_NOT_EMPTY\""
        );
        assert_eq!(ErrorCode::NoUpdates.http_status(), 400);
        assert_eq!(ErrorCode::UnsupportedLocale.http_status(), 400);
        assert_eq!(ErrorCode::InvalidGroup.http_status(), 400);
        assert_eq!(ErrorCode::CategoryNotEmpty.http_status(), 409);
    }

    /// S4 media codes serialize SCREAMING_SNAKE and map to the `product-media.ts` `sendError`
    /// statuses (`INVALID_MEDIA` 400, `FILE_TOO_LARGE` 413).
    #[test]
    fn s4_media_codes_serialize_and_map_status() {
        assert_eq!(
            serde_json::to_string(&ErrorCode::InvalidMedia).unwrap(),
            "\"INVALID_MEDIA\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::FileTooLarge).unwrap(),
            "\"FILE_TOO_LARGE\""
        );
        assert_eq!(ErrorCode::InvalidMedia.http_status(), 400);
        assert_eq!(ErrorCode::FileTooLarge.http_status(), 413);
    }

    /// S5 orders/money codes serialize SCREAMING_SNAKE and map to the `orders.ts`/`customer/orders.ts`
    /// `sendError` statuses (the code strings are the FE-branchable contract).
    #[test]
    fn s5_orders_codes_serialize_and_map_status() {
        assert_eq!(
            serde_json::to_string(&ErrorCode::MinOrderNotMet).unwrap(),
            "\"MIN_ORDER_NOT_MET\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::CashAmountTooLow).unwrap(),
            "\"CASH_AMOUNT_TOO_LOW\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::IdempotencyKeyReused).unwrap(),
            "\"IDEMPOTENCY_KEY_REUSED\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::IdempotencyConflict).unwrap(),
            "\"IDEMPOTENCY_CONFLICT\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::CancelNotPermitted).unwrap(),
            "\"CANCEL_NOT_PERMITTED\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::UseDeliverFlow).unwrap(),
            "\"USE_DELIVER_FLOW\""
        );
        // Status parity with the Node call sites.
        assert_eq!(ErrorCode::MinOrderNotMet.http_status(), 422);
        assert_eq!(ErrorCode::CashAmountTooLow.http_status(), 422);
        assert_eq!(ErrorCode::IdempotencyKeyReused.http_status(), 422);
        assert_eq!(ErrorCode::NotDeliverable.http_status(), 422);
        assert_eq!(ErrorCode::DeliveryNotConfigured.http_status(), 422);
        assert_eq!(ErrorCode::DuplicateModifier.http_status(), 422);
        assert_eq!(ErrorCode::IdempotencyConflict.http_status(), 409);
        assert_eq!(ErrorCode::AssignmentActive.http_status(), 409);
        assert_eq!(ErrorCode::UseDeliverFlow.http_status(), 409);
        assert_eq!(ErrorCode::CancelNotAllowedStatus.http_status(), 409);
        assert_eq!(ErrorCode::NotPublished.http_status(), 409);
        assert_eq!(ErrorCode::CancelNotPermitted.http_status(), 403);
        assert_eq!(ErrorCode::CancelWindowExpired.http_status(), 410);
    }
}
