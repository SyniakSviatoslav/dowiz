//! S3 catalog/admin CRUD — modifier groups + modifiers (owner-route census rows ~30-36). Ports
//! `apps/api/src/routes/owner/modifier-groups.ts` (231 lines, 7 ops) VERBATIM. Every op is
//! OWNER+LOC (`:locationId` in every path) — there are no JWT-alias (`resolve_owner_location`)
//! variants in this file.
//!
//! ## Auth + in-transaction membership re-check
//! See `routes/owner/mod.rs` module doc: every handler calls [`require_location_access`] first
//! (the fast-path, out-of-band pre-check against `AuthState.repo`), then every
//! `PgModifierGroupsRepo` method calls [`assert_active_owner_membership`] as the FIRST statement
//! inside its OWN `with_user`-seated transaction (S3 breaker finding C1+H4 — that transaction runs
//! on a DIFFERENT connection than the out-of-band pre-check, so the pre-check alone is not the
//! security boundary). A `false` in-transaction result is surfaced as [`Gated::NotAMember`], which
//! every handler below maps to the SAME 404 `require_location_access` would have produced.
//!
//! ## Writes: `db::with_user`, never `db::with_tenant`
//! Every write additionally carries the exact `WHERE location_id = $n` predicate (or ownership-
//! fold-in `INSERT ... SELECT ... WHERE location_id = $n`) the TS SQL has — see `crate::db` module
//! doc (Q-GUC-FAMILY).
//!
//! ## Response shape
//! Every response is a HAND-MAPPED camelCase object, never a raw row passthrough:
//! `modifierCount` is hardcoded `0` on create (`modifier-groups.ts:44`) and on the PATCH response
//! (`:113`) — a brand-new/just-patched group's count is NEVER recomputed there, even though the
//! LIST op (`:69`) computes a real `COUNT(m.id)`. This is carried verbatim, not "fixed".

use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Path};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use domain::ErrorCode;
use serde::{Deserialize, Deserializer, Serialize};
use tower_http::request_id::RequestId;
use uuid::Uuid;

use crate::auth::extractors::OwnerClaimsExt;
use crate::error::ApiError;
use crate::repo::RepoError;
use crate::routes::owner::{assert_active_owner_membership, require_location_access};

// ── Wire enum (MENU-AVAILABILITY additive render hint) ──

/// `modifier_groups.display_type` — a nullable `text` column with a CHECK constraint (NOT a pg
/// enum), see `1790000000060_modifier-display-type.ts`. `None` preserves the legacy
/// `max_select === 1` inference (`modifier-groups.ts:26`, `.nullish()`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum DisplayType {
    Radio,
    Checkbox,
    Select,
    Quantity,
}

impl DisplayType {
    /// Used by `PgModifierGroupsRepo` to bind the column on write — unreachable until the S3 lead
    /// wires `PgModifierGroupsRepo` into `AppState` (module doc), same posture as `db.rs`'s
    /// `with_tenant` allow.
    #[allow(
        dead_code,
        reason = "used by PgModifierGroupsRepo, wired in at S3 lead-integration time"
    )]
    pub const fn as_str(self) -> &'static str {
        match self {
            DisplayType::Radio => "radio",
            DisplayType::Checkbox => "checkbox",
            DisplayType::Select => "select",
            DisplayType::Quantity => "quantity",
        }
    }

    /// Defensive parse of the DB's `text` column — the CHECK constraint guarantees only these
    /// four values or NULL are ever stored, so an unrecognized value here means the DB and this
    /// binary have drifted; `None` (silently falling back to legacy inference) is the same
    /// posture `display_type ?? null` already takes for an absent value.
    #[allow(
        dead_code,
        reason = "used by PgModifierGroupsRepo's row_to_group* helpers, wired in at S3 lead-integration time"
    )]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "radio" => Some(DisplayType::Radio),
            "checkbox" => Some(DisplayType::Checkbox),
            "select" => Some(DisplayType::Select),
            "quantity" => Some(DisplayType::Quantity),
            _ => None,
        }
    }
}

/// The serde "double option" idiom: distinguishes an ABSENT key (outer `None`, `#[serde(default)]`
/// applies) from a key present with `null` (`Some(None)`) from a key present with a value
/// (`Some(Some(v))`) — needed ONLY for `UpdateModifierGroupRequest::display_type`
/// (`.nullable().optional()`, `modifier-groups.ts:85`); every other PATCH field here is a plain
/// `.optional()` (absent vs "clear it" isn't a distinction those fields make).
fn deserialize_some<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(Some)
}

fn default_min_select() -> i32 {
    0
}
fn default_max_select() -> i32 {
    1
}
fn default_available() -> bool {
    true
}

// ── Repo-layer rows/inputs ──

#[derive(Debug, Clone, PartialEq)]
pub struct ModifierGroupRow {
    pub id: Uuid,
    pub name: String,
    pub min_select: i32,
    pub max_select: i32,
    pub required: bool,
    pub display_type: Option<DisplayType>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModifierGroupListRow {
    pub id: Uuid,
    pub name: String,
    pub min_select: i32,
    pub max_select: i32,
    pub required: bool,
    pub display_type: Option<DisplayType>,
    /// `COUNT(m.id)::int` — a REAL count (`modifier-groups.ts:60,69`), unlike the hardcoded `0`
    /// on create/PATCH responses.
    pub modifier_count: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModifierRow {
    pub id: Uuid,
    pub group_id: Uuid,
    pub name: String,
    pub price_delta: i32,
    pub available: bool,
    pub sort_order: i32,
}

#[derive(Debug, Clone)]
pub struct NewModifierGroup {
    pub name: String,
    pub min_select: i32,
    pub max_select: i32,
    pub required: bool,
    pub display_type: Option<DisplayType>,
}

/// Every field `None` means "not present in the PATCH body" — `display_type` is the tri-state
/// exception (`Some(None)` means "present, clear it").
#[derive(Debug, Clone, Default)]
pub struct ModifierGroupPatch {
    pub name: Option<String>,
    pub min_select: Option<i32>,
    pub max_select: Option<i32>,
    pub required: Option<bool>,
    pub display_type: Option<Option<DisplayType>>,
}

#[derive(Debug, Clone)]
pub struct NewModifier {
    pub name: String,
    pub price_delta: i32,
    pub available: bool,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Default)]
pub struct ModifierPatch {
    pub name: Option<String>,
    pub price_delta: Option<i32>,
    pub available: Option<bool>,
    pub sort_order: Option<i32>,
}

/// Distinguishes "the in-transaction re-check (`assert_active_owner_membership`) found the caller
/// is NOT a live active owner member of this location" from "the op's own row-level result" — the
/// former is a SEPARATE, EARLIER 404 than any op-specific not-found (S3 breaker C1+H4). Every
/// handler below maps `NotAMember` to the same 404 `require_location_access` would have produced;
/// `Found(_)` carries the op's normal result (which may ITSELF be an `Option::None` "not found",
/// e.g. op 5's fold-in-INSERT miss).
#[derive(Debug, Clone, PartialEq)]
pub enum Gated<T> {
    NotAMember,
    Found(T),
}

#[async_trait::async_trait]
pub trait ModifierGroupsRepo: Send + Sync {
    /// `INSERT ... RETURNING *` (`modifier-groups.ts:36-42`).
    async fn create_group(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        input: NewModifierGroup,
    ) -> Result<Gated<ModifierGroupRow>, RepoError>;

    /// `SELECT mg.*, COUNT(m.id)::int ... GROUP BY mg.id ORDER BY mg.created_at ASC`
    /// (`modifier-groups.ts:59-67`).
    async fn list_groups(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
    ) -> Result<Gated<Vec<ModifierGroupListRow>>, RepoError>;

    /// Dynamic `UPDATE ... SET ... WHERE location_id = $1 AND id = $2 RETURNING *`
    /// (`modifier-groups.ts:96-109`). `Found(None)` = 0 rows updated (unknown/foreign id, 404).
    async fn update_group(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        id: Uuid,
        patch: ModifierGroupPatch,
    ) -> Result<Gated<Option<ModifierGroupRow>>, RepoError>;

    /// `DELETE ... WHERE location_id = $1 AND id = $2 RETURNING id` (`modifier-groups.ts:128`).
    /// `Found(false)` = 0 rows deleted (404).
    async fn delete_group(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        id: Uuid,
    ) -> Result<Gated<bool>, RepoError>;

    /// Fold-in `INSERT ... SELECT ... FROM modifier_groups mg WHERE mg.id = $2 AND
    /// mg.location_id = $1 RETURNING *` (`modifier-groups.ts:157-164`) — a foreign/unknown
    /// `group_id` inserts 0 rows; `Found(None)` is exactly that case (404 "Modifier group not
    /// found", `:166`), NOT a separate existence check.
    async fn create_modifier(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        group_id: Uuid,
        input: NewModifier,
    ) -> Result<Gated<Option<ModifierRow>>, RepoError>;

    /// Dynamic `UPDATE modifiers SET ... WHERE location_id = $1 AND id = $2 RETURNING *`
    /// (`modifier-groups.ts:193-206`).
    async fn update_modifier(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        id: Uuid,
        patch: ModifierPatch,
    ) -> Result<Gated<Option<ModifierRow>>, RepoError>;

    /// `DELETE FROM modifiers WHERE location_id = $1 AND id = $2 RETURNING id`
    /// (`modifier-groups.ts:225`).
    async fn delete_modifier(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        id: Uuid,
    ) -> Result<Gated<bool>, RepoError>;
}

#[derive(Clone)]
pub struct ModifierGroupsState {
    pub auth: crate::auth::AuthState,
    pub repo: Arc<dyn ModifierGroupsRepo>,
}

// ── Wire DTOs ──

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateModifierGroupRequest {
    pub name: String,
    #[serde(default = "default_min_select")]
    pub min_select: i32,
    #[serde(default = "default_max_select")]
    pub max_select: i32,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub display_type: Option<DisplayType>,
}

#[derive(Debug, Default, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateModifierGroupRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub min_select: Option<i32>,
    #[serde(default)]
    pub max_select: Option<i32>,
    #[serde(default)]
    pub required: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_some")]
    #[schema(value_type = Option<DisplayType>)]
    pub display_type: Option<Option<DisplayType>>,
}

impl UpdateModifierGroupRequest {
    /// Mirrors `Object.keys(updates).length === 0` (`modifier-groups.ts:94`) — every field
    /// (including the tri-state `display_type`, where `Some(None)` still counts as present) must
    /// be the OUTER `None` for this to be true.
    fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.min_select.is_none()
            && self.max_select.is_none()
            && self.required.is_none()
            && self.display_type.is_none()
    }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ModifierGroupResponse {
    pub id: Uuid,
    pub name: String,
    pub min_select: i32,
    pub max_select: i32,
    pub required: bool,
    pub display_type: Option<DisplayType>,
    pub modifier_count: i32,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ModifierGroupListResponse {
    pub data: Vec<ModifierGroupResponse>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateModifierRequest {
    pub name: String,
    #[serde(default)]
    pub price_delta: i32,
    #[serde(default = "default_available")]
    pub available: bool,
    #[serde(default)]
    pub sort_order: i32,
}

#[derive(Debug, Default, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateModifierRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub price_delta: Option<i32>,
    #[serde(default)]
    pub available: Option<bool>,
    #[serde(default)]
    pub sort_order: Option<i32>,
}

impl UpdateModifierRequest {
    /// Mirrors `Object.keys(updates).length === 0` (`modifier-groups.ts:191`).
    fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.price_delta.is_none()
            && self.available.is_none()
            && self.sort_order.is_none()
    }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ModifierResponse {
    pub id: Uuid,
    pub group_id: Uuid,
    pub name: String,
    pub price_delta: i32,
    pub available: bool,
    pub sort_order: i32,
}

fn group_response(row: ModifierGroupRow, modifier_count: i32) -> ModifierGroupResponse {
    ModifierGroupResponse {
        id: row.id,
        name: row.name,
        min_select: row.min_select,
        max_select: row.max_select,
        required: row.required,
        display_type: row.display_type,
        modifier_count,
    }
}

fn modifier_response(row: ModifierRow) -> ModifierResponse {
    ModifierResponse {
        id: row.id,
        group_id: row.group_id,
        name: row.name,
        price_delta: row.price_delta,
        available: row.available,
        sort_order: row.sort_order,
    }
}

fn not_found(correlation_id: String) -> ApiError {
    ApiError::new(ErrorCode::NotFound, "Not found", correlation_id)
}

fn internal_error(correlation_id: String) -> ApiError {
    ApiError::new(ErrorCode::Internal, "internal_error", correlation_id)
}

// ── Handlers ──

/// `POST /api/owner/locations/{locationId}/modifier-groups` (`modifier-groups.ts:14-46`).
#[utoipa::path(
    post,
    path = "/api/owner/locations/{locationId}/modifier-groups",
    tag = "owner-catalog",
    params(("locationId" = Uuid, Path)),
    request_body = CreateModifierGroupRequest,
    responses(
        (status = 201, description = "Created", body = ModifierGroupResponse),
        (status = 404, description = "Location not an active owner membership", body = domain::ErrorEnvelope),
    )
)]
pub async fn create_modifier_group(
    Extension(state): Extension<ModifierGroupsState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(location_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<CreateModifierGroupRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = crate::routes::correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    let input = NewModifierGroup {
        name: body.name,
        min_select: body.min_select,
        max_select: body.max_select,
        required: body.required,
        display_type: body.display_type,
    };

    match state
        .repo
        .create_group(owner.user_id, location_id, input)
        .await
        .map_err(|_err| internal_error(correlation_id.clone()))?
    {
        Gated::NotAMember => Err(not_found(correlation_id)),
        Gated::Found(row) => Ok((StatusCode::CREATED, Json(group_response(row, 0)))),
    }
}

/// `GET /api/owner/locations/{locationId}/modifier-groups` (`modifier-groups.ts:48-71`).
#[utoipa::path(
    get,
    path = "/api/owner/locations/{locationId}/modifier-groups",
    tag = "owner-catalog",
    params(("locationId" = Uuid, Path)),
    responses(
        (status = 200, description = "Modifier groups for this location", body = ModifierGroupListResponse),
        (status = 404, description = "Location not an active owner membership", body = domain::ErrorEnvelope),
    )
)]
pub async fn list_modifier_groups(
    Extension(state): Extension<ModifierGroupsState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(location_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = crate::routes::correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    match state
        .repo
        .list_groups(owner.user_id, location_id)
        .await
        .map_err(|_err| internal_error(correlation_id.clone()))?
    {
        Gated::NotAMember => Err(not_found(correlation_id)),
        Gated::Found(rows) => Ok(Json(ModifierGroupListResponse {
            data: rows
                .into_iter()
                .map(|r| {
                    group_response(
                        ModifierGroupRow {
                            id: r.id,
                            name: r.name,
                            min_select: r.min_select,
                            max_select: r.max_select,
                            required: r.required,
                            display_type: r.display_type,
                        },
                        r.modifier_count,
                    )
                })
                .collect(),
        })),
    }
}

/// `PATCH /api/owner/locations/{locationId}/modifier-groups/{id}` (`modifier-groups.ts:73-115`).
#[utoipa::path(
    patch,
    path = "/api/owner/locations/{locationId}/modifier-groups/{id}",
    tag = "owner-catalog",
    params(("locationId" = Uuid, Path), ("id" = Uuid, Path)),
    request_body = UpdateModifierGroupRequest,
    responses(
        (status = 200, description = "Updated", body = ModifierGroupResponse),
        (status = 400, description = "VALIDATION_FAILED — No updates provided", body = domain::ErrorEnvelope),
        (status = 404, description = "Not found", body = domain::ErrorEnvelope),
    )
)]
pub async fn update_modifier_group(
    Extension(state): Extension<ModifierGroupsState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path((location_id, id)): Path<(Uuid, Uuid)>,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<UpdateModifierGroupRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = crate::routes::correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    if body.is_empty() {
        return Err(ApiError::validation_failed_400(
            "No updates provided",
            correlation_id,
        ));
    }

    let patch = ModifierGroupPatch {
        name: body.name,
        min_select: body.min_select,
        max_select: body.max_select,
        required: body.required,
        display_type: body.display_type,
    };

    match state
        .repo
        .update_group(owner.user_id, location_id, id, patch)
        .await
        .map_err(|_err| internal_error(correlation_id.clone()))?
    {
        Gated::NotAMember => Err(not_found(correlation_id)),
        Gated::Found(None) => Err(not_found(correlation_id)),
        Gated::Found(Some(row)) => Ok(Json(group_response(row, 0))),
    }
}

/// `DELETE /api/owner/locations/{locationId}/modifier-groups/{id}` (`modifier-groups.ts:117-133`).
#[utoipa::path(
    delete,
    path = "/api/owner/locations/{locationId}/modifier-groups/{id}",
    tag = "owner-catalog",
    params(("locationId" = Uuid, Path), ("id" = Uuid, Path)),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Not found", body = domain::ErrorEnvelope),
    )
)]
pub async fn delete_modifier_group(
    Extension(state): Extension<ModifierGroupsState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path((location_id, id)): Path<(Uuid, Uuid)>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = crate::routes::correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    match state
        .repo
        .delete_group(owner.user_id, location_id, id)
        .await
        .map_err(|_err| internal_error(correlation_id.clone()))?
    {
        Gated::NotAMember => Err(not_found(correlation_id)),
        Gated::Found(false) => Err(not_found(correlation_id)),
        Gated::Found(true) => Ok(StatusCode::NO_CONTENT),
    }
}

/// `POST /api/owner/locations/{locationId}/modifier-groups/{groupId}/modifiers`
/// (`modifier-groups.ts:136-170`).
#[utoipa::path(
    post,
    path = "/api/owner/locations/{locationId}/modifier-groups/{groupId}/modifiers",
    tag = "owner-catalog",
    params(("locationId" = Uuid, Path), ("groupId" = Uuid, Path)),
    request_body = CreateModifierRequest,
    responses(
        (status = 201, description = "Created", body = ModifierResponse),
        (status = 404, description = "Location not a membership, or unknown/foreign groupId", body = domain::ErrorEnvelope),
    )
)]
pub async fn create_modifier(
    Extension(state): Extension<ModifierGroupsState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path((location_id, group_id)): Path<(Uuid, Uuid)>,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<CreateModifierRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = crate::routes::correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    let input = NewModifier {
        name: body.name,
        price_delta: body.price_delta,
        available: body.available,
        sort_order: body.sort_order,
    };

    match state
        .repo
        .create_modifier(owner.user_id, location_id, group_id, input)
        .await
        .map_err(|_err| internal_error(correlation_id.clone()))?
    {
        Gated::NotAMember => Err(not_found(correlation_id)),
        // The fold-in-INSERT matched 0 rows — a foreign/unknown groupId (modifier-groups.ts:166's
        // exact message, distinct from the generic "Not found" every other 404 in this file uses).
        Gated::Found(None) => Err(ApiError::new(
            ErrorCode::NotFound,
            "Modifier group not found",
            correlation_id,
        )),
        Gated::Found(Some(row)) => Ok((StatusCode::CREATED, Json(modifier_response(row)))),
    }
}

/// `PATCH /api/owner/locations/{locationId}/modifiers/{id}` (`modifier-groups.ts:172-212`).
#[utoipa::path(
    patch,
    path = "/api/owner/locations/{locationId}/modifiers/{id}",
    tag = "owner-catalog",
    params(("locationId" = Uuid, Path), ("id" = Uuid, Path)),
    request_body = UpdateModifierRequest,
    responses(
        (status = 200, description = "Updated", body = ModifierResponse),
        (status = 400, description = "VALIDATION_FAILED — No updates provided", body = domain::ErrorEnvelope),
        (status = 404, description = "Not found", body = domain::ErrorEnvelope),
    )
)]
pub async fn update_modifier(
    Extension(state): Extension<ModifierGroupsState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path((location_id, id)): Path<(Uuid, Uuid)>,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<UpdateModifierRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = crate::routes::correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    if body.is_empty() {
        return Err(ApiError::validation_failed_400(
            "No updates provided",
            correlation_id,
        ));
    }

    let patch = ModifierPatch {
        name: body.name,
        price_delta: body.price_delta,
        available: body.available,
        sort_order: body.sort_order,
    };

    match state
        .repo
        .update_modifier(owner.user_id, location_id, id, patch)
        .await
        .map_err(|_err| internal_error(correlation_id.clone()))?
    {
        Gated::NotAMember => Err(not_found(correlation_id)),
        Gated::Found(None) => Err(not_found(correlation_id)),
        Gated::Found(Some(row)) => Ok(Json(modifier_response(row))),
    }
}

/// `DELETE /api/owner/locations/{locationId}/modifiers/{id}` (`modifier-groups.ts:214-230`).
#[utoipa::path(
    delete,
    path = "/api/owner/locations/{locationId}/modifiers/{id}",
    tag = "owner-catalog",
    params(("locationId" = Uuid, Path), ("id" = Uuid, Path)),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Not found", body = domain::ErrorEnvelope),
    )
)]
pub async fn delete_modifier(
    Extension(state): Extension<ModifierGroupsState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path((location_id, id)): Path<(Uuid, Uuid)>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = crate::routes::correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    match state
        .repo
        .delete_modifier(owner.user_id, location_id, id)
        .await
        .map_err(|_err| internal_error(correlation_id.clone()))?
    {
        Gated::NotAMember => Err(not_found(correlation_id)),
        Gated::Found(false) => Err(not_found(correlation_id)),
        Gated::Found(true) => Ok(StatusCode::NO_CONTENT),
    }
}

// ── Real sqlx-backed repo ──
//
// Everything below (through `PgModifierGroupsRepo`) is only reachable once the S3 lead wires
// `PgModifierGroupsRepo` into `AppState` (module doc — "the lead does that at integration"); until
// then it is unconstructed/uncalled by design, same posture `db.rs` documents for `with_tenant`
// pre-courier-surface. The fake-repo-backed handler tests above already exercise every request/
// response/error-mapping path this build was scoped to prove.

#[allow(
    dead_code,
    reason = "wired in at S3 lead-integration time — see 'Real sqlx-backed repo' section doc above"
)]
fn txn_err(err: crate::db::TenantTxnError) -> RepoError {
    // TenantTxnError isn't an sqlx::Error (it also covers commit/rollback failures) — RepoError's
    // sole variant wraps sqlx::Error, so this folds the whole family into `Protocol`, a generic
    // "something unexpected happened talking to the database" carrier. Every caller only ever
    // reads this via the `Err(_err) => internal_error(...)` branch above, never the detail.
    RepoError(sqlx::Error::Protocol(err.to_string()))
}

#[allow(
    dead_code,
    clippy::type_complexity,
    reason = "wired in at S3 lead-integration time — see 'Real sqlx-backed repo' section doc above"
)]
fn row_to_group(
    (id, name, min_select, max_select, required, display_type): (
        Uuid,
        String,
        i32,
        i32,
        bool,
        Option<String>,
    ),
) -> ModifierGroupRow {
    ModifierGroupRow {
        id,
        name,
        min_select,
        max_select,
        required,
        display_type: display_type.and_then(|s| DisplayType::parse(&s)),
    }
}

#[allow(
    dead_code,
    clippy::type_complexity,
    reason = "wired in at S3 lead-integration time — see 'Real sqlx-backed repo' section doc above"
)]
fn row_to_group_with_count(
    (id, name, min_select, max_select, required, display_type, modifier_count): (
        Uuid,
        String,
        i32,
        i32,
        bool,
        Option<String>,
        i32,
    ),
) -> ModifierGroupListRow {
    ModifierGroupListRow {
        id,
        name,
        min_select,
        max_select,
        required,
        display_type: display_type.and_then(|s| DisplayType::parse(&s)),
        modifier_count,
    }
}

#[allow(
    dead_code,
    reason = "wired in at S3 lead-integration time — see 'Real sqlx-backed repo' section doc above"
)]
fn row_to_modifier(
    (id, group_id, name, price_delta, available, sort_order): (Uuid, Uuid, String, i32, bool, i32),
) -> ModifierRow {
    ModifierRow {
        id,
        group_id,
        name,
        price_delta,
        available,
        sort_order,
    }
}

#[allow(
    dead_code,
    reason = "wired in at S3 lead-integration time — see 'Real sqlx-backed repo' section doc above"
)]
pub struct PgModifierGroupsRepo {
    pool: sqlx::PgPool,
}

impl PgModifierGroupsRepo {
    #[allow(
        dead_code,
        reason = "wired in at S3 lead-integration time — see 'Real sqlx-backed repo' section doc above"
    )]
    pub fn new(pool: sqlx::PgPool) -> Self {
        PgModifierGroupsRepo { pool }
    }
}

#[async_trait::async_trait]
impl ModifierGroupsRepo for PgModifierGroupsRepo {
    async fn create_group(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        input: NewModifierGroup,
    ) -> Result<Gated<ModifierGroupRow>, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(Gated::NotAMember);
                }
                let row: (Uuid, String, i32, i32, bool, Option<String>) = sqlx::query_as(
                    "INSERT INTO modifier_groups (location_id, name, min_select, max_select, required, display_type)
                     VALUES ($1, $2, $3, $4, $5, $6)
                     RETURNING id, name, min_select, max_select, required, display_type",
                )
                .bind(location_id)
                .bind(&input.name)
                .bind(input.min_select)
                .bind(input.max_select)
                .bind(input.required)
                .bind(input.display_type.map(DisplayType::as_str))
                .fetch_one(&mut **txn)
                .await?;
                Ok(Gated::Found(row_to_group(row)))
            })
        })
        .await
        .map_err(txn_err)
    }

    async fn list_groups(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
    ) -> Result<Gated<Vec<ModifierGroupListRow>>, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(Gated::NotAMember);
                }
                let rows: Vec<(Uuid, String, i32, i32, bool, Option<String>, i32)> = sqlx::query_as(
                    "SELECT mg.id, mg.name, mg.min_select, mg.max_select, mg.required, mg.display_type,
                            COUNT(m.id)::int AS modifier_count
                       FROM modifier_groups mg
                       LEFT JOIN modifiers m ON m.group_id = mg.id AND m.location_id = mg.location_id
                      WHERE mg.location_id = $1
                      GROUP BY mg.id
                      ORDER BY mg.created_at ASC",
                )
                .bind(location_id)
                .fetch_all(&mut **txn)
                .await?;
                Ok(Gated::Found(
                    rows.into_iter().map(row_to_group_with_count).collect(),
                ))
            })
        })
        .await
        .map_err(txn_err)
    }

    async fn update_group(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        id: Uuid,
        patch: ModifierGroupPatch,
    ) -> Result<Gated<Option<ModifierGroupRow>>, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(Gated::NotAMember);
                }
                let mut qb: sqlx::QueryBuilder<sqlx::Postgres> =
                    sqlx::QueryBuilder::new("UPDATE modifier_groups SET ");
                {
                    let mut sep = qb.separated(", ");
                    if let Some(v) = patch.name {
                        sep.push("name = ").push_bind_unseparated(v);
                    }
                    if let Some(v) = patch.min_select {
                        sep.push("min_select = ").push_bind_unseparated(v);
                    }
                    if let Some(v) = patch.max_select {
                        sep.push("max_select = ").push_bind_unseparated(v);
                    }
                    if let Some(v) = patch.required {
                        sep.push("required = ").push_bind_unseparated(v);
                    }
                    if let Some(v) = patch.display_type {
                        sep.push("display_type = ")
                            .push_bind_unseparated(v.map(DisplayType::as_str));
                    }
                }
                qb.push(" WHERE location_id = ").push_bind(location_id);
                qb.push(" AND id = ").push_bind(id);
                qb.push(" RETURNING id, name, min_select, max_select, required, display_type");

                let row: Option<(Uuid, String, i32, i32, bool, Option<String>)> =
                    qb.build_query_as().fetch_optional(&mut **txn).await?;
                Ok(Gated::Found(row.map(row_to_group)))
            })
        })
        .await
        .map_err(txn_err)
    }

    async fn delete_group(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        id: Uuid,
    ) -> Result<Gated<bool>, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(Gated::NotAMember);
                }
                let result =
                    sqlx::query("DELETE FROM modifier_groups WHERE location_id = $1 AND id = $2")
                        .bind(location_id)
                        .bind(id)
                        .execute(&mut **txn)
                        .await?;
                Ok(Gated::Found(result.rows_affected() > 0))
            })
        })
        .await
        .map_err(txn_err)
    }

    async fn create_modifier(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        group_id: Uuid,
        input: NewModifier,
    ) -> Result<Gated<Option<ModifierRow>>, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(Gated::NotAMember);
                }
                // Fold group ownership into the INSERT (modifier-groups.ts:156-164) — a
                // foreign/unknown group_id matches 0 rows in the FROM/WHERE, inserting nothing.
                let row: Option<(Uuid, Uuid, String, i32, bool, i32)> = sqlx::query_as(
                    "INSERT INTO modifiers (location_id, group_id, name, price_delta, available, sort_order)
                     SELECT $1, mg.id, $3, $4, $5, $6
                       FROM modifier_groups mg
                      WHERE mg.id = $2 AND mg.location_id = $1
                     RETURNING id, group_id, name, price_delta, available, sort_order",
                )
                .bind(location_id)
                .bind(group_id)
                .bind(&input.name)
                .bind(input.price_delta)
                .bind(input.available)
                .bind(input.sort_order)
                .fetch_optional(&mut **txn)
                .await?;
                Ok(Gated::Found(row.map(row_to_modifier)))
            })
        })
        .await
        .map_err(txn_err)
    }

    async fn update_modifier(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        id: Uuid,
        patch: ModifierPatch,
    ) -> Result<Gated<Option<ModifierRow>>, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(Gated::NotAMember);
                }
                let mut qb: sqlx::QueryBuilder<sqlx::Postgres> =
                    sqlx::QueryBuilder::new("UPDATE modifiers SET ");
                {
                    let mut sep = qb.separated(", ");
                    if let Some(v) = patch.name {
                        sep.push("name = ").push_bind_unseparated(v);
                    }
                    if let Some(v) = patch.price_delta {
                        sep.push("price_delta = ").push_bind_unseparated(v);
                    }
                    if let Some(v) = patch.available {
                        sep.push("available = ").push_bind_unseparated(v);
                    }
                    if let Some(v) = patch.sort_order {
                        sep.push("sort_order = ").push_bind_unseparated(v);
                    }
                }
                qb.push(" WHERE location_id = ").push_bind(location_id);
                qb.push(" AND id = ").push_bind(id);
                qb.push(" RETURNING id, group_id, name, price_delta, available, sort_order");

                let row: Option<(Uuid, Uuid, String, i32, bool, i32)> =
                    qb.build_query_as().fetch_optional(&mut **txn).await?;
                Ok(Gated::Found(row.map(row_to_modifier)))
            })
        })
        .await
        .map_err(txn_err)
    }

    async fn delete_modifier(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        id: Uuid,
    ) -> Result<Gated<bool>, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(Gated::NotAMember);
                }
                let result =
                    sqlx::query("DELETE FROM modifiers WHERE location_id = $1 AND id = $2")
                        .bind(location_id)
                        .bind(id)
                        .execute(&mut **txn)
                        .await?;
                Ok(Gated::Found(result.rows_affected() > 0))
            })
        })
        .await
        .map_err(txn_err)
    }
}

// ── Fake repo (cfg(test) stub, mirrors crate::repo::fake::FakeRepo's style) ──

#[cfg(test)]
pub mod fake {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Debug, Clone)]
    pub struct FakeGroup {
        pub location_id: Uuid,
        pub name: String,
        pub min_select: i32,
        pub max_select: i32,
        pub required: bool,
        pub display_type: Option<DisplayType>,
    }

    #[derive(Debug, Clone)]
    pub struct FakeModifier {
        pub location_id: Uuid,
        pub group_id: Uuid,
        pub name: String,
        pub price_delta: i32,
        pub available: bool,
        pub sort_order: i32,
    }

    #[derive(Default)]
    pub struct FakeModifierGroupsRepo {
        pub groups: Mutex<HashMap<Uuid, FakeGroup>>,
        pub modifiers: Mutex<HashMap<Uuid, FakeModifier>>,
        /// When set, every method short-circuits to `Gated::NotAMember` — simulates the
        /// in-transaction `assert_active_owner_membership` re-check failing (S3 breaker C1+H4),
        /// which a fake in-process repo otherwise cannot exercise (no real Postgres transaction).
        pub force_not_a_member: Mutex<bool>,
    }

    #[async_trait::async_trait]
    impl ModifierGroupsRepo for FakeModifierGroupsRepo {
        async fn create_group(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            input: NewModifierGroup,
        ) -> Result<Gated<ModifierGroupRow>, RepoError> {
            if *self.force_not_a_member.lock().unwrap() {
                return Ok(Gated::NotAMember);
            }
            let id = Uuid::new_v4();
            self.groups.lock().unwrap().insert(
                id,
                FakeGroup {
                    location_id,
                    name: input.name.clone(),
                    min_select: input.min_select,
                    max_select: input.max_select,
                    required: input.required,
                    display_type: input.display_type,
                },
            );
            Ok(Gated::Found(ModifierGroupRow {
                id,
                name: input.name,
                min_select: input.min_select,
                max_select: input.max_select,
                required: input.required,
                display_type: input.display_type,
            }))
        }

        async fn list_groups(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
        ) -> Result<Gated<Vec<ModifierGroupListRow>>, RepoError> {
            if *self.force_not_a_member.lock().unwrap() {
                return Ok(Gated::NotAMember);
            }
            let groups = self.groups.lock().unwrap();
            let modifiers = self.modifiers.lock().unwrap();
            let mut rows: Vec<(Uuid, FakeGroup)> = groups
                .iter()
                .filter(|(_, g)| g.location_id == location_id)
                .map(|(id, g)| (*id, g.clone()))
                .collect();
            rows.sort_by_key(|(id, _)| *id);
            let data = rows
                .into_iter()
                .map(|(id, g)| {
                    let modifier_count =
                        i32::try_from(modifiers.values().filter(|m| m.group_id == id).count())
                            .unwrap_or(i32::MAX);
                    ModifierGroupListRow {
                        id,
                        name: g.name,
                        min_select: g.min_select,
                        max_select: g.max_select,
                        required: g.required,
                        display_type: g.display_type,
                        modifier_count,
                    }
                })
                .collect();
            Ok(Gated::Found(data))
        }

        async fn update_group(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            id: Uuid,
            patch: ModifierGroupPatch,
        ) -> Result<Gated<Option<ModifierGroupRow>>, RepoError> {
            if *self.force_not_a_member.lock().unwrap() {
                return Ok(Gated::NotAMember);
            }
            let mut groups = self.groups.lock().unwrap();
            let Some(group) = groups.get_mut(&id) else {
                return Ok(Gated::Found(None));
            };
            if group.location_id != location_id {
                return Ok(Gated::Found(None));
            }
            if let Some(name) = patch.name {
                group.name = name;
            }
            if let Some(v) = patch.min_select {
                group.min_select = v;
            }
            if let Some(v) = patch.max_select {
                group.max_select = v;
            }
            if let Some(v) = patch.required {
                group.required = v;
            }
            if let Some(v) = patch.display_type {
                group.display_type = v;
            }
            Ok(Gated::Found(Some(ModifierGroupRow {
                id,
                name: group.name.clone(),
                min_select: group.min_select,
                max_select: group.max_select,
                required: group.required,
                display_type: group.display_type,
            })))
        }

        async fn delete_group(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            id: Uuid,
        ) -> Result<Gated<bool>, RepoError> {
            if *self.force_not_a_member.lock().unwrap() {
                return Ok(Gated::NotAMember);
            }
            let mut groups = self.groups.lock().unwrap();
            let deleted = matches!(groups.get(&id), Some(g) if g.location_id == location_id);
            if deleted {
                groups.remove(&id);
            }
            Ok(Gated::Found(deleted))
        }

        async fn create_modifier(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            group_id: Uuid,
            input: NewModifier,
        ) -> Result<Gated<Option<ModifierRow>>, RepoError> {
            if *self.force_not_a_member.lock().unwrap() {
                return Ok(Gated::NotAMember);
            }
            let belongs = matches!(
                self.groups.lock().unwrap().get(&group_id),
                Some(g) if g.location_id == location_id
            );
            if !belongs {
                return Ok(Gated::Found(None));
            }
            let id = Uuid::new_v4();
            self.modifiers.lock().unwrap().insert(
                id,
                FakeModifier {
                    location_id,
                    group_id,
                    name: input.name.clone(),
                    price_delta: input.price_delta,
                    available: input.available,
                    sort_order: input.sort_order,
                },
            );
            Ok(Gated::Found(Some(ModifierRow {
                id,
                group_id,
                name: input.name,
                price_delta: input.price_delta,
                available: input.available,
                sort_order: input.sort_order,
            })))
        }

        async fn update_modifier(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            id: Uuid,
            patch: ModifierPatch,
        ) -> Result<Gated<Option<ModifierRow>>, RepoError> {
            if *self.force_not_a_member.lock().unwrap() {
                return Ok(Gated::NotAMember);
            }
            let mut modifiers = self.modifiers.lock().unwrap();
            let Some(m) = modifiers.get_mut(&id) else {
                return Ok(Gated::Found(None));
            };
            if m.location_id != location_id {
                return Ok(Gated::Found(None));
            }
            if let Some(name) = patch.name {
                m.name = name;
            }
            if let Some(v) = patch.price_delta {
                m.price_delta = v;
            }
            if let Some(v) = patch.available {
                m.available = v;
            }
            if let Some(v) = patch.sort_order {
                m.sort_order = v;
            }
            Ok(Gated::Found(Some(ModifierRow {
                id,
                group_id: m.group_id,
                name: m.name.clone(),
                price_delta: m.price_delta,
                available: m.available,
                sort_order: m.sort_order,
            })))
        }

        async fn delete_modifier(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            id: Uuid,
        ) -> Result<Gated<bool>, RepoError> {
            if *self.force_not_a_member.lock().unwrap() {
                return Ok(Gated::NotAMember);
            }
            let mut modifiers = self.modifiers.lock().unwrap();
            let deleted = matches!(modifiers.get(&id), Some(m) if m.location_id == location_id);
            if deleted {
                modifiers.remove(&id);
            }
            Ok(Gated::Found(deleted))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::FakeModifierGroupsRepo;
    use super::*;
    use crate::auth::claims::OwnerClaims;
    use crate::auth::repo::fake::FakeAuthRepo;
    use axum::http::HeaderValue;
    use std::sync::Mutex;

    fn test_state(
        repo: FakeModifierGroupsRepo,
        user_id: Uuid,
        active_location: Uuid,
    ) -> ModifierGroupsState {
        let auth_repo = Arc::new(FakeAuthRepo {
            active_owner_locations: Mutex::new(
                [(user_id, vec![active_location])].into_iter().collect(),
            ),
            ..Default::default()
        });
        ModifierGroupsState {
            auth: crate::auth::AuthState::test_state(auth_repo),
            repo: Arc::new(repo),
        }
    }

    fn owner(user_id: Uuid) -> OwnerClaimsExt {
        OwnerClaimsExt(OwnerClaims::new(user_id, None))
    }

    fn request_id() -> Extension<RequestId> {
        Extension(RequestId::new(HeaderValue::from_static("corr-1")))
    }

    // ── op 1: create_modifier_group ──

    #[tokio::test]
    async fn create_modifier_group_happy_path_201_with_zero_modifier_count() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = test_state(FakeModifierGroupsRepo::default(), user_id, loc);

        let response = create_modifier_group(
            Extension(state),
            owner(user_id),
            Path(loc),
            request_id(),
            Json(CreateModifierGroupRequest {
                name: "Size".to_string(),
                min_select: 1,
                max_select: 1,
                required: true,
                display_type: Some(DisplayType::Radio),
            }),
        )
        .await
        .unwrap()
        .into_response();

        assert_eq!(response.status(), StatusCode::CREATED);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["name"], "Size");
        assert_eq!(body["minSelect"], 1);
        assert_eq!(body["maxSelect"], 1);
        assert_eq!(body["required"], true);
        assert_eq!(body["displayType"], "radio");
        assert_eq!(
            body["modifierCount"], 0,
            "hardcoded 0 on create (modifier-groups.ts:44), never computed"
        );
    }

    #[tokio::test]
    async fn create_modifier_group_cross_location_404() {
        let user_id = Uuid::new_v4();
        let mine = Uuid::new_v4();
        let theirs = Uuid::new_v4();
        let state = test_state(FakeModifierGroupsRepo::default(), user_id, mine);

        let err = crate::error::expect_err(
            create_modifier_group(
                Extension(state),
                owner(user_id),
                Path(theirs),
                request_id(),
                Json(CreateModifierGroupRequest {
                    name: "Size".to_string(),
                    min_select: 0,
                    max_select: 1,
                    required: false,
                    display_type: None,
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn create_modifier_group_in_transaction_membership_recheck_maps_to_404() {
        // S3 breaker C1+H4: even though the out-of-band pre-check passes, a `Gated::NotAMember`
        // from the repo layer (simulating the in-transaction re-check failing) must ALSO 404.
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeModifierGroupsRepo::default();
        *repo.force_not_a_member.lock().unwrap() = true;
        let state = test_state(repo, user_id, loc);

        let err = crate::error::expect_err(
            create_modifier_group(
                Extension(state),
                owner(user_id),
                Path(loc),
                request_id(),
                Json(CreateModifierGroupRequest {
                    name: "Size".to_string(),
                    min_select: 0,
                    max_select: 1,
                    required: false,
                    display_type: None,
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    #[test]
    fn create_modifier_group_request_rejects_unknown_fields() {
        let json = serde_json::json!({ "name": "Size", "bogus": 1 });
        assert!(serde_json::from_value::<CreateModifierGroupRequest>(json).is_err());
    }

    #[test]
    fn create_modifier_group_request_applies_defaults() {
        let json = serde_json::json!({ "name": "Size" });
        let req: CreateModifierGroupRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.min_select, 0);
        assert_eq!(req.max_select, 1);
        assert!(!req.required);
        assert_eq!(req.display_type, None);
    }

    // ── op 2: list_modifier_groups ──

    #[tokio::test]
    async fn list_modifier_groups_returns_real_modifier_count() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeModifierGroupsRepo::default();
        let group_id = Uuid::new_v4();
        repo.groups.lock().unwrap().insert(
            group_id,
            fake::FakeGroup {
                location_id: loc,
                name: "Size".to_string(),
                min_select: 1,
                max_select: 1,
                required: true,
                display_type: None,
            },
        );
        repo.modifiers.lock().unwrap().insert(
            Uuid::new_v4(),
            fake::FakeModifier {
                location_id: loc,
                group_id,
                name: "Small".to_string(),
                price_delta: 0,
                available: true,
                sort_order: 0,
            },
        );
        let state = test_state(repo, user_id, loc);

        let response =
            list_modifier_groups(Extension(state), owner(user_id), Path(loc), request_id())
                .await
                .unwrap()
                .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["data"][0]["modifierCount"], 1);
    }

    #[tokio::test]
    async fn list_modifier_groups_cross_location_404() {
        let user_id = Uuid::new_v4();
        let mine = Uuid::new_v4();
        let theirs = Uuid::new_v4();
        let state = test_state(FakeModifierGroupsRepo::default(), user_id, mine);
        let err = crate::error::expect_err(
            list_modifier_groups(Extension(state), owner(user_id), Path(theirs), request_id())
                .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    // ── op 3: update_modifier_group ──

    #[tokio::test]
    async fn update_modifier_group_happy_path() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeModifierGroupsRepo::default();
        let group_id = Uuid::new_v4();
        repo.groups.lock().unwrap().insert(
            group_id,
            fake::FakeGroup {
                location_id: loc,
                name: "Size".to_string(),
                min_select: 0,
                max_select: 1,
                required: false,
                display_type: None,
            },
        );
        let state = test_state(repo, user_id, loc);

        let response = update_modifier_group(
            Extension(state),
            owner(user_id),
            Path((loc, group_id)),
            request_id(),
            Json(UpdateModifierGroupRequest {
                name: Some("Renamed".to_string()),
                ..Default::default()
            }),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["name"], "Renamed");
        assert_eq!(
            body["modifierCount"], 0,
            "hardcoded 0 on PATCH too (modifier-groups.ts:113)"
        );
    }

    #[tokio::test]
    async fn update_modifier_group_cross_location_404() {
        let user_id = Uuid::new_v4();
        let mine = Uuid::new_v4();
        let theirs = Uuid::new_v4();
        let state = test_state(FakeModifierGroupsRepo::default(), user_id, mine);
        let err = crate::error::expect_err(
            update_modifier_group(
                Extension(state),
                owner(user_id),
                Path((theirs, Uuid::new_v4())),
                request_id(),
                Json(UpdateModifierGroupRequest {
                    name: Some("x".to_string()),
                    ..Default::default()
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn update_modifier_group_empty_body_400_validation_failed() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = test_state(FakeModifierGroupsRepo::default(), user_id, loc);
        let err = crate::error::expect_err(
            update_modifier_group(
                Extension(state),
                owner(user_id),
                Path((loc, Uuid::new_v4())),
                request_id(),
                Json(UpdateModifierGroupRequest::default()),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::ValidationFailed);
        assert_eq!(
            err.into_response().status(),
            StatusCode::BAD_REQUEST,
            "S3 catalog VALIDATION_FAILED is 400, not the S1-default 422"
        );
    }

    #[tokio::test]
    async fn update_modifier_group_not_found_404() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = test_state(FakeModifierGroupsRepo::default(), user_id, loc);
        let err = crate::error::expect_err(
            update_modifier_group(
                Extension(state),
                owner(user_id),
                Path((loc, Uuid::new_v4())),
                request_id(),
                Json(UpdateModifierGroupRequest {
                    name: Some("x".to_string()),
                    ..Default::default()
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn update_modifier_group_null_display_type_counts_as_an_update() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeModifierGroupsRepo::default();
        let group_id = Uuid::new_v4();
        repo.groups.lock().unwrap().insert(
            group_id,
            fake::FakeGroup {
                location_id: loc,
                name: "Size".to_string(),
                min_select: 0,
                max_select: 1,
                required: false,
                display_type: Some(DisplayType::Radio),
            },
        );
        let state = test_state(repo, user_id, loc);

        // A body of exactly {"display_type": null} — NOT empty (Some(None) counts as present),
        // and clears the previously-set value.
        let body: UpdateModifierGroupRequest =
            serde_json::from_value(serde_json::json!({ "display_type": null })).unwrap();
        assert!(!body.is_empty());

        let response = update_modifier_group(
            Extension(state),
            owner(user_id),
            Path((loc, group_id)),
            request_id(),
            Json(body),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["displayType"], serde_json::Value::Null);
    }

    #[test]
    fn update_modifier_group_request_rejects_unknown_fields() {
        let json = serde_json::json!({ "bogus": 1 });
        assert!(serde_json::from_value::<UpdateModifierGroupRequest>(json).is_err());
    }

    #[tokio::test]
    async fn update_modifier_group_in_transaction_membership_recheck_maps_to_404() {
        // Proves the Gated<Option<T>> shape (update/patch ops) ALSO maps NotAMember to 404, not
        // just the Gated<T> shape (create) covered above.
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeModifierGroupsRepo::default();
        *repo.force_not_a_member.lock().unwrap() = true;
        let state = test_state(repo, user_id, loc);
        let err = crate::error::expect_err(
            update_modifier_group(
                Extension(state),
                owner(user_id),
                Path((loc, Uuid::new_v4())),
                request_id(),
                Json(UpdateModifierGroupRequest {
                    name: Some("x".to_string()),
                    ..Default::default()
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    // ── op 4: delete_modifier_group ──

    #[tokio::test]
    async fn delete_modifier_group_happy_path_204() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeModifierGroupsRepo::default();
        let group_id = Uuid::new_v4();
        repo.groups.lock().unwrap().insert(
            group_id,
            fake::FakeGroup {
                location_id: loc,
                name: "Size".to_string(),
                min_select: 0,
                max_select: 1,
                required: false,
                display_type: None,
            },
        );
        let state = test_state(repo, user_id, loc);
        let response = delete_modifier_group(
            Extension(state),
            owner(user_id),
            Path((loc, group_id)),
            request_id(),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn delete_modifier_group_cross_location_404() {
        let user_id = Uuid::new_v4();
        let mine = Uuid::new_v4();
        let theirs = Uuid::new_v4();
        let state = test_state(FakeModifierGroupsRepo::default(), user_id, mine);
        let err = crate::error::expect_err(
            delete_modifier_group(
                Extension(state),
                owner(user_id),
                Path((theirs, Uuid::new_v4())),
                request_id(),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn delete_modifier_group_not_found_404() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = test_state(FakeModifierGroupsRepo::default(), user_id, loc);
        let err = crate::error::expect_err(
            delete_modifier_group(
                Extension(state),
                owner(user_id),
                Path((loc, Uuid::new_v4())),
                request_id(),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    // ── op 5: create_modifier ──

    #[tokio::test]
    async fn create_modifier_happy_path_201() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeModifierGroupsRepo::default();
        let group_id = Uuid::new_v4();
        repo.groups.lock().unwrap().insert(
            group_id,
            fake::FakeGroup {
                location_id: loc,
                name: "Size".to_string(),
                min_select: 0,
                max_select: 1,
                required: false,
                display_type: None,
            },
        );
        let state = test_state(repo, user_id, loc);

        let response = create_modifier(
            Extension(state),
            owner(user_id),
            Path((loc, group_id)),
            request_id(),
            Json(CreateModifierRequest {
                name: "Small".to_string(),
                price_delta: 0,
                available: true,
                sort_order: 0,
            }),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::CREATED);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["groupId"], group_id.to_string());
    }

    #[tokio::test]
    async fn create_modifier_cross_location_404() {
        let user_id = Uuid::new_v4();
        let mine = Uuid::new_v4();
        let theirs = Uuid::new_v4();
        let state = test_state(FakeModifierGroupsRepo::default(), user_id, mine);
        let err = crate::error::expect_err(
            create_modifier(
                Extension(state),
                owner(user_id),
                Path((theirs, Uuid::new_v4())),
                request_id(),
                Json(CreateModifierRequest {
                    name: "Small".to_string(),
                    price_delta: 0,
                    available: true,
                    sort_order: 0,
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn create_modifier_unknown_group_404_via_fold_in_insert() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = test_state(FakeModifierGroupsRepo::default(), user_id, loc);
        let err = crate::error::expect_err(
            create_modifier(
                Extension(state),
                owner(user_id),
                Path((loc, Uuid::new_v4())),
                request_id(),
                Json(CreateModifierRequest {
                    name: "Small".to_string(),
                    price_delta: 0,
                    available: true,
                    sort_order: 0,
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
        assert_eq!(err.envelope.message, "Modifier group not found");
    }

    #[test]
    fn create_modifier_request_rejects_unknown_fields() {
        let json = serde_json::json!({ "name": "Small", "bogus": 1 });
        assert!(serde_json::from_value::<CreateModifierRequest>(json).is_err());
    }

    // ── op 6: update_modifier ──

    #[tokio::test]
    async fn update_modifier_happy_path() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeModifierGroupsRepo::default();
        let group_id = Uuid::new_v4();
        let modifier_id = Uuid::new_v4();
        repo.modifiers.lock().unwrap().insert(
            modifier_id,
            fake::FakeModifier {
                location_id: loc,
                group_id,
                name: "Small".to_string(),
                price_delta: 0,
                available: true,
                sort_order: 0,
            },
        );
        let state = test_state(repo, user_id, loc);

        let response = update_modifier(
            Extension(state),
            owner(user_id),
            Path((loc, modifier_id)),
            request_id(),
            Json(UpdateModifierRequest {
                price_delta: Some(150),
                ..Default::default()
            }),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["priceDelta"], 150);
    }

    #[tokio::test]
    async fn update_modifier_cross_location_404() {
        let user_id = Uuid::new_v4();
        let mine = Uuid::new_v4();
        let theirs = Uuid::new_v4();
        let state = test_state(FakeModifierGroupsRepo::default(), user_id, mine);
        let err = crate::error::expect_err(
            update_modifier(
                Extension(state),
                owner(user_id),
                Path((theirs, Uuid::new_v4())),
                request_id(),
                Json(UpdateModifierRequest {
                    price_delta: Some(1),
                    ..Default::default()
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn update_modifier_empty_body_400_validation_failed() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = test_state(FakeModifierGroupsRepo::default(), user_id, loc);
        let err = crate::error::expect_err(
            update_modifier(
                Extension(state),
                owner(user_id),
                Path((loc, Uuid::new_v4())),
                request_id(),
                Json(UpdateModifierRequest::default()),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::ValidationFailed);
        assert_eq!(err.into_response().status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn update_modifier_not_found_404() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = test_state(FakeModifierGroupsRepo::default(), user_id, loc);
        let err = crate::error::expect_err(
            update_modifier(
                Extension(state),
                owner(user_id),
                Path((loc, Uuid::new_v4())),
                request_id(),
                Json(UpdateModifierRequest {
                    price_delta: Some(1),
                    ..Default::default()
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    #[test]
    fn update_modifier_request_rejects_unknown_fields() {
        let json = serde_json::json!({ "bogus": 1 });
        assert!(serde_json::from_value::<UpdateModifierRequest>(json).is_err());
    }

    // ── op 7: delete_modifier ──

    #[tokio::test]
    async fn delete_modifier_happy_path_204() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeModifierGroupsRepo::default();
        let modifier_id = Uuid::new_v4();
        repo.modifiers.lock().unwrap().insert(
            modifier_id,
            fake::FakeModifier {
                location_id: loc,
                group_id: Uuid::new_v4(),
                name: "Small".to_string(),
                price_delta: 0,
                available: true,
                sort_order: 0,
            },
        );
        let state = test_state(repo, user_id, loc);
        let response = delete_modifier(
            Extension(state),
            owner(user_id),
            Path((loc, modifier_id)),
            request_id(),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn delete_modifier_cross_location_404() {
        let user_id = Uuid::new_v4();
        let mine = Uuid::new_v4();
        let theirs = Uuid::new_v4();
        let state = test_state(FakeModifierGroupsRepo::default(), user_id, mine);
        let err = crate::error::expect_err(
            delete_modifier(
                Extension(state),
                owner(user_id),
                Path((theirs, Uuid::new_v4())),
                request_id(),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn delete_modifier_not_found_404() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = test_state(FakeModifierGroupsRepo::default(), user_id, loc);
        let err = crate::error::expect_err(
            delete_modifier(
                Extension(state),
                owner(user_id),
                Path((loc, Uuid::new_v4())),
                request_id(),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }
}
