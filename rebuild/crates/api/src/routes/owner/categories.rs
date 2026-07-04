//! S3 catalog/admin CRUD — categories. Ports `apps/api/src/routes/owner/categories.ts` verbatim
//! (owner-route census rows #15-22, `docs/design/rebuild-plan/inventory/10-api-realtime-jobs.md:124-137`).
//! 8 ops: 5 OWNER+LOC (`:locationId` in the path) + 3 OWNER-only `/api/owner/menu/categories`
//! JWT-alias ops. See `crate::routes::owner` module doc for the shared auth/write-pattern contract
//! this file follows (extractor + `require_location_access`/`resolve_owner_location` +
//! `assert_active_owner_membership` + `db::with_user`).
//!
//! ## The `toCategoryApiShape` wire shape (`apps/api/src/lib/row-transformers.ts:1-8`)
//! Every category row is wrapped through `toCategoryApiShape` before going on the wire:
//! `{ id, name, sortOrder: row.sort_order ?? 0, productCount: Number(row.product_count ?? 0) }`.
//! `CategoryResponse`/`CategoryRow::into` below reproduce this exactly — `product_count` is
//! `None` (-> `productCount: 0`) for every op whose SQL never selects it (ops 1-4), and `Some(n)`
//! for the two ops that do (op #6's real `COUNT(p.id)`, op #7's hardcoded `0`).
//!
//! ## In-transaction membership re-check (S3 breaker finding C1+H4, 2026-07-04)
//! `require_location_access`/`resolve_owner_location` read live membership state through
//! `AuthState.repo` — a DIFFERENT connection/pool than the one `db::with_user` later seats
//! `app.user_id` on for the actual write. Per `crate::routes::owner`'s module doc, every op here
//! ALSO calls [`super::assert_active_owner_membership`] as the FIRST statement inside its
//! `with_user` transaction (belt-and-suspenders under NOBYPASSRLS) and folds a `false` result into
//! the SAME 404 the row-not-found path already produces — see each `PgCategoriesRepo` method.
//!
//! ## Quirks carried verbatim (NOT bugs to fix)
//! - Op #7 (`POST /api/owner/menu/categories`) does not accept `sort_order` (DB column default
//!   applies) and hardcodes `0::int AS product_count` in its `RETURNING` — a brand-new category
//!   has none (`categories.ts:224-227`). Op #6's `GET` computes a REAL `COUNT(p.id)` — the two are
//!   deliberately different (`categories.ts:198-206`).
//! - Ops #5/#8 (DELETE) run a same-transaction "does this category still have products" pre-check
//!   BEFORE the delete; a non-empty category 409s WITHOUT deleting, and the category row itself is
//!   never re-checked for existence in that branch (`categories.ts:161-184`, `:244-257`). Only when
//!   the pre-check finds zero products does the actual `DELETE ... RETURNING id` run, and a
//!   0-rowcount result there is the 404. `delete` (this file) implements both ops with ONE shared
//!   repo method — the two TS bodies are structurally identical (WHERE-predicate param order
//!   differs cosmetically only), the two error MESSAGES differ ("Not found"/"Category contains
//!   products" for #5 vs "Category not found"/"Category contains products. Move or delete them
//!   first." for #8) and are kept distinct at the HANDLER layer, not the repo layer.
//! - Op #1's request body accepts `image_key` (`categories.ts:29`) but the INSERT never references
//!   it (`categories.ts:40-44`) — accepted for schema parity, silently discarded, never persisted.
//!
//! ## Judgment calls (no TS-specified error code covers these, flagged rather than silently added)
//! - Zod's `.min(1)`/`.max(200)` string-length bounds on `name` (`categories.ts:27,119,215`) are
//!   NOT re-enforced here: the owner-route census lists no error code for a validation failure on
//!   any of these 8 ops, this crate has no validation-attribute crate in its dependency graph, and
//!   an out-of-band length check would need to invent a wire error the TS source doesn't document
//!   for this file. `name` is ported as a plain `String`.
//! - Querystring `.strict()` on op #2 (`categories.ts:57-61`) is not re-enforced as
//!   `deny_unknown_fields` on `ListCategoriesQuery`: axum's `Query` extractor deserializes via
//!   `serde_urlencoded`, whose interaction with `deny_unknown_fields` is not a pattern used
//!   elsewhere in this crate, so it's left off rather than risk an unverified interaction; JSON
//!   bodies (ops #1/#4/#7) DO carry `deny_unknown_fields` per the build brief.

use axum::Json;
use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use tower_http::request_id::RequestId;
use utoipa::ToSchema;
use uuid::Uuid;

use domain::ErrorCode;

use crate::auth::AuthState;
use crate::auth::extractors::OwnerClaimsExt;
use crate::error::ApiError;
use crate::repo::RepoError;
use crate::routes::correlation_id_string;

use super::{assert_active_owner_membership, require_location_access, resolve_owner_location};

// ── State + repo trait ──────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct CategoriesState {
    pub auth: AuthState,
    pub repo: std::sync::Arc<dyn CategoriesRepo>,
}

/// A category row projected to exactly the columns `toCategoryApiShape` reads.
/// `product_count: None` means the query never selected it (ops #1-4) — `CategoryResponse`
/// defaults that to `0`, matching `row.product_count ?? row.productCount ?? 0` in the TS
/// transformer. `Some(n)` is only ever produced by ops #6/#7.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryRow {
    pub id: Uuid,
    pub name: String,
    pub sort_order: i32,
    pub product_count: Option<i32>,
}

/// Op #5/#8 DELETE's three-way outcome (see module doc for the two-step check-then-delete order).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteOutcome {
    Deleted,
    NotEmpty,
    NotFound,
}

#[async_trait::async_trait]
pub trait CategoriesRepo: Send + Sync {
    /// Op #1 (`categories.ts:20-49`). `Ok(None)` = the in-transaction membership re-check
    /// (S3 breaker C1+H4) failed — the handler maps it to the same 404 `require_location_access`'s
    /// out-of-band pre-check would give.
    async fn create(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        name: String,
        sort_order: Option<i32>,
    ) -> Result<Option<CategoryRow>, RepoError>;

    /// Op #2 (`categories.ts:51-86`), cursor-paged by `id ASC`. `Ok(None)` = membership re-check
    /// failed (see `create`'s doc).
    async fn list(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        cursor: Option<Uuid>,
        limit: i64,
    ) -> Result<Option<Vec<CategoryRow>>, RepoError>;

    /// Op #3 (`categories.ts:88-110`). `Ok(None)` covers BOTH the membership re-check failing AND
    /// the row genuinely not existing — `categories.ts:107` gives the identical 404 NOT_FOUND
    /// "Not found" for both, so no separate variant is needed to keep them distinguishable.
    async fn get(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        id: Uuid,
    ) -> Result<Option<CategoryRow>, RepoError>;

    /// Op #4 (`categories.ts:112-147`), partial update via `COALESCE`. `Ok(None)` covers the
    /// membership re-check failing OR the `UPDATE` matching zero rows (same reasoning as `get`).
    async fn update(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        id: Uuid,
        name: Option<String>,
        sort_order: Option<i32>,
    ) -> Result<Option<CategoryRow>, RepoError>;

    /// Ops #5 AND #8 (`categories.ts:149-185`, `:233-259`) — ONE shared method: both TS bodies run
    /// the identical products-pre-check-then-delete sequence against `(category_id, location_id)`,
    /// differing only in error MESSAGE text, which the two handlers supply separately.
    /// `DeleteOutcome::NotFound` also covers a failed membership re-check (same reasoning as `get`).
    async fn delete(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        id: Uuid,
    ) -> Result<DeleteOutcome, RepoError>;

    /// Op #6 (`categories.ts:189-209`) — real `COUNT(p.id)` per category, ordered by `sort_order`.
    /// `Ok(None)` = membership re-check failed.
    async fn list_with_counts(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
    ) -> Result<Option<Vec<CategoryRow>>, RepoError>;

    /// Op #7 (`categories.ts:211-231`) — no `sort_order` in the body, hardcoded `product_count: 0`.
    /// `Ok(None)` = membership re-check failed.
    async fn create_alias(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        name: String,
    ) -> Result<Option<CategoryRow>, RepoError>;
}

// ── DTOs ─────────────────────────────────────────────────────────────────────────────────────

/// Op #1 body (`categories.ts:26-30`, `.strict()`). `image_key` is accepted for schema parity but
/// never persisted — see module doc.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateCategoryRequest {
    pub name: String,
    #[serde(default)]
    pub sort_order: Option<i32>,
    #[serde(default)]
    #[allow(
        dead_code,
        reason = "accepted for schema parity (categories.ts:29) but never persisted — the INSERT never references it, see module doc"
    )]
    pub image_key: Option<String>,
}

/// Op #2 querystring (`categories.ts:57-60`). See module doc for why `deny_unknown_fields` is not
/// applied here.
#[derive(Debug, Deserialize)]
pub struct ListCategoriesQuery {
    #[serde(default)]
    pub cursor: Option<Uuid>,
    #[serde(default)]
    pub limit: Option<i64>,
}

/// Op #4 body (`categories.ts:118-121`, `.strict()`). Both fields plain `Option<T>` — no
/// tri-state/nullable needed (`categories.ts:136-141`'s `COALESCE($3, name)` / `COALESCE($4,
/// sort_order)`): `None` = absent = don't touch that column.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateCategoryRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub sort_order: Option<i32>,
}

/// Op #7 body (`categories.ts:215`, `.strict()`) — `name` only, no `sort_order`.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateCategoryAliasRequest {
    pub name: String,
}

/// The `toCategoryApiShape` wire shape (`row-transformers.ts:1-8`).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CategoryResponse {
    pub id: Uuid,
    pub name: String,
    #[serde(rename = "sortOrder")]
    pub sort_order: i32,
    #[serde(rename = "productCount")]
    pub product_count: i32,
}

impl From<CategoryRow> for CategoryResponse {
    fn from(row: CategoryRow) -> Self {
        CategoryResponse {
            id: row.id,
            name: row.name,
            sort_order: row.sort_order,
            product_count: row.product_count.unwrap_or(0),
        }
    }
}

/// Op #2's `{ data: [...] }` envelope (`categories.ts:84`).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListCategoriesResponse {
    pub data: Vec<CategoryResponse>,
}

fn internal_error(correlation_id: String) -> ApiError {
    ApiError::new(ErrorCode::Internal, "internal_error", correlation_id)
}

fn not_found(correlation_id: String) -> ApiError {
    ApiError::new(ErrorCode::NotFound, "Not found", correlation_id)
}

// ── Handlers: OWNER+LOC (`:locationId` in the path) ────────────────────────────────────────────

/// `POST /api/owner/locations/{locationId}/categories` (op #1, `categories.ts:20-49`) -> 201.
#[utoipa::path(
    post,
    path = "/api/owner/locations/{locationId}/categories",
    params(("locationId" = Uuid, Path)),
    request_body = CreateCategoryRequest,
    responses(
        (status = 201, description = "Created", body = CategoryResponse),
        (status = 404, description = "Foreign/unknown location", body = domain::ErrorEnvelope),
    ),
    tag = "owner-catalog"
)]
pub async fn create_category(
    Extension(state): Extension<CategoriesState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(location_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<CreateCategoryRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    let row = state
        .repo
        .create(owner.user_id, location_id, body.name, body.sort_order)
        .await
        .map_err(|_err| internal_error(correlation_id.clone()))?
        .ok_or_else(|| not_found(correlation_id))?;

    Ok((StatusCode::CREATED, Json(CategoryResponse::from(row))))
}

/// `GET /api/owner/locations/{locationId}/categories` (op #2, `categories.ts:51-86`) -> 200
/// `{data:[]}`, cursor-paged by `id ASC`.
#[utoipa::path(
    get,
    path = "/api/owner/locations/{locationId}/categories",
    params(
        ("locationId" = Uuid, Path),
        ("cursor" = Option<Uuid>, Query),
        ("limit" = Option<i64>, Query),
    ),
    responses(
        (status = 200, description = "Cursor-paged categories", body = ListCategoriesResponse),
        (status = 404, description = "Foreign/unknown location", body = domain::ErrorEnvelope),
    ),
    tag = "owner-catalog"
)]
pub async fn list_categories(
    Extension(state): Extension<CategoriesState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(location_id): Path<Uuid>,
    Query(params): Query<ListCategoriesQuery>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    // z.coerce.number().min(1).max(100).default(50) (categories.ts:59).
    let limit = params.limit.unwrap_or(50).clamp(1, 100);

    let rows = state
        .repo
        .list(owner.user_id, location_id, params.cursor, limit)
        .await
        .map_err(|_err| internal_error(correlation_id.clone()))?
        .ok_or_else(|| not_found(correlation_id))?;

    Ok(Json(ListCategoriesResponse {
        data: rows.into_iter().map(CategoryResponse::from).collect(),
    }))
}

/// `GET /api/owner/locations/{locationId}/categories/{id}` (op #3, `categories.ts:88-110`) -> 200
/// or 404 NOT_FOUND.
#[utoipa::path(
    get,
    path = "/api/owner/locations/{locationId}/categories/{id}",
    params(("locationId" = Uuid, Path), ("id" = Uuid, Path)),
    responses(
        (status = 200, description = "Category", body = CategoryResponse),
        (status = 404, description = "Not found", body = domain::ErrorEnvelope),
    ),
    tag = "owner-catalog"
)]
pub async fn get_category(
    Extension(state): Extension<CategoriesState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path((location_id, id)): Path<(Uuid, Uuid)>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    let row = state
        .repo
        .get(owner.user_id, location_id, id)
        .await
        .map_err(|_err| internal_error(correlation_id.clone()))?
        .ok_or_else(|| not_found(correlation_id))?;

    Ok(Json(CategoryResponse::from(row)))
}

/// `PATCH /api/owner/locations/{locationId}/categories/{id}` (op #4, `categories.ts:112-147`) ->
/// 200, 400 NO_UPDATES, or 404 NOT_FOUND.
#[utoipa::path(
    patch,
    path = "/api/owner/locations/{locationId}/categories/{id}",
    params(("locationId" = Uuid, Path), ("id" = Uuid, Path)),
    request_body = UpdateCategoryRequest,
    responses(
        (status = 200, description = "Updated", body = CategoryResponse),
        (status = 400, description = "No updates provided", body = domain::ErrorEnvelope),
        (status = 404, description = "Not found", body = domain::ErrorEnvelope),
    ),
    tag = "owner-catalog"
)]
pub async fn update_category(
    Extension(state): Extension<CategoriesState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path((location_id, id)): Path<(Uuid, Uuid)>,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<UpdateCategoryRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    // Object.keys(updates).length === 0 (categories.ts:129-131).
    if body.name.is_none() && body.sort_order.is_none() {
        return Err(ApiError::new(
            ErrorCode::NoUpdates,
            "No updates provided",
            correlation_id,
        ));
    }

    let row = state
        .repo
        .update(owner.user_id, location_id, id, body.name, body.sort_order)
        .await
        .map_err(|_err| internal_error(correlation_id.clone()))?
        .ok_or_else(|| not_found(correlation_id))?;

    Ok(Json(CategoryResponse::from(row)))
}

/// `DELETE /api/owner/locations/{locationId}/categories/{id}` (op #5, `categories.ts:149-185`) ->
/// 204, 409 CATEGORY_NOT_EMPTY, or 404 NOT_FOUND.
#[utoipa::path(
    delete,
    path = "/api/owner/locations/{locationId}/categories/{id}",
    params(("locationId" = Uuid, Path), ("id" = Uuid, Path)),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Not found", body = domain::ErrorEnvelope),
        (status = 409, description = "Category contains products", body = domain::ErrorEnvelope),
    ),
    tag = "owner-catalog"
)]
pub async fn delete_category(
    Extension(state): Extension<CategoriesState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path((location_id, id)): Path<(Uuid, Uuid)>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    match state
        .repo
        .delete(owner.user_id, location_id, id)
        .await
        .map_err(|_err| internal_error(correlation_id.clone()))?
    {
        DeleteOutcome::Deleted => Ok(StatusCode::NO_CONTENT),
        DeleteOutcome::NotEmpty => Err(ApiError::new(
            ErrorCode::CategoryNotEmpty,
            "Category contains products",
            correlation_id,
        )),
        DeleteOutcome::NotFound => Err(not_found(correlation_id)),
    }
}

// ── Handlers: OWNER-only `/api/owner/menu/categories` JWT aliases ─────────────────────────────

/// `GET /api/owner/menu/categories` (op #6, `categories.ts:189-209`) -> 200 array w/
/// `productCount`, or 401 UNAUTHORIZED.
#[utoipa::path(
    get,
    path = "/api/owner/menu/categories",
    responses(
        (status = 200, description = "Categories with live product counts", body = Vec<CategoryResponse>),
        (status = 401, description = "No active owner membership", body = domain::ErrorEnvelope),
    ),
    tag = "owner-catalog"
)]
pub async fn list_categories_alias(
    Extension(state): Extension<CategoriesState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let location_id = resolve_owner_location(&state.auth, &owner, &correlation_id).await?;

    let rows = state
        .repo
        .list_with_counts(owner.user_id, location_id)
        .await
        .map_err(|_err| internal_error(correlation_id.clone()))?
        .ok_or_else(|| not_found(correlation_id))?;

    Ok(Json(
        rows.into_iter()
            .map(CategoryResponse::from)
            .collect::<Vec<_>>(),
    ))
}

/// `POST /api/owner/menu/categories` (op #7, `categories.ts:211-231`) -> 201, or 401 UNAUTHORIZED.
#[utoipa::path(
    post,
    path = "/api/owner/menu/categories",
    request_body = CreateCategoryAliasRequest,
    responses(
        (status = 201, description = "Created (product_count hardcoded 0)", body = CategoryResponse),
        (status = 401, description = "No active owner membership", body = domain::ErrorEnvelope),
    ),
    tag = "owner-catalog"
)]
pub async fn create_category_alias(
    Extension(state): Extension<CategoriesState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<CreateCategoryAliasRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let location_id = resolve_owner_location(&state.auth, &owner, &correlation_id).await?;

    let row = state
        .repo
        .create_alias(owner.user_id, location_id, body.name)
        .await
        .map_err(|_err| internal_error(correlation_id.clone()))?
        .ok_or_else(|| not_found(correlation_id))?;

    Ok((StatusCode::CREATED, Json(CategoryResponse::from(row))))
}

/// `DELETE /api/owner/menu/categories/{id}` (op #8, `categories.ts:233-259`) -> 204, 401
/// UNAUTHORIZED, 409 CATEGORY_NOT_EMPTY, or 404 NOT_FOUND.
#[utoipa::path(
    delete,
    path = "/api/owner/menu/categories/{id}",
    params(("id" = Uuid, Path)),
    responses(
        (status = 204, description = "Deleted"),
        (status = 401, description = "No active owner membership", body = domain::ErrorEnvelope),
        (status = 404, description = "Category not found", body = domain::ErrorEnvelope),
        (status = 409, description = "Category contains products", body = domain::ErrorEnvelope),
    ),
    tag = "owner-catalog"
)]
pub async fn delete_category_alias(
    Extension(state): Extension<CategoriesState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let location_id = resolve_owner_location(&state.auth, &owner, &correlation_id).await?;

    match state
        .repo
        .delete(owner.user_id, location_id, id)
        .await
        .map_err(|_err| internal_error(correlation_id.clone()))?
    {
        DeleteOutcome::Deleted => Ok(StatusCode::NO_CONTENT),
        DeleteOutcome::NotEmpty => Err(ApiError::new(
            ErrorCode::CategoryNotEmpty,
            "Category contains products. Move or delete them first.",
            correlation_id,
        )),
        DeleteOutcome::NotFound => Err(ApiError::new(
            ErrorCode::NotFound,
            "Category not found",
            correlation_id,
        )),
    }
}

// ── PgCategoriesRepo ─────────────────────────────────────────────────────────────────────────

/// Constructed by the lead's integration wiring (the `CategoriesState` assembly point in
/// `main.rs`, out of this file's scope per the build brief) — not by any test in this file, which
/// exercises handlers against `fake::FakeCategoriesRepo` instead. Same "genuinely unused until a
/// caller outside this crate slice wires it up" posture `db.rs` already documents for
/// `with_tenant`/`Pools.session`.
#[allow(
    dead_code,
    reason = "constructed by the lead's CategoriesState wiring at integration — see struct doc"
)]
pub struct PgCategoriesRepo {
    pool: sqlx::PgPool,
}

#[allow(
    dead_code,
    reason = "constructed by the lead's CategoriesState wiring at integration — see struct doc"
)]
impl PgCategoriesRepo {
    pub fn new(pool: sqlx::PgPool) -> Self {
        PgCategoriesRepo { pool }
    }
}

/// Maps `with_user`'s transaction-lifecycle error onto `RepoError` (which wraps a single
/// `sqlx::Error`) — this repo is the first `with_user` caller outside `db.rs`'s own tests, so no
/// existing conversion exists to reuse. `WorkThenRollbackFailed` collapses to its `work` error,
/// the substantive failure. Only called from `PgCategoriesRepo`'s methods below, so it shares that
/// type's "unused until integration" posture.
#[allow(
    dead_code,
    reason = "only reachable through PgCategoriesRepo, unused until the lead's integration wiring — see that struct's doc"
)]
fn map_txn_err(err: crate::db::TenantTxnError) -> RepoError {
    use crate::db::TenantTxnError;
    match err {
        TenantTxnError::Begin(e)
        | TenantTxnError::SetTenant(e)
        | TenantTxnError::Work(e)
        | TenantTxnError::Commit(e) => RepoError(e),
        TenantTxnError::WorkThenRollbackFailed { work, .. } => RepoError(work),
    }
}

#[async_trait::async_trait]
impl CategoriesRepo for PgCategoriesRepo {
    async fn create(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        name: String,
        sort_order: Option<i32>,
    ) -> Result<Option<CategoryRow>, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, move |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(None);
                }
                let row: (Uuid, String, i32) = sqlx::query_as(
                    "INSERT INTO categories (location_id, name, sort_order)
                     VALUES ($1, $2, COALESCE($3, 0))
                     RETURNING id, name, sort_order",
                )
                .bind(location_id)
                .bind(&name)
                .bind(sort_order)
                .fetch_one(&mut **txn)
                .await?;
                Ok(Some(row))
            })
        })
        .await
        .map(|opt| {
            opt.map(|(id, name, sort_order)| CategoryRow {
                id,
                name,
                sort_order,
                product_count: None,
            })
        })
        .map_err(map_txn_err)
    }

    async fn list(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        cursor: Option<Uuid>,
        limit: i64,
    ) -> Result<Option<Vec<CategoryRow>>, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, move |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(None);
                }
                let rows: Vec<(Uuid, String, i32)> = if let Some(cursor) = cursor {
                    sqlx::query_as(
                        "SELECT id, name, sort_order FROM categories
                         WHERE location_id = $1 AND id > $3
                         ORDER BY id ASC LIMIT $2",
                    )
                    .bind(location_id)
                    .bind(limit)
                    .bind(cursor)
                    .fetch_all(&mut **txn)
                    .await?
                } else {
                    sqlx::query_as(
                        "SELECT id, name, sort_order FROM categories
                         WHERE location_id = $1
                         ORDER BY id ASC LIMIT $2",
                    )
                    .bind(location_id)
                    .bind(limit)
                    .fetch_all(&mut **txn)
                    .await?
                };
                Ok(Some(rows))
            })
        })
        .await
        .map(|opt| {
            opt.map(|rows| {
                rows.into_iter()
                    .map(|(id, name, sort_order)| CategoryRow {
                        id,
                        name,
                        sort_order,
                        product_count: None,
                    })
                    .collect()
            })
        })
        .map_err(map_txn_err)
    }

    async fn get(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        id: Uuid,
    ) -> Result<Option<CategoryRow>, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, move |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(None);
                }
                let row: Option<(Uuid, String, i32)> = sqlx::query_as(
                    "SELECT id, name, sort_order FROM categories WHERE location_id = $1 AND id = $2",
                )
                .bind(location_id)
                .bind(id)
                .fetch_optional(&mut **txn)
                .await?;
                Ok(row)
            })
        })
        .await
        .map(|opt| {
            opt.map(|(id, name, sort_order)| CategoryRow {
                id,
                name,
                sort_order,
                product_count: None,
            })
        })
        .map_err(map_txn_err)
    }

    async fn update(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        id: Uuid,
        name: Option<String>,
        sort_order: Option<i32>,
    ) -> Result<Option<CategoryRow>, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, move |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(None);
                }
                let row: Option<(Uuid, String, i32)> = sqlx::query_as(
                    "UPDATE categories
                     SET name = COALESCE($3, name),
                         sort_order = COALESCE($4, sort_order)
                     WHERE location_id = $1 AND id = $2
                     RETURNING id, name, sort_order",
                )
                .bind(location_id)
                .bind(id)
                .bind(name)
                .bind(sort_order)
                .fetch_optional(&mut **txn)
                .await?;
                Ok(row)
            })
        })
        .await
        .map(|opt| {
            opt.map(|(id, name, sort_order)| CategoryRow {
                id,
                name,
                sort_order,
                product_count: None,
            })
        })
        .map_err(map_txn_err)
    }

    async fn delete(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        id: Uuid,
    ) -> Result<DeleteOutcome, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, move |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(DeleteOutcome::NotFound);
                }
                // Pre-check (categories.ts:164-170): a non-empty category 409s WITHOUT deleting.
                let has_products: Option<(Uuid,)> = sqlx::query_as(
                    "SELECT id FROM products WHERE category_id = $1 AND location_id = $2 LIMIT 1",
                )
                .bind(id)
                .bind(location_id)
                .fetch_optional(&mut **txn)
                .await?;
                if has_products.is_some() {
                    return Ok(DeleteOutcome::NotEmpty);
                }
                let deleted: Option<(Uuid,)> = sqlx::query_as(
                    "DELETE FROM categories WHERE location_id = $1 AND id = $2 RETURNING id",
                )
                .bind(location_id)
                .bind(id)
                .fetch_optional(&mut **txn)
                .await?;
                Ok(if deleted.is_some() {
                    DeleteOutcome::Deleted
                } else {
                    DeleteOutcome::NotFound
                })
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn list_with_counts(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
    ) -> Result<Option<Vec<CategoryRow>>, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, move |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(None);
                }
                let rows: Vec<(Uuid, String, i32, i32)> = sqlx::query_as(
                    "SELECT c.id, c.name, c.sort_order, COUNT(p.id)::int AS product_count
                     FROM categories c
                     LEFT JOIN products p ON p.category_id = c.id AND p.location_id = $1
                     WHERE c.location_id = $1
                     GROUP BY c.id, c.name, c.sort_order
                     ORDER BY c.sort_order",
                )
                .bind(location_id)
                .fetch_all(&mut **txn)
                .await?;
                Ok(Some(rows))
            })
        })
        .await
        .map(|opt| {
            opt.map(|rows| {
                rows.into_iter()
                    .map(|(id, name, sort_order, product_count)| CategoryRow {
                        id,
                        name,
                        sort_order,
                        product_count: Some(product_count),
                    })
                    .collect()
            })
        })
        .map_err(map_txn_err)
    }

    async fn create_alias(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        name: String,
    ) -> Result<Option<CategoryRow>, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, move |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(None);
                }
                // No sort_order in the body (categories.ts:215,224) — DB column default applies.
                // Hardcoded 0::int product_count (categories.ts:225) — a brand-new category has
                // none; deliberately NOT the real COUNT() list_with_counts computes.
                let row: (Uuid, String, i32, i32) = sqlx::query_as(
                    "INSERT INTO categories (location_id, name) VALUES ($1, $2)
                     RETURNING id, name, sort_order, 0::int AS product_count",
                )
                .bind(location_id)
                .bind(&name)
                .fetch_one(&mut **txn)
                .await?;
                Ok(Some(row))
            })
        })
        .await
        .map(|opt| {
            opt.map(|(id, name, sort_order, product_count)| CategoryRow {
                id,
                name,
                sort_order,
                product_count: Some(product_count),
            })
        })
        .map_err(map_txn_err)
    }
}

// ── FakeCategoriesRepo (test-only) ──────────────────────────────────────────────────────────

#[cfg(test)]
pub mod fake {
    //! Mutex<HashMap>-backed stub, mirroring `crate::repo::fake::FakeRepo` /
    //! `crate::auth::repo::fake::FakeAuthRepo`'s style — no live Postgres needed for handler tests.

    use super::{CategoriesRepo, CategoryRow, DeleteOutcome, RepoError};
    use std::collections::{HashMap, HashSet};
    use std::sync::Mutex;
    use uuid::Uuid;

    #[derive(Default)]
    pub struct FakeCategoriesRepo {
        /// category_id -> (location_id, row).
        pub categories: Mutex<HashMap<Uuid, (Uuid, CategoryRow)>>,
        /// category_ids the DELETE pre-check should treat as still having products.
        pub has_products: Mutex<HashSet<Uuid>>,
        /// Locations where the in-transaction membership re-check (S3 breaker C1+H4) should FAIL
        /// — lets a test simulate a TOCTOU revoke between the extractor pre-check and the write.
        pub membership_denied: Mutex<HashSet<Uuid>>,
    }

    impl FakeCategoriesRepo {
        /// Seed a category fixture directly (bypassing `create`) so `get`/`list`/`update`/`delete`
        /// tests can set up rows with a chosen `id`.
        pub fn seed(&self, location_id: Uuid, row: CategoryRow) {
            self.categories
                .lock()
                .unwrap()
                .insert(row.id, (location_id, row));
        }

        pub fn mark_not_empty(&self, category_id: Uuid) {
            self.has_products.lock().unwrap().insert(category_id);
        }

        pub fn deny_membership(&self, location_id: Uuid) {
            self.membership_denied.lock().unwrap().insert(location_id);
        }

        fn membership_ok(&self, location_id: Uuid) -> bool {
            !self
                .membership_denied
                .lock()
                .unwrap()
                .contains(&location_id)
        }
    }

    #[async_trait::async_trait]
    impl CategoriesRepo for FakeCategoriesRepo {
        async fn create(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            name: String,
            sort_order: Option<i32>,
        ) -> Result<Option<CategoryRow>, RepoError> {
            if !self.membership_ok(location_id) {
                return Ok(None);
            }
            let id = Uuid::new_v4();
            let row = CategoryRow {
                id,
                name,
                sort_order: sort_order.unwrap_or(0),
                product_count: None,
            };
            self.categories
                .lock()
                .unwrap()
                .insert(id, (location_id, row.clone()));
            Ok(Some(row))
        }

        async fn list(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            cursor: Option<Uuid>,
            limit: i64,
        ) -> Result<Option<Vec<CategoryRow>>, RepoError> {
            if !self.membership_ok(location_id) {
                return Ok(None);
            }
            let cats = self.categories.lock().unwrap();
            let mut rows: Vec<CategoryRow> = cats
                .values()
                .filter(|(loc, _)| *loc == location_id)
                .map(|(_, r)| r.clone())
                .filter(|r| cursor.is_none_or(|c| r.id > c))
                .collect();
            rows.sort_by_key(|r| r.id);
            rows.truncate(usize::try_from(limit).unwrap_or(0));
            Ok(Some(rows))
        }

        async fn get(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            id: Uuid,
        ) -> Result<Option<CategoryRow>, RepoError> {
            if !self.membership_ok(location_id) {
                return Ok(None);
            }
            Ok(self
                .categories
                .lock()
                .unwrap()
                .get(&id)
                .filter(|(loc, _)| *loc == location_id)
                .map(|(_, r)| r.clone()))
        }

        async fn update(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            id: Uuid,
            name: Option<String>,
            sort_order: Option<i32>,
        ) -> Result<Option<CategoryRow>, RepoError> {
            if !self.membership_ok(location_id) {
                return Ok(None);
            }
            let mut cats = self.categories.lock().unwrap();
            match cats.get_mut(&id) {
                Some((loc, row)) if *loc == location_id => {
                    if let Some(n) = name {
                        row.name = n;
                    }
                    if let Some(s) = sort_order {
                        row.sort_order = s;
                    }
                    Ok(Some(row.clone()))
                }
                _ => Ok(None),
            }
        }

        async fn delete(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            id: Uuid,
        ) -> Result<DeleteOutcome, RepoError> {
            if !self.membership_ok(location_id) {
                return Ok(DeleteOutcome::NotFound);
            }
            let belongs = self
                .categories
                .lock()
                .unwrap()
                .get(&id)
                .is_some_and(|(loc, _)| *loc == location_id);
            if !belongs {
                return Ok(DeleteOutcome::NotFound);
            }
            if self.has_products.lock().unwrap().contains(&id) {
                return Ok(DeleteOutcome::NotEmpty);
            }
            self.categories.lock().unwrap().remove(&id);
            Ok(DeleteOutcome::Deleted)
        }

        async fn list_with_counts(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
        ) -> Result<Option<Vec<CategoryRow>>, RepoError> {
            if !self.membership_ok(location_id) {
                return Ok(None);
            }
            let cats = self.categories.lock().unwrap();
            let mut rows: Vec<CategoryRow> = cats
                .values()
                .filter(|(loc, _)| *loc == location_id)
                .map(|(_, r)| {
                    let mut r = r.clone();
                    if r.product_count.is_none() {
                        r.product_count = Some(0);
                    }
                    r
                })
                .collect();
            rows.sort_by_key(|r| r.sort_order);
            Ok(Some(rows))
        }

        async fn create_alias(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            name: String,
        ) -> Result<Option<CategoryRow>, RepoError> {
            if !self.membership_ok(location_id) {
                return Ok(None);
            }
            let id = Uuid::new_v4();
            let row = CategoryRow {
                id,
                name,
                sort_order: 0,
                product_count: Some(0),
            };
            self.categories
                .lock()
                .unwrap()
                .insert(id, (location_id, row.clone()));
            Ok(Some(row))
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::fake::FakeCategoriesRepo;
    use super::*;
    use crate::auth::claims::OwnerClaims;
    use crate::auth::repo::fake::FakeAuthRepo;
    use std::sync::{Arc, Mutex};

    fn request_id() -> RequestId {
        RequestId::new(axum::http::HeaderValue::from_static("corr-1"))
    }

    fn owner_with_location(user_id: Uuid, loc: Uuid) -> AuthState {
        AuthState::test_state(Arc::new(FakeAuthRepo {
            active_owner_locations: Mutex::new([(user_id, vec![loc])].into_iter().collect()),
            ..Default::default()
        }))
    }

    fn state_with(repo: FakeCategoriesRepo, auth: AuthState) -> CategoriesState {
        CategoriesState {
            auth,
            repo: Arc::new(repo),
        }
    }

    // ── op #1 create_category ──

    #[tokio::test]
    async fn create_category_201_happy_path() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = state_with(
            FakeCategoriesRepo::default(),
            owner_with_location(user_id, loc),
        );
        let owner = OwnerClaims::new(user_id, Some(loc));

        let response = create_category(
            Extension(state),
            OwnerClaimsExt(owner),
            Path(loc),
            Extension(request_id()),
            Json(CreateCategoryRequest {
                name: "Pizza".to_string(),
                sort_order: Some(2),
                image_key: None,
            }),
        )
        .await
        .unwrap()
        .into_response();

        assert_eq!(response.status(), StatusCode::CREATED);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: CategoryResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body.name, "Pizza");
        assert_eq!(body.sort_order, 2);
        assert_eq!(body.product_count, 0);
    }

    #[tokio::test]
    async fn create_category_404_for_a_foreign_location() {
        let user_id = Uuid::new_v4();
        let mine = Uuid::new_v4();
        let theirs = Uuid::new_v4();
        let state = state_with(
            FakeCategoriesRepo::default(),
            owner_with_location(user_id, mine),
        );
        let owner = OwnerClaims::new(user_id, Some(mine));

        let err = crate::error::expect_err(
            create_category(
                Extension(state),
                OwnerClaimsExt(owner),
                Path(theirs),
                Extension(request_id()),
                Json(CreateCategoryRequest {
                    name: "Pizza".to_string(),
                    sort_order: None,
                    image_key: None,
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn create_category_404_when_in_transaction_membership_recheck_fails() {
        // S3 breaker C1+H4: the extractor pre-check passes (loc IS an active membership per
        // AuthState.repo) but the in-transaction re-check on the write's own connection fails —
        // proves the handler still 404s rather than trusting the out-of-band check alone.
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeCategoriesRepo::default();
        repo.deny_membership(loc);
        let state = state_with(repo, owner_with_location(user_id, loc));
        let owner = OwnerClaims::new(user_id, Some(loc));

        let err = crate::error::expect_err(
            create_category(
                Extension(state),
                OwnerClaimsExt(owner),
                Path(loc),
                Extension(request_id()),
                Json(CreateCategoryRequest {
                    name: "Pizza".to_string(),
                    sort_order: None,
                    image_key: None,
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    #[test]
    fn create_category_request_rejects_an_unknown_field() {
        let json = serde_json::json!({ "name": "Pizza", "extra": "nope" });
        assert!(serde_json::from_value::<CreateCategoryRequest>(json).is_err());
    }

    // ── op #2 list_categories ──

    #[tokio::test]
    async fn list_categories_200_happy_path() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeCategoriesRepo::default();
        repo.seed(
            loc,
            CategoryRow {
                id: Uuid::new_v4(),
                name: "Drinks".to_string(),
                sort_order: 0,
                product_count: None,
            },
        );
        let state = state_with(repo, owner_with_location(user_id, loc));
        let owner = OwnerClaims::new(user_id, Some(loc));

        let response = list_categories(
            Extension(state),
            OwnerClaimsExt(owner),
            Path(loc),
            Query(ListCategoriesQuery {
                cursor: None,
                limit: None,
            }),
            Extension(request_id()),
        )
        .await
        .unwrap()
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: ListCategoriesResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body.data.len(), 1);
        assert_eq!(body.data[0].name, "Drinks");
    }

    #[tokio::test]
    async fn list_categories_404_for_a_foreign_location() {
        let user_id = Uuid::new_v4();
        let mine = Uuid::new_v4();
        let theirs = Uuid::new_v4();
        let state = state_with(
            FakeCategoriesRepo::default(),
            owner_with_location(user_id, mine),
        );
        let owner = OwnerClaims::new(user_id, Some(mine));

        let err = crate::error::expect_err(
            list_categories(
                Extension(state),
                OwnerClaimsExt(owner),
                Path(theirs),
                Query(ListCategoriesQuery {
                    cursor: None,
                    limit: None,
                }),
                Extension(request_id()),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    // ── op #3 get_category ──

    #[tokio::test]
    async fn get_category_200_happy_path() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let id = Uuid::new_v4();
        let repo = FakeCategoriesRepo::default();
        repo.seed(
            loc,
            CategoryRow {
                id,
                name: "Pizza".to_string(),
                sort_order: 0,
                product_count: None,
            },
        );
        let state = state_with(repo, owner_with_location(user_id, loc));
        let owner = OwnerClaims::new(user_id, Some(loc));

        let response = get_category(
            Extension(state),
            OwnerClaimsExt(owner),
            Path((loc, id)),
            Extension(request_id()),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn get_category_404_for_a_foreign_location() {
        let user_id = Uuid::new_v4();
        let mine = Uuid::new_v4();
        let theirs = Uuid::new_v4();
        let state = state_with(
            FakeCategoriesRepo::default(),
            owner_with_location(user_id, mine),
        );
        let owner = OwnerClaims::new(user_id, Some(mine));

        let err = crate::error::expect_err(
            get_category(
                Extension(state),
                OwnerClaimsExt(owner),
                Path((theirs, Uuid::new_v4())),
                Extension(request_id()),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn get_category_404_when_row_does_not_exist() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = state_with(
            FakeCategoriesRepo::default(),
            owner_with_location(user_id, loc),
        );
        let owner = OwnerClaims::new(user_id, Some(loc));

        let err = crate::error::expect_err(
            get_category(
                Extension(state),
                OwnerClaimsExt(owner),
                Path((loc, Uuid::new_v4())),
                Extension(request_id()),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    // ── op #4 update_category ──

    #[tokio::test]
    async fn update_category_200_happy_path() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let id = Uuid::new_v4();
        let repo = FakeCategoriesRepo::default();
        repo.seed(
            loc,
            CategoryRow {
                id,
                name: "Old".to_string(),
                sort_order: 0,
                product_count: None,
            },
        );
        let state = state_with(repo, owner_with_location(user_id, loc));
        let owner = OwnerClaims::new(user_id, Some(loc));

        let response = update_category(
            Extension(state),
            OwnerClaimsExt(owner),
            Path((loc, id)),
            Extension(request_id()),
            Json(UpdateCategoryRequest {
                name: Some("New".to_string()),
                sort_order: None,
            }),
        )
        .await
        .unwrap()
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: CategoryResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body.name, "New");
    }

    #[tokio::test]
    async fn update_category_400_no_updates_on_an_empty_body() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = state_with(
            FakeCategoriesRepo::default(),
            owner_with_location(user_id, loc),
        );
        let owner = OwnerClaims::new(user_id, Some(loc));

        let err = crate::error::expect_err(
            update_category(
                Extension(state),
                OwnerClaimsExt(owner),
                Path((loc, Uuid::new_v4())),
                Extension(request_id()),
                Json(UpdateCategoryRequest {
                    name: None,
                    sort_order: None,
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NoUpdates);
    }

    #[tokio::test]
    async fn update_category_404_for_a_foreign_location() {
        let user_id = Uuid::new_v4();
        let mine = Uuid::new_v4();
        let theirs = Uuid::new_v4();
        let state = state_with(
            FakeCategoriesRepo::default(),
            owner_with_location(user_id, mine),
        );
        let owner = OwnerClaims::new(user_id, Some(mine));

        let err = crate::error::expect_err(
            update_category(
                Extension(state),
                OwnerClaimsExt(owner),
                Path((theirs, Uuid::new_v4())),
                Extension(request_id()),
                Json(UpdateCategoryRequest {
                    name: Some("New".to_string()),
                    sort_order: None,
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn update_category_404_when_row_does_not_exist() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = state_with(
            FakeCategoriesRepo::default(),
            owner_with_location(user_id, loc),
        );
        let owner = OwnerClaims::new(user_id, Some(loc));

        let err = crate::error::expect_err(
            update_category(
                Extension(state),
                OwnerClaimsExt(owner),
                Path((loc, Uuid::new_v4())),
                Extension(request_id()),
                Json(UpdateCategoryRequest {
                    name: Some("New".to_string()),
                    sort_order: None,
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    #[test]
    fn update_category_request_rejects_an_unknown_field() {
        let json = serde_json::json!({ "name": "New", "extra": "nope" });
        assert!(serde_json::from_value::<UpdateCategoryRequest>(json).is_err());
    }

    // ── op #5 delete_category ──

    #[tokio::test]
    async fn delete_category_204_happy_path() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let id = Uuid::new_v4();
        let repo = FakeCategoriesRepo::default();
        repo.seed(
            loc,
            CategoryRow {
                id,
                name: "Pizza".to_string(),
                sort_order: 0,
                product_count: None,
            },
        );
        let state = state_with(repo, owner_with_location(user_id, loc));
        let owner = OwnerClaims::new(user_id, Some(loc));

        let response = delete_category(
            Extension(state),
            OwnerClaimsExt(owner),
            Path((loc, id)),
            Extension(request_id()),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn delete_category_404_for_a_foreign_location() {
        let user_id = Uuid::new_v4();
        let mine = Uuid::new_v4();
        let theirs = Uuid::new_v4();
        let state = state_with(
            FakeCategoriesRepo::default(),
            owner_with_location(user_id, mine),
        );
        let owner = OwnerClaims::new(user_id, Some(mine));

        let err = crate::error::expect_err(
            delete_category(
                Extension(state),
                OwnerClaimsExt(owner),
                Path((theirs, Uuid::new_v4())),
                Extension(request_id()),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn delete_category_404_when_row_does_not_exist() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = state_with(
            FakeCategoriesRepo::default(),
            owner_with_location(user_id, loc),
        );
        let owner = OwnerClaims::new(user_id, Some(loc));

        let err = crate::error::expect_err(
            delete_category(
                Extension(state),
                OwnerClaimsExt(owner),
                Path((loc, Uuid::new_v4())),
                Extension(request_id()),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn delete_category_409_when_category_still_has_products() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let id = Uuid::new_v4();
        let repo = FakeCategoriesRepo::default();
        repo.seed(
            loc,
            CategoryRow {
                id,
                name: "Pizza".to_string(),
                sort_order: 0,
                product_count: None,
            },
        );
        repo.mark_not_empty(id);
        let state = state_with(repo, owner_with_location(user_id, loc));
        let owner = OwnerClaims::new(user_id, Some(loc));

        let err = crate::error::expect_err(
            delete_category(
                Extension(state),
                OwnerClaimsExt(owner),
                Path((loc, id)),
                Extension(request_id()),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::CategoryNotEmpty);
    }

    #[tokio::test]
    async fn delete_category_404_when_in_transaction_membership_recheck_fails() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let id = Uuid::new_v4();
        let repo = FakeCategoriesRepo::default();
        repo.seed(
            loc,
            CategoryRow {
                id,
                name: "Pizza".to_string(),
                sort_order: 0,
                product_count: None,
            },
        );
        repo.deny_membership(loc);
        let state = state_with(repo, owner_with_location(user_id, loc));
        let owner = OwnerClaims::new(user_id, Some(loc));

        let err = crate::error::expect_err(
            delete_category(
                Extension(state),
                OwnerClaimsExt(owner),
                Path((loc, id)),
                Extension(request_id()),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    // ── op #6 list_categories_alias ──

    #[tokio::test]
    async fn list_categories_alias_200_with_product_counts() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeCategoriesRepo::default();
        repo.seed(
            loc,
            CategoryRow {
                id: Uuid::new_v4(),
                name: "Pizza".to_string(),
                sort_order: 0,
                product_count: Some(3),
            },
        );
        let state = state_with(repo, owner_with_location(user_id, loc));
        let owner = OwnerClaims::new(user_id, Some(loc));

        let response = list_categories_alias(
            Extension(state),
            OwnerClaimsExt(owner),
            Extension(request_id()),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: Vec<CategoryResponse> = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body.len(), 1);
        assert_eq!(body[0].product_count, 3);
    }

    #[tokio::test]
    async fn list_categories_alias_401_when_no_active_membership() {
        let user_id = Uuid::new_v4();
        let state = state_with(
            FakeCategoriesRepo::default(),
            AuthState::test_state(Arc::new(FakeAuthRepo::default())),
        );
        let owner = OwnerClaims::new(user_id, None);

        let err = crate::error::expect_err(
            list_categories_alias(
                Extension(state),
                OwnerClaimsExt(owner),
                Extension(request_id()),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::Unauthorized);
    }

    // ── op #7 create_category_alias ──

    #[tokio::test]
    async fn create_category_alias_201_hardcodes_zero_product_count() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = state_with(
            FakeCategoriesRepo::default(),
            owner_with_location(user_id, loc),
        );
        let owner = OwnerClaims::new(user_id, Some(loc));

        let response = create_category_alias(
            Extension(state),
            OwnerClaimsExt(owner),
            Extension(request_id()),
            Json(CreateCategoryAliasRequest {
                name: "Drinks".to_string(),
            }),
        )
        .await
        .unwrap()
        .into_response();

        assert_eq!(response.status(), StatusCode::CREATED);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: CategoryResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body.product_count, 0);
    }

    #[tokio::test]
    async fn create_category_alias_401_when_no_active_membership() {
        let user_id = Uuid::new_v4();
        let state = state_with(
            FakeCategoriesRepo::default(),
            AuthState::test_state(Arc::new(FakeAuthRepo::default())),
        );
        let owner = OwnerClaims::new(user_id, None);

        let err = crate::error::expect_err(
            create_category_alias(
                Extension(state),
                OwnerClaimsExt(owner),
                Extension(request_id()),
                Json(CreateCategoryAliasRequest {
                    name: "Drinks".to_string(),
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::Unauthorized);
    }

    #[test]
    fn create_category_alias_request_rejects_an_unknown_field() {
        let json = serde_json::json!({ "name": "Drinks", "sort_order": 1 });
        assert!(serde_json::from_value::<CreateCategoryAliasRequest>(json).is_err());
    }

    // ── op #8 delete_category_alias ──

    #[tokio::test]
    async fn delete_category_alias_204_happy_path() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let id = Uuid::new_v4();
        let repo = FakeCategoriesRepo::default();
        repo.seed(
            loc,
            CategoryRow {
                id,
                name: "Pizza".to_string(),
                sort_order: 0,
                product_count: Some(0),
            },
        );
        let state = state_with(repo, owner_with_location(user_id, loc));
        let owner = OwnerClaims::new(user_id, Some(loc));

        let response = delete_category_alias(
            Extension(state),
            OwnerClaimsExt(owner),
            Path(id),
            Extension(request_id()),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn delete_category_alias_401_when_no_active_membership() {
        let user_id = Uuid::new_v4();
        let state = state_with(
            FakeCategoriesRepo::default(),
            AuthState::test_state(Arc::new(FakeAuthRepo::default())),
        );
        let owner = OwnerClaims::new(user_id, None);

        let err = crate::error::expect_err(
            delete_category_alias(
                Extension(state),
                OwnerClaimsExt(owner),
                Path(Uuid::new_v4()),
                Extension(request_id()),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::Unauthorized);
    }

    #[tokio::test]
    async fn delete_category_alias_409_when_category_still_has_products() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let id = Uuid::new_v4();
        let repo = FakeCategoriesRepo::default();
        repo.seed(
            loc,
            CategoryRow {
                id,
                name: "Pizza".to_string(),
                sort_order: 0,
                product_count: Some(2),
            },
        );
        repo.mark_not_empty(id);
        let state = state_with(repo, owner_with_location(user_id, loc));
        let owner = OwnerClaims::new(user_id, Some(loc));

        let err = crate::error::expect_err(
            delete_category_alias(
                Extension(state),
                OwnerClaimsExt(owner),
                Path(id),
                Extension(request_id()),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::CategoryNotEmpty);
    }

    #[tokio::test]
    async fn delete_category_alias_404_when_row_does_not_exist() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = state_with(
            FakeCategoriesRepo::default(),
            owner_with_location(user_id, loc),
        );
        let owner = OwnerClaims::new(user_id, Some(loc));

        let err = crate::error::expect_err(
            delete_category_alias(
                Extension(state),
                OwnerClaimsExt(owner),
                Path(Uuid::new_v4()),
                Extension(request_id()),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }
}
