//! S4 media surface — product-image upload + in-process transcode. Ports
//! `apps/api/src/routes/spa-proxy.ts:213-261`
//! (`POST /api/owner/menu/products/:productId/image`).
//!
//! ## REV-S4-5 (FIX-IN-PORT, Q-GUC-PRODIMG) — the raw `db.query` write goes through `with_user`
//! `spa-proxy.ts:252`'s `UPDATE products SET image_key…` ran on the RAW pool, no GUC — a
//! CONFIRMED breaker/packet finding that matches 0 rows post-NOBYPASSRLS-flip. This port routes
//! the write through `db::with_user` + `assert_active_owner_membership` (the S3 pattern).
//!
//! ## Image processing (REV-S4-1 / REV-S4-4)
//! `crate::media::processor::transcode` (product profile, 800×800/q82) — bomb-capped decode +
//! explicit EXIF orientation (see that module's doc for why the spike's benchmarked entry point
//! is unsafe and unused here).
//!
//! ## CARRY quirks (verbatim)
//! - Q-SHA-PROCESSED: the key is `${locId}/${pid}-${sha256(processed_bytes).slice(0,12)}.webp` —
//!   hashed over the SERVER-PROCESSED bytes (not the raw upload), so a re-upload always gets a
//!   fresh URL under `/images/*`'s 1-year immutable cache (`spa-proxy.ts:235`, rationale carried
//!   verbatim in the doc comment on [`content_hashed_key`]).
//! - Q-CLEANUP-SWALLOW: the old-key best-effort delete never fails the user-visible upload
//!   (`spa-proxy.ts:257-259`) — errors are logged, not surfaced.
//! - Multipart size ceiling: Node's global `@fastify/multipart` limit is 10 MB
//!   (`server.ts:355-356`, `bodyLimit: 10 * 1024 * 1024`) — this route has no PER-ROUTE override
//!   in the old TS, so it inherits that global. Ported as an explicit `DefaultBodyLimit` on this
//!   route (`routes/owner/mod.rs`) — axum has no implicit global multipart cap, so this must be
//!   stated, not assumed.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Multipart, Path};
use axum::response::IntoResponse;
use sha2::{Digest, Sha256};
use tower_http::request_id::RequestId;
use uuid::Uuid;

use domain::ErrorCode;

use crate::db;
use crate::error::ApiError;
use crate::media::processor::{ImageProcessor, PRODUCT_PROFILE};
use crate::repo::RepoError;
use crate::routes::correlation_id_string;
use crate::storage::Storage;

use super::{assert_active_owner_membership, resolve_owner_location};
use crate::auth::extractors::OwnerClaimsExt;

/// Node's global multipart `fileSize` limit (`server.ts:355-356`) — this route has no
/// route-specific override, so it inherits the global.
pub const MAX_UPLOAD_BYTES: usize = 10 * 1024 * 1024;

#[derive(Clone)]
pub struct ProductImageState {
    pub auth: crate::auth::AuthState,
    pub repo: Arc<dyn ProductImageRepo>,
    pub storage: Arc<dyn Storage>,
    pub processor: Arc<dyn ImageProcessor>,
    pub app_base_url: String,
}

#[async_trait::async_trait]
pub trait ProductImageRepo: Send + Sync {
    /// Membership assert + `products` ownership check + `UPDATE image_key/image_url`, in ONE
    /// `with_user`-seated transaction (REV-S4-5). Returns the PRIOR `image_key` (for the
    /// best-effort old-object cleanup, Q-CLEANUP-SWALLOW) alongside whether the row was found.
    async fn update_image(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
        new_key: &str,
        new_url: &str,
    ) -> Result<Option<Option<String>>, RepoError>;
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

/// `${locId}/${pid}-${sha256(processed).slice(0,12)}.webp` (`spa-proxy.ts:235`) — hashed over the
/// SERVER-PROCESSED bytes. CARRY rationale verbatim: `/images/*` serves
/// `max-age=31536000, immutable`, so a fixed key would pin the FIRST upload forever; hashing the
/// processed bytes means a changed image is always a fresh URL the CDN fetches new.
fn content_hashed_key(location_id: Uuid, product_id: Uuid, processed: &[u8]) -> String {
    let hash = hex::encode(Sha256::digest(processed));
    format!("{location_id}/{product_id}-{}.webp", &hash[..12])
}

#[utoipa::path(
    post,
    path = "/api/owner/menu/products/{productId}/image",
    params(("productId" = Uuid, Path)),
    responses(
        (status = 200, description = "Uploaded + transcoded product image"),
        (status = 400, description = "No file uploaded / invalid image", body = domain::ErrorEnvelope),
        (status = 401, description = "Unauthorized", body = domain::ErrorEnvelope),
        (status = 404, description = "Product not found", body = domain::ErrorEnvelope),
        (status = 413, description = "File exceeds size limit", body = domain::ErrorEnvelope),
    ),
    tag = "owner-media"
)]
pub async fn upload_product_image(
    Extension(state): Extension<ProductImageState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(product_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let location_id = resolve_owner_location(&state.auth, &owner, &correlation_id).await?;

    // A single `request.file()`-equivalent read (`spa-proxy.ts:216`) — the FIRST multipart part,
    // whatever its field name, matching the old TS's unconditional `await request.file()` (no
    // field-name filtering, unlike entry-photo's `routes::media_public`, which DOES require the
    // "file" field specifically).
    let Ok(Some(field)) = multipart.next_field().await else {
        return Err(ApiError::new(
            ErrorCode::ValidationFailed,
            "No file uploaded",
            correlation_id,
        ));
    };
    let bytes = field.bytes().await.map_err(|_multipart_err| {
        ApiError::new(
            ErrorCode::ValidationFailed,
            "Invalid multipart body",
            correlation_id.clone(),
        )
    })?;
    if bytes.len() > MAX_UPLOAD_BYTES {
        return Err(ApiError::new(
            ErrorCode::FileTooLarge,
            "File exceeds size limit",
            correlation_id,
        ));
    }
    let buffer = bytes.to_vec();

    let processed = state
        .processor
        .process(&buffer, PRODUCT_PROFILE)
        .map_err(|err| {
            ApiError::new(
                ErrorCode::ValidationFailed,
                format!("Invalid image file: {err}"),
                correlation_id.clone(),
            )
        })?;

    let key = content_hashed_key(location_id, product_id, &processed);
    let image_url =
        crate::service::get_image_url(Some(&key), None, &state.app_base_url).unwrap_or_default();

    let prior = state
        .repo
        .update_image(owner.user_id, location_id, product_id, &key, &image_url)
        .await
        .map_err(|_err| {
            ApiError::new(
                ErrorCode::Internal,
                "internal_error",
                correlation_id.clone(),
            )
        })?;
    let Some(old_key) = prior else {
        return Err(ApiError::new(
            ErrorCode::NotFound,
            "Product not found",
            correlation_id,
        ));
    };

    state.storage.put(&key, processed).await.map_err(|err| {
        ApiError::new(
            ErrorCode::Internal,
            format!("Failed to store image: {err}"),
            correlation_id.clone(),
        )
    })?;

    // Q-CLEANUP-SWALLOW (CARRY verbatim): best-effort — a failed delete must never fail the
    // user-visible upload. Skip external-URL legacy keys (never a storage-backed key).
    if let Some(old_key) = old_key
        && old_key != key
        && !old_key.starts_with("http://")
        && !old_key.starts_with("https://")
    {
        // Deliberately swallowed (CARRY, see comment above) — `drop(...)` explicitly consumes the
        // `#[must_use]` `Result` rather than `let _ = `, which clippy flags on that type.
        drop(state.storage.delete(&old_key).await);
    }

    Ok(Json(
        serde_json::json!({ "imageUrl": image_url, "imageKey": key }),
    ))
}

// ─────────────────────────────────────────────────────────────────────────────────────────────
// PgProductImageRepo
// ─────────────────────────────────────────────────────────────────────────────────────────────

pub struct PgProductImageRepo {
    pool: sqlx::PgPool,
}

impl PgProductImageRepo {
    pub fn new(pool: sqlx::PgPool) -> Self {
        PgProductImageRepo { pool }
    }
}

#[async_trait::async_trait]
impl ProductImageRepo for PgProductImageRepo {
    async fn update_image(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
        new_key: &str,
        new_url: &str,
    ) -> Result<Option<Option<String>>, RepoError> {
        // Owned, not borrowed: the `with_user` closure's HRTB (`for<'t> ... + Send + 't`) requires
        // anything it captures by reference to be valid for an ARBITRARY (universally quantified)
        // lifetime — in practice that forces borrowed captures to be `'static`. Converting to an
        // owned `String` here (matching every other repo method's binds in this codebase) sidesteps
        // that entirely rather than fighting the borrow checker over a two-field bind.
        let new_key = new_key.to_string();
        let new_url = new_url.to_string();
        let outcome = db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(None);
                }
                let old_key: Option<Option<String>> = sqlx::query_scalar(
                    "SELECT image_key FROM products WHERE id = $1 AND location_id = $2",
                )
                .bind(product_id)
                .bind(location_id)
                .fetch_optional(&mut **txn)
                .await?;
                let Some(old_key) = old_key else {
                    return Ok(None);
                };
                sqlx::query(
                    "UPDATE products SET image_key = $1, image_url = $2 WHERE id = $3 AND location_id = $4",
                )
                .bind(new_key)
                .bind(new_url)
                .bind(product_id)
                .bind(location_id)
                .execute(&mut **txn)
                .await?;
                Ok(Some(old_key))
            })
        })
        .await
        .map_err(map_txn_err)?;
        Ok(outcome)
    }
}

#[cfg(test)]
pub mod fake {
    use super::ProductImageRepo;
    use crate::repo::RepoError;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use uuid::Uuid;

    #[derive(Default)]
    pub struct FakeProductImageRepo {
        pub memberships: Mutex<std::collections::HashSet<Uuid>>,
        /// `product_id -> (location_id, current image_key)`.
        pub products: Mutex<HashMap<Uuid, (Uuid, Option<String>)>>,
    }

    #[async_trait::async_trait]
    impl ProductImageRepo for FakeProductImageRepo {
        async fn update_image(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            product_id: Uuid,
            new_key: &str,
            _new_url: &str,
        ) -> Result<Option<Option<String>>, RepoError> {
            if !self.memberships.lock().unwrap().contains(&location_id) {
                return Ok(None);
            }
            let mut products = self.products.lock().unwrap();
            let Some((loc, old_key)) = products.get(&product_id).cloned() else {
                return Ok(None);
            };
            if loc != location_id {
                return Ok(None);
            }
            products.insert(product_id, (location_id, Some(new_key.to_string())));
            Ok(Some(old_key))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_hashed_key_changes_when_bytes_change() {
        let loc = Uuid::new_v4();
        let pid = Uuid::new_v4();
        let a = content_hashed_key(loc, pid, b"one");
        let b = content_hashed_key(loc, pid, b"two");
        assert_ne!(a, b, "different processed bytes must yield different keys");
        assert!(a.starts_with(&format!("{loc}/{pid}-")));
        assert!(a.ends_with(".webp"));
    }

    #[test]
    fn content_hashed_key_is_stable_for_identical_bytes() {
        let loc = Uuid::new_v4();
        let pid = Uuid::new_v4();
        assert_eq!(
            content_hashed_key(loc, pid, b"same"),
            content_hashed_key(loc, pid, b"same")
        );
    }

    #[tokio::test]
    async fn update_image_returns_none_for_a_foreign_location() {
        use fake::FakeProductImageRepo;
        let repo = FakeProductImageRepo::default();
        let loc = Uuid::new_v4();
        let other_loc = Uuid::new_v4();
        let pid = Uuid::new_v4();
        repo.memberships.lock().unwrap().insert(loc);
        repo.products.lock().unwrap().insert(pid, (other_loc, None));

        let result = repo
            .update_image(Uuid::new_v4(), loc, pid, "k", "u")
            .await
            .unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn update_image_returns_the_prior_key_for_cleanup() {
        use fake::FakeProductImageRepo;
        let repo = FakeProductImageRepo::default();
        let loc = Uuid::new_v4();
        let pid = Uuid::new_v4();
        repo.memberships.lock().unwrap().insert(loc);
        repo.products
            .lock()
            .unwrap()
            .insert(pid, (loc, Some("old-key.webp".to_string())));

        let result = repo
            .update_image(Uuid::new_v4(), loc, pid, "new-key.webp", "u")
            .await
            .unwrap();
        assert_eq!(result, Some(Some("old-key.webp".to_string())));
    }
}
