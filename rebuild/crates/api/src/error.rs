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

    fn status(&self) -> StatusCode {
        status_for_code(self.envelope.code)
    }
}

/// Pure function (no `Response`/IO) so the code -> status mapping is unit-testable directly.
/// Delegates to `domain::ErrorCode::http_status` — the ONE code -> status table (see that
/// function's doc); this wrapper only exists to produce the `axum` type instead of a bare `u16`.
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
}
