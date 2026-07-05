//! S4 media surface — the UNAUTHENTICATED half: the token-proxy-PUT upload endpoint (REV-S4-2)
//! and the public entry-photo upload (REV-S4-6). Deliberately mounted OUTSIDE the
//! `bearer_and_dev_gate`/`OwnerClaimsExt` layer stack `routes/owner/mod.rs` uses — by design,
//! POSSESSION of a valid upload token is the entire authorization model for the proxy-PUT
//! endpoint (exactly like a real presigned URL never carried an `Authorization` header either),
//! and entry-photo has NO auth at all, matching `spa-proxy.ts:268`'s current behavior. Both
//! still mount only when S2's auth env is present (`main.rs`) — "dark exactly when S2 is dark"
//! per the build brief, even though neither op itself needs a JWT.

use std::sync::Arc;

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Extension, Multipart, Path};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::IntoResponse;
use serde::Serialize;
use tower_http::request_id::RequestId;
use uuid::Uuid;

use domain::ErrorCode;

use crate::error::ApiError;
use crate::media::processor::{ENTRY_PHOTO_PROFILE, ImageProcessor};
use crate::media::upload_token::UploadTokenSigner;
use crate::media::validation::sniff_mime;
use crate::routes::correlation_id_string;
use crate::storage::Storage;

/// `spa-proxy.ts:271`'s `limits: { fileSize: 8 * 1024 * 1024 }` — CARRY verbatim.
pub const ENTRY_PHOTO_MAX_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone)]
pub struct MediaPublicState {
    pub storage: Arc<dyn Storage>,
    pub processor: Arc<dyn ImageProcessor>,
    /// `None` when `MEDIA_UPLOAD_TOKEN_SECRET` is absent — the proxy-PUT endpoint then 503s
    /// (mirrors `product_media::ProductMediaState.token_signer`'s posture).
    pub token_signer: Option<Arc<UploadTokenSigner>>,
    pub app_base_url: String,
    /// REV-S4-6 kill-switch (`config::MediaConfig::entry_photo_enabled`) — ops can disable the
    /// unauthenticated front door instantly without a deploy.
    pub entry_photo_enabled: bool,
}

/// Assembles BOTH unauthenticated S4 routes into their own small router — deliberately NOT
/// merged into `routes::owner::owner_catalog_router` (that router's `bearer_and_dev_gate` layer
/// would 401 every request here before the token/kill-switch checks even ran). Still mounted
/// only when S2's auth env is present (`main.rs`), matching the build brief's "dark exactly when
/// S2 is dark" instruction even though neither op needs a JWT.
///
/// Layer stack per route (REV-S4-1/S4-6): a `DefaultBodyLimit` (axum has NO implicit global
/// multipart/body cap — must be stated per route, unlike Node's `@fastify/multipart` global
/// default) sized to the largest legitimate payload for that route, PLUS entry-photo's
/// two-tier rate limit (`RateLimitLayer` per-IP 8/min CARRY, `GlobalRateLimitLayer` the REV-S4-6
/// cross-tenant cap — breaker M3's "no number" gap, see `config::MediaConfig` for the chosen
/// default + rationale).
pub fn media_public_router(
    state: MediaPublicState,
    entry_photo_global_cap_per_minute: u32,
) -> axum::Router {
    use axum::extract::DefaultBodyLimit;
    use axum::routing::{post, put};
    use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};

    let correlation_header = axum::http::HeaderName::from_static("x-correlation-id");

    axum::Router::new()
        .route(
            "/api/public/entry-photo",
            post(upload_entry_photo)
                .layer(DefaultBodyLimit::max(ENTRY_PHOTO_MAX_BYTES))
                .layer(crate::middleware::ratelimit::RateLimitLayer::new(
                    8,
                    std::time::Duration::from_secs(60),
                ))
                .layer(crate::middleware::ratelimit::GlobalRateLimitLayer::new(
                    entry_photo_global_cap_per_minute,
                    std::time::Duration::from_secs(60),
                )),
        )
        .route(
            "/api/media/upload/{token}",
            // Sized to the largest legitimate product-media item (a video clip, 25 MB —
            // `media::validation::MAX_VIDEO_BYTES`) — the token's OWN `max_bytes` claim is the
            // real per-item enforcement (checked inside the handler); this is only the outer
            // framework-level safety net so a body is never buffered past a sane ceiling.
            put(proxy_put_upload).layer(DefaultBodyLimit::max(
                usize::try_from(crate::media::validation::MAX_VIDEO_BYTES).unwrap_or(usize::MAX),
            )),
        )
        .layer(axum::Extension(state))
        .layer(PropagateRequestIdLayer::new(correlation_header.clone()))
        .layer(SetRequestIdLayer::new(correlation_header, MakeRequestUuid))
}

// ─────────────────────────────────────────────────────────────────────────────────────────────
// Token-proxy-PUT (REV-S4-2)
// ─────────────────────────────────────────────────────────────────────────────────────────────

/// `PUT /api/media/upload/{token}` — authorized SOLELY by the opaque token in the path (no
/// bearer/JWT anywhere in this handler). Validates the token (signature + TTL), cross-checks the
/// ACTUAL request against every scope the token carries (`content_type`, `max_bytes`), then
/// writes the bytes to the token's `key` verbatim — no further mime/size decisions are made here,
/// they were already made at presign time (`product_media::presign_product_media`).
#[utoipa::path(
    put,
    path = "/api/media/upload/{token}",
    params(("token" = String, Path)),
    // Explicit override: the real extractor is `axum::body::Bytes` (raw bytes, no schema),
    // which does not implement `utoipa::ToSchema` — `String` here is documentation-only and
    // does not change the actual handler's runtime extractor below.
    request_body(content = String, description = "Raw file bytes (binary)"),
    responses(
        (status = 204, description = "Stored"),
        (status = 400, description = "Invalid/expired/mismatched token", body = domain::ErrorEnvelope),
        (status = 413, description = "Body exceeds the token's declared max_bytes", body = domain::ErrorEnvelope),
        (status = 503, description = "Upload token signing unavailable", body = domain::ErrorEnvelope),
    ),
    tag = "media-upload"
)]
pub async fn proxy_put_upload(
    Extension(state): Extension<MediaPublicState>,
    Path(token): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);

    let Some(signer) = &state.token_signer else {
        return Err(ApiError::new(
            ErrorCode::ServiceUnavailable,
            "Upload token signing unavailable",
            correlation_id,
        ));
    };

    let claims = signer.verify_now(&token).map_err(|_err| {
        ApiError::new(
            ErrorCode::ValidationFailed,
            "Invalid or expired upload token",
            correlation_id.clone(),
        )
    })?;

    if u64::try_from(body.len()).unwrap_or(u64::MAX) > claims.max_bytes {
        return Err(ApiError::new(
            ErrorCode::FileTooLarge,
            "Body exceeds the token's declared size",
            correlation_id,
        ));
    }

    // Cross-check the PUT's declared Content-Type against the token's scope — the real
    // presigned-URL design signed `ContentType` into the canonical request too (a mismatched
    // header there fails signature verification server-side); this reproduces the same
    // "the declared type is part of what was authorized" property.
    let declared_ct = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if declared_ct != claims.content_type {
        return Err(ApiError::new(
            ErrorCode::ValidationFailed,
            "Content-Type does not match the authorized upload scope",
            correlation_id,
        ));
    }

    state
        .storage
        .put(&claims.key, body.to_vec())
        .await
        .map_err(|err| {
            ApiError::new(
                ErrorCode::Internal,
                format!("Failed to store object: {err}"),
                correlation_id,
            )
        })?;

    Ok(StatusCode::NO_CONTENT)
}

// ─────────────────────────────────────────────────────────────────────────────────────────────
// Entry-photo (REV-S4-6)
// ─────────────────────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct EntryPhotoResponse {
    pub key: String,
    pub url: String,
}

/// `POST /api/public/entry-photo` — anonymous entry-anchor photo upload (UX-3). CARRIES the flow
/// (`spa-proxy.ts:268-293`) with the REV-S4-6 compensating controls made concrete:
/// - **Magic-byte sniff before decode** (Q4b floor, FIX-IN-PORT over the old client-declared-
///   mimetype-header check) — closes the "claim `image/` header, send a bomb" gap and feeds
///   REV-S4-1's decode-bomb bound.
/// - **Per-IP 8/min** (`RateLimitLayer`, mounted on this route in `main.rs`) — CARRY, unchanged
///   number.
/// - **Global cap + kill-switch** (`GlobalRateLimitLayer` + `entry_photo_enabled`) — REV-S4-6,
///   see `config::MediaConfig` for the chosen number + rationale (breaker M3's "no number" gap).
/// - **Decode-bomb bound** (REV-S4-1) — via `crate::media::processor::transcode`'s `Limits`.
///
/// ## REV-S4-7 — erasure-graph link (ETHICAL-STOP, counsel) — CARRIED, NOT YET CLOSED
/// This route writes NO DB row (CARRY) — the returned `key` is echoed back by the anonymous
/// client at order-create and stored on `orders.delivery_photo_key`
/// (`packages/db/migrations/1790000000039_order-entry-photo.ts`). The council's ETHICAL-STOP
/// (REV-S4-7) requires that key be REACHABLE from the GDPR erasure graph — verified AGAINST THE
/// LIVE OLD-STACK CODE during this build: `anonymizeOrder`
/// (`apps/api/src/lib/anonymizer/index.ts:214-283`) nulls `delivery_address`/
/// `delivery_instructions`/etc but does **NOT** null `delivery_photo_key` and never purges the R2
/// object (only `avatar_key`/customers gets an object purge, `:158-176`). **The operator-approved
/// lift option (1) — extend the avatar-purge pattern to `delivery_photo_key` in the OLD-stack
/// anonymizer — has NOT shipped as of this S4 build.** That fix is old-stack TypeScript (outside
/// `rebuild/`, this lane's scope) and must land as its OWN change with its own red→green
/// guardrail + ledger row per the resolution's requirement — flagged prominently in the build
/// report as a still-open red-line PII item, not silently assumed done.
///
/// ## L1 (breaker, register) — unbound key handed to S5
/// The `key` this route returns is an unauthenticated, server-UNBOUND opaque string — nothing
/// here records which order (if any) ever claims it. Its only real protection is
/// unguessability (a random UUID) plus `/images/*`'s traversal-only guard (no tenant/auth check
/// by design — menu images are meant to be public). S5's order-create council must NOT assume S4
/// validated `entryPhotoKey` against anything; carried forward as a flag for that surface,
/// not fixed here (out of S4 scope).
#[utoipa::path(
    post,
    path = "/api/public/entry-photo",
    responses(
        (status = 200, description = "Entry photo stored", body = EntryPhotoResponse),
        (status = 400, description = "No file uploaded / invalid image", body = domain::ErrorEnvelope),
        (status = 413, description = "File exceeds maximum size", body = domain::ErrorEnvelope),
        (status = 503, description = "Entry-photo upload disabled (kill-switch)", body = domain::ErrorEnvelope),
    ),
    tag = "media-upload"
)]
pub async fn upload_entry_photo(
    Extension(state): Extension<MediaPublicState>,
    Extension(request_id): Extension<RequestId>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);

    if !state.entry_photo_enabled {
        return Err(ApiError::new(
            ErrorCode::ServiceUnavailable,
            "Entry-photo upload is disabled",
            correlation_id,
        ));
    }

    let mut buffer: Option<Vec<u8>> = None;
    let mut saw_file_field = false;
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() != Some("file") {
            continue;
        }
        saw_file_field = true;
        let bytes = field.bytes().await.map_err(|_err| {
            ApiError::new(
                ErrorCode::ValidationFailed,
                "Invalid multipart body",
                correlation_id.clone(),
            )
        })?;
        if bytes.len() > ENTRY_PHOTO_MAX_BYTES {
            return Err(ApiError::new(
                ErrorCode::FileTooLarge,
                "File exceeds maximum size",
                correlation_id,
            ));
        }
        buffer = Some(bytes.to_vec());
        break;
    }
    if !saw_file_field {
        return Err(ApiError::new(
            ErrorCode::ValidationFailed,
            "Image must be sent as the \"file\" field",
            correlation_id,
        ));
    }
    let Some(buffer) = buffer else {
        return Err(ApiError::new(
            ErrorCode::ValidationFailed,
            "No file uploaded",
            correlation_id,
        ));
    };

    // REV-S4-6 Q4b floor: reject anything that doesn't actually sniff as a real image container
    // BEFORE decode — a claimed `image/*` header proves nothing about the bytes behind it.
    if sniff_mime(&buffer).is_none() {
        return Err(ApiError::new(
            ErrorCode::ValidationFailed,
            "Must be an image",
            correlation_id,
        ));
    }

    let processed = state
        .processor
        .process(&buffer, ENTRY_PHOTO_PROFILE)
        .map_err(|_err| {
            // Don't leak decoder internals to an anonymous caller (CARRY, `spa-proxy.ts:281-284`).
            ApiError::new(
                ErrorCode::ValidationFailed,
                "Invalid image file",
                correlation_id.clone(),
            )
        })?;

    let key = format!("entry-photos/{}.webp", Uuid::new_v4());
    state.storage.put(&key, processed).await.map_err(|err| {
        ApiError::new(
            ErrorCode::Internal,
            format!("Failed to store image: {err}"),
            correlation_id.clone(),
        )
    })?;

    let url =
        crate::service::get_image_url(Some(&key), None, &state.app_base_url).unwrap_or_default();
    Ok(Json(EntryPhotoResponse { key, url }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::processor::RustImageProcessor;
    use crate::storage::LocalFsStorage;
    use axum::extract::{FromRequest, Request};
    use axum::response::Response;

    fn state(
        entry_photo_enabled: bool,
        token_signer: Option<Arc<UploadTokenSigner>>,
    ) -> MediaPublicState {
        MediaPublicState {
            storage: Arc::new(LocalFsStorage::new(std::env::temp_dir())),
            processor: Arc::new(RustImageProcessor),
            token_signer,
            app_base_url: "https://dowiz.fly.dev".to_string(),
            entry_photo_enabled,
        }
    }

    fn request_id() -> Extension<RequestId> {
        Extension(RequestId::new(axum::http::HeaderValue::from_static(
            "corr-1",
        )))
    }

    fn tiny_png() -> Vec<u8> {
        use image::codecs::png::PngEncoder;
        use image::{ExtendedColorType, ImageEncoder, Rgb, RgbImage};
        let img = RgbImage::from_pixel(4, 4, Rgb([10, 20, 30]));
        let mut out = Vec::new();
        PngEncoder::new(&mut out)
            .write_image(&img, 4, 4, ExtendedColorType::Rgb8)
            .unwrap();
        out
    }

    /// Entry-photo's REV-S4-6 Q4b sniff floor is DELIBERATELY narrower than "any image `image`
    /// can decode" — the council resolution's own wording is "reject anything not actually
    /// webp/jpeg" (reusing the EXISTING `sniffMime`, which never recognized PNG either — see
    /// `media::validation::sniff_mime`'s doc). So entry-photo's OWN happy-path fixtures must be
    /// JPEG/WebP, not PNG (`tiny_png` above is still fine for product-image/theme-logo, which
    /// have no sniff gate and accept any `image`-decodable input format).
    fn tiny_jpeg() -> Vec<u8> {
        use image::codecs::jpeg::JpegEncoder;
        use image::{Rgb, RgbImage};
        let img = RgbImage::from_pixel(4, 4, Rgb([10, 20, 30]));
        let mut out = Vec::new();
        JpegEncoder::new(&mut out).encode_image(&img).unwrap();
        out
    }

    async fn multipart_with_field(field_name: &str, bytes: &[u8]) -> Multipart {
        let boundary = "entryphotoboundary";
        let mut body = Vec::new();
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            format!(
                "Content-Disposition: form-data; name=\"{field_name}\"; filename=\"f.png\"\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(b"Content-Type: image/png\r\n\r\n");
        body.extend_from_slice(bytes);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        let request = Request::builder()
            .method("POST")
            .header(
                "content-type",
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(axum::body::Body::from(body))
            .unwrap();
        Multipart::from_request(request, &()).await.unwrap()
    }

    async fn body_json(response: Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn entry_photo_kill_switch_503s_when_disabled() {
        let err = crate::error::expect_err(
            upload_entry_photo(
                Extension(state(false, None)),
                request_id(),
                multipart_with_field("file", &tiny_png()).await,
            )
            .await
            .map(|r| r.into_response()),
        );
        assert_eq!(err.envelope.code, ErrorCode::ServiceUnavailable);
    }

    #[tokio::test]
    async fn entry_photo_rejects_wrong_field_name() {
        let err = crate::error::expect_err(
            upload_entry_photo(
                Extension(state(true, None)),
                request_id(),
                multipart_with_field("not-file", &tiny_png()).await,
            )
            .await
            .map(|r| r.into_response()),
        );
        assert_eq!(err.envelope.code, ErrorCode::ValidationFailed);
    }

    #[tokio::test]
    async fn entry_photo_rejects_non_image_bytes_even_with_an_image_looking_field() {
        // REV-S4-6 Q4b floor: bytes that don't sniff as a real image are rejected regardless of
        // what Content-Type the multipart part CLAIMED.
        let err = crate::error::expect_err(
            upload_entry_photo(
                Extension(state(true, None)),
                request_id(),
                multipart_with_field("file", b"not-an-image-at-all").await,
            )
            .await
            .map(|r| r.into_response()),
        );
        assert_eq!(err.envelope.code, ErrorCode::ValidationFailed);
    }

    #[tokio::test]
    async fn entry_photo_200s_and_returns_a_key_under_entry_photos_prefix() {
        let response = upload_entry_photo(
            Extension(state(true, None)),
            request_id(),
            multipart_with_field("file", &tiny_jpeg()).await,
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert!(json["key"].as_str().unwrap().starts_with("entry-photos/"));
        assert!(json["key"].as_str().unwrap().ends_with(".webp"));
    }

    // ── proxy-PUT ────────────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn proxy_put_503s_when_no_signer_configured() {
        let err = crate::error::expect_err(
            proxy_put_upload(
                Extension(state(true, None)),
                Path("anything".to_string()),
                request_id(),
                HeaderMap::new(),
                Bytes::from_static(b"data"),
            )
            .await
            .map(|r| r.into_response()),
        );
        assert_eq!(err.envelope.code, ErrorCode::ServiceUnavailable);
    }

    #[tokio::test]
    async fn proxy_put_rejects_an_invalid_token() {
        let signer = Arc::new(UploadTokenSigner::new(vec![5u8; 32]));
        let err = crate::error::expect_err(
            proxy_put_upload(
                Extension(state(true, Some(signer))),
                Path("garbage-token".to_string()),
                request_id(),
                HeaderMap::new(),
                Bytes::from_static(b"data"),
            )
            .await
            .map(|r| r.into_response()),
        );
        assert_eq!(err.envelope.code, ErrorCode::ValidationFailed);
    }

    #[tokio::test]
    async fn proxy_put_rejects_a_body_over_the_tokens_max_bytes() {
        let signer = Arc::new(UploadTokenSigner::new(vec![5u8; 32]));
        let token = signer.mint_now("loc/prod/image/abc.webp", "image/webp", 3);
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, "image/webp".parse().unwrap());

        let err = crate::error::expect_err(
            proxy_put_upload(
                Extension(state(true, Some(signer))),
                Path(token),
                request_id(),
                headers,
                Bytes::from_static(b"way-too-long-a-body"),
            )
            .await
            .map(|r| r.into_response()),
        );
        assert_eq!(err.envelope.code, ErrorCode::FileTooLarge);
    }

    #[tokio::test]
    async fn proxy_put_rejects_a_content_type_outside_the_tokens_scope() {
        let signer = Arc::new(UploadTokenSigner::new(vec![5u8; 32]));
        let token = signer.mint_now("loc/prod/image/abc.webp", "image/webp", 100);
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, "image/jpeg".parse().unwrap());

        let err = crate::error::expect_err(
            proxy_put_upload(
                Extension(state(true, Some(signer))),
                Path(token),
                request_id(),
                headers,
                Bytes::from_static(b"data"),
            )
            .await
            .map(|r| r.into_response()),
        );
        assert_eq!(err.envelope.code, ErrorCode::ValidationFailed);
    }

    #[tokio::test]
    async fn proxy_put_204s_and_stores_the_bytes_at_the_tokens_key() {
        let signer = Arc::new(UploadTokenSigner::new(vec![5u8; 32]));
        let token = signer.mint_now("some/loc/product/image/abc123.webp", "image/webp", 100);
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, "image/webp".parse().unwrap());
        let s = state(true, Some(signer));

        let response = proxy_put_upload(
            Extension(s.clone()),
            Path(token),
            request_id(),
            headers,
            Bytes::from_static(b"real-bytes"),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        let stored = s
            .storage
            .get("some/loc/product/image/abc123.webp")
            .await
            .unwrap();
        assert_eq!(stored, Some(b"real-bytes".to_vec()));
    }

    // ── media_public_router wiring ──────────────────────────────────────────────────────────

    #[test]
    fn media_public_router_builds_without_panicking() {
        let _router = media_public_router(state(true, None), 60);
    }

    #[tokio::test]
    async fn media_public_router_serves_entry_photo_with_no_bearer_token_required() {
        use tower::ServiceExt;
        let app = media_public_router(state(true, None), 60);
        let (boundary, body) = {
            let boundary = "routerentryphoto";
            let mut body = Vec::new();
            body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
            body.extend_from_slice(
                b"Content-Disposition: form-data; name=\"file\"; filename=\"f.jpg\"\r\n",
            );
            body.extend_from_slice(b"Content-Type: image/jpeg\r\n\r\n");
            body.extend_from_slice(&tiny_jpeg());
            body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
            (boundary.to_string(), body)
        };
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/public/entry-photo")
                    .header(
                        "content-type",
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    // Deliberately NO `authorization` header — proves this route is reachable
                    // without a bearer token, unlike anything mounted under
                    // `owner_catalog_router`'s `bearer_and_dev_gate`.
                    .body(axum::body::Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "entry-photo must be reachable with no Authorization header at all"
        );
    }
}
