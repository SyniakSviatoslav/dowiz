//! Maps `domain::ErrorEnvelope`/`ErrorCode` onto an axum `Response` — the ADR-0010 envelope is
//! the wire contract; this module is the one place that decides the HTTP status per code.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use domain::{ErrorCode, ErrorEnvelope};

#[derive(Debug)]
pub struct ApiError {
    pub envelope: ErrorEnvelope,
}

impl ApiError {
    pub fn new(
        code: ErrorCode,
        message: impl Into<String>,
        correlation_id: impl Into<String>,
    ) -> Self {
        ApiError {
            envelope: ErrorEnvelope::new(code, message, correlation_id),
        }
    }

    /// S3 catalog/admin CRUD VALIDATION_FAILED is deliberately **400**, not 422 (owner-route
    /// census: `modifier-groups.ts:94,191` / `menu-availability.ts:110` all
    /// `reply.sendError(400, 'VALIDATION_FAILED', ...)`). The S1-established
    /// `ErrorCode::ValidationFailed.http_status()` is 422 for the S1 surface and must stay
    /// untouched (money-newtype/domain code-preserving), so this forces the wire status to 400
    /// the same way `crate::auth::error::AuthEnvelopeError::validation_failed` already does for S2
    /// — same code string, per-surface status override, no change to the shared `domain` table.
    pub fn validation_failed_400(
        message: impl Into<String>,
        correlation_id: impl Into<String>,
    ) -> Self {
        let mut envelope = ErrorEnvelope::new(ErrorCode::ValidationFailed, message, correlation_id);
        envelope.status = 400;
        ApiError { envelope }
    }

    /// The wire status. Reads `self.envelope.status` (populated from `code.http_status()` at
    /// `ErrorEnvelope::new` time, and the ONLY field `validation_failed_400` above overrides)
    /// rather than recomputing from `self.envelope.code` — so a status override actually changes
    /// what `IntoResponse` sends, not just the field on the JSON body S1/S2 already relied on.
    fn status(&self) -> StatusCode {
        StatusCode::from_u16(self.envelope.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

/// Test-only: pins the `domain::ErrorCode::http_status` table (the ONE code -> status source of
/// truth) as an `axum::http::StatusCode` for direct assertion. `#[cfg(test)]` because
/// `ApiError::status` no longer calls this in production (it reads `self.envelope.status`, which
/// is `code.http_status()` UNLESS a constructor like `validation_failed_400` overrode it) — this
/// wrapper would otherwise be unused outside tests.
#[cfg(test)]
fn status_for_code(code: ErrorCode) -> StatusCode {
    StatusCode::from_u16(code.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();
        (status, Json(self.envelope)).into_response()
    }
}

/// Test-only helper: extracts the `Err` side of a handler's `Result<impl IntoResponse, ApiError>`.
/// Plain `.unwrap_err()` doesn't work here — it requires the `Ok` type to implement `Debug`, and
/// handlers deliberately return an opaque `impl IntoResponse` (unnameable, so it can't be given a
/// `Debug` impl) rather than a concrete response type.
#[cfg(test)]
pub(crate) fn expect_err<T>(result: Result<T, ApiError>) -> ApiError {
    match result {
        Err(e) => e,
        Ok(_) => panic!("expected an ApiError, got Ok"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_implemented_maps_to_501() {
        assert_eq!(
            status_for_code(ErrorCode::NotImplemented),
            StatusCode::NOT_IMPLEMENTED
        );
    }

    #[test]
    fn state_machine_codes_map_to_409_conflict() {
        for code in [
            ErrorCode::Conflict,
            ErrorCode::IllegalTransition,
            ErrorCode::ScaffoldDisabled,
            ErrorCode::SameStatus,
        ] {
            assert_eq!(
                status_for_code(code),
                StatusCode::CONFLICT,
                "{code:?} must map to 409"
            );
        }
    }

    #[test]
    fn rate_limit_maps_to_429() {
        assert_eq!(
            status_for_code(ErrorCode::RateLimit),
            StatusCode::TOO_MANY_REQUESTS
        );
    }

    #[test]
    fn s1_additions_map_to_503_and_400() {
        assert_eq!(
            status_for_code(ErrorCode::ServiceUnavailable),
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(
            status_for_code(ErrorCode::InvalidKey),
            StatusCode::BAD_REQUEST
        );
    }

    /// S3 catalog CRUD sends VALIDATION_FAILED at 400 (owner-route census), NOT the S1-table's 422
    /// — `validation_failed_400` must override the WIRE status (not just carry the right code
    /// string), since `IntoResponse` reads `ApiError::status()`, not `envelope.code.http_status()`
    /// directly.
    #[test]
    fn validation_failed_400_overrides_the_s1_422_default() {
        assert_eq!(
            status_for_code(ErrorCode::ValidationFailed),
            StatusCode::UNPROCESSABLE_ENTITY,
            "the S1-established default must stay 422 — this override must not mutate the shared table"
        );
        let err = ApiError::validation_failed_400("No updates provided", "corr-1");
        assert_eq!(err.envelope.code, ErrorCode::ValidationFailed);
        assert_eq!(
            err.envelope.status, 400,
            "the JSON body's status field is 400"
        );
        assert_eq!(
            err.status(),
            StatusCode::BAD_REQUEST,
            "the actual wire status (what IntoResponse sends) must also be 400"
        );
    }
}
