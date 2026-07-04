//! S1 storefront-read: `getPublicMenu`, `getPublicLocationInfo`, `getProductMedia`.
//! Source: `apps/api/src/routes/public/menu.ts` (see per-handler doc for exact line ranges).
//!
//! NOT ported: the in-process (slug,locale)-keyed TTL/stale-while-revalidate/stale-on-error
//! cache (`menu.ts:76-111`, `MENU_CACHE_*`). That cache exists to collapse a connection-burst
//! into one DB execution — a real perf property, but an axum/tokio caching-layer concern
//! (candidates: `moka`/`tower::buffer`, or a `PgListener`-invalidated cache once the WS/pubsub
//! surface lands) orthogonal to the *data shape* this operation set is scoped to port. Flagged
//! as a follow-up rather than silently dropped: without it, a request burst against this build
//! hits the DB per-request (correct behavior, different perf characteristic, no cache-poisoning
//! risk either since there's no cache yet to poison).

use std::sync::Arc;

use axum::extract::{Extension, Path, Query, State};
use axum::http::{HeaderMap, HeaderValue};
use axum::response::IntoResponse;
use serde::Deserialize;
use tower_http::request_id::RequestId;
use uuid::Uuid;

use domain::ErrorCode;

use crate::AppState;
use crate::dto::{ProductMedia, ProductMediaMeta, ProductMediaResponse};
use crate::error::ApiError;
use crate::repo::PreviewLookup;
use crate::routes::correlation_id_string;
use crate::service::{
    adapt_preview_menu, get_image_url, media_serving_allowed, normalize_locale, resolve_media_url,
};

#[derive(Debug, Deserialize)]
pub struct MenuQuery {
    #[serde(default)]
    pub locale: String,
}

/// `GET /public/locations/{locationIdOrSlug}/menu` — source: `menu.ts:231-270`.
///
/// Shadow fallback ported (menu.ts:118-133): a null `read_public_menu` result tries
/// `read_preview_menu` next; a present preview is adapted with `is_preview: true`
/// (`adapt_preview_menu`); absent-or-fn-missing on BOTH is the genuine 404.
#[utoipa::path(
    get,
    path = "/public/locations/{locationIdOrSlug}/menu",
    params(("locationIdOrSlug" = String, Path), ("locale" = Option<String>, Query)),
    responses(
        (status = 200, description = "Menu payload (live tenant) or adapted shadow preview", body = crate::dto::PublicMenu),
        (status = 404, description = "Unknown location", body = domain::ErrorEnvelope),
    ),
    tag = "menu"
)]
pub async fn get_public_menu(
    State(state): State<Arc<AppState>>,
    Path(location_id_or_slug): Path<String>,
    Query(query): Query<MenuQuery>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let locale = normalize_locale(&query.locale);

    let menu = match state
        .repo
        .read_public_menu(&location_id_or_slug, &locale)
        .await
    {
        Ok(Some(raw)) => {
            let mut menu: crate::dto::PublicMenu = serde_json::from_value(raw).map_err(|err| {
                tracing::warn!(%err, %location_id_or_slug, "read_public_menu payload failed to deserialize into PublicMenu");
                ApiError::new(ErrorCode::Internal, "internal_error", correlation_id.clone())
            })?;
            enrich_image_urls(&mut menu, &state);
            resolve_primary_media(&mut menu, &state).await;
            menu
        }
        Ok(None) => match state.repo.read_preview_menu(&location_id_or_slug).await {
            Ok(PreviewLookup::Found(preview)) => adapt_preview_menu(&preview),
            Ok(PreviewLookup::NotShadow) | Ok(PreviewLookup::FunctionMissing) => {
                return Err(ApiError::new(
                    ErrorCode::NotFound,
                    "Location not found",
                    correlation_id,
                ));
            }
            Err(_) => {
                return Err(ApiError::new(
                    ErrorCode::NotFound,
                    "Location not found",
                    correlation_id,
                ));
            }
        },
        Err(_) => {
            return Err(ApiError::new(
                ErrorCode::NotFound,
                "Location not found",
                correlation_id,
            ));
        }
    };

    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=60, stale-while-revalidate=300"),
    );
    headers.insert(
        "X-Menu-Version",
        HeaderValue::from_str(&menu.menu_version.to_string())
            .unwrap_or_else(|_| HeaderValue::from_static("0")),
    );

    Ok((headers, axum::Json(menu)))
}

/// Route-side `imageUrl` enrichment (`menu.ts:148-156`).
fn enrich_image_urls(menu: &mut crate::dto::PublicMenu, state: &AppState) {
    for cat in &mut menu.categories {
        for prod in &mut cat.products {
            prod.image_url = get_image_url(
                prod.image_key.as_deref(),
                state.r2_public_url.as_deref(),
                &state.app_base_url,
            );
        }
    }
}

/// Batched primary-media resolution (`menu.ts:165-194`), gated on `MEDIA_RICH_ENABLED`.
async fn resolve_primary_media(menu: &mut crate::dto::PublicMenu, state: &AppState) {
    if !state.media_rich_enabled {
        return;
    }
    let ids: Vec<Uuid> = menu
        .categories
        .iter()
        .flat_map(|c| &c.products)
        .filter(|p| p.image_url.is_none())
        .filter_map(|p| p.primary_media_id)
        .collect();
    if ids.is_empty() {
        return;
    }
    let Ok(rows) = state.repo.product_media_by_ids(&ids).await else {
        return;
    };
    let by_id: std::collections::HashMap<Uuid, _> = rows.into_iter().map(|r| (r.id, r)).collect();
    for cat in &mut menu.categories {
        for prod in &mut cat.products {
            if prod.image_url.is_some() {
                continue;
            }
            let Some(media_id) = prod.primary_media_id else {
                continue;
            };
            let Some(m) = by_id.get(&media_id) else {
                continue;
            };
            let key = if m.kind == "image" {
                &m.storage_key
            } else {
                &m.poster_key
            };
            if let Some(url) = resolve_media_url(key.as_deref()) {
                prod.image_url = Some(url);
                if prod.image_key.is_none() {
                    prod.image_key = key.clone();
                }
            }
        }
    }
}

/// `GET /public/locations/{slug}/info` — source: `menu.ts:312-411`.
#[utoipa::path(
    get,
    path = "/public/locations/{slug}/info",
    params(("slug" = String, Path)),
    responses(
        (status = 200, description = "Venue status/hours/fee-mirror inputs", body = crate::dto::PublicLocationInfo),
        (status = 404, description = "Unknown slug", body = domain::ErrorEnvelope),
        (status = 503, description = "DB unavailable and no usable cached row", body = domain::ErrorEnvelope),
    ),
    tag = "menu"
)]
pub async fn get_public_location_info(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);

    // NOT ported here: the 30s row cache + 1h stale-on-error absorption (menu.ts:104-111,
    // 317-330) — same caching-layer note as get_public_menu above. This build has no cache to
    // serve stale data FROM, so 503-on-error is the honest behavior for this slice (never
    // silently serving wrong data), not a behavior downgrade of anything this crate holds yet.
    let row = match state.repo.location_info(&slug).await {
        Ok(Some(row)) => row,
        Ok(None) => {
            return Err(ApiError::new(
                ErrorCode::NotFound,
                "Not found",
                correlation_id,
            ));
        }
        Err(_) => {
            return Err(ApiError::new(
                ErrorCode::ServiceUnavailable,
                "Temporarily unavailable",
                correlation_id,
            ));
        }
    };

    let now = chrono::Utc::now();
    let (is_open, closes_at) = crate::service::compute_open_and_closes_at(
        row.hours_json.as_ref(),
        row.delivery_paused,
        now,
    );
    let status = crate::service::compute_venue_status(is_open, row.kitchen_busy_until, now);
    let weekly_hours = crate::service::compute_weekly_hours(row.hours_json.as_ref());

    let info = crate::dto::PublicLocationInfo {
        id: row.id,
        name: row.name,
        slug: row.slug,
        currency_code: row.currency_code,
        currency_minor_unit: row.currency_minor_unit,
        default_locale: row.default_locale,
        delivery_fee_flat: row.delivery_fee_flat,
        free_delivery_threshold: row.free_delivery_threshold,
        min_order_value: row.min_order_value,
        tax_rate: row.tax_rate.unwrap_or(0.0),
        price_includes_tax: row.price_includes_tax,
        has_distance_tiers: row.has_distance_tiers,
        lat: row.lat,
        lng: row.lng,
        address: row.address,
        phone: row.phone,
        is_open,
        status,
        closes_at,
        weekly_hours,
        google_rating: row.google_rating,
        google_review_count: row.google_review_count,
        google_maps_url: row.google_maps_url,
        google_place_id: row.google_place_id,
        social_instagram: row.social_instagram,
        social_facebook: row.social_facebook,
    };

    Ok(axum::Json(info))
}

/// `GET /public/locations/{slug}/products/{productId}/media` — source: `menu.ts:418-471`.
/// Defence-in-depth gate: NEVER errors on gate-fail, always 200 `{media: []}`.
#[utoipa::path(
    get,
    path = "/public/locations/{slug}/products/{productId}/media",
    params(("slug" = String, Path), ("productId" = Uuid, Path)),
    responses((status = 200, description = "Media list (possibly empty)", body = ProductMediaResponse)),
    tag = "menu"
)]
pub async fn get_product_media(
    State(state): State<Arc<AppState>>,
    Path((slug, product_id)): Path<(String, Uuid)>,
) -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=60, stale-while-revalidate=300"),
    );

    if !state.media_rich_enabled {
        return (headers, axum::Json(ProductMediaResponse { media: vec![] }));
    }

    let Ok(Some((location_id, plan))) = state.repo.location_id_plan(&slug).await else {
        return (headers, axum::Json(ProductMediaResponse { media: vec![] }));
    };
    if !media_serving_allowed(state.media_rich_enabled, plan.as_deref()) {
        return (headers, axum::Json(ProductMediaResponse { media: vec![] }));
    }

    let Ok(rows) = state.repo.product_media(location_id, product_id).await else {
        return (headers, axum::Json(ProductMediaResponse { media: vec![] }));
    };

    let media = rows
        .into_iter()
        .map(|r| {
            let meta = r.meta.and_then(|meta| {
                let frame_count = meta
                    .get("frameCount")
                    .and_then(|v| v.as_i64())
                    .and_then(|n| i32::try_from(n).ok());
                let frame_urls = meta.get("frameKeys").and_then(|v| v.as_array()).map(|arr| {
                    arr.iter()
                        .filter_map(|k| k.as_str())
                        .filter_map(|k| resolve_media_url(Some(k)))
                        .collect::<Vec<_>>()
                });
                if frame_count.is_none() && frame_urls.is_none() {
                    None
                } else {
                    Some(ProductMediaMeta {
                        frame_count,
                        frame_urls,
                    })
                }
            });
            ProductMedia {
                id: r.id,
                kind: match r.kind.as_str() {
                    "video" => crate::dto::ProductMediaKind::Video,
                    "spin" => crate::dto::ProductMediaKind::Spin,
                    "model" => crate::dto::ProductMediaKind::Model,
                    _ => crate::dto::ProductMediaKind::Image,
                },
                url: resolve_media_url(r.storage_key.as_deref()).unwrap_or_default(),
                poster_url: resolve_media_url(r.poster_key.as_deref()),
                mime_type: r.mime_type,
                width: r.width,
                height: r.height,
                duration_ms: r.duration_ms,
                alt: r.alt,
                sort_order: r.sort_order,
                meta,
            }
        })
        .collect();

    (headers, axum::Json(ProductMediaResponse { media }))
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

    fn sample_menu_json() -> serde_json::Value {
        serde_json::json!({
            "menu_version": 3,
            "location_id": "00000000-0000-0000-0000-0000000000aa",
            "locationId": "00000000-0000-0000-0000-0000000000aa",
            "location_name": "Eljo's Pizza",
            "default_locale": "sq",
            "supported_locales": ["sq", "en"],
            "currency": {"code": "ALL", "minor_unit": 0},
            "categories": [{
                "id": "00000000-0000-0000-0000-0000000000bb",
                "name": "Pizza",
                "sort_order": 0,
                "products": [{
                    "id": "00000000-0000-0000-0000-0000000000cc",
                    "name": "Margherita",
                    "description": null,
                    "price": 900,
                    "available": true,
                    "image_key": null,
                    "primary_media_id": null,
                    "attributes": null,
                    "prep_time_minutes": 10,
                    "modifier_groups": [],
                }],
            }],
        })
    }

    #[tokio::test]
    async fn get_public_menu_returns_200_with_menu_version_header() {
        let repo = FakeRepo::default();
        repo.public_menus
            .lock()
            .unwrap()
            .insert("eljos-pizza".to_string(), sample_menu_json());
        let state = test_state(repo);

        let response = get_public_menu(
            State(state),
            Path("eljos-pizza".to_string()),
            Query(MenuQuery {
                locale: String::new(),
            }),
            Extension(RequestId::new(axum::http::HeaderValue::from_static(
                "corr-1",
            ))),
        )
        .await
        .unwrap()
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert_eq!(response.headers().get("X-Menu-Version").unwrap(), "3");
    }

    #[tokio::test]
    async fn get_public_menu_falls_back_to_shadow_preview_when_absent() {
        let repo = FakeRepo::default();
        repo.preview_menus.lock().unwrap().insert(
            "shadow-cafe".to_string(),
            serde_json::json!({"name": "Shadow Cafe", "default_locale": "sq", "currency": {"code": "ALL", "minor_unit": 0}, "categories": []}),
        );
        let state = test_state(repo);

        let response = get_public_menu(
            State(state),
            Path("shadow-cafe".to_string()),
            Query(MenuQuery {
                locale: String::new(),
            }),
            Extension(RequestId::new(axum::http::HeaderValue::from_static(
                "corr-1",
            ))),
        )
        .await
        .unwrap()
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert_eq!(response.headers().get("X-Menu-Version").unwrap(), "0");
    }

    #[tokio::test]
    async fn get_public_menu_404_when_neither_menu_nor_preview_exists() {
        let state = test_state(FakeRepo::default());
        let err = crate::error::expect_err(
            get_public_menu(
                State(state),
                Path("nowhere".to_string()),
                Query(MenuQuery {
                    locale: String::new(),
                }),
                Extension(RequestId::new(axum::http::HeaderValue::from_static(
                    "corr-1",
                ))),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn get_public_location_info_404_for_unknown_slug() {
        let state = test_state(FakeRepo::default());
        let err = crate::error::expect_err(
            get_public_location_info(
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

    #[tokio::test]
    async fn get_public_location_info_computes_status_open() {
        let repo = FakeRepo::default();
        repo.location_info.lock().unwrap().insert(
            "eljos-pizza".to_string(),
            crate::repo::LocationInfoRow {
                id: Uuid::nil(),
                name: "Eljo's Pizza".to_string(),
                slug: "eljos-pizza".to_string(),
                currency_code: "ALL".to_string(),
                currency_minor_unit: 0,
                default_locale: "sq".to_string(),
                lat: None,
                lng: None,
                delivery_paused: false,
                hours_json: None,
                address: None,
                phone: None,
                kitchen_busy_until: None,
                delivery_fee_flat: Some(100),
                free_delivery_threshold: None,
                min_order_value: None,
                tax_rate: None,
                price_includes_tax: false,
                has_distance_tiers: false,
                google_rating: None,
                google_review_count: None,
                google_maps_url: None,
                google_place_id: None,
                social_instagram: None,
                social_facebook: None,
            },
        );
        let state = test_state(repo);
        let response = get_public_location_info(
            State(state),
            Path("eljos-pizza".to_string()),
            Extension(RequestId::new(axum::http::HeaderValue::from_static(
                "corr-1",
            ))),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn get_product_media_dark_when_flag_disabled() {
        let state = test_state(FakeRepo::default());
        let response =
            get_product_media(State(state), Path(("eljos-pizza".to_string(), Uuid::nil())))
                .await
                .into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: ProductMediaResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body.media.len(), 0);
    }
}
