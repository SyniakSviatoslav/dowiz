//! S1 storefront-read: `getImage`, `getMediaObject`.
//! Source: `apps/api/src/routes/spa-proxy.ts:158-211`.

use std::sync::Arc;

use axum::extract::{Extension, Path, State};
use axum::http::header;
use axum::response::IntoResponse;
use tower_http::request_id::RequestId;

use domain::ErrorCode;

use crate::AppState;
use crate::error::ApiError;
use crate::routes::correlation_id_string;
use crate::storage::{media_content_type, validate_object_key};

/// `GET /images/{key}` (wildcard) — source: `spa-proxy.ts:158-178`. Always `image/webp` (the
/// upload pipeline transcodes to webp). x-quirk: a storage-layer error is returned as 404, not
/// 5xx — an outage is indistinguishable from a missing object (preserved verbatim).
#[utoipa::path(
    get,
    path = "/images/{key}",
    params(("key" = String, Path)),
    responses(
        (status = 200, description = "Binary webp", content_type = "image/webp"),
        (status = 400, description = "Traversal-shaped key", body = domain::ErrorEnvelope),
        (status = 404, description = "Missing object or storage error", body = domain::ErrorEnvelope),
    ),
    tag = "media"
)]
pub async fn get_image(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    validate_object_key(&key)
        .map_err(|e| e.into_api_error("Invalid image key", correlation_id.clone()))?;

    match state.storage.get(&key).await {
        Ok(Some(bytes)) => Ok((
            [
                (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
                (header::CONTENT_TYPE, "image/webp"),
            ],
            bytes,
        )),
        Ok(None) | Err(_) => Err(ApiError::new(
            ErrorCode::NotFound,
            "Image not found",
            correlation_id,
        )),
    }
}

/// `GET /media/{key}` (wildcard) — source: `spa-proxy.ts:184-211`. Same traversal guard +
/// 404-on-error quirk as `/images/*`; Content-Type derived from the key extension.
#[utoipa::path(
    get,
    path = "/media/{key}",
    params(("key" = String, Path)),
    responses(
        (status = 200, description = "Binary media (type by extension)"),
        (status = 400, description = "Traversal-shaped key", body = domain::ErrorEnvelope),
        (status = 404, description = "Missing object or storage error", body = domain::ErrorEnvelope),
    ),
    tag = "media"
)]
pub async fn get_media_object(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    validate_object_key(&key)
        .map_err(|e| e.into_api_error("Invalid media key", correlation_id.clone()))?;
    let content_type = media_content_type(&key);

    match state.storage.get(&key).await {
        Ok(Some(bytes)) => Ok((
            [
                (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
                (header::CONTENT_TYPE, content_type),
            ],
            bytes,
        )),
        Ok(None) | Err(_) => Err(ApiError::new(
            ErrorCode::NotFound,
            "Media not found",
            correlation_id,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::fake::FakeRepo;
    use crate::storage::LocalFsStorage;

    fn test_state(base_dir: std::path::PathBuf) -> Arc<AppState> {
        Arc::new(AppState {
            repo: Arc::new(FakeRepo::default()),
            storage: Arc::new(LocalFsStorage::new(base_dir)),
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

    #[tokio::test]
    async fn get_image_rejects_traversal_key_with_400() {
        let state = test_state(std::env::temp_dir());
        let err = crate::error::expect_err(
            get_image(
                State(state),
                Path("../etc/passwd".to_string()),
                request_id(),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::InvalidKey);
    }

    #[tokio::test]
    async fn get_image_404_when_missing() {
        let state = test_state(std::env::temp_dir().join("dowiz-rebuild-images-missing"));
        let err = crate::error::expect_err(
            get_image(
                State(state),
                Path("no-such-key.webp".to_string()),
                request_id(),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn get_image_200_serves_bytes_as_webp() {
        let dir = std::env::temp_dir().join(format!("dowiz-rebuild-test-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("present.webp"), b"fake-bytes")
            .await
            .unwrap();
        let state = test_state(dir.clone());

        let response = get_image(State(state), Path("present.webp".to_string()), request_id())
            .await
            .unwrap()
            .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "image/webp"
        );

        tokio::fs::remove_dir_all(&dir).await.ok();
    }

    #[tokio::test]
    async fn get_media_object_rejects_traversal_key_with_400() {
        let state = test_state(std::env::temp_dir());
        let err = crate::error::expect_err(
            get_media_object(State(state), Path("a\\b".to_string()), request_id()).await,
        );
        assert_eq!(err.envelope.code, ErrorCode::InvalidKey);
    }

    #[tokio::test]
    async fn get_media_object_derives_content_type_from_extension() {
        let dir = std::env::temp_dir().join(format!("dowiz-rebuild-test-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("clip.mp4"), b"fake-mp4")
            .await
            .unwrap();
        let state = test_state(dir.clone());

        let response = get_media_object(State(state), Path("clip.mp4".to_string()), request_id())
            .await
            .unwrap()
            .into_response();
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "video/mp4"
        );

        tokio::fs::remove_dir_all(&dir).await.ok();
    }
}
