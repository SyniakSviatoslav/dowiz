//! S1 storefront-read: `getWebManifest`.
//! Source: `apps/api/src/routes/public/pwa.ts:7-51`.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::header;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::AppState;

const DEFAULT_THEME_COLOR: &str = "#ea4f16";
const DEFAULT_BG_COLOR: &str = "#121212";

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ManifestIcon {
    pub src: String,
    pub sizes: String,
    #[serde(rename = "type")]
    pub mime_type: String,
    pub purpose: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WebManifest {
    pub name: String,
    pub short_name: String,
    pub description: String,
    pub start_url: String,
    pub display: String,
    pub background_color: String,
    pub theme_color: String,
    pub icons: Vec<ManifestIcon>,
}

/// Ports `pwa.ts:40`'s truncation verbatim: name > 12 chars -> first 12 + `"..."`.
pub fn short_name(name: &str) -> String {
    if name.chars().count() > 12 {
        let truncated: String = name.chars().take(12).collect();
        format!("{truncated}...")
    } else {
        name.to_string()
    }
}

fn build_manifest(slug: &str, name: &str, theme_color: &str) -> WebManifest {
    WebManifest {
        name: name.to_string(),
        short_name: short_name(name),
        description: format!("Order food delivery from {name}"),
        start_url: format!("/s/{slug}?source=pwa"),
        display: "standalone".to_string(),
        background_color: DEFAULT_BG_COLOR.to_string(),
        theme_color: theme_color.to_string(),
        icons: vec![
            ManifestIcon {
                src: "/icons/icon-192.png".to_string(),
                sizes: "192x192".to_string(),
                mime_type: "image/png".to_string(),
                purpose: "any maskable".to_string(),
            },
            ManifestIcon {
                src: "/icons/icon-512.png".to_string(),
                sizes: "512x512".to_string(),
                mime_type: "image/png".to_string(),
                purpose: "any maskable".to_string(),
            },
        ],
    }
}

/// `GET /s/{slug}/manifest.webmanifest` — source: `pwa.ts:7-51`. x-quirk: DB failure -> generic
/// manifest with the slug as name, still 200 — never 500s.
#[utoipa::path(
    get,
    path = "/s/{slug}/manifest.webmanifest",
    params(("slug" = String, Path)),
    responses((status = 200, description = "Web-app manifest", body = WebManifest)),
    tag = "pwa"
)]
pub async fn get_web_manifest(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    let (name, theme_color) = match state.repo.manifest_info(&slug).await {
        Ok(Some((name, primary_color))) if !name.is_empty() => (
            name,
            primary_color.unwrap_or_else(|| DEFAULT_THEME_COLOR.to_string()),
        ),
        _ => (slug.clone(), DEFAULT_THEME_COLOR.to_string()),
    };

    let manifest = build_manifest(&slug, &name, &theme_color);

    (
        [
            (header::CONTENT_TYPE, "application/manifest+json"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        axum::Json(manifest),
    )
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
    fn short_name_truncates_over_12_chars() {
        assert_eq!(short_name("Eljo's Pizza House"), "Eljo's Pizza...");
        assert_eq!(short_name("Short"), "Short");
        assert_eq!(short_name("ExactlyTwelve"), "ExactlyTwelv...");
    }

    #[tokio::test]
    async fn get_web_manifest_uses_slug_as_name_when_lookup_fails() {
        let state = test_state(FakeRepo::default());
        let response = get_web_manifest(State(state), Path("unknown-slug".to_string()))
            .await
            .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let manifest: WebManifest = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(manifest.name, "unknown-slug");
        assert_eq!(manifest.theme_color, DEFAULT_THEME_COLOR);
    }

    #[tokio::test]
    async fn get_web_manifest_uses_real_name_and_theme_color() {
        let repo = FakeRepo::default();
        repo.manifest_info.lock().unwrap().insert(
            "eljos-pizza".to_string(),
            ("Eljo's Pizza".to_string(), Some("#123456".to_string())),
        );
        let state = test_state(repo);
        let response = get_web_manifest(State(state), Path("eljos-pizza".to_string()))
            .await
            .into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let manifest: WebManifest = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(manifest.name, "Eljo's Pizza");
        assert_eq!(manifest.theme_color, "#123456");
        assert_eq!(manifest.start_url, "/s/eljos-pizza?source=pwa");
    }
}
