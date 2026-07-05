//! S3 catalog/admin CRUD — `products` vertical. Ports `apps/api/src/routes/owner/products.ts`
//! (522 lines, 14 ops) VERBATIM: same status codes, same `ErrorCode`s, same message strings,
//! same validation shapes/quirks. See `routes/owner/mod.rs` module doc for the shared
//! auth/write-pattern contract (`require_location_access`/`resolve_owner_location`/
//! `assert_active_owner_membership`, `db::with_user`) every submodule in this directory follows.
//!
//! ## Op list (method, path, TS line)
//! 1. POST   `/api/owner/locations/:locationId/products` (products.ts:15) — create, 201
//! 2. GET    `/api/owner/locations/:locationId/products` (products.ts:52) — cursor-paged list, 200
//! 3. GET    `.../products/:id` (products.ts:99) — 200 or 404 NOT_FOUND "Not found"
//! 4. PATCH  `.../products/:id` (products.ts:117) — 200, 400 NO_UPDATES, or 404 NOT_FOUND
//! 5. DELETE `.../products/:id` (products.ts:168) — 204 or 404 NOT_FOUND "Not found"
//! 6. PUT    `.../products/:id/translations/:locale` (products.ts:187) — 200, 400
//!    UNSUPPORTED_LOCALE "unsupported locale", or 404 NOT_FOUND "Not found"
//! 7. GET    `.../products/:id/translations` (products.ts:239) — 200 `{data:[]}` ALWAYS, even for
//!    a nonexistent product id (x-quirk, carried verbatim — no 404 path exists for this op).
//! 8. DELETE `.../products/:id/translations/:locale` (products.ts:263) — 204 or 404 NOT_FOUND
//! 9. PUT    `.../products/:id/modifier-groups` (products.ts:289) — sync array, 200
//!    `{success:true}`, 404 NOT_FOUND "Product not found", or 400 INVALID_GROUP "Modifier group
//!    not found"
//! 10. GET   `.../products/:id/modifier-groups` (products.ts:346) — 200 `{data:[]}`
//! 11. GET   `/api/owner/menu/products` (products.ts:372) — JWT-alias, 200 array or 401
//! 12. POST  `/api/owner/menu/products` (products.ts:396) — JWT-alias create, 201 or 401
//! 13. PATCH `/api/owner/menu/products/:productId` (products.ts:440) — 200, 401, or 404
//!     "Product not found"
//! 14. DELETE `/api/owner/menu/products/:productId` (products.ts:501) — 204, 401, or 404
//!     "Product not found"
//!
//! ## In-transaction membership re-check (S3 breaker finding C1+H4)
//! Every `PgProductsRepo` method calls `super::assert_active_owner_membership` as the FIRST
//! statement inside its own `db::with_user`-seated transaction (see that function's doc in
//! `routes/owner/mod.rs`). The extractor-level `require_location_access`/`resolve_owner_location`
//! pre-checks above stay in place as a fast-path, but they run against `AuthState.repo` — a
//! DIFFERENT connection/pool than the transaction that actually performs the write — so they are
//! NOT, by themselves, the security boundary. A `false` membership result is mapped to exactly the
//! same outcome a genuine "row not found" would produce for that op (`Ok(None)`/`Ok(false)`/the
//! op's `NotFound`-shaped enum variant), so the handler's existing 404 path also covers this case.
//! For the two pure-INSERT ops (`create`/`create_for_menu`, ops #1/#12 — no existing row to
//! not-find), the check still runs first and a failure surfaces as a synthetic 404 (`Not found` /
//! op-appropriate message) — this is NEW, defense-in-depth behavior with no TS equivalent (TS has
//! no error path there at all), justified because the extractor-level check should make it
//! unreachable in practice; only a same-request revocation race would ever hit it.
//! `FakeProductsRepo` (test-only) does NOT model this check — it has no real transaction/connection
//! to run it on. This mirrors the posture `crates/api/src/db.rs` already takes for `with_user`
//! itself: the guarantee is proven by the `#[ignore]`-gated live-Postgres test on
//! `assert_active_owner_membership` in `routes/owner/mod.rs`, not by the fake-repo unit tests here.
//!
//! ## Writes: `db::with_user`, never `db::with_tenant`
//! Every read/write goes through `db::with_user(pool, owner_user_id, ...)` (seats `app.user_id`).
//! `db::with_tenant` (`app.current_tenant`) is never called — that GUC family is reserved for the
//! courier/service surfaces. Every query ALSO carries an explicit `location_id` predicate
//! (WHERE/JOIN/fold-in INSERT...SELECT...WHERE), copied verbatim from the TS SQL, independent of
//! RLS enforcement.
//!
//! ## Scoping judgment call: `RETURNING`/`SELECT` column lists, not `RETURNING *`/`mg.*`/`pt.*`
//! The live `products` table has accreted many columns from unrelated features (product-media,
//! sensor-bus/ETA, acquisition-source tracking — see `packages/db/migrations/1790000000054/66/68`)
//! that `products.ts` itself never reads or writes. Rather than modeling every column ever added
//! by another vertical's migration, `ProductRow`/`ProductModifierGroupRow` select the EXACT columns
//! `products.ts` operates on (verified against `packages/db/migrations/1780310072731_menu.ts`,
//! `.../1780338982012_product_attributes_images.ts`, `.../1790000000065_products-prep-time.ts`,
//! `.../1780338982010_menu_modifiers.ts`, `.../1790000000060_modifier-display-type.ts`) — this is
//! an explicit, typed projection rather than `RETURNING *`/`mg.*`/`pt.*`, and is a deliberate
//! parity judgment call (flagged in the build report), not an oversight.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use tower_http::request_id::RequestId;
use uuid::Uuid;

use domain::ErrorCode;

use crate::auth::extractors::OwnerClaimsExt;
use crate::db;
use crate::error::ApiError;
use crate::repo::RepoError;
use crate::routes::correlation_id_string;

use super::{assert_active_owner_membership, require_location_access, resolve_owner_location};

// ─────────────────────────────────────────────────────────────────────────────────────────────
// State + repo trait
// ─────────────────────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ProductsState {
    pub auth: crate::auth::AuthState,
    pub repo: Arc<dyn ProductsRepo>,
    /// Threaded into `crate::service::get_image_url` for the `/api/owner/menu/products*` mapped
    /// response's `imageUrl` field (`product-mapper.ts` parity) — mirrors `AppState`'s S1 fields.
    pub app_base_url: String,
    pub r2_public_url: Option<String>,
}

/// One row of `products` — the exact column projection this file's ops read/write (see module
/// doc "Scoping judgment call"). Shared by BOTH the OWNER+LOC raw-shape ops (1-5, snake_case wire
/// shape — TS sends the raw pg row) and the OWNER-only JWT-alias ops (11-14, which map this row
/// through `map_product_row` into `MappedProduct`'s camelCase shape).
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ProductRow {
    pub id: Uuid,
    pub location_id: Uuid,
    pub category_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub price: i32,
    pub prep_time_minutes: i32,
    pub is_available: bool,
    pub image_key: Option<String>,
    pub attributes: serde_json::Value,
    pub sort_order: i32,
    #[serde(serialize_with = "crate::dto::serialize_js_instant")]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ProductTranslationRow {
    pub product_id: Uuid,
    pub locale: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ProductModifierGroupRow {
    pub sort_order: i32,
    pub id: Uuid,
    pub location_id: Uuid,
    pub name: String,
    pub min_select: i32,
    pub max_select: i32,
    pub required: bool,
    pub display_type: Option<String>,
    #[serde(serialize_with = "crate::dto::serialize_js_instant")]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct NewProduct {
    pub category_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub price: i32,
    pub prep_time_minutes: i32,
    pub available: bool,
    pub image_key: Option<String>,
    pub attributes: serde_json::Value,
    /// Ignored by `create_for_menu` (op #12's INSERT has no `sort_order` column — TS relies on
    /// the DB `DEFAULT 0`, `products.ts:431-434`); used by `create` (op #1).
    pub sort_order: i32,
}

#[derive(Debug, Clone, Default)]
pub struct ListFilter {
    pub category_id: Option<Uuid>,
    pub available: Option<bool>,
    pub cursor: Option<Uuid>,
    pub limit: i64,
}

/// The resolved (alias-folded) patch for `update_for_menu` (op #13) — the wire DTO
/// (`MenuUpdateProductRequest`) is translated into this by the handler (category_id/categoryId
/// alias resolution), then the repo does the existing-attrs fetch + `apply_attrs_patch` merge +
/// `COALESCE` update in one transaction (`products.ts:462-497`).
#[derive(Debug, Clone, Default)]
pub struct MenuProductPatch {
    pub name: Option<String>,
    pub price: Option<i32>,
    pub prep_time_minutes: Option<i32>,
    pub description: Option<String>,
    pub available: Option<bool>,
    pub category_id: Option<Uuid>,
    pub image_key: Option<String>,
    /// Tri-state (absent / explicit-null / value) — `products.ts:476` checks `!== undefined`
    /// only, so an explicit `null` DOES write `null` into `attributes.stock_count` (distinct from
    /// absent, which leaves it untouched).
    pub stock_count: Option<Option<i64>>,
    pub taste: Option<Option<serde_json::Value>>,
    pub recipe_lines: Option<Option<serde_json::Value>>,
    /// NOT tri-state — `products.ts:479` checks `!== undefined && !== null`, so absent AND
    /// explicit-null both skip the merge (asymmetric vs. the three fields above; carried
    /// verbatim).
    pub attributes_extra: Option<serde_json::Value>,
}

pub enum TranslationUpsertOutcome {
    UnsupportedLocale,
    NotFound,
    Ok(ProductTranslationRow),
}

pub enum SyncOutcome {
    ProductNotFound,
    InvalidGroup,
    Success,
}

#[async_trait::async_trait]
pub trait ProductsRepo: Send + Sync {
    async fn create(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        input: NewProduct,
    ) -> Result<Option<ProductRow>, RepoError>;

    async fn list(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        filter: ListFilter,
    ) -> Result<Vec<ProductRow>, RepoError>;

    async fn get(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        id: Uuid,
    ) -> Result<Option<ProductRow>, RepoError>;

    async fn update(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        id: Uuid,
        patch: UpdateProductRequest,
    ) -> Result<Option<ProductRow>, RepoError>;

    async fn delete(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        id: Uuid,
    ) -> Result<bool, RepoError>;

    async fn upsert_translation(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
        locale: String,
        name: String,
        description: Option<String>,
    ) -> Result<TranslationUpsertOutcome, RepoError>;

    async fn list_translations(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
    ) -> Result<Vec<ProductTranslationRow>, RepoError>;

    async fn delete_translation(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
        locale: String,
    ) -> Result<bool, RepoError>;

    async fn sync_modifier_groups(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
        items: Vec<ModifierGroupSyncItem>,
    ) -> Result<SyncOutcome, RepoError>;

    async fn list_modifier_groups(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
    ) -> Result<Vec<ProductModifierGroupRow>, RepoError>;

    async fn list_for_menu(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        category_id: Option<Uuid>,
    ) -> Result<Vec<ProductRow>, RepoError>;

    async fn create_for_menu(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        input: NewProduct,
    ) -> Result<Option<ProductRow>, RepoError>;

    async fn update_for_menu(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
        patch: MenuProductPatch,
    ) -> Result<Option<ProductRow>, RepoError>;

    async fn delete_for_menu(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
    ) -> Result<bool, RepoError>;
}

/// Converts `db::TenantTxnError` (the `with_user` transaction wrapper's error) into `RepoError`.
/// Every variant already wraps a real `sqlx::Error` (see `db.rs`) EXCEPT
/// `WorkThenRollbackFailed`, which wraps two — this uses the `work` error (the original failure;
/// the rollback failure is a secondary, already-logged-by-`with_user`-callers concern) per the
/// build brief's guidance.
#[allow(
    dead_code,
    reason = "only PgProductsRepo (the real DB impl, wired into ProductsState at main.rs \
              integration time, out of this submodule's scope) calls this; this file's own \
              tests exercise FakeProductsRepo instead"
)]
fn map_txn_err(err: db::TenantTxnError) -> RepoError {
    match err {
        db::TenantTxnError::Begin(e)
        | db::TenantTxnError::SetTenant(e)
        | db::TenantTxnError::Work(e)
        | db::TenantTxnError::Commit(e) => RepoError(e),
        db::TenantTxnError::WorkThenRollbackFailed { work, .. } => RepoError(work),
    }
}

/// `attributes.stock_count`/`.taste`/`.bom` overlay + free-form `attributes` merge, shared by the
/// `/api/owner/menu/products*` create (op #12, `existing = {}`) and update (op #13, `existing` =
/// the row's current `attributes`) paths — ports `products.ts:424-428` (create) /
/// `products.ts:475-479` (update) verbatim, including the asymmetric null-handling documented on
/// `MenuProductPatch`.
fn apply_attrs_patch(
    existing: serde_json::Value,
    stock_count: Option<Option<i64>>,
    taste: Option<Option<serde_json::Value>>,
    recipe_lines: Option<Option<serde_json::Value>>,
    attributes_extra: Option<serde_json::Value>,
) -> serde_json::Value {
    let mut map = match existing {
        serde_json::Value::Object(m) => m,
        _ => serde_json::Map::new(),
    };
    if let Some(sc) = stock_count {
        map.insert(
            "stock_count".to_string(),
            sc.map(serde_json::Value::from)
                .unwrap_or(serde_json::Value::Null),
        );
    }
    if let Some(t) = taste {
        map.insert("taste".to_string(), t.unwrap_or(serde_json::Value::Null));
    }
    if let Some(rl) = recipe_lines {
        map.insert("bom".to_string(), rl.unwrap_or(serde_json::Value::Null));
    }
    if let Some(serde_json::Value::Object(extra)) = attributes_extra {
        for (k, v) in extra {
            map.insert(k, v);
        }
    }
    serde_json::Value::Object(map)
}

/// `mapProductRow` (`product-mapper.ts:5-29`) verbatim: derives `allergens`/`stockCount`/
/// `taste`/`recipeLines` from the `attributes` jsonb blob and computes `imageUrl`.
fn map_product_row(row: ProductRow, state: &ProductsState) -> MappedProduct {
    let image_url = crate::service::get_image_url(
        row.image_key.as_deref(),
        state.r2_public_url.as_deref(),
        &state.app_base_url,
    );
    let bom: Vec<serde_json::Value> = row
        .attributes
        .get("bom")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut allergens: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for line in &bom {
        if let Some(arr) = line.get("allergens").and_then(|v| v.as_array()) {
            for a in arr {
                if let Some(s) = a.as_str() {
                    allergens.insert(s.to_string());
                }
            }
        }
    }
    let stock_count = row
        .attributes
        .get("stock_count")
        .and_then(serde_json::Value::as_i64);
    let taste = row.attributes.get("taste").cloned();
    let recipe_lines = if bom.is_empty() {
        None
    } else {
        Some(serde_json::Value::Array(bom))
    };
    let allergens_out = if allergens.is_empty() {
        None
    } else {
        Some(allergens.into_iter().collect())
    };
    let attributes = if row.attributes.is_null() {
        None
    } else {
        Some(row.attributes.clone())
    };
    MappedProduct {
        id: row.id,
        name: row.name,
        price: row.price,
        prep_time_minutes: Some(row.prep_time_minutes),
        description: row.description,
        available: row.is_available,
        category_id: row.category_id,
        image_url,
        image_key: row.image_key,
        sort_order: row.sort_order,
        stock_count,
        taste,
        recipe_lines,
        allergens: allergens_out,
        attributes,
        created_at: row.created_at,
    }
}

/// Deserializes `Option<Option<T>>` so absent/explicit-null/value are distinguishable
/// (the standard `serde_with::rust::double_option` idiom, hand-rolled since this crate doesn't
/// depend on `serde_with`): `#[serde(default)]` on the field handles "key absent" (the
/// `deserialize_with` fn below is never even called then); when the key IS present, `Option<T>`'s
/// own `Deserialize` turns JSON `null` into `None` and any other value into `Some(value)`, and
/// wrapping that in one more `Some(...)` here yields the outer "key was present" marker.
fn deserialize_double_option<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    serde::Deserialize::deserialize(deserializer).map(Some)
}

fn default_prep_time() -> i32 {
    15
}

fn default_true() -> bool {
    true
}

fn default_limit() -> i64 {
    50
}

// ─────────────────────────────────────────────────────────────────────────────────────────────
// Wire DTOs
// ─────────────────────────────────────────────────────────────────────────────────────────────

/// `products.ts:21-32` (`.strict()`).
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateProductRequest {
    #[serde(default)]
    pub category_id: Option<Uuid>,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub price: i32,
    #[serde(default = "default_prep_time")]
    pub prep_time_minutes: i32,
    #[serde(default = "default_true")]
    pub available: bool,
    #[serde(default)]
    pub image_key: Option<String>,
    #[serde(default)]
    pub attributes: Option<serde_json::Value>,
    #[serde(default)]
    pub sort_order: i32,
}

/// `products.ts:58-63` (`.strict()`).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListProductsQuery {
    #[serde(default)]
    pub cursor: Option<Uuid>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub category_id: Option<Uuid>,
    #[serde(default)]
    pub available: Option<String>,
}

/// `products.ts:123-133` (`.strict()`). Tri-state (`Option<Option<T>>`) on every `.nullable()`
/// Zod field — see module doc / build brief for why this is the correct port of
/// `Object.entries(updates)`'s absent-vs-explicit-null distinction.
#[derive(Debug, Default, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateProductRequest {
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub category_id: Option<Option<Uuid>>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub description: Option<Option<String>>,
    #[serde(default)]
    pub price: Option<i32>,
    #[serde(default)]
    pub prep_time_minutes: Option<i32>,
    #[serde(default)]
    pub available: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub image_key: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub attributes: Option<Option<serde_json::Value>>,
    #[serde(default)]
    pub sort_order: Option<i32>,
}

/// `Object.keys(updates).length === 0` (`products.ts:141`) — every field absent.
fn update_product_is_empty(patch: &UpdateProductRequest) -> bool {
    patch.category_id.is_none()
        && patch.name.is_none()
        && patch.description.is_none()
        && patch.price.is_none()
        && patch.prep_time_minutes.is_none()
        && patch.available.is_none()
        && patch.image_key.is_none()
        && patch.attributes.is_none()
        && patch.sort_order.is_none()
}

/// `products.ts:193-196` (`.strict()`). Plain `Option<String>` on `description` is correct here
/// (unlike op #4): this is a PUT/upsert that always overwrites both columns, no COALESCE/leave-
/// unchanged semantics exist for it (`products.ts:213-221` binds `description` directly either
/// way).
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PutTranslationRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// One item of the `products.ts:295-298` sync array body (`.strict()` per item).
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ModifierGroupSyncItem {
    pub group_id: Uuid,
    #[serde(default)]
    pub sort_order: i32,
}

/// `products.ts:376` (`.strict()`).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MenuListQuery {
    #[serde(default)]
    pub category_id: Option<Uuid>,
}

/// `products.ts:401-415` (`.strip()` — unknown fields silently dropped, NOT rejected; hence no
/// `deny_unknown_fields` here, matching serde's default lenient behavior).
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct MenuCreateProductRequest {
    pub name: String,
    pub price: i32,
    #[serde(default = "default_prep_time")]
    pub prep_time_minutes: i32,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub available: Option<bool>,
    #[serde(default)]
    pub category_id: Option<Uuid>,
    #[serde(default, rename = "categoryId")]
    pub category_id_camel: Option<Uuid>,
    #[serde(default)]
    pub image_key: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_double_option",
        rename = "stockCount"
    )]
    pub stock_count: Option<Option<i64>>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub taste: Option<Option<serde_json::Value>>,
    #[serde(
        default,
        deserialize_with = "deserialize_double_option",
        rename = "recipeLines"
    )]
    pub recipe_lines: Option<Option<serde_json::Value>>,
    #[serde(default)]
    pub attributes: Option<serde_json::Value>,
}

/// `products.ts:446-459` (`.strip()`).
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct MenuUpdateProductRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub price: Option<i32>,
    #[serde(default)]
    pub prep_time_minutes: Option<i32>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub available: Option<bool>,
    #[serde(default)]
    pub category_id: Option<Uuid>,
    #[serde(default, rename = "categoryId")]
    pub category_id_camel: Option<Uuid>,
    #[serde(default)]
    pub image_key: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_double_option",
        rename = "stockCount"
    )]
    pub stock_count: Option<Option<i64>>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub taste: Option<Option<serde_json::Value>>,
    #[serde(
        default,
        deserialize_with = "deserialize_double_option",
        rename = "recipeLines"
    )]
    pub recipe_lines: Option<Option<serde_json::Value>>,
    #[serde(default)]
    pub attributes: Option<serde_json::Value>,
}

/// `mapProductRow`'s output shape (`product-mapper.ts:11-28`) — camelCase, used by ops #11-13.
#[derive(Debug, Clone, Serialize)]
pub struct MappedProduct {
    pub id: Uuid,
    pub name: String,
    pub price: i32,
    #[serde(rename = "prepTimeMinutes")]
    pub prep_time_minutes: Option<i32>,
    pub description: Option<String>,
    pub available: bool,
    #[serde(rename = "categoryId")]
    pub category_id: Option<Uuid>,
    #[serde(rename = "imageUrl")]
    pub image_url: Option<String>,
    #[serde(rename = "imageKey")]
    pub image_key: Option<String>,
    #[serde(rename = "sortOrder")]
    pub sort_order: i32,
    #[serde(rename = "stockCount")]
    pub stock_count: Option<i64>,
    pub taste: Option<serde_json::Value>,
    #[serde(rename = "recipeLines")]
    pub recipe_lines: Option<serde_json::Value>,
    pub allergens: Option<Vec<String>>,
    pub attributes: Option<serde_json::Value>,
    #[serde(
        rename = "createdAt",
        serialize_with = "crate::dto::serialize_js_instant"
    )]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ─────────────────────────────────────────────────────────────────────────────────────────────
// Handlers — OWNER+LOC ops (1-10)
// ─────────────────────────────────────────────────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/api/owner/locations/{locationId}/products",
    params(("locationId" = Uuid, Path)),
    responses(
        (status = 201, description = "Product created"),
        (status = 404, description = "Location not on caller's active owner memberships", body = domain::ErrorEnvelope),
    ),
    tag = "owner-products"
)]
pub async fn create_product(
    Extension(state): Extension<ProductsState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(location_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<CreateProductRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    let input = NewProduct {
        category_id: body.category_id,
        name: body.name,
        description: body.description,
        price: body.price,
        prep_time_minutes: body.prep_time_minutes,
        available: body.available,
        image_key: body.image_key,
        attributes: body.attributes.unwrap_or_else(|| serde_json::json!({})),
        sort_order: body.sort_order,
    };
    let row = state
        .repo
        .create(owner.user_id, location_id, input)
        .await
        .map_err(|_err| {
            ApiError::new(
                ErrorCode::Internal,
                "internal_error",
                correlation_id.clone(),
            )
        })?
        .ok_or_else(|| ApiError::new(ErrorCode::NotFound, "Not found", correlation_id))?;
    Ok((StatusCode::CREATED, Json(row)))
}

#[utoipa::path(
    get,
    path = "/api/owner/locations/{locationId}/products",
    params(("locationId" = Uuid, Path)),
    responses((status = 200, description = "Cursor-paged product list")),
    tag = "owner-products"
)]
pub async fn list_products(
    Extension(state): Extension<ProductsState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(location_id): Path<Uuid>,
    Query(params): Query<ListProductsQuery>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    let available = match params.available.as_deref() {
        Some("true") => Some(true),
        Some("false") => Some(false),
        _ => None,
    };
    let filter = ListFilter {
        category_id: params.category_id,
        available,
        cursor: params.cursor,
        limit: params.limit.clamp(1, 100),
    };
    let rows = state
        .repo
        .list(owner.user_id, location_id, filter)
        .await
        .map_err(|_err| ApiError::new(ErrorCode::Internal, "internal_error", correlation_id))?;
    Ok(Json(serde_json::json!({ "data": rows })))
}

#[utoipa::path(
    get,
    path = "/api/owner/locations/{locationId}/products/{id}",
    params(("locationId" = Uuid, Path), ("id" = Uuid, Path)),
    responses(
        (status = 200, description = "Product"),
        (status = 404, description = "Not found", body = domain::ErrorEnvelope),
    ),
    tag = "owner-products"
)]
pub async fn get_product(
    Extension(state): Extension<ProductsState>,
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
        .map_err(|_err| {
            ApiError::new(
                ErrorCode::Internal,
                "internal_error",
                correlation_id.clone(),
            )
        })?
        .ok_or_else(|| ApiError::new(ErrorCode::NotFound, "Not found", correlation_id))?;
    Ok(Json(row))
}

#[utoipa::path(
    patch,
    path = "/api/owner/locations/{locationId}/products/{id}",
    params(("locationId" = Uuid, Path), ("id" = Uuid, Path)),
    responses(
        (status = 200, description = "Updated product"),
        (status = 400, description = "No updates provided", body = domain::ErrorEnvelope),
        (status = 404, description = "Not found", body = domain::ErrorEnvelope),
    ),
    tag = "owner-products"
)]
pub async fn update_product(
    Extension(state): Extension<ProductsState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path((location_id, id)): Path<(Uuid, Uuid)>,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<UpdateProductRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    if update_product_is_empty(&body) {
        return Err(ApiError::new(
            ErrorCode::NoUpdates,
            "No updates provided",
            correlation_id,
        ));
    }

    let row = state
        .repo
        .update(owner.user_id, location_id, id, body)
        .await
        .map_err(|_err| {
            ApiError::new(
                ErrorCode::Internal,
                "internal_error",
                correlation_id.clone(),
            )
        })?
        .ok_or_else(|| ApiError::new(ErrorCode::NotFound, "Not found", correlation_id))?;
    Ok(Json(row))
}

#[utoipa::path(
    delete,
    path = "/api/owner/locations/{locationId}/products/{id}",
    params(("locationId" = Uuid, Path), ("id" = Uuid, Path)),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Not found", body = domain::ErrorEnvelope),
    ),
    tag = "owner-products"
)]
pub async fn delete_product(
    Extension(state): Extension<ProductsState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path((location_id, id)): Path<(Uuid, Uuid)>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    let deleted = state
        .repo
        .delete(owner.user_id, location_id, id)
        .await
        .map_err(|_err| {
            ApiError::new(
                ErrorCode::Internal,
                "internal_error",
                correlation_id.clone(),
            )
        })?;
    if !deleted {
        return Err(ApiError::new(
            ErrorCode::NotFound,
            "Not found",
            correlation_id,
        ));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    put,
    path = "/api/owner/locations/{locationId}/products/{id}/translations/{locale}",
    params(("locationId" = Uuid, Path), ("id" = Uuid, Path), ("locale" = String, Path)),
    responses(
        (status = 200, description = "Upserted translation"),
        (status = 400, description = "unsupported locale", body = domain::ErrorEnvelope),
        (status = 404, description = "Not found", body = domain::ErrorEnvelope),
    ),
    tag = "owner-products"
)]
pub async fn put_product_translation(
    Extension(state): Extension<ProductsState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path((location_id, id, locale)): Path<(Uuid, Uuid, String)>,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<PutTranslationRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    let outcome = state
        .repo
        .upsert_translation(
            owner.user_id,
            location_id,
            id,
            locale,
            body.name,
            body.description,
        )
        .await
        .map_err(|_err| {
            ApiError::new(
                ErrorCode::Internal,
                "internal_error",
                correlation_id.clone(),
            )
        })?;
    match outcome {
        TranslationUpsertOutcome::UnsupportedLocale => Err(ApiError::new(
            ErrorCode::UnsupportedLocale,
            "unsupported locale",
            correlation_id,
        )),
        TranslationUpsertOutcome::NotFound => Err(ApiError::new(
            ErrorCode::NotFound,
            "Not found",
            correlation_id,
        )),
        TranslationUpsertOutcome::Ok(row) => Ok(Json(row)),
    }
}

#[utoipa::path(
    get,
    path = "/api/owner/locations/{locationId}/products/{id}/translations",
    params(("locationId" = Uuid, Path), ("id" = Uuid, Path)),
    responses((status = 200, description = "Translation list — ALWAYS 200, even for an unknown product id")),
    tag = "owner-products"
)]
pub async fn list_product_translations(
    Extension(state): Extension<ProductsState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path((location_id, id)): Path<(Uuid, Uuid)>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    let rows = state
        .repo
        .list_translations(owner.user_id, location_id, id)
        .await
        .map_err(|_err| ApiError::new(ErrorCode::Internal, "internal_error", correlation_id))?;
    Ok(Json(serde_json::json!({ "data": rows })))
}

#[utoipa::path(
    delete,
    path = "/api/owner/locations/{locationId}/products/{id}/translations/{locale}",
    params(("locationId" = Uuid, Path), ("id" = Uuid, Path), ("locale" = String, Path)),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Not found", body = domain::ErrorEnvelope),
    ),
    tag = "owner-products"
)]
pub async fn delete_product_translation(
    Extension(state): Extension<ProductsState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path((location_id, id, locale)): Path<(Uuid, Uuid, String)>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    let deleted = state
        .repo
        .delete_translation(owner.user_id, location_id, id, locale)
        .await
        .map_err(|_err| {
            ApiError::new(
                ErrorCode::Internal,
                "internal_error",
                correlation_id.clone(),
            )
        })?;
    if !deleted {
        return Err(ApiError::new(
            ErrorCode::NotFound,
            "Not found",
            correlation_id,
        ));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    put,
    path = "/api/owner/locations/{locationId}/products/{id}/modifier-groups",
    params(("locationId" = Uuid, Path), ("id" = Uuid, Path)),
    responses(
        (status = 200, description = "{success:true}"),
        (status = 400, description = "Modifier group not found", body = domain::ErrorEnvelope),
        (status = 404, description = "Product not found", body = domain::ErrorEnvelope),
    ),
    tag = "owner-products"
)]
pub async fn sync_product_modifier_groups(
    Extension(state): Extension<ProductsState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path((location_id, id)): Path<(Uuid, Uuid)>,
    Extension(request_id): Extension<RequestId>,
    Json(items): Json<Vec<ModifierGroupSyncItem>>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    let outcome = state
        .repo
        .sync_modifier_groups(owner.user_id, location_id, id, items)
        .await
        .map_err(|_err| {
            ApiError::new(
                ErrorCode::Internal,
                "internal_error",
                correlation_id.clone(),
            )
        })?;
    match outcome {
        SyncOutcome::ProductNotFound => Err(ApiError::new(
            ErrorCode::NotFound,
            "Product not found",
            correlation_id,
        )),
        SyncOutcome::InvalidGroup => Err(ApiError::new(
            ErrorCode::InvalidGroup,
            "Modifier group not found",
            correlation_id,
        )),
        SyncOutcome::Success => Ok(Json(serde_json::json!({ "success": true }))),
    }
}

#[utoipa::path(
    get,
    path = "/api/owner/locations/{locationId}/products/{id}/modifier-groups",
    params(("locationId" = Uuid, Path), ("id" = Uuid, Path)),
    responses((status = 200, description = "Modifier group list")),
    tag = "owner-products"
)]
pub async fn list_product_modifier_groups(
    Extension(state): Extension<ProductsState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path((location_id, id)): Path<(Uuid, Uuid)>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    let rows = state
        .repo
        .list_modifier_groups(owner.user_id, location_id, id)
        .await
        .map_err(|_err| ApiError::new(ErrorCode::Internal, "internal_error", correlation_id))?;
    Ok(Json(serde_json::json!({ "data": rows })))
}

// ─────────────────────────────────────────────────────────────────────────────────────────────
// Handlers — OWNER-only JWT-alias ops (11-14)
// ─────────────────────────────────────────────────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/owner/menu/products",
    responses(
        (status = 200, description = "Mapped product list"),
        (status = 401, description = "Unauthorized", body = domain::ErrorEnvelope),
    ),
    tag = "owner-products"
)]
pub async fn list_menu_products(
    Extension(state): Extension<ProductsState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Query(params): Query<MenuListQuery>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let location_id = resolve_owner_location(&state.auth, &owner, &correlation_id).await?;

    let rows = state
        .repo
        .list_for_menu(owner.user_id, location_id, params.category_id)
        .await
        .map_err(|_err| ApiError::new(ErrorCode::Internal, "internal_error", correlation_id))?;
    let mapped: Vec<MappedProduct> = rows
        .into_iter()
        .map(|r| map_product_row(r, &state))
        .collect();
    Ok(Json(mapped))
}

#[utoipa::path(
    post,
    path = "/api/owner/menu/products",
    responses(
        (status = 201, description = "Mapped created product"),
        (status = 401, description = "Unauthorized", body = domain::ErrorEnvelope),
    ),
    tag = "owner-products"
)]
pub async fn create_menu_product(
    Extension(state): Extension<ProductsState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<MenuCreateProductRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let location_id = resolve_owner_location(&state.auth, &owner, &correlation_id).await?;

    let category_id = body.category_id.or(body.category_id_camel);
    let attributes = apply_attrs_patch(
        serde_json::json!({}),
        body.stock_count,
        body.taste,
        body.recipe_lines,
        body.attributes,
    );
    let input = NewProduct {
        category_id,
        name: body.name,
        description: body.description,
        price: body.price,
        prep_time_minutes: body.prep_time_minutes,
        available: body.available.unwrap_or(true),
        image_key: body.image_key,
        attributes,
        sort_order: 0,
    };
    let row = state
        .repo
        .create_for_menu(owner.user_id, location_id, input)
        .await
        .map_err(|_err| {
            ApiError::new(
                ErrorCode::Internal,
                "internal_error",
                correlation_id.clone(),
            )
        })?
        .ok_or_else(|| ApiError::new(ErrorCode::NotFound, "Not found", correlation_id))?;
    Ok((StatusCode::CREATED, Json(map_product_row(row, &state))))
}

#[utoipa::path(
    patch,
    path = "/api/owner/menu/products/{productId}",
    params(("productId" = Uuid, Path)),
    responses(
        (status = 200, description = "Mapped updated product"),
        (status = 401, description = "Unauthorized", body = domain::ErrorEnvelope),
        (status = 404, description = "Product not found", body = domain::ErrorEnvelope),
    ),
    tag = "owner-products"
)]
pub async fn update_menu_product(
    Extension(state): Extension<ProductsState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(product_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<MenuUpdateProductRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let location_id = resolve_owner_location(&state.auth, &owner, &correlation_id).await?;

    let category_id = body.category_id.or(body.category_id_camel);
    let patch = MenuProductPatch {
        name: body.name,
        price: body.price,
        prep_time_minutes: body.prep_time_minutes,
        description: body.description,
        available: body.available,
        category_id,
        image_key: body.image_key,
        stock_count: body.stock_count,
        taste: body.taste,
        recipe_lines: body.recipe_lines,
        attributes_extra: body.attributes,
    };
    let row = state
        .repo
        .update_for_menu(owner.user_id, location_id, product_id, patch)
        .await
        .map_err(|_err| {
            ApiError::new(
                ErrorCode::Internal,
                "internal_error",
                correlation_id.clone(),
            )
        })?
        .ok_or_else(|| ApiError::new(ErrorCode::NotFound, "Product not found", correlation_id))?;
    Ok(Json(map_product_row(row, &state)))
}

#[utoipa::path(
    delete,
    path = "/api/owner/menu/products/{productId}",
    params(("productId" = Uuid, Path)),
    responses(
        (status = 204, description = "Deleted"),
        (status = 401, description = "Unauthorized", body = domain::ErrorEnvelope),
        (status = 404, description = "Product not found", body = domain::ErrorEnvelope),
    ),
    tag = "owner-products"
)]
pub async fn delete_menu_product(
    Extension(state): Extension<ProductsState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(product_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let location_id = resolve_owner_location(&state.auth, &owner, &correlation_id).await?;

    let deleted = state
        .repo
        .delete_for_menu(owner.user_id, location_id, product_id)
        .await
        .map_err(|_err| {
            ApiError::new(
                ErrorCode::Internal,
                "internal_error",
                correlation_id.clone(),
            )
        })?;
    if !deleted {
        return Err(ApiError::new(
            ErrorCode::NotFound,
            "Product not found",
            correlation_id,
        ));
    }
    Ok(StatusCode::NO_CONTENT)
}

// ─────────────────────────────────────────────────────────────────────────────────────────────
// PgProductsRepo — the real sqlx-backed implementation
// ─────────────────────────────────────────────────────────────────────────────────────────────

#[allow(
    dead_code,
    reason = "used only by PgProductsRepo's methods below, not yet constructed anywhere until \
              main.rs wires ProductsState with a real pool (lead's integration task)"
)]
const PRODUCT_COLUMNS: &str = "id, location_id, category_id, name, description, price, \
    prep_time_minutes, is_available, image_key, attributes, sort_order, created_at";

#[allow(
    dead_code,
    reason = "the real DB-backed ProductsRepo impl; not yet constructed until main.rs wires \
              ProductsState with a live pool (lead's integration task, out of this submodule's \
              scope) — this file's own tests exercise FakeProductsRepo instead"
)]
pub struct PgProductsRepo {
    pool: sqlx::PgPool,
}

impl PgProductsRepo {
    #[allow(
        dead_code,
        reason = "constructor for the not-yet-wired PgProductsRepo, see struct doc above"
    )]
    pub fn new(pool: sqlx::PgPool) -> Self {
        PgProductsRepo { pool }
    }
}

#[async_trait::async_trait]
impl ProductsRepo for PgProductsRepo {
    async fn create(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        input: NewProduct,
    ) -> Result<Option<ProductRow>, RepoError> {
        let row = db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(None);
                }
                let sql = format!(
                    "INSERT INTO products (location_id, category_id, name, description, price, prep_time_minutes, is_available, image_key, attributes, sort_order)
                     VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
                     RETURNING {PRODUCT_COLUMNS}"
                );
                let row = sqlx::query_as::<_, ProductRow>(&sql)
                    .bind(location_id)
                    .bind(input.category_id)
                    .bind(input.name)
                    .bind(input.description)
                    .bind(input.price)
                    .bind(input.prep_time_minutes)
                    .bind(input.available)
                    .bind(input.image_key)
                    .bind(input.attributes)
                    .bind(input.sort_order)
                    .fetch_one(&mut **txn)
                    .await?;
                Ok(Some(row))
            })
        })
        .await
        .map_err(map_txn_err)?;
        Ok(row)
    }

    async fn list(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        filter: ListFilter,
    ) -> Result<Vec<ProductRow>, RepoError> {
        let rows = db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(Vec::new());
                }
                let mut qb: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(format!(
                    "SELECT {PRODUCT_COLUMNS} FROM products WHERE location_id = "
                ));
                qb.push_bind(location_id);
                if let Some(cat) = filter.category_id {
                    qb.push(" AND category_id = ");
                    qb.push_bind(cat);
                }
                if let Some(avail) = filter.available {
                    qb.push(" AND is_available = ");
                    qb.push_bind(avail);
                }
                if let Some(cursor) = filter.cursor {
                    qb.push(" AND id > ");
                    qb.push_bind(cursor);
                }
                qb.push(" ORDER BY id ASC LIMIT ");
                qb.push_bind(filter.limit);
                let rows = qb
                    .build_query_as::<ProductRow>()
                    .fetch_all(&mut **txn)
                    .await?;
                Ok(rows)
            })
        })
        .await
        .map_err(map_txn_err)?;
        Ok(rows)
    }

    async fn get(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        id: Uuid,
    ) -> Result<Option<ProductRow>, RepoError> {
        let row = db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(None);
                }
                let sql = format!(
                    "SELECT {PRODUCT_COLUMNS} FROM products WHERE location_id = $1 AND id = $2"
                );
                let row = sqlx::query_as::<_, ProductRow>(&sql)
                    .bind(location_id)
                    .bind(id)
                    .fetch_optional(&mut **txn)
                    .await?;
                Ok(row)
            })
        })
        .await
        .map_err(map_txn_err)?;
        Ok(row)
    }

    async fn update(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        id: Uuid,
        patch: UpdateProductRequest,
    ) -> Result<Option<ProductRow>, RepoError> {
        let row = db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(None);
                }
                let mut qb: sqlx::QueryBuilder<sqlx::Postgres> =
                    sqlx::QueryBuilder::new("UPDATE products SET ");
                {
                    let mut sep = qb.separated(", ");
                    if let Some(v) = patch.name {
                        sep.push("name = ");
                        sep.push_bind_unseparated(v);
                    }
                    if let Some(v) = patch.price {
                        sep.push("price = ");
                        sep.push_bind_unseparated(v);
                    }
                    if let Some(v) = patch.prep_time_minutes {
                        sep.push("prep_time_minutes = ");
                        sep.push_bind_unseparated(v);
                    }
                    if let Some(v) = patch.available {
                        sep.push("is_available = ");
                        sep.push_bind_unseparated(v);
                    }
                    if let Some(v) = patch.sort_order {
                        sep.push("sort_order = ");
                        sep.push_bind_unseparated(v);
                    }
                    if let Some(v) = patch.category_id {
                        sep.push("category_id = ");
                        sep.push_bind_unseparated(v);
                    }
                    if let Some(v) = patch.description {
                        sep.push("description = ");
                        sep.push_bind_unseparated(v);
                    }
                    if let Some(v) = patch.image_key {
                        sep.push("image_key = ");
                        sep.push_bind_unseparated(v);
                    }
                    if let Some(v) = patch.attributes {
                        sep.push("attributes = ");
                        sep.push_bind_unseparated(v);
                    }
                }
                qb.push(" WHERE location_id = ");
                qb.push_bind(location_id);
                qb.push(" AND id = ");
                qb.push_bind(id);
                qb.push(format!(" RETURNING {PRODUCT_COLUMNS}"));
                let row = qb
                    .build_query_as::<ProductRow>()
                    .fetch_optional(&mut **txn)
                    .await?;
                Ok(row)
            })
        })
        .await
        .map_err(map_txn_err)?;
        Ok(row)
    }

    async fn delete(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        id: Uuid,
    ) -> Result<bool, RepoError> {
        let deleted = db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(false);
                }
                let result = sqlx::query("DELETE FROM products WHERE location_id = $1 AND id = $2")
                    .bind(location_id)
                    .bind(id)
                    .execute(&mut **txn)
                    .await?;
                Ok(result.rows_affected() > 0)
            })
        })
        .await
        .map_err(map_txn_err)?;
        Ok(deleted)
    }

    async fn upsert_translation(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
        locale: String,
        name: String,
        description: Option<String>,
    ) -> Result<TranslationUpsertOutcome, RepoError> {
        let outcome = db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(TranslationUpsertOutcome::NotFound);
                }
                let supported: Option<(Vec<String>,)> =
                    sqlx::query_as("SELECT supported_locales FROM locations WHERE id = $1")
                        .bind(location_id)
                        .fetch_optional(&mut **txn)
                        .await?;
                let is_supported = supported
                    .map(|(locales,)| locales.contains(&locale))
                    .unwrap_or(false);
                if !is_supported {
                    return Ok(TranslationUpsertOutcome::UnsupportedLocale);
                }
                let row: Option<ProductTranslationRow> = sqlx::query_as(
                    "INSERT INTO product_translations (product_id, locale, name, description)
                     SELECT p.id, $2, $3, $4
                     FROM products p
                     WHERE p.id = $1 AND p.location_id = $5
                     ON CONFLICT (product_id, locale) DO UPDATE SET name = EXCLUDED.name, description = EXCLUDED.description
                     RETURNING product_id, locale, name, description",
                )
                .bind(product_id)
                .bind(&locale)
                .bind(&name)
                .bind(&description)
                .bind(location_id)
                .fetch_optional(&mut **txn)
                .await?;
                Ok(match row {
                    Some(r) => TranslationUpsertOutcome::Ok(r),
                    None => TranslationUpsertOutcome::NotFound,
                })
            })
        })
        .await
        .map_err(map_txn_err)?;
        Ok(outcome)
    }

    async fn list_translations(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
    ) -> Result<Vec<ProductTranslationRow>, RepoError> {
        let rows = db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(Vec::new());
                }
                let rows = sqlx::query_as::<_, ProductTranslationRow>(
                    "SELECT pt.product_id, pt.locale, pt.name, pt.description
                     FROM product_translations pt
                     JOIN products p ON p.id = pt.product_id AND p.location_id = $2
                     WHERE pt.product_id = $1",
                )
                .bind(product_id)
                .bind(location_id)
                .fetch_all(&mut **txn)
                .await?;
                Ok(rows)
            })
        })
        .await
        .map_err(map_txn_err)?;
        Ok(rows)
    }

    async fn delete_translation(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
        locale: String,
    ) -> Result<bool, RepoError> {
        let deleted = db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(false);
                }
                let result = sqlx::query(
                    "DELETE FROM product_translations pt
                     USING products p
                     WHERE p.id = pt.product_id AND p.location_id = $3
                       AND pt.product_id = $1 AND pt.locale = $2",
                )
                .bind(product_id)
                .bind(&locale)
                .bind(location_id)
                .execute(&mut **txn)
                .await?;
                Ok(result.rows_affected() > 0)
            })
        })
        .await
        .map_err(map_txn_err)?;
        Ok(deleted)
    }

    async fn sync_modifier_groups(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
        items: Vec<ModifierGroupSyncItem>,
    ) -> Result<SyncOutcome, RepoError> {
        let outcome = db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(SyncOutcome::ProductNotFound);
                }
                let owns: Option<i32> = sqlx::query_scalar(
                    "SELECT 1 FROM products WHERE id = $1 AND location_id = $2",
                )
                .bind(product_id)
                .bind(location_id)
                .fetch_optional(&mut **txn)
                .await?;
                if owns.is_none() {
                    return Ok(SyncOutcome::ProductNotFound);
                }

                // Pre-validate every group_id belongs to this location BEFORE mutating anything —
                // externally equivalent to the TS "mutate then rollback on first bad item"
                // behavior (final DB state is unchanged either way on a 400), without needing to
                // encode a business-rule failure through `sqlx::Error` for `with_user` to roll
                // back (see module doc).
                for item in &items {
                    let exists: Option<Uuid> = sqlx::query_scalar(
                        "SELECT id FROM modifier_groups WHERE id = $1 AND location_id = $2",
                    )
                    .bind(item.group_id)
                    .bind(location_id)
                    .fetch_optional(&mut **txn)
                    .await?;
                    if exists.is_none() {
                        return Ok(SyncOutcome::InvalidGroup);
                    }
                }

                sqlx::query("DELETE FROM product_modifier_groups WHERE product_id = $1 AND location_id = $2")
                    .bind(product_id)
                    .bind(location_id)
                    .execute(&mut **txn)
                    .await?;

                for item in &items {
                    sqlx::query(
                        "INSERT INTO product_modifier_groups (product_id, group_id, sort_order, location_id)
                         VALUES ($1, $2, $3, $4)",
                    )
                    .bind(product_id)
                    .bind(item.group_id)
                    .bind(item.sort_order)
                    .bind(location_id)
                    .execute(&mut **txn)
                    .await?;
                }

                Ok(SyncOutcome::Success)
            })
        })
        .await
        .map_err(map_txn_err)?;
        Ok(outcome)
    }

    async fn list_modifier_groups(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
    ) -> Result<Vec<ProductModifierGroupRow>, RepoError> {
        let rows = db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(Vec::new());
                }
                let rows = sqlx::query_as::<_, ProductModifierGroupRow>(
                    "SELECT pmg.sort_order, mg.id, mg.location_id, mg.name, mg.min_select, mg.max_select,
                            mg.required, mg.display_type, mg.created_at
                     FROM product_modifier_groups pmg
                     JOIN modifier_groups mg ON pmg.group_id = mg.id AND mg.location_id = $2
                     WHERE pmg.product_id = $1 AND pmg.location_id = $2
                     ORDER BY pmg.sort_order ASC",
                )
                .bind(product_id)
                .bind(location_id)
                .fetch_all(&mut **txn)
                .await?;
                Ok(rows)
            })
        })
        .await
        .map_err(map_txn_err)?;
        Ok(rows)
    }

    async fn list_for_menu(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        category_id: Option<Uuid>,
    ) -> Result<Vec<ProductRow>, RepoError> {
        let rows = db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(Vec::new());
                }
                let mut qb: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(format!(
                    "SELECT {PRODUCT_COLUMNS} FROM products WHERE location_id = "
                ));
                qb.push_bind(location_id);
                if let Some(cat) = category_id {
                    qb.push(" AND category_id = ");
                    qb.push_bind(cat);
                }
                qb.push(" ORDER BY sort_order");
                let rows = qb
                    .build_query_as::<ProductRow>()
                    .fetch_all(&mut **txn)
                    .await?;
                Ok(rows)
            })
        })
        .await
        .map_err(map_txn_err)?;
        Ok(rows)
    }

    async fn create_for_menu(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        input: NewProduct,
    ) -> Result<Option<ProductRow>, RepoError> {
        let row = db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(None);
                }
                let sql = format!(
                    "INSERT INTO products (location_id, category_id, name, description, price, prep_time_minutes, is_available, image_key, attributes)
                     VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
                     RETURNING {PRODUCT_COLUMNS}"
                );
                let row = sqlx::query_as::<_, ProductRow>(&sql)
                    .bind(location_id)
                    .bind(input.category_id)
                    .bind(input.name)
                    .bind(input.description)
                    .bind(input.price)
                    .bind(input.prep_time_minutes)
                    .bind(input.available)
                    .bind(input.image_key)
                    .bind(input.attributes)
                    .fetch_one(&mut **txn)
                    .await?;
                Ok(Some(row))
            })
        })
        .await
        .map_err(map_txn_err)?;
        Ok(row)
    }

    async fn update_for_menu(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
        patch: MenuProductPatch,
    ) -> Result<Option<ProductRow>, RepoError> {
        let row = db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(None);
                }
                let existing: Option<(serde_json::Value,)> = sqlx::query_as(
                    "SELECT attributes FROM products WHERE id = $1 AND location_id = $2",
                )
                .bind(product_id)
                .bind(location_id)
                .fetch_optional(&mut **txn)
                .await?;
                let Some((existing_attrs,)) = existing else {
                    return Ok(None);
                };
                let merged = apply_attrs_patch(
                    existing_attrs,
                    patch.stock_count,
                    patch.taste,
                    patch.recipe_lines,
                    patch.attributes_extra,
                );
                let sql = format!(
                    "UPDATE products SET
                        name = COALESCE($3, name),
                        price = COALESCE($4, price),
                        description = COALESCE($5, description),
                        is_available = COALESCE($6, is_available),
                        category_id = COALESCE($7, category_id),
                        image_key = COALESCE($8, image_key),
                        prep_time_minutes = COALESCE($10, prep_time_minutes),
                        attributes = $9
                     WHERE id = $1 AND location_id = $2
                     RETURNING {PRODUCT_COLUMNS}"
                );
                let row = sqlx::query_as::<_, ProductRow>(&sql)
                    .bind(product_id)
                    .bind(location_id)
                    .bind(patch.name)
                    .bind(patch.price)
                    .bind(patch.description)
                    .bind(patch.available)
                    .bind(patch.category_id)
                    .bind(patch.image_key)
                    .bind(merged)
                    .bind(patch.prep_time_minutes)
                    .fetch_optional(&mut **txn)
                    .await?;
                Ok(row)
            })
        })
        .await
        .map_err(map_txn_err)?;
        Ok(row)
    }

    async fn delete_for_menu(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
    ) -> Result<bool, RepoError> {
        let deleted = db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(false);
                }
                let result = sqlx::query("DELETE FROM products WHERE id = $1 AND location_id = $2")
                    .bind(product_id)
                    .bind(location_id)
                    .execute(&mut **txn)
                    .await?;
                Ok(result.rows_affected() > 0)
            })
        })
        .await
        .map_err(map_txn_err)?;
        Ok(deleted)
    }
}

// ─────────────────────────────────────────────────────────────────────────────────────────────
// FakeProductsRepo — in-memory `cfg(test)` stub (no real transaction, so it does NOT model the
// `assert_active_owner_membership` in-transaction re-check — see module doc).
// ─────────────────────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
pub mod fake {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    pub struct FakeProductsRepo {
        pub products: Mutex<HashMap<Uuid, ProductRow>>,
        pub translations: Mutex<HashMap<(Uuid, String), ProductTranslationRow>>,
        pub modifier_links: Mutex<HashMap<Uuid, Vec<(Uuid, i32)>>>,
        pub modifier_groups: Mutex<HashMap<Uuid, ProductModifierGroupRow>>,
        pub supported_locales: Mutex<HashMap<Uuid, Vec<String>>>,
    }

    #[async_trait::async_trait]
    impl ProductsRepo for FakeProductsRepo {
        async fn create(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            input: NewProduct,
        ) -> Result<Option<ProductRow>, RepoError> {
            let row = ProductRow {
                id: Uuid::new_v4(),
                location_id,
                category_id: input.category_id,
                name: input.name,
                description: input.description,
                price: input.price,
                prep_time_minutes: input.prep_time_minutes,
                is_available: input.available,
                image_key: input.image_key,
                attributes: input.attributes,
                sort_order: input.sort_order,
                created_at: chrono::Utc::now(),
            };
            self.products.lock().unwrap().insert(row.id, row.clone());
            Ok(Some(row))
        }

        async fn list(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            filter: ListFilter,
        ) -> Result<Vec<ProductRow>, RepoError> {
            let mut rows: Vec<ProductRow> = self
                .products
                .lock()
                .unwrap()
                .values()
                .filter(|r| r.location_id == location_id)
                .filter(|r| match filter.category_id {
                    Some(c) => r.category_id == Some(c),
                    None => true,
                })
                .filter(|r| match filter.available {
                    Some(a) => r.is_available == a,
                    None => true,
                })
                .filter(|r| match filter.cursor {
                    Some(c) => r.id > c,
                    None => true,
                })
                .cloned()
                .collect();
            rows.sort_by_key(|r| r.id);
            let limit = usize::try_from(filter.limit.max(0)).unwrap_or(usize::MAX);
            rows.truncate(limit);
            Ok(rows)
        }

        async fn get(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            id: Uuid,
        ) -> Result<Option<ProductRow>, RepoError> {
            Ok(self
                .products
                .lock()
                .unwrap()
                .get(&id)
                .filter(|r| r.location_id == location_id)
                .cloned())
        }

        async fn update(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            id: Uuid,
            patch: UpdateProductRequest,
        ) -> Result<Option<ProductRow>, RepoError> {
            let mut products = self.products.lock().unwrap();
            let Some(row) = products.get_mut(&id) else {
                return Ok(None);
            };
            if row.location_id != location_id {
                return Ok(None);
            }
            if let Some(v) = patch.category_id {
                row.category_id = v;
            }
            if let Some(v) = patch.name {
                row.name = v;
            }
            if let Some(v) = patch.description {
                row.description = v;
            }
            if let Some(v) = patch.price {
                row.price = v;
            }
            if let Some(v) = patch.prep_time_minutes {
                row.prep_time_minutes = v;
            }
            if let Some(v) = patch.available {
                row.is_available = v;
            }
            if let Some(v) = patch.image_key {
                row.image_key = v;
            }
            if let Some(v) = patch.attributes {
                row.attributes = v.unwrap_or(serde_json::Value::Null);
            }
            if let Some(v) = patch.sort_order {
                row.sort_order = v;
            }
            Ok(Some(row.clone()))
        }

        async fn delete(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            id: Uuid,
        ) -> Result<bool, RepoError> {
            let mut products = self.products.lock().unwrap();
            if products
                .get(&id)
                .is_some_and(|r| r.location_id == location_id)
            {
                products.remove(&id);
                Ok(true)
            } else {
                Ok(false)
            }
        }

        async fn upsert_translation(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            product_id: Uuid,
            locale: String,
            name: String,
            description: Option<String>,
        ) -> Result<TranslationUpsertOutcome, RepoError> {
            let supported = self
                .supported_locales
                .lock()
                .unwrap()
                .get(&location_id)
                .cloned()
                .unwrap_or_default();
            if !supported.contains(&locale) {
                return Ok(TranslationUpsertOutcome::UnsupportedLocale);
            }
            let exists = self
                .products
                .lock()
                .unwrap()
                .get(&product_id)
                .is_some_and(|r| r.location_id == location_id);
            if !exists {
                return Ok(TranslationUpsertOutcome::NotFound);
            }
            let row = ProductTranslationRow {
                product_id,
                locale: locale.clone(),
                name,
                description,
            };
            self.translations
                .lock()
                .unwrap()
                .insert((product_id, locale), row.clone());
            Ok(TranslationUpsertOutcome::Ok(row))
        }

        async fn list_translations(
            &self,
            _owner_user_id: Uuid,
            _location_id: Uuid,
            product_id: Uuid,
        ) -> Result<Vec<ProductTranslationRow>, RepoError> {
            Ok(self
                .translations
                .lock()
                .unwrap()
                .values()
                .filter(|t| t.product_id == product_id)
                .cloned()
                .collect())
        }

        async fn delete_translation(
            &self,
            _owner_user_id: Uuid,
            _location_id: Uuid,
            product_id: Uuid,
            locale: String,
        ) -> Result<bool, RepoError> {
            Ok(self
                .translations
                .lock()
                .unwrap()
                .remove(&(product_id, locale))
                .is_some())
        }

        async fn sync_modifier_groups(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            product_id: Uuid,
            items: Vec<ModifierGroupSyncItem>,
        ) -> Result<SyncOutcome, RepoError> {
            let product_exists = self
                .products
                .lock()
                .unwrap()
                .get(&product_id)
                .is_some_and(|r| r.location_id == location_id);
            if !product_exists {
                return Ok(SyncOutcome::ProductNotFound);
            }
            {
                let groups = self.modifier_groups.lock().unwrap();
                for item in &items {
                    match groups.get(&item.group_id) {
                        Some(g) if g.location_id == location_id => {}
                        _ => return Ok(SyncOutcome::InvalidGroup),
                    }
                }
            }
            let links = items
                .into_iter()
                .map(|i| (i.group_id, i.sort_order))
                .collect();
            self.modifier_links
                .lock()
                .unwrap()
                .insert(product_id, links);
            Ok(SyncOutcome::Success)
        }

        async fn list_modifier_groups(
            &self,
            _owner_user_id: Uuid,
            _location_id: Uuid,
            product_id: Uuid,
        ) -> Result<Vec<ProductModifierGroupRow>, RepoError> {
            let links = self
                .modifier_links
                .lock()
                .unwrap()
                .get(&product_id)
                .cloned()
                .unwrap_or_default();
            let groups = self.modifier_groups.lock().unwrap();
            let mut rows: Vec<ProductModifierGroupRow> = links
                .into_iter()
                .filter_map(|(gid, sort_order)| {
                    groups.get(&gid).map(|g| ProductModifierGroupRow {
                        sort_order,
                        ..g.clone()
                    })
                })
                .collect();
            rows.sort_by_key(|r| r.sort_order);
            Ok(rows)
        }

        async fn list_for_menu(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            category_id: Option<Uuid>,
        ) -> Result<Vec<ProductRow>, RepoError> {
            let mut rows: Vec<ProductRow> = self
                .products
                .lock()
                .unwrap()
                .values()
                .filter(|r| r.location_id == location_id)
                .filter(|r| match category_id {
                    Some(c) => r.category_id == Some(c),
                    None => true,
                })
                .cloned()
                .collect();
            rows.sort_by_key(|r| r.sort_order);
            Ok(rows)
        }

        async fn create_for_menu(
            &self,
            owner_user_id: Uuid,
            location_id: Uuid,
            input: NewProduct,
        ) -> Result<Option<ProductRow>, RepoError> {
            self.create(owner_user_id, location_id, input).await
        }

        async fn update_for_menu(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            product_id: Uuid,
            patch: MenuProductPatch,
        ) -> Result<Option<ProductRow>, RepoError> {
            let mut products = self.products.lock().unwrap();
            let Some(row) = products.get_mut(&product_id) else {
                return Ok(None);
            };
            if row.location_id != location_id {
                return Ok(None);
            }
            if let Some(v) = patch.name {
                row.name = v;
            }
            if let Some(v) = patch.price {
                row.price = v;
            }
            if let Some(v) = patch.prep_time_minutes {
                row.prep_time_minutes = v;
            }
            if let Some(v) = patch.description {
                row.description = Some(v);
            }
            if let Some(v) = patch.available {
                row.is_available = v;
            }
            if let Some(v) = patch.category_id {
                row.category_id = Some(v);
            }
            if let Some(v) = patch.image_key {
                row.image_key = Some(v);
            }
            row.attributes = apply_attrs_patch(
                row.attributes.clone(),
                patch.stock_count,
                patch.taste,
                patch.recipe_lines,
                patch.attributes_extra,
            );
            Ok(Some(row.clone()))
        }

        async fn delete_for_menu(
            &self,
            owner_user_id: Uuid,
            location_id: Uuid,
            product_id: Uuid,
        ) -> Result<bool, RepoError> {
            self.delete(owner_user_id, location_id, product_id).await
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::fake::FakeProductsRepo;
    use super::*;
    use crate::auth::AuthState;
    use crate::auth::repo::fake::FakeAuthRepo;
    use crate::error::expect_err;
    use axum::body::to_bytes;
    use std::sync::Mutex as StdMutex;

    fn test_state(repo: FakeProductsRepo, owner_id: Uuid, loc: Uuid) -> ProductsState {
        let auth_repo = FakeAuthRepo {
            active_owner_locations: StdMutex::new([(owner_id, vec![loc])].into_iter().collect()),
            ..Default::default()
        };
        ProductsState {
            auth: AuthState::test_state(Arc::new(auth_repo)),
            repo: Arc::new(repo),
            app_base_url: "https://dowiz.fly.dev".to_string(),
            r2_public_url: None,
        }
    }

    fn owner(user_id: Uuid, active_location_id: Option<Uuid>) -> OwnerClaimsExt {
        OwnerClaimsExt(crate::auth::claims::OwnerClaims::new(
            user_id,
            active_location_id,
        ))
    }

    fn req_id() -> Extension<RequestId> {
        Extension(RequestId::new(axum::http::HeaderValue::from_static(
            "corr-1",
        )))
    }

    async fn json_body(response: axum::response::Response) -> serde_json::Value {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        if bytes.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        }
    }

    // ── op #1: create_product ──

    #[tokio::test]
    async fn create_product_happy_path_201() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = test_state(FakeProductsRepo::default(), user_id, loc);
        let resp = create_product(
            Extension(state),
            owner(user_id, None),
            Path(loc),
            req_id(),
            Json(CreateProductRequest {
                category_id: None,
                name: "Pizza".to_string(),
                description: None,
                price: 900,
                prep_time_minutes: 15,
                available: true,
                image_key: None,
                attributes: None,
                sort_order: 0,
            }),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = json_body(resp).await;
        assert_eq!(body["name"], "Pizza");
        assert_eq!(body["price"], 900);
    }

    #[tokio::test]
    async fn create_product_cross_location_404() {
        let user_id = Uuid::new_v4();
        let mine = Uuid::new_v4();
        let theirs = Uuid::new_v4();
        let state = test_state(FakeProductsRepo::default(), user_id, mine);
        let err = expect_err(
            create_product(
                Extension(state),
                owner(user_id, None),
                Path(theirs),
                req_id(),
                Json(CreateProductRequest {
                    category_id: None,
                    name: "Pizza".to_string(),
                    description: None,
                    price: 900,
                    prep_time_minutes: 15,
                    available: true,
                    image_key: None,
                    attributes: None,
                    sort_order: 0,
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    #[test]
    fn create_product_request_rejects_unknown_field() {
        let json = serde_json::json!({
            "name": "Pizza", "price": 900, "extra": "nope",
        });
        assert!(serde_json::from_value::<CreateProductRequest>(json).is_err());
    }

    #[test]
    fn create_product_request_defaults_prep_time_and_available() {
        let json = serde_json::json!({ "name": "Pizza", "price": 900 });
        let parsed: CreateProductRequest = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.prep_time_minutes, 15);
        assert!(parsed.available);
        assert_eq!(parsed.sort_order, 0);
    }

    // ── op #2: list_products ──

    #[tokio::test]
    async fn list_products_happy_path_200() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeProductsRepo::default();
        repo.create(
            user_id,
            loc,
            NewProduct {
                category_id: None,
                name: "A".to_string(),
                description: None,
                price: 100,
                prep_time_minutes: 15,
                available: true,
                image_key: None,
                attributes: serde_json::json!({}),
                sort_order: 0,
            },
        )
        .await
        .unwrap();
        let state = test_state(repo, user_id, loc);
        let resp = list_products(
            Extension(state),
            owner(user_id, None),
            Path(loc),
            Query(ListProductsQuery {
                cursor: None,
                limit: 50,
                category_id: None,
                available: None,
            }),
            req_id(),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = json_body(resp).await;
        assert_eq!(body["data"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn list_products_cross_location_404_at_extractor_gate() {
        let user_id = Uuid::new_v4();
        let mine = Uuid::new_v4();
        let theirs = Uuid::new_v4();
        let state = test_state(FakeProductsRepo::default(), user_id, mine);
        let err = expect_err(
            list_products(
                Extension(state),
                owner(user_id, None),
                Path(theirs),
                Query(ListProductsQuery {
                    cursor: None,
                    limit: 50,
                    category_id: None,
                    available: None,
                }),
                req_id(),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    // ── op #3: get_product ──

    #[tokio::test]
    async fn get_product_happy_path_200() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeProductsRepo::default();
        let created = repo
            .create(
                user_id,
                loc,
                NewProduct {
                    category_id: None,
                    name: "A".to_string(),
                    description: None,
                    price: 100,
                    prep_time_minutes: 15,
                    available: true,
                    image_key: None,
                    attributes: serde_json::json!({}),
                    sort_order: 0,
                },
            )
            .await
            .unwrap()
            .unwrap();
        let state = test_state(repo, user_id, loc);
        let resp = get_product(
            Extension(state),
            owner(user_id, None),
            Path((loc, created.id)),
            req_id(),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn get_product_404_when_id_missing() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = test_state(FakeProductsRepo::default(), user_id, loc);
        let err = expect_err(
            get_product(
                Extension(state),
                owner(user_id, None),
                Path((loc, Uuid::new_v4())),
                req_id(),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
        assert_eq!(err.envelope.message, "Not found");
    }

    // ── op #4: update_product ──

    #[tokio::test]
    async fn update_product_no_updates_400() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = test_state(FakeProductsRepo::default(), user_id, loc);
        let err = expect_err(
            update_product(
                Extension(state),
                owner(user_id, None),
                Path((loc, Uuid::new_v4())),
                req_id(),
                Json(UpdateProductRequest::default()),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NoUpdates);
        assert_eq!(err.envelope.message, "No updates provided");
    }

    #[tokio::test]
    async fn update_product_happy_path_200() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeProductsRepo::default();
        let created = repo
            .create(
                user_id,
                loc,
                NewProduct {
                    category_id: None,
                    name: "A".to_string(),
                    description: None,
                    price: 100,
                    prep_time_minutes: 15,
                    available: true,
                    image_key: None,
                    attributes: serde_json::json!({}),
                    sort_order: 0,
                },
            )
            .await
            .unwrap()
            .unwrap();
        let state = test_state(repo, user_id, loc);
        let patch = UpdateProductRequest {
            name: Some("B".to_string()),
            ..Default::default()
        };
        let resp = update_product(
            Extension(state),
            owner(user_id, None),
            Path((loc, created.id)),
            req_id(),
            Json(patch),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = json_body(resp).await;
        assert_eq!(body["name"], "B");
    }

    #[tokio::test]
    async fn update_product_404_when_id_missing() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = test_state(FakeProductsRepo::default(), user_id, loc);
        let patch = UpdateProductRequest {
            name: Some("B".to_string()),
            ..Default::default()
        };
        let err = expect_err(
            update_product(
                Extension(state),
                owner(user_id, None),
                Path((loc, Uuid::new_v4())),
                req_id(),
                Json(patch),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    #[test]
    fn update_product_request_rejects_unknown_field() {
        let json = serde_json::json!({ "extra": 1 });
        assert!(serde_json::from_value::<UpdateProductRequest>(json).is_err());
    }

    #[test]
    fn update_product_request_distinguishes_absent_null_and_value() {
        let absent: UpdateProductRequest = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(absent.description, None);

        let explicit_null: UpdateProductRequest =
            serde_json::from_value(serde_json::json!({ "description": null })).unwrap();
        assert_eq!(explicit_null.description, Some(None));

        let value: UpdateProductRequest =
            serde_json::from_value(serde_json::json!({ "description": "hi" })).unwrap();
        assert_eq!(value.description, Some(Some("hi".to_string())));
    }

    // ── op #5: delete_product ──

    #[tokio::test]
    async fn delete_product_happy_path_204() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeProductsRepo::default();
        let created = repo
            .create(
                user_id,
                loc,
                NewProduct {
                    category_id: None,
                    name: "A".to_string(),
                    description: None,
                    price: 100,
                    prep_time_minutes: 15,
                    available: true,
                    image_key: None,
                    attributes: serde_json::json!({}),
                    sort_order: 0,
                },
            )
            .await
            .unwrap()
            .unwrap();
        let state = test_state(repo, user_id, loc);
        let resp = delete_product(
            Extension(state),
            owner(user_id, None),
            Path((loc, created.id)),
            req_id(),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn delete_product_404_when_id_missing() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = test_state(FakeProductsRepo::default(), user_id, loc);
        let err = expect_err(
            delete_product(
                Extension(state),
                owner(user_id, None),
                Path((loc, Uuid::new_v4())),
                req_id(),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    // ── op #6: put_product_translation ──

    #[tokio::test]
    async fn put_translation_unsupported_locale_400() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeProductsRepo::default();
        repo.supported_locales
            .lock()
            .unwrap()
            .insert(loc, vec!["sq".to_string()]);
        let created = repo
            .create(
                user_id,
                loc,
                NewProduct {
                    category_id: None,
                    name: "A".to_string(),
                    description: None,
                    price: 100,
                    prep_time_minutes: 15,
                    available: true,
                    image_key: None,
                    attributes: serde_json::json!({}),
                    sort_order: 0,
                },
            )
            .await
            .unwrap()
            .unwrap();
        let state = test_state(repo, user_id, loc);
        let err = expect_err(
            put_product_translation(
                Extension(state),
                owner(user_id, None),
                Path((loc, created.id, "de".to_string())),
                req_id(),
                Json(PutTranslationRequest {
                    name: "Pizza".to_string(),
                    description: None,
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::UnsupportedLocale);
        assert_eq!(err.envelope.message, "unsupported locale");
    }

    #[tokio::test]
    async fn put_translation_happy_path_200() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeProductsRepo::default();
        repo.supported_locales
            .lock()
            .unwrap()
            .insert(loc, vec!["sq".to_string(), "en".to_string()]);
        let created = repo
            .create(
                user_id,
                loc,
                NewProduct {
                    category_id: None,
                    name: "A".to_string(),
                    description: None,
                    price: 100,
                    prep_time_minutes: 15,
                    available: true,
                    image_key: None,
                    attributes: serde_json::json!({}),
                    sort_order: 0,
                },
            )
            .await
            .unwrap()
            .unwrap();
        let state = test_state(repo, user_id, loc);
        let resp = put_product_translation(
            Extension(state),
            owner(user_id, None),
            Path((loc, created.id, "en".to_string())),
            req_id(),
            Json(PutTranslationRequest {
                name: "Pizza".to_string(),
                description: None,
            }),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn put_translation_404_when_product_missing() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeProductsRepo::default();
        repo.supported_locales
            .lock()
            .unwrap()
            .insert(loc, vec!["en".to_string()]);
        let state = test_state(repo, user_id, loc);
        let err = expect_err(
            put_product_translation(
                Extension(state),
                owner(user_id, None),
                Path((loc, Uuid::new_v4(), "en".to_string())),
                req_id(),
                Json(PutTranslationRequest {
                    name: "Pizza".to_string(),
                    description: None,
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    // ── op #7: list_product_translations (never 404s) ──

    #[tokio::test]
    async fn list_translations_always_200_even_for_nonexistent_product() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = test_state(FakeProductsRepo::default(), user_id, loc);
        let resp = list_product_translations(
            Extension(state),
            owner(user_id, None),
            Path((loc, Uuid::new_v4())),
            req_id(),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = json_body(resp).await;
        assert_eq!(body["data"].as_array().unwrap().len(), 0);
    }

    // ── op #8: delete_product_translation ──

    #[tokio::test]
    async fn delete_translation_404_when_missing() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = test_state(FakeProductsRepo::default(), user_id, loc);
        let err = expect_err(
            delete_product_translation(
                Extension(state),
                owner(user_id, None),
                Path((loc, Uuid::new_v4(), "en".to_string())),
                req_id(),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn delete_translation_happy_path_204() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeProductsRepo::default();
        repo.supported_locales
            .lock()
            .unwrap()
            .insert(loc, vec!["en".to_string()]);
        let created = repo
            .create(
                user_id,
                loc,
                NewProduct {
                    category_id: None,
                    name: "A".to_string(),
                    description: None,
                    price: 100,
                    prep_time_minutes: 15,
                    available: true,
                    image_key: None,
                    attributes: serde_json::json!({}),
                    sort_order: 0,
                },
            )
            .await
            .unwrap()
            .unwrap();
        repo.upsert_translation(
            user_id,
            loc,
            created.id,
            "en".to_string(),
            "Pizza".to_string(),
            None,
        )
        .await
        .unwrap();
        let state = test_state(repo, user_id, loc);
        let resp = delete_product_translation(
            Extension(state),
            owner(user_id, None),
            Path((loc, created.id, "en".to_string())),
            req_id(),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    // ── op #9: sync_product_modifier_groups ──

    #[tokio::test]
    async fn sync_modifier_groups_product_not_found_404() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = test_state(FakeProductsRepo::default(), user_id, loc);
        let err = expect_err(
            sync_product_modifier_groups(
                Extension(state),
                owner(user_id, None),
                Path((loc, Uuid::new_v4())),
                req_id(),
                Json(vec![]),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
        assert_eq!(err.envelope.message, "Product not found");
    }

    #[tokio::test]
    async fn sync_modifier_groups_invalid_group_400() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeProductsRepo::default();
        let created = repo
            .create(
                user_id,
                loc,
                NewProduct {
                    category_id: None,
                    name: "A".to_string(),
                    description: None,
                    price: 100,
                    prep_time_minutes: 15,
                    available: true,
                    image_key: None,
                    attributes: serde_json::json!({}),
                    sort_order: 0,
                },
            )
            .await
            .unwrap()
            .unwrap();
        let state = test_state(repo, user_id, loc);
        let err = expect_err(
            sync_product_modifier_groups(
                Extension(state),
                owner(user_id, None),
                Path((loc, created.id)),
                req_id(),
                Json(vec![ModifierGroupSyncItem {
                    group_id: Uuid::new_v4(),
                    sort_order: 0,
                }]),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::InvalidGroup);
        assert_eq!(err.envelope.message, "Modifier group not found");
    }

    #[tokio::test]
    async fn sync_modifier_groups_happy_path_200() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeProductsRepo::default();
        let created = repo
            .create(
                user_id,
                loc,
                NewProduct {
                    category_id: None,
                    name: "A".to_string(),
                    description: None,
                    price: 100,
                    prep_time_minutes: 15,
                    available: true,
                    image_key: None,
                    attributes: serde_json::json!({}),
                    sort_order: 0,
                },
            )
            .await
            .unwrap()
            .unwrap();
        let group_id = Uuid::new_v4();
        repo.modifier_groups.lock().unwrap().insert(
            group_id,
            ProductModifierGroupRow {
                sort_order: 0,
                id: group_id,
                location_id: loc,
                name: "Size".to_string(),
                min_select: 0,
                max_select: 1,
                required: false,
                display_type: None,
                created_at: chrono::Utc::now(),
            },
        );
        let state = test_state(repo, user_id, loc);
        let resp = sync_product_modifier_groups(
            Extension(state),
            owner(user_id, None),
            Path((loc, created.id)),
            req_id(),
            Json(vec![ModifierGroupSyncItem {
                group_id,
                sort_order: 1,
            }]),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = json_body(resp).await;
        assert_eq!(body["success"], true);
    }

    // ── op #10: list_product_modifier_groups ──

    #[tokio::test]
    async fn list_modifier_groups_empty_when_none_synced() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = test_state(FakeProductsRepo::default(), user_id, loc);
        let resp = list_product_modifier_groups(
            Extension(state),
            owner(user_id, None),
            Path((loc, Uuid::new_v4())),
            req_id(),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = json_body(resp).await;
        assert_eq!(body["data"].as_array().unwrap().len(), 0);
    }

    // ── op #11: list_menu_products ──

    #[tokio::test]
    async fn list_menu_products_unauthorized_401_when_no_membership() {
        let user_id = Uuid::new_v4();
        let repo = FakeAuthRepo::default();
        let state = ProductsState {
            auth: AuthState::test_state(Arc::new(repo)),
            repo: Arc::new(FakeProductsRepo::default()),
            app_base_url: "https://dowiz.fly.dev".to_string(),
            r2_public_url: None,
        };
        let err = expect_err(
            list_menu_products(
                Extension(state),
                owner(user_id, None),
                Query(MenuListQuery { category_id: None }),
                req_id(),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::Unauthorized);
    }

    #[tokio::test]
    async fn list_menu_products_happy_path_200_mapped_shape() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeProductsRepo::default();
        repo.create(
            user_id,
            loc,
            NewProduct {
                category_id: None,
                name: "A".to_string(),
                description: None,
                price: 100,
                prep_time_minutes: 15,
                available: true,
                image_key: None,
                attributes: serde_json::json!({ "stock_count": 5 }),
                sort_order: 0,
            },
        )
        .await
        .unwrap();
        let state = test_state(repo, user_id, loc);
        let resp = list_menu_products(
            Extension(state),
            owner(user_id, Some(loc)),
            Query(MenuListQuery { category_id: None }),
            req_id(),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = json_body(resp).await;
        let arr = body.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["stockCount"], 5);
        assert!(arr[0].get("prepTimeMinutes").is_some());
    }

    // ── op #12: create_menu_product ──

    #[tokio::test]
    async fn create_menu_product_happy_path_201_merges_attrs() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = test_state(FakeProductsRepo::default(), user_id, loc);
        let resp = create_menu_product(
            Extension(state),
            owner(user_id, Some(loc)),
            req_id(),
            Json(MenuCreateProductRequest {
                name: "Pizza".to_string(),
                price: 900,
                prep_time_minutes: 15,
                description: None,
                available: None,
                category_id: None,
                category_id_camel: None,
                image_key: None,
                stock_count: Some(Some(5)),
                taste: None,
                recipe_lines: None,
                attributes: None,
            }),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = json_body(resp).await;
        assert_eq!(body["stockCount"], 5);
        assert_eq!(body["available"], true);
    }

    // ── op #13: update_menu_product ──

    #[tokio::test]
    async fn update_menu_product_404_when_missing() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = test_state(FakeProductsRepo::default(), user_id, loc);
        let err = expect_err(
            update_menu_product(
                Extension(state),
                owner(user_id, Some(loc)),
                Path(Uuid::new_v4()),
                req_id(),
                Json(MenuUpdateProductRequest {
                    name: Some("B".to_string()),
                    price: None,
                    prep_time_minutes: None,
                    description: None,
                    available: None,
                    category_id: None,
                    category_id_camel: None,
                    image_key: None,
                    stock_count: None,
                    taste: None,
                    recipe_lines: None,
                    attributes: None,
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
        assert_eq!(err.envelope.message, "Product not found");
    }

    #[tokio::test]
    async fn update_menu_product_explicit_null_stock_count_clears_it() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeProductsRepo::default();
        let created = repo
            .create(
                user_id,
                loc,
                NewProduct {
                    category_id: None,
                    name: "A".to_string(),
                    description: None,
                    price: 100,
                    prep_time_minutes: 15,
                    available: true,
                    image_key: None,
                    attributes: serde_json::json!({ "stock_count": 5 }),
                    sort_order: 0,
                },
            )
            .await
            .unwrap()
            .unwrap();
        let state = test_state(repo, user_id, loc);
        let resp = update_menu_product(
            Extension(state),
            owner(user_id, Some(loc)),
            Path(created.id),
            req_id(),
            Json(MenuUpdateProductRequest {
                name: None,
                price: None,
                prep_time_minutes: None,
                description: None,
                available: None,
                category_id: None,
                category_id_camel: None,
                image_key: None,
                stock_count: Some(None),
                taste: None,
                recipe_lines: None,
                attributes: None,
            }),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = json_body(resp).await;
        assert_eq!(body["stockCount"], serde_json::Value::Null);
    }

    #[test]
    fn menu_update_product_request_stock_count_null_vs_absent() {
        let absent: MenuUpdateProductRequest =
            serde_json::from_value(serde_json::json!({})).unwrap();
        assert!(absent.stock_count.is_none());
        let explicit_null: MenuUpdateProductRequest =
            serde_json::from_value(serde_json::json!({ "stockCount": null })).unwrap();
        assert_eq!(explicit_null.stock_count, Some(None));
    }

    // ── op #14: delete_menu_product ──

    #[tokio::test]
    async fn delete_menu_product_404_when_missing() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = test_state(FakeProductsRepo::default(), user_id, loc);
        let err = expect_err(
            delete_menu_product(
                Extension(state),
                owner(user_id, Some(loc)),
                Path(Uuid::new_v4()),
                req_id(),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
        assert_eq!(err.envelope.message, "Product not found");
    }

    #[tokio::test]
    async fn delete_menu_product_happy_path_204() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeProductsRepo::default();
        let created = repo
            .create(
                user_id,
                loc,
                NewProduct {
                    category_id: None,
                    name: "A".to_string(),
                    description: None,
                    price: 100,
                    prep_time_minutes: 15,
                    available: true,
                    image_key: None,
                    attributes: serde_json::json!({}),
                    sort_order: 0,
                },
            )
            .await
            .unwrap()
            .unwrap();
        let state = test_state(repo, user_id, loc);
        let resp = delete_menu_product(
            Extension(state),
            owner(user_id, Some(loc)),
            Path(created.id),
            req_id(),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    // NOTE ON AUTH COVERAGE: `OwnerClaimsExt` is structurally unconstructible from a non-owner
    // token (S2 extractor-level narrowing, already proven by S2's own extractor tests in
    // `auth/extractors.rs`), so there is no "wrong role reaches this handler" case to write here —
    // a courier/customer claim simply cannot be wrapped in `OwnerClaimsExt` in the first place.
    // Likewise, ops #11-14 (OWNER-only) have no cross-location case: `resolve_owner_location`
    // always resolves to the CALLER'S OWN active location, so there is no "target another
    // tenant's location" request shape to construct for those ops.
}
