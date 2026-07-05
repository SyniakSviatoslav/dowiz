//! S1 storefront-read: `getExchangeRate`.
//! Source: `apps/api/src/routes/public/rates.ts:14-48`.

use std::sync::Arc;

use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::AppState;

/// The static ALL->EUR fallback rate (`rates.ts:32`) served when `exchange_rates` is empty
/// (fresh deploy or a transient FX-worker outage) — `fetchedAt` pinned to the Unix epoch so the
/// FE can distinguish a live rate from this fallback.
pub const FALLBACK_RATE: f64 = 0.0099;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ExchangeRateResponse {
    pub base: String,
    pub target: String,
    /// FX float — deliberately NOT integer money (display-only secondary-currency rate).
    pub rate: f64,
    #[serde(rename = "fetchedAt")]
    pub fetched_at: String,
}

/// `GET /v1/rates` — source: `rates.ts:14-48`. Always 200 on the query-succeeded path (empty
/// table -> static fallback, never a 404/503).
#[utoipa::path(
    get,
    path = "/v1/rates",
    responses((status = 200, description = "Live or static-fallback ALL->EUR rate", body = ExchangeRateResponse)),
    tag = "rates"
)]
pub async fn get_exchange_rate(State(state): State<Arc<AppState>>) -> axum::response::Response {
    match state.repo.latest_exchange_rate().await {
        Ok(Some((rate, fetched_at))) => axum::Json(ExchangeRateResponse {
            base: "ALL".to_string(),
            target: "EUR".to_string(),
            rate,
            // `rates.ts:42` emits `fetched_at.toISOString()` — millis + trailing `Z`, never
            // chrono's default micros/`+00:00`.
            fetched_at: fetched_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        })
        .into_response(),
        // Empty table (fresh deploy / transient FX-worker outage) OR a query error: the fallback
        // branch is the ONLY one that sets Cache-Control (x-cache) — the live-rate path above
        // deliberately sends none.
        _ => (
            [(header::CACHE_CONTROL, "public, max-age=300")],
            axum::Json(ExchangeRateResponse {
                base: "ALL".to_string(),
                target: "EUR".to_string(),
                rate: FALLBACK_RATE,
                // `rates.ts:33` emits `new Date(0).toISOString()` — "1970-01-01T00:00:00.000Z".
                #[allow(clippy::unwrap_used, reason = "timestamp 0 is always a valid DateTime")]
                fetched_at: chrono::DateTime::<chrono::Utc>::from_timestamp(0, 0)
                    .unwrap()
                    .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            }),
        )
            .into_response(),
    }
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

    #[tokio::test]
    async fn get_exchange_rate_falls_back_when_table_empty() {
        let state = test_state(FakeRepo::default());
        let response = get_exchange_rate(State(state)).await.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "public, max-age=300",
            "the fallback branch is the ONLY one that sets Cache-Control (x-cache)"
        );
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let rate: ExchangeRateResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(rate.rate, FALLBACK_RATE);
        assert!(rate.fetched_at.starts_with("1970-01-01"));
    }

    #[tokio::test]
    async fn get_exchange_rate_serves_live_rate_without_cache_header() {
        let repo = FakeRepo::default();
        *repo.exchange_rate.lock().unwrap() = Some((0.0105, chrono::Utc::now()));
        let state = test_state(repo);
        let response = get_exchange_rate(State(state)).await.into_response();
        assert!(response.headers().get(header::CACHE_CONTROL).is_none());
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let rate: ExchangeRateResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(rate.rate, 0.0105);
    }
}
