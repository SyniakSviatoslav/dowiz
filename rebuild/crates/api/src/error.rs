//! Maps `domain::ErrorEnvelope`/`ErrorCode` onto an axum `Response` — the ADR-0010 envelope is
//! the wire contract; this module is the one place that decides the HTTP status per code.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use domain::{ErrorCode, ErrorEnvelope};

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
fn status_for_code(code: ErrorCode) -> StatusCode {
    match code {
        ErrorCode::NotFound => StatusCode::NOT_FOUND,
        ErrorCode::Unauthorized => StatusCode::UNAUTHORIZED,
        ErrorCode::Forbidden => StatusCode::FORBIDDEN,
        ErrorCode::ValidationFailed => StatusCode::UNPROCESSABLE_ENTITY,
        ErrorCode::RateLimit => StatusCode::TOO_MANY_REQUESTS,
        ErrorCode::Conflict
        | ErrorCode::IllegalTransition
        | ErrorCode::ScaffoldDisabled
        | ErrorCode::SameStatus => StatusCode::CONFLICT,
        ErrorCode::NotImplemented => StatusCode::NOT_IMPLEMENTED,
        ErrorCode::Internal => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();
        (status, Json(self.envelope)).into_response()
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
}
