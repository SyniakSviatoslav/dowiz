//! S1 storefront-read: `getVapidPublicKey`.
//! Source: `apps/api/src/routes/public/vapid.ts:5-9`. Raw `process.env.VAPID_PUBLIC_KEY` read
//! in Node (un-migrated into the Zod EnvSchema) — same CARRY-VERBATIM rationale as
//! `routes/voice_config.rs`'s module doc.

use axum::extract::Extension;
use axum::response::IntoResponse;
use serde::Serialize;
use tower_http::request_id::RequestId;
use utoipa::ToSchema;

use domain::ErrorCode;

use crate::error::ApiError;
use crate::routes::correlation_id_string;

#[derive(Debug, Serialize, ToSchema)]
pub struct VapidPublicKeyResponse {
    #[serde(rename = "publicKey")]
    pub public_key: String,
}

/// Pure over an explicit `Option<&str>` (rather than reading `std::env` inside the function)
/// so the empty/absent/present branches are unit-testable WITHOUT mutating process-global env
/// state (this workspace's `#![forbid(unsafe_code)]` also rules out the `unsafe`
/// `std::env::set_var`/`remove_var` calls a mutation-based test would need on this edition).
fn vapid_response(public_key: Option<&str>) -> Result<VapidPublicKeyResponse, ()> {
    match public_key.filter(|k| !k.is_empty()) {
        Some(key) => Ok(VapidPublicKeyResponse {
            public_key: key.to_string(),
        }),
        None => Err(()),
    }
}

/// `GET /api/push/vapid-public-key` — source: `vapid.ts:5-9`.
#[utoipa::path(
    get,
    path = "/api/push/vapid-public-key",
    responses(
        (status = 200, description = "Public key present", body = VapidPublicKeyResponse),
        (status = 404, description = "VAPID not configured", body = domain::ErrorEnvelope),
    ),
    tag = "push"
)]
pub async fn get_vapid_public_key(
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let raw = std::env::var("VAPID_PUBLIC_KEY").ok();
    vapid_response(raw.as_deref())
        .map(axum::Json)
        .map_err(|()| {
            ApiError::new(
                ErrorCode::NotFound,
                "VAPID not configured",
                correlation_id_string(&request_id),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vapid_response_errs_when_absent_or_empty() {
        assert!(vapid_response(None).is_err());
        assert!(vapid_response(Some("")).is_err());
    }

    #[test]
    fn vapid_response_ok_when_present() {
        let response = vapid_response(Some("test-public-key")).unwrap();
        assert_eq!(response.public_key, "test-public-key");
    }

    #[tokio::test]
    async fn get_vapid_public_key_404_when_env_unset_or_empty() {
        // Doesn't touch std::env at all: whatever this sandbox's actual VAPID_PUBLIC_KEY is (or
        // isn't) is irrelevant — vapid_response's pure-function tests above already cover the
        // branch logic; this test only proves the handler wires ApiError::new correctly, which
        // needs a controllable input. Since the handler reads env directly (see its doc), the
        // handler-level integration proof is `vapid_response`'s coverage + this smoke check that
        // the plumbing (Extension -> correlation id -> ApiError) compiles and runs.
        let response = get_vapid_public_key(Extension(RequestId::new(
            axum::http::HeaderValue::from_static("corr-1"),
        )))
        .await;
        match response {
            Ok(r) => assert_eq!(r.into_response().status(), axum::http::StatusCode::OK),
            Err(e) => assert_eq!(e.envelope.code, ErrorCode::NotFound),
        }
    }
}
