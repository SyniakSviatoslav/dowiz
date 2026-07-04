//! S1 storefront-read: `getPublicTheme`, `getThemeCss`.
//! Sources: `apps/api/src/routes/spa-proxy.ts:506-528` and `apps/api/src/routes/public/theme.ts`.

use std::sync::Arc;

use axum::extract::{Extension, Path, Query, State};
use axum::http::header;
use axum::response::IntoResponse;
use serde::Deserialize;
use tower_http::request_id::RequestId;

use domain::ErrorCode;

use crate::AppState;
use crate::dto::PublicTheme;
use crate::error::ApiError;
use crate::routes::correlation_id_string;

/// The exact fallback stylesheet (`theme.ts:8`) — byte-identical so `getThemeCss`'s "never
/// errors, always renders a styled storefront" invariant holds even with zero DB access.
pub const DEFAULT_CSS: &str = ":root{--brand-primary:#ea4f16;--brand-primary-hover:#ffa12e;--brand-bg:#121212;--brand-surface:#1e1e1e;--brand-text:#ffffff;--brand-text-muted:#a8a8a8;--brand-border:#2c2c2c;--brand-radius:12px;--color-success:#059669;--color-warning:#D97706;--color-danger:#DC2626;--color-info:#2563EB}@media(prefers-color-scheme:dark){:root{--brand-bg:#0F172A;--brand-surface:#1E293B;--brand-text:#F1F5F9;--brand-text-muted:#94A3B8;--brand-border:#334155}}";

/// `GET /api/public/theme/{slug}` — source: `spa-proxy.ts:506-528`.
#[utoipa::path(
    get,
    path = "/api/public/theme/{slug}",
    params(("slug" = String, Path)),
    responses(
        (status = 200, description = "Tenant branding", body = PublicTheme),
        (status = 404, description = "Unknown slug", body = domain::ErrorEnvelope),
    ),
    tag = "theme"
)]
pub async fn get_public_theme(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let Some((location_id, location_name, supported_locales)) =
        state.repo.theme_location(&slug).await.ok().flatten()
    else {
        return Err(ApiError::new(
            ErrorCode::NotFound,
            "Not found",
            correlation_id_string(&request_id),
        ));
    };

    let colors = state.repo.theme_colors(location_id).await.ok().flatten();

    Ok(axum::Json(PublicTheme {
        primary_color: colors.as_ref().and_then(|c| c.primary_color.clone()),
        bg_color: colors.as_ref().and_then(|c| c.bg_color.clone()),
        text_color: colors.as_ref().and_then(|c| c.text_color.clone()),
        logo_url: colors.as_ref().and_then(|c| c.logo_url.clone()),
        location_name,
        heading_font: colors.as_ref().and_then(|c| c.heading_font.clone()),
        body_font: colors.as_ref().and_then(|c| c.body_font.clone()),
        supported_locales,
    }))
}

#[derive(Debug, Deserialize)]
pub struct ThemeCssQuery {
    pub hash: Option<String>,
}

/// `GET /public/locations/{locationId}/theme.css` — source: `theme.ts:10-50`. x-quirk: NEVER
/// errors — any lookup miss falls back to `DEFAULT_CSS` with 200 (storefront must never render
/// unstyled).
#[utoipa::path(
    get,
    path = "/public/locations/{locationId}/theme.css",
    params(("locationId" = String, Path), ("hash" = Option<String>, Query)),
    responses((status = 200, description = "Tenant CSS or DEFAULT_CSS fallback (always 200)")),
    tag = "theme"
)]
pub async fn get_theme_css(
    State(state): State<Arc<AppState>>,
    Path(location_id): Path<String>,
    Query(query): Query<ThemeCssQuery>,
) -> impl IntoResponse {
    let hash = query.hash.filter(|h| !h.is_empty());

    let css = async {
        let location_uuid = state
            .repo
            .resolve_location_uuid(&location_id)
            .await
            .ok()
            .flatten()?;
        state
            .repo
            .theme_css_body(location_uuid, hash.as_deref())
            .await
            .ok()
            .flatten()
    }
    .await;

    let cache_control = if hash.is_some() {
        "public, max-age=31536000, immutable"
    } else {
        "public, max-age=60"
    };

    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, cache_control),
        ],
        css.unwrap_or_else(|| DEFAULT_CSS.to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::ThemeRow;
    use crate::repo::fake::FakeRepo;
    use crate::storage::LocalFsStorage;
    use uuid::Uuid;

    fn test_state(repo: FakeRepo) -> Arc<AppState> {
        Arc::new(AppState {
            repo: Arc::new(repo),
            storage: Arc::new(LocalFsStorage::new(std::env::temp_dir())),
            media_rich_enabled: false,
            app_base_url: "https://dowiz.fly.dev".to_string(),
            r2_public_url: None,
        })
    }

    fn request_id() -> Extension<RequestId> {
        Extension(RequestId::new(axum::http::HeaderValue::from_static(
            "corr-1",
        )))
    }

    async fn body_json(response: axum::response::Response) -> PublicTheme {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn get_public_theme_404_for_unknown_slug() {
        let state = test_state(FakeRepo::default());
        let err = crate::error::expect_err(
            get_public_theme(State(state), Path("nowhere".to_string()), request_id()).await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn get_public_theme_supported_locales_null_when_column_not_array() {
        let repo = FakeRepo::default();
        let loc_id = Uuid::new_v4();
        repo.theme_locations.lock().unwrap().insert(
            "eljos-pizza".to_string(),
            (loc_id, "Eljo's Pizza".to_string(), None),
        );
        let state = test_state(repo);
        let theme = body_json(
            get_public_theme(State(state), Path("eljos-pizza".to_string()), request_id())
                .await
                .unwrap()
                .into_response(),
        )
        .await;
        assert!(theme.supported_locales.is_none());
    }

    #[tokio::test]
    async fn get_public_theme_defaults_colors_when_no_theme_row() {
        let repo = FakeRepo::default();
        let loc_id = Uuid::new_v4();
        repo.theme_locations.lock().unwrap().insert(
            "eljos-pizza".to_string(),
            (
                loc_id,
                "Eljo's Pizza".to_string(),
                Some(vec!["sq".to_string()]),
            ),
        );
        let state = test_state(repo);
        let theme = body_json(
            get_public_theme(State(state), Path("eljos-pizza".to_string()), request_id())
                .await
                .unwrap()
                .into_response(),
        )
        .await;
        assert!(theme.primary_color.is_none());
        assert_eq!(theme.location_name, "Eljo's Pizza");
    }

    #[tokio::test]
    async fn get_theme_css_falls_back_to_default_when_nothing_found() {
        let state = test_state(FakeRepo::default());
        let response = get_theme_css(
            State(state),
            Path("does-not-exist".to_string()),
            Query(ThemeCssQuery { hash: None }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(bytes, DEFAULT_CSS.as_bytes());
    }

    #[tokio::test]
    async fn get_theme_css_serves_tenant_css_when_present() {
        let repo = FakeRepo::default();
        let loc_id = Uuid::new_v4();
        repo.location_uuids
            .lock()
            .unwrap()
            .insert("eljos-pizza".to_string(), loc_id);
        repo.theme_css_bodies
            .lock()
            .unwrap()
            .insert(loc_id, ":root{--brand-primary:#ff0000}".to_string());
        let state = test_state(repo);
        let response = get_theme_css(
            State(state),
            Path("eljos-pizza".to_string()),
            Query(ThemeCssQuery { hash: None }),
        )
        .await
        .into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(bytes, b":root{--brand-primary:#ff0000}".as_slice());
    }

    #[test]
    fn theme_row_type_used() {
        // keeps ThemeRow import intentional (constructed indirectly via FakeRepo in other tests)
        let _ = ThemeRow {
            primary_color: None,
            bg_color: None,
            text_color: None,
            logo_url: None,
            heading_font: None,
            body_font: None,
        };
    }
}
