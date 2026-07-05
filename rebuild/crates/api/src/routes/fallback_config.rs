//! S1 storefront-read: `getFallbackConfig`.
//! Source: `apps/api/src/routes/public/fallback-config.ts:9-32`.

use std::sync::Arc;

use axum::extract::{Extension, Path, State};
use axum::response::IntoResponse;
use serde::Serialize;
use tower_http::request_id::RequestId;
use utoipa::ToSchema;

use domain::ErrorCode;

use crate::AppState;
use crate::error::ApiError;
use crate::routes::correlation_id_string;

#[derive(Debug, Serialize, ToSchema)]
pub struct FallbackConfigResponse {
    pub phone: Option<String>,
    #[serde(rename = "showPhoneOnError")]
    pub show_phone_on_error: bool,
    #[serde(rename = "showPhoneOnOffline")]
    pub show_phone_on_offline: bool,
}

/// Ports the `config.phone || row.public_phone || null`, `show* !== false` defaulting
/// (`fallback-config.ts:27-30`) as a pure function over the raw JSONB + column, so the
/// default-true-unless-explicitly-false semantics are independently testable.
pub fn build_fallback_config(
    config: &serde_json::Value,
    public_phone: Option<&str>,
) -> FallbackConfigResponse {
    let phone = config
        .get("phone")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .or_else(|| public_phone.map(str::to_string));
    let show_phone_on_error =
        config.get("show_phone_on_error") != Some(&serde_json::Value::Bool(false));
    let show_phone_on_offline =
        config.get("show_phone_on_offline") != Some(&serde_json::Value::Bool(false));
    FallbackConfigResponse {
        phone,
        show_phone_on_error,
        show_phone_on_offline,
    }
}

/// `GET /api/public/locations/{slug}/fallback-config` — source: `fallback-config.ts:9-32`.
#[utoipa::path(
    get,
    path = "/api/public/locations/{slug}/fallback-config",
    params(("slug" = String, Path)),
    responses(
        (status = 200, description = "Fallback contact config", body = FallbackConfigResponse),
        (status = 404, description = "Unknown slug", body = domain::ErrorEnvelope),
    ),
    tag = "fallback"
)]
pub async fn get_fallback_config(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let Some((config, public_phone)) = state.repo.fallback_config(&slug).await.ok().flatten()
    else {
        return Err(ApiError::new(
            ErrorCode::NotFound,
            "Not found",
            correlation_id,
        ));
    };
    Ok(axum::Json(build_fallback_config(
        &config,
        public_phone.as_deref(),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::fake::FakeRepo;
    use crate::storage::LocalFsStorage;

    fn test_state(repo: FakeRepo) -> Arc<AppState> {
        Arc::new(AppState {
            repo: Arc::new(repo),
            storage: Arc::new(LocalFsStorage::new(std::env::temp_dir())),
            media_rich_enabled: false,
            app_base_url: "https://dowiz.fly.dev".to_string(),
            r2_public_url: None,
        })
    }

    #[test]
    fn build_fallback_config_defaults_show_flags_true() {
        let config = serde_json::json!({});
        let result = build_fallback_config(&config, Some("+355000"));
        assert_eq!(result.phone.as_deref(), Some("+355000"));
        assert!(result.show_phone_on_error);
        assert!(result.show_phone_on_offline);
    }

    #[test]
    fn build_fallback_config_config_phone_wins_over_location_phone() {
        let config = serde_json::json!({"phone": "+355111"});
        let result = build_fallback_config(&config, Some("+355000"));
        assert_eq!(result.phone.as_deref(), Some("+355111"));
    }

    #[test]
    fn build_fallback_config_explicit_false_disables_flag() {
        let config = serde_json::json!({"show_phone_on_error": false});
        let result = build_fallback_config(&config, None);
        assert!(!result.show_phone_on_error);
        assert!(result.show_phone_on_offline);
    }

    #[test]
    fn build_fallback_config_null_phone_both_sources() {
        let config = serde_json::json!({});
        let result = build_fallback_config(&config, None);
        assert!(result.phone.is_none());
    }

    #[tokio::test]
    async fn get_fallback_config_404_for_unknown_slug() {
        let state = test_state(FakeRepo::default());
        let err = crate::error::expect_err(
            get_fallback_config(
                State(state),
                Path("nowhere".to_string()),
                Extension(RequestId::new(axum::http::HeaderValue::from_static(
                    "corr-1",
                ))),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }
}
