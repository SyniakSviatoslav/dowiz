//! S4 media surface — ADR-0002 product-media seam. Ports `apps/api/src/routes/owner/product-media.ts`
//! (presign, confirm, set-primary, reorder, available-toggle) — see
//! `docs/design/rebuild-media-s4-council/resolution.md` for the frozen REV-S4-1..9 set.
//!
//! ## Op list (method, path, TS line)
//! 1. POST  `/api/owner/menu/products/{productId}/media/presign` (`product-media.ts:82`)
//! 2. POST  `/api/owner/menu/products/{productId}/media/confirm` (`:178`)
//! 3. POST  `/api/owner/menu/products/{productId}/media/{mediaId}/set-primary` (`:271`)
//! 4. POST  `/api/owner/menu/products/{productId}/media/reorder` (`:303`)
//! 5. PATCH `/api/owner/menu/products/{productId}/media/{mediaId}` (`:330`, available toggle)
//!
//! None of these carry `:locationId` in the URL — `product-media.ts` resolves the caller's
//! location via `getOwnerLocation` (the JWT-alias pattern), so every handler here uses
//! [`resolve_owner_location`], matching `products.rs` ops 11-14.
//!
//! ## REV-S4-2 — token-proxy-PUT replaces the rejected hand-rolled SigV4 presign
//! `presign_product_media` no longer returns a real S3-presigned URL. It mints an opaque,
//! HMAC-signed [`crate::media::upload_token`] per item (scoped to `key`+`content_type`+
//! `max_bytes`, 300s TTL — `TOKEN_TTL_SECONDS` parity with the old `PRESIGN_TTL_SECONDS`) and
//! returns a URL pointing at this crate's own token-proxy-PUT endpoint
//! (`routes/media_upload.rs`). The RESPONSE SHAPE — `{uploads:[{key,url,sha256,poster}],
//! expiresIn}` — is unchanged, preserving the FE contract per the council RESOLVE.
//!
//! ## REV-S4-5 — every DB touch through `with_user` + in-txn membership
//! `product-media.ts` itself was INCONSISTENT here: `confirm` already used `withTenant`
//! (`:242`), but `getOwnerLocation`'s membership read and `productInLocation` both ran on the
//! raw pool. This port routes EVERY op (including presign's read-only budget/product checks)
//! through `db::with_user` + [`super::assert_active_owner_membership`] — the S3 pattern — rather
//! than reproducing that inconsistency (a judgment call: `product_media`'s own `public_select`
//! RLS policy makes the specific reads harmless either way, but uniformity here means no future
//! reader has to re-derive which of five near-identical ops was the "safe" raw one).
//!
//! ## Quirk register (CARRY, unless noted)
//! - Q-PRESIGN-TTL: 300s TTL — CARRY, now enforced by the upload token's own TTL.
//! - Q-KEY-DERIVE: keys are server-built from the membership-resolved `locId`; `confirm` rejects
//!   any `storageKey` outside `${locId}/${productId}/` — CARRY verbatim (the sole cross-tenant
//!   WRITE boundary on the object plane).
//! - Q-SHA-DECLARED: the `product_media` key uses the CLIENT-declared sha256, never re-verified
//!   against the stored bytes (confirm sniffs the mime magic bytes, not a re-hash) — CARRY (a
//!   naming scheme, not an integrity proof; Q7 in the council packet is recorded, not resolved,
//!   here).
//! - Q-NO-SVG: the mime allow-list never admits SVG — CARRY (a security invariant).
//! - L2 (breaker): `confirm`'s `sort_order` is a read-then-insert race under concurrent confirms
//!   for one product — CARRY (owner-only, low-frequency).
//! - Q-BUDGET-SHAPE: the 413 budget-exceeded body is a bare non-envelope
//!   `{error,used,incoming,limit}` (`product-media.ts:128-131`) — CARRY, see [`BudgetExceededBody`].

use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Path};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use tower_http::request_id::RequestId;
use uuid::Uuid;

use domain::ErrorCode;

use crate::auth::extractors::OwnerClaimsExt;
use crate::db;
use crate::error::ApiError;
use crate::media::upload_token::UploadTokenSigner;
use crate::media::validation::{
    self, check_budget, check_frame_count, ext_for_mime, is_allowed_mime, is_allowed_poster_mime,
    max_bytes_for_mime, sum_incoming_bytes,
};
use crate::repo::RepoError;
use crate::routes::correlation_id_string;
use crate::storage::Storage;

use super::{assert_active_owner_membership, resolve_owner_location};

const KINDS: [&str; 4] = ["image", "video", "spin", "model"];
/// `PRESIGN_TTL_SECONDS` parity (`product-media.ts:29`).
const PRESIGN_TTL_SECONDS: u64 = crate::media::upload_token::TOKEN_TTL_SECONDS;

// ─────────────────────────────────────────────────────────────────────────────────────────────
// State + repo trait
// ─────────────────────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ProductMediaState {
    pub auth: crate::auth::AuthState,
    pub repo: Arc<dyn ProductMediaRepo>,
    pub storage: Arc<dyn Storage>,
    /// `None` when `MEDIA_UPLOAD_TOKEN_SECRET` is absent — `presign` then degrades to 503
    /// SERVICE_UNAVAILABLE (mirrors the old TS behavior when R2 config was unset,
    /// `product-media.ts:138-140`), never a boot failure.
    pub token_signer: Option<Arc<UploadTokenSigner>>,
    pub app_base_url: String,
}

pub enum PrecheckOutcome {
    NotFound,
    Ok { used_bytes: u64 },
}

pub struct ConfirmInput {
    pub kind: String,
    pub storage_key: String,
    pub mime_type: String,
    pub bytes: u64,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub duration_ms: Option<i32>,
    pub poster_key: Option<String>,
    pub alt: Option<String>,
    pub frame_keys: Vec<String>,
}

pub enum ConfirmOutcome {
    NotFound,
    Ok { id: Uuid, sort_order: i32 },
}

pub enum SetPrimaryOutcome {
    NotFound,
    Ok { changed: bool },
}

#[async_trait::async_trait]
pub trait ProductMediaRepo: Send + Sync {
    /// Membership assert + `productInLocation` check + `locationUsedBytes` read, in one
    /// `with_user`-seated transaction (REV-S4-5).
    async fn presign_precheck(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
    ) -> Result<PrecheckOutcome, RepoError>;

    async fn confirm(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
        input: ConfirmInput,
    ) -> Result<ConfirmOutcome, RepoError>;

    async fn set_primary(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
        media_id: Uuid,
    ) -> Result<SetPrimaryOutcome, RepoError>;

    /// Always `Ok(())` on a passing membership check — CARRY: a wrong product/location row
    /// simply matches 0 rows per iteration, exactly like the old TS's unchecked UPDATE loop.
    /// `Err(RepoError)` only for a genuine membership failure (mapped to 404 by the handler) or a
    /// real DB error.
    async fn reorder(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
        order: Vec<Uuid>,
    ) -> Result<bool, RepoError>;

    async fn set_available(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
        media_id: Uuid,
        available: bool,
    ) -> Result<Option<bool>, RepoError>;
}

fn map_txn_err(err: db::TenantTxnError) -> RepoError {
    match err {
        db::TenantTxnError::Begin(e)
        | db::TenantTxnError::SetTenant(e)
        | db::TenantTxnError::Work(e)
        | db::TenantTxnError::Commit(e) => RepoError(e),
        db::TenantTxnError::WorkThenRollbackFailed { work, .. } => RepoError(work),
    }
}

// ─────────────────────────────────────────────────────────────────────────────────────────────
// Wire DTOs
// ─────────────────────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PresignItem {
    pub mime_type: String,
    pub bytes: u64,
    pub sha256: String,
    #[serde(default)]
    pub poster: bool,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct PresignRequest {
    pub kind: String,
    #[serde(default)]
    pub items: Vec<PresignItem>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PresignUpload {
    pub key: String,
    pub url: String,
    pub sha256: String,
    pub poster: bool,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PresignResponse {
    pub uploads: Vec<PresignUpload>,
    pub expires_in: u64,
}

/// Q-BUDGET-SHAPE (CARRY): the 413 budget-exceeded response is a bare, non-`ErrorEnvelope` body
/// (`product-media.ts:128-131`) — mirrors how S2 carries its own divergent bare shapes
/// (`ClaimBareError` etc., `crates/api/src/auth/error.rs`).
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct BudgetExceededBody {
    pub error: String,
    pub used: u64,
    pub incoming: u64,
    pub limit: u64,
}

impl IntoResponse for BudgetExceededBody {
    fn into_response(self) -> Response {
        (StatusCode::PAYLOAD_TOO_LARGE, Json(self)).into_response()
    }
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmRequest {
    pub kind: String,
    pub storage_key: String,
    pub mime_type: String,
    #[serde(default)]
    pub bytes: u64,
    #[serde(default)]
    pub width: Option<i32>,
    #[serde(default)]
    pub height: Option<i32>,
    #[serde(default)]
    pub duration_ms: Option<i32>,
    #[serde(default)]
    pub poster_key: Option<String>,
    #[serde(default)]
    pub alt: Option<String>,
    #[serde(default)]
    pub frame_keys: Vec<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmResponse {
    pub id: Uuid,
    pub sort_order: i32,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ReorderRequest {
    pub order: Vec<Uuid>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct AvailableToggleRequest {
    pub available: bool,
}

// ─────────────────────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/api/owner/menu/products/{productId}/media/presign",
    params(("productId" = Uuid, Path)),
    responses(
        (status = 200, description = "Token-proxy-PUT upload URLs", body = PresignResponse),
        (status = 400, description = "Invalid media kind/item shape", body = domain::ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = domain::ErrorEnvelope),
        (status = 404, description = "Product not found", body = domain::ErrorEnvelope),
        (status = 413, description = "Storage budget exceeded", body = BudgetExceededBody),
        (status = 503, description = "Upload token signing unavailable", body = domain::ErrorEnvelope),
    ),
    tag = "owner-media"
)]
pub async fn presign_product_media(
    Extension(state): Extension<ProductMediaState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(product_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<PresignRequest>,
) -> Result<Response, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let location_id = resolve_owner_location(&state.auth, &owner, &correlation_id).await?;

    if !KINDS.contains(&body.kind.as_str()) {
        return Err(ApiError::new(
            ErrorCode::InvalidMedia,
            "Invalid media kind",
            correlation_id,
        ));
    }
    if body.items.is_empty() {
        return Err(ApiError::new(
            ErrorCode::ValidationFailed,
            "No items",
            correlation_id,
        ));
    }
    if body.kind == "spin" {
        let fc = check_frame_count(body.items.len());
        if !fc.ok {
            return Err(ApiError::new(
                ErrorCode::InvalidMedia,
                fc.reason.unwrap_or_default(),
                correlation_id,
            ));
        }
    }

    for item in &body.items {
        if !is_sha256_shape(&item.sha256) {
            return Err(ApiError::new(
                ErrorCode::InvalidMedia,
                "Invalid sha256",
                correlation_id,
            ));
        }
        let allowed = if item.poster {
            is_allowed_poster_mime(&item.mime_type)
        } else {
            is_allowed_mime(&item.mime_type)
        };
        if !allowed {
            return Err(ApiError::new(
                ErrorCode::InvalidMedia,
                format!("Disallowed mime: {}", item.mime_type),
                correlation_id,
            ));
        }
        if item.bytes == 0 {
            return Err(ApiError::new(
                ErrorCode::InvalidMedia,
                "Invalid bytes",
                correlation_id,
            ));
        }
        if item.bytes > max_bytes_for_mime(&item.mime_type) {
            return Err(ApiError::new(
                ErrorCode::FileTooLarge,
                format!("File exceeds size limit for {}", item.mime_type),
                correlation_id,
            ));
        }
    }

    let precheck = state
        .repo
        .presign_precheck(owner.user_id, location_id, product_id)
        .await
        .map_err(|_err| {
            ApiError::new(
                ErrorCode::Internal,
                "internal_error",
                correlation_id.clone(),
            )
        })?;
    let used_bytes = match precheck {
        PrecheckOutcome::NotFound => {
            return Err(ApiError::new(
                ErrorCode::NotFound,
                "Product not found",
                correlation_id,
            ));
        }
        PrecheckOutcome::Ok { used_bytes } => used_bytes,
    };

    let incoming: u64 = sum_incoming_bytes(&body.items.iter().map(|i| i.bytes).collect::<Vec<_>>());
    let budget = check_budget(used_bytes, incoming, validation::LOCATION_BUDGET_BYTES);
    if !budget.ok {
        return Ok(BudgetExceededBody {
            error: "Storage budget exceeded".to_string(),
            used: budget.used,
            incoming: budget.incoming,
            limit: budget.limit,
        }
        .into_response());
    }

    let Some(signer) = &state.token_signer else {
        return Err(ApiError::new(
            ErrorCode::ServiceUnavailable,
            "Upload token signing unavailable",
            correlation_id,
        ));
    };

    let mut uploads = Vec::with_capacity(body.items.len());
    for item in &body.items {
        let Some(ext) = ext_for_mime(&item.mime_type) else {
            return Err(ApiError::new(
                ErrorCode::InvalidMedia,
                format!("Disallowed mime: {}", item.mime_type),
                correlation_id,
            ));
        };
        let sub_kind = if item.poster {
            format!("{}-poster", body.kind)
        } else {
            body.kind.clone()
        };
        let key = format!(
            "{location_id}/{product_id}/{sub_kind}/{}.{ext}",
            &item.sha256[..12]
        );
        let token = signer.mint_now(&key, &item.mime_type, item.bytes);
        let url = format!("{}/api/media/upload/{token}", state.app_base_url);
        uploads.push(PresignUpload {
            key,
            url,
            sha256: item.sha256.clone(),
            poster: item.poster,
        });
    }

    Ok(Json(PresignResponse {
        uploads,
        expires_in: PRESIGN_TTL_SECONDS,
    })
    .into_response())
}

fn is_sha256_shape(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

#[utoipa::path(
    post,
    path = "/api/owner/menu/products/{productId}/media/confirm",
    params(("productId" = Uuid, Path)),
    responses(
        (status = 201, description = "Media row created", body = ConfirmResponse),
        (status = 400, description = "Invalid media/validation failure", body = domain::ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = domain::ErrorEnvelope),
        (status = 404, description = "Product not found", body = domain::ErrorEnvelope),
    ),
    tag = "owner-media"
)]
pub async fn confirm_product_media(
    Extension(state): Extension<ProductMediaState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(product_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<ConfirmRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let location_id = resolve_owner_location(&state.auth, &owner, &correlation_id).await?;

    if !KINDS.contains(&body.kind.as_str()) {
        return Err(ApiError::new(
            ErrorCode::InvalidMedia,
            "Invalid media kind",
            correlation_id,
        ));
    }
    if body.storage_key.is_empty() {
        return Err(ApiError::new(
            ErrorCode::ValidationFailed,
            "Missing storageKey",
            correlation_id,
        ));
    }
    if !is_allowed_mime(&body.mime_type) {
        return Err(ApiError::new(
            ErrorCode::InvalidMedia,
            format!("Disallowed mime: {}", body.mime_type),
            correlation_id,
        ));
    }
    // Q-KEY-DERIVE (CARRY verbatim): the key must live under THIS tenant's product prefix.
    let expected_prefix = format!("{location_id}/{product_id}/");
    if !body.storage_key.starts_with(&expected_prefix) {
        return Err(ApiError::new(
            ErrorCode::ValidationFailed,
            "Key outside tenant prefix",
            correlation_id,
        ));
    }
    // Defense-in-depth (SSG sentinel S4 LOW): reject path traversal after the prefix so the
    // tenant-prefix check can't be walked out of (unexploitable on R2's literal keys, but the
    // LocalFs dev provider would otherwise honour `..`).
    if body.storage_key.contains("..") {
        return Err(ApiError::new(
            ErrorCode::ValidationFailed,
            "Invalid storageKey",
            correlation_id,
        ));
    }
    if body.kind == "spin" {
        let fc = check_frame_count(body.frame_keys.len());
        if !fc.ok {
            return Err(ApiError::new(
                ErrorCode::InvalidMedia,
                fc.reason.unwrap_or_default(),
                correlation_id,
            ));
        }
    }

    // Re-validate the actual stored bytes by sniffing the magic number (CARRY —
    // `product-media.ts:220-233`; a no-op when the object is absent, e.g. LocalFs in dev).
    if let Ok(Some(buf)) = state.storage.get(&body.storage_key).await
        && !validation::magic_bytes_match(&buf, &body.mime_type)
    {
        return Err(ApiError::new(
            ErrorCode::InvalidMedia,
            "Stored bytes do not match declared type",
            correlation_id,
        ));
    }

    let input = ConfirmInput {
        kind: body.kind,
        storage_key: body.storage_key,
        mime_type: body.mime_type,
        bytes: body.bytes,
        width: body.width,
        height: body.height,
        duration_ms: body.duration_ms,
        poster_key: body.poster_key,
        alt: body.alt,
        frame_keys: body.frame_keys,
    };

    let outcome = state
        .repo
        .confirm(owner.user_id, location_id, product_id, input)
        .await
        .map_err(|_err| {
            ApiError::new(
                ErrorCode::Internal,
                "internal_error",
                correlation_id.clone(),
            )
        })?;
    match outcome {
        ConfirmOutcome::NotFound => Err(ApiError::new(
            ErrorCode::NotFound,
            "Product not found",
            correlation_id,
        )),
        ConfirmOutcome::Ok { id, sort_order } => Ok((
            StatusCode::CREATED,
            Json(ConfirmResponse { id, sort_order }),
        )),
    }
}

#[utoipa::path(
    post,
    path = "/api/owner/menu/products/{productId}/media/{mediaId}/set-primary",
    params(("productId" = Uuid, Path), ("mediaId" = Uuid, Path)),
    responses(
        (status = 200, description = "Primary media set"),
        (status = 401, description = "Unauthorized", body = domain::ErrorEnvelope),
        (status = 404, description = "Not found", body = domain::ErrorEnvelope),
    ),
    tag = "owner-media"
)]
pub async fn set_primary_product_media(
    Extension(state): Extension<ProductMediaState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path((product_id, media_id)): Path<(Uuid, Uuid)>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let location_id = resolve_owner_location(&state.auth, &owner, &correlation_id).await?;

    let outcome = state
        .repo
        .set_primary(owner.user_id, location_id, product_id, media_id)
        .await
        .map_err(|_err| {
            ApiError::new(
                ErrorCode::Internal,
                "internal_error",
                correlation_id.clone(),
            )
        })?;
    match outcome {
        SetPrimaryOutcome::NotFound => Err(ApiError::new(
            ErrorCode::NotFound,
            "Not found",
            correlation_id,
        )),
        SetPrimaryOutcome::Ok { changed } => {
            Ok(Json(serde_json::json!({ "ok": true, "changed": changed })))
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/owner/menu/products/{productId}/media/reorder",
    params(("productId" = Uuid, Path)),
    request_body = ReorderRequest,
    responses(
        (status = 200, description = "Reordered"),
        (status = 400, description = "order must be an array of media ids", body = domain::ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = domain::ErrorEnvelope),
    ),
    tag = "owner-media"
)]
pub async fn reorder_product_media(
    Extension(state): Extension<ProductMediaState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(product_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<ReorderRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let location_id = resolve_owner_location(&state.auth, &owner, &correlation_id).await?;

    state
        .repo
        .reorder(owner.user_id, location_id, product_id, body.order)
        .await
        .map_err(|_err| ApiError::new(ErrorCode::Internal, "internal_error", correlation_id))?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

#[utoipa::path(
    patch,
    path = "/api/owner/menu/products/{productId}/media/{mediaId}",
    params(("productId" = Uuid, Path), ("mediaId" = Uuid, Path)),
    request_body = AvailableToggleRequest,
    responses(
        (status = 200, description = "Availability toggled"),
        (status = 401, description = "Unauthorized", body = domain::ErrorEnvelope),
        (status = 404, description = "Not found", body = domain::ErrorEnvelope),
    ),
    tag = "owner-media"
)]
pub async fn set_product_media_available(
    Extension(state): Extension<ProductMediaState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path((product_id, media_id)): Path<(Uuid, Uuid)>,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<AvailableToggleRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let location_id = resolve_owner_location(&state.auth, &owner, &correlation_id).await?;

    let updated = state
        .repo
        .set_available(
            owner.user_id,
            location_id,
            product_id,
            media_id,
            body.available,
        )
        .await
        .map_err(|_err| {
            ApiError::new(
                ErrorCode::Internal,
                "internal_error",
                correlation_id.clone(),
            )
        })?;
    match updated {
        None => Err(ApiError::new(
            ErrorCode::NotFound,
            "Not found",
            correlation_id,
        )),
        Some(available) => Ok(Json(
            serde_json::json!({ "id": media_id, "available": available }),
        )),
    }
}

// ─────────────────────────────────────────────────────────────────────────────────────────────
// PgProductMediaRepo — the real sqlx-backed implementation
// ─────────────────────────────────────────────────────────────────────────────────────────────

pub struct PgProductMediaRepo {
    pool: sqlx::PgPool,
}

impl PgProductMediaRepo {
    pub fn new(pool: sqlx::PgPool) -> Self {
        PgProductMediaRepo { pool }
    }
}

#[async_trait::async_trait]
impl ProductMediaRepo for PgProductMediaRepo {
    async fn presign_precheck(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
    ) -> Result<PrecheckOutcome, RepoError> {
        let outcome = db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(PrecheckOutcome::NotFound);
                }
                let owns: Option<i32> =
                    sqlx::query_scalar("SELECT 1 FROM products WHERE id = $1 AND location_id = $2")
                        .bind(product_id)
                        .bind(location_id)
                        .fetch_optional(&mut **txn)
                        .await?;
                if owns.is_none() {
                    return Ok(PrecheckOutcome::NotFound);
                }
                let used: i64 = sqlx::query_scalar(
                    "SELECT COALESCE(SUM(bytes), 0)::bigint FROM product_media WHERE location_id = $1",
                )
                .bind(location_id)
                .fetch_one(&mut **txn)
                .await?;
                Ok(PrecheckOutcome::Ok {
                    used_bytes: u64::try_from(used).unwrap_or(0),
                })
            })
        })
        .await
        .map_err(map_txn_err)?;
        Ok(outcome)
    }

    async fn confirm(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
        input: ConfirmInput,
    ) -> Result<ConfirmOutcome, RepoError> {
        let outcome = db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(ConfirmOutcome::NotFound);
                }
                let owns: Option<i32> =
                    sqlx::query_scalar("SELECT 1 FROM products WHERE id = $1 AND location_id = $2")
                        .bind(product_id)
                        .bind(location_id)
                        .fetch_optional(&mut **txn)
                        .await?;
                if owns.is_none() {
                    return Ok(ConfirmOutcome::NotFound);
                }

                // L2 (breaker, CARRY): read-then-insert sort_order race under concurrent confirms.
                let next_sort_order: i32 = sqlx::query_scalar(
                    "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM product_media WHERE product_id = $1",
                )
                .bind(product_id)
                .fetch_one(&mut **txn)
                .await?;

                let meta = if input.kind == "spin" {
                    serde_json::json!({ "frameKeys": input.frame_keys, "frameCount": input.frame_keys.len() })
                } else {
                    serde_json::json!({})
                };

                let id = Uuid::new_v4();
                let row: (Uuid, i32) = sqlx::query_as(
                    // $4::product_media_kind — `kind` is an enum column bound as text; Postgres
                    // won't coerce a bound text param to an enum, so uncast this INSERT 500'd
                    // every media confirm (same class as orders.type::order_type — staging oracle
                    // 2026-07-05).
                    "INSERT INTO product_media
                       (id, location_id, product_id, kind, storage_key, mime_type, bytes, width,
                        height, duration_ms, poster_key, alt, sort_order, available, meta)
                     VALUES ($1,$2,$3,$4::product_media_kind,$5,$6,$7,$8,$9,$10,$11,$12,$13,true,$14)
                     RETURNING id, sort_order",
                )
                .bind(id)
                .bind(location_id)
                .bind(product_id)
                .bind(&input.kind)
                .bind(&input.storage_key)
                .bind(&input.mime_type)
                .bind(i64::try_from(input.bytes).unwrap_or(i64::MAX))
                .bind(input.width)
                .bind(input.height)
                .bind(input.duration_ms)
                .bind(&input.poster_key)
                .bind(&input.alt)
                .bind(next_sort_order)
                .bind(meta)
                .fetch_one(&mut **txn)
                .await?;

                Ok(ConfirmOutcome::Ok {
                    id: row.0,
                    sort_order: row.1,
                })
            })
        })
        .await
        .map_err(map_txn_err)?;
        Ok(outcome)
    }

    async fn set_primary(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
        media_id: Uuid,
    ) -> Result<SetPrimaryOutcome, RepoError> {
        let outcome = db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(SetPrimaryOutcome::NotFound);
                }
                let cur: Option<Option<Uuid>> = sqlx::query_scalar(
                    "SELECT primary_media_id FROM products WHERE id = $1 AND location_id = $2",
                )
                .bind(product_id)
                .bind(location_id)
                .fetch_optional(&mut **txn)
                .await?;
                let Some(current_primary) = cur else {
                    return Ok(SetPrimaryOutcome::NotFound);
                };
                let owns: Option<i32> = sqlx::query_scalar(
                    "SELECT 1 FROM product_media WHERE id = $1 AND product_id = $2 AND location_id = $3",
                )
                .bind(media_id)
                .bind(product_id)
                .bind(location_id)
                .fetch_optional(&mut **txn)
                .await?;
                if owns.is_none() {
                    return Ok(SetPrimaryOutcome::NotFound);
                }
                if current_primary == Some(media_id) {
                    return Ok(SetPrimaryOutcome::Ok { changed: false });
                }
                sqlx::query("UPDATE products SET primary_media_id = $1 WHERE id = $2")
                    .bind(media_id)
                    .bind(product_id)
                    .execute(&mut **txn)
                    .await?;
                Ok(SetPrimaryOutcome::Ok { changed: true })
            })
        })
        .await
        .map_err(map_txn_err)?;
        Ok(outcome)
    }

    async fn reorder(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
        order: Vec<Uuid>,
    ) -> Result<bool, RepoError> {
        db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(false);
                }
                for (i, media_id) in order.into_iter().enumerate() {
                    sqlx::query(
                        "UPDATE product_media SET sort_order = $1
                          WHERE id = $2 AND product_id = $3 AND location_id = $4",
                    )
                    .bind(i32::try_from(i).unwrap_or(i32::MAX))
                    .bind(media_id)
                    .bind(product_id)
                    .bind(location_id)
                    .execute(&mut **txn)
                    .await?;
                }
                Ok(true)
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn set_available(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
        media_id: Uuid,
        available: bool,
    ) -> Result<Option<bool>, RepoError> {
        db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(None);
                }
                let updated: Option<bool> = sqlx::query_scalar(
                    "UPDATE product_media SET available = $1
                      WHERE id = $2 AND product_id = $3 AND location_id = $4
                      RETURNING available",
                )
                .bind(available)
                .bind(media_id)
                .bind(product_id)
                .bind(location_id)
                .fetch_optional(&mut **txn)
                .await?;
                Ok(updated)
            })
        })
        .await
        .map_err(map_txn_err)
    }
}

// ─────────────────────────────────────────────────────────────────────────────────────────────
// Fake repo (cfg(test))
// ─────────────────────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
pub mod fake {
    use super::{
        ConfirmInput, ConfirmOutcome, PrecheckOutcome, ProductMediaRepo, SetPrimaryOutcome,
    };
    use crate::repo::RepoError;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use uuid::Uuid;

    #[derive(Debug, Clone)]
    pub struct StoredMedia {
        pub id: Uuid,
        pub product_id: Uuid,
        pub location_id: Uuid,
        pub sort_order: i32,
        pub available: bool,
    }

    #[derive(Default)]
    pub struct FakeProductMediaRepo {
        /// `(location_id) -> membership present`.
        pub memberships: Mutex<std::collections::HashSet<Uuid>>,
        /// `product_id -> location_id` (which products "exist" and where).
        pub products: Mutex<HashMap<Uuid, Uuid>>,
        /// `product_id -> primary_media_id`.
        pub primary: Mutex<HashMap<Uuid, Uuid>>,
        pub used_bytes: Mutex<HashMap<Uuid, u64>>,
        pub media: Mutex<Vec<StoredMedia>>,
    }

    #[async_trait::async_trait]
    impl ProductMediaRepo for FakeProductMediaRepo {
        async fn presign_precheck(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            product_id: Uuid,
        ) -> Result<PrecheckOutcome, RepoError> {
            if !self.memberships.lock().unwrap().contains(&location_id) {
                return Ok(PrecheckOutcome::NotFound);
            }
            if self.products.lock().unwrap().get(&product_id) != Some(&location_id) {
                return Ok(PrecheckOutcome::NotFound);
            }
            let used = *self
                .used_bytes
                .lock()
                .unwrap()
                .get(&location_id)
                .unwrap_or(&0);
            Ok(PrecheckOutcome::Ok { used_bytes: used })
        }

        async fn confirm(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            product_id: Uuid,
            _input: ConfirmInput,
        ) -> Result<ConfirmOutcome, RepoError> {
            if !self.memberships.lock().unwrap().contains(&location_id) {
                return Ok(ConfirmOutcome::NotFound);
            }
            if self.products.lock().unwrap().get(&product_id) != Some(&location_id) {
                return Ok(ConfirmOutcome::NotFound);
            }
            let mut media = self.media.lock().unwrap();
            let sort_order = media
                .iter()
                .filter(|m| m.product_id == product_id)
                .map(|m| m.sort_order)
                .max()
                .map(|m| m + 1)
                .unwrap_or(0);
            let id = Uuid::new_v4();
            media.push(StoredMedia {
                id,
                product_id,
                location_id,
                sort_order,
                available: true,
            });
            Ok(ConfirmOutcome::Ok { id, sort_order })
        }

        async fn set_primary(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            product_id: Uuid,
            media_id: Uuid,
        ) -> Result<SetPrimaryOutcome, RepoError> {
            if !self.memberships.lock().unwrap().contains(&location_id) {
                return Ok(SetPrimaryOutcome::NotFound);
            }
            if self.products.lock().unwrap().get(&product_id) != Some(&location_id) {
                return Ok(SetPrimaryOutcome::NotFound);
            }
            let owns = self
                .media
                .lock()
                .unwrap()
                .iter()
                .any(|m| m.id == media_id && m.product_id == product_id);
            if !owns {
                return Ok(SetPrimaryOutcome::NotFound);
            }
            let mut primary = self.primary.lock().unwrap();
            if primary.get(&product_id) == Some(&media_id) {
                return Ok(SetPrimaryOutcome::Ok { changed: false });
            }
            primary.insert(product_id, media_id);
            Ok(SetPrimaryOutcome::Ok { changed: true })
        }

        async fn reorder(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            product_id: Uuid,
            order: Vec<Uuid>,
        ) -> Result<bool, RepoError> {
            if !self.memberships.lock().unwrap().contains(&location_id) {
                return Ok(false);
            }
            let mut media = self.media.lock().unwrap();
            for (i, media_id) in order.into_iter().enumerate() {
                if let Some(m) = media.iter_mut().find(|m| {
                    m.id == media_id && m.product_id == product_id && m.location_id == location_id
                }) {
                    m.sort_order = i32::try_from(i).unwrap_or(i32::MAX);
                }
            }
            Ok(true)
        }

        async fn set_available(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            product_id: Uuid,
            media_id: Uuid,
            available: bool,
        ) -> Result<Option<bool>, RepoError> {
            if !self.memberships.lock().unwrap().contains(&location_id) {
                return Ok(None);
            }
            let mut media = self.media.lock().unwrap();
            let Some(m) = media.iter_mut().find(|m| {
                m.id == media_id && m.product_id == product_id && m.location_id == location_id
            }) else {
                return Ok(None);
            };
            m.available = available;
            Ok(Some(available))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::FakeProductMediaRepo;
    use super::*;
    use crate::auth::AuthState;
    use crate::auth::claims::OwnerClaims;
    use crate::auth::repo::fake::FakeAuthRepo;
    use crate::media::upload_token::UploadTokenSigner;
    use crate::storage::LocalFsStorage;
    use std::sync::Mutex;

    fn test_state(user_id: Uuid, loc: Uuid, repo: FakeProductMediaRepo) -> ProductMediaState {
        repo.memberships.lock().unwrap().insert(loc);
        let auth_repo = Arc::new(FakeAuthRepo {
            active_owner_locations: Mutex::new([(user_id, vec![loc])].into_iter().collect()),
            ..Default::default()
        });
        ProductMediaState {
            auth: AuthState::test_state(auth_repo),
            repo: Arc::new(repo),
            storage: Arc::new(LocalFsStorage::new(std::env::temp_dir())),
            token_signer: Some(Arc::new(UploadTokenSigner::new(vec![9u8; 32]))),
            app_base_url: "https://dowiz.fly.dev".to_string(),
        }
    }

    fn owner_ext(user_id: Uuid) -> OwnerClaimsExt {
        OwnerClaimsExt(OwnerClaims::new(user_id, None))
    }

    fn request_id() -> Extension<RequestId> {
        Extension(RequestId::new(axum::http::HeaderValue::from_static(
            "corr-1",
        )))
    }

    fn sha256_fixture() -> String {
        "a".repeat(64)
    }

    #[tokio::test]
    async fn presign_mints_a_token_proxy_put_url_not_a_real_s3_url() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let product_id = Uuid::new_v4();
        let repo = FakeProductMediaRepo::default();
        repo.products.lock().unwrap().insert(product_id, loc);
        let state = test_state(user_id, loc, repo);

        let body = PresignRequest {
            kind: "image".to_string(),
            items: vec![PresignItem {
                mime_type: "image/webp".to_string(),
                bytes: 1000,
                sha256: sha256_fixture(),
                poster: false,
            }],
        };
        let response = presign_product_media(
            Extension(state),
            owner_ext(user_id),
            Path(product_id),
            request_id(),
            Json(body),
        )
        .await
        .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let url = json["uploads"][0]["url"].as_str().unwrap();
        assert!(
            url.contains("/api/media/upload/"),
            "must point at OUR proxy endpoint, not a real S3/R2 presigned URL: {url}"
        );
        assert_eq!(json["expiresIn"], 300);
    }

    #[tokio::test]
    async fn presign_rejects_svg_even_if_declared_as_allowed_mime_shape() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let product_id = Uuid::new_v4();
        let repo = FakeProductMediaRepo::default();
        repo.products.lock().unwrap().insert(product_id, loc);
        let state = test_state(user_id, loc, repo);

        let body = PresignRequest {
            kind: "image".to_string(),
            items: vec![PresignItem {
                mime_type: "image/svg+xml".to_string(),
                bytes: 1000,
                sha256: sha256_fixture(),
                poster: false,
            }],
        };
        let err = crate::error::expect_err(
            presign_product_media(
                Extension(state),
                owner_ext(user_id),
                Path(product_id),
                request_id(),
                Json(body),
            )
            .await
            .map(|_| ()),
        );
        assert_eq!(err.envelope.code, ErrorCode::InvalidMedia);
    }

    #[tokio::test]
    async fn presign_413s_with_the_bare_budget_body_when_over_limit() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let product_id = Uuid::new_v4();
        let repo = FakeProductMediaRepo::default();
        repo.products.lock().unwrap().insert(product_id, loc);
        repo.used_bytes
            .lock()
            .unwrap()
            .insert(loc, validation::LOCATION_BUDGET_BYTES);
        let state = test_state(user_id, loc, repo);

        let body = PresignRequest {
            kind: "image".to_string(),
            items: vec![PresignItem {
                mime_type: "image/webp".to_string(),
                bytes: 1,
                sha256: sha256_fixture(),
                poster: false,
            }],
        };
        let response = presign_product_media(
            Extension(state),
            owner_ext(user_id),
            Path(product_id),
            request_id(),
            Json(body),
        )
        .await
        .unwrap();
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["error"], "Storage budget exceeded");
        assert!(
            json.get("code").is_none(),
            "bare shape, not an ErrorEnvelope"
        );
    }

    #[tokio::test]
    async fn confirm_rejects_a_storage_key_outside_the_tenant_prefix() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let product_id = Uuid::new_v4();
        let other_loc = Uuid::new_v4();
        let repo = FakeProductMediaRepo::default();
        repo.products.lock().unwrap().insert(product_id, loc);
        let state = test_state(user_id, loc, repo);

        let body = ConfirmRequest {
            kind: "image".to_string(),
            storage_key: format!("{other_loc}/{product_id}/image/abc.webp"),
            mime_type: "image/webp".to_string(),
            bytes: 100,
            width: None,
            height: None,
            duration_ms: None,
            poster_key: None,
            alt: None,
            frame_keys: vec![],
        };
        let err = crate::error::expect_err(
            confirm_product_media(
                Extension(state),
                owner_ext(user_id),
                Path(product_id),
                request_id(),
                Json(body),
            )
            .await
            .map(|_| ()),
        );
        assert_eq!(err.envelope.code, ErrorCode::ValidationFailed);
    }

    #[tokio::test]
    async fn confirm_rejects_path_traversal_inside_the_tenant_prefix() {
        // A key that STARTS with the tenant prefix but walks out via `..` must be rejected
        // (SSG sentinel S4 LOW, defense-in-depth for the LocalFs dev provider).
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let product_id = Uuid::new_v4();
        let repo = FakeProductMediaRepo::default();
        repo.products.lock().unwrap().insert(product_id, loc);
        let state = test_state(user_id, loc, repo);

        let body = ConfirmRequest {
            kind: "image".to_string(),
            storage_key: format!("{loc}/{product_id}/../../etc/passwd.webp"),
            mime_type: "image/webp".to_string(),
            bytes: 100,
            width: None,
            height: None,
            duration_ms: None,
            poster_key: None,
            alt: None,
            frame_keys: vec![],
        };
        let err = crate::error::expect_err(
            confirm_product_media(
                Extension(state),
                owner_ext(user_id),
                Path(product_id),
                request_id(),
                Json(body),
            )
            .await
            .map(|_| ()),
        );
        assert_eq!(err.envelope.code, ErrorCode::ValidationFailed);
    }

    #[tokio::test]
    async fn confirm_201s_and_assigns_sort_order() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let product_id = Uuid::new_v4();
        let repo = FakeProductMediaRepo::default();
        repo.products.lock().unwrap().insert(product_id, loc);
        let state = test_state(user_id, loc, repo);

        let body = ConfirmRequest {
            kind: "image".to_string(),
            storage_key: format!("{loc}/{product_id}/image/abc.webp"),
            mime_type: "image/webp".to_string(),
            bytes: 100,
            width: Some(800),
            height: Some(600),
            duration_ms: None,
            poster_key: None,
            alt: None,
            frame_keys: vec![],
        };
        let response = confirm_product_media(
            Extension(state),
            owner_ext(user_id),
            Path(product_id),
            request_id(),
            Json(body),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn set_primary_is_a_no_op_when_already_primary() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let product_id = Uuid::new_v4();
        let media_id = Uuid::new_v4();
        let repo = FakeProductMediaRepo::default();
        repo.products.lock().unwrap().insert(product_id, loc);
        repo.media.lock().unwrap().push(fake::StoredMedia {
            id: media_id,
            product_id,
            location_id: loc,
            sort_order: 0,
            available: true,
        });
        repo.primary.lock().unwrap().insert(product_id, media_id);
        let state = test_state(user_id, loc, repo);

        let response = set_primary_product_media(
            Extension(state),
            owner_ext(user_id),
            Path((product_id, media_id)),
            request_id(),
        )
        .await
        .unwrap()
        .into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["changed"], false);
    }

    #[tokio::test]
    async fn available_toggle_404s_for_a_foreign_media_id() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let product_id = Uuid::new_v4();
        let repo = FakeProductMediaRepo::default();
        repo.products.lock().unwrap().insert(product_id, loc);
        let state = test_state(user_id, loc, repo);

        let err = crate::error::expect_err(
            set_product_media_available(
                Extension(state),
                owner_ext(user_id),
                Path((product_id, Uuid::new_v4())),
                request_id(),
                Json(AvailableToggleRequest { available: false }),
            )
            .await
            .map(|_| ()),
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    #[test]
    fn sha256_shape_validation() {
        assert!(is_sha256_shape(&"a".repeat(64)));
        assert!(!is_sha256_shape(&"a".repeat(63)));
        assert!(!is_sha256_shape("not-hex-zzzz"));
    }
}
