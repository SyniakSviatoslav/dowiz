//! S3 catalog/admin CRUD — menu-availability vertical. Ports `apps/api/src/routes/owner/
//! menu-availability.ts` (owner-route census rows #80-83) VERBATIM: the venue busy-window toggle
//! plus the `menu_schedules` CRUD (mealtime/availability windows per product or category).
//!
//! ## The 4 ops
//!   1. `PATCH  /api/owner/locations/{locationId}/kitchen-busy`      (menu-availability.ts:22)
//!   2. `GET    /api/owner/locations/{locationId}/menu-schedules`    (menu-availability.ts:76)
//!   3. `POST   /api/owner/locations/{locationId}/menu-schedules`    (menu-availability.ts:95)
//!   4. `DELETE /api/owner/locations/{locationId}/menu-schedules/{id}` (menu-availability.ts:142)
//!
//! Every op is OWNER+LOC: [`super::require_location_access`] gates the out-of-band fast path, then
//! every write ALSO re-checks membership IN-TRANSACTION via [`super::assert_active_owner_membership`]
//! (S3 breaker finding C1+H4 — `AuthState.repo` and the `with_user`-seated connection are different
//! pools/sessions; the in-transaction check is the actual security boundary once RLS is enforced,
//! the extractor-level check is a cheap fast-path only). A `false` membership result is a 404
//! (`ErrorCode::NotFound`, existence-hiding, never 403) — see each `PgMenuAvailabilityRepo` method
//! below, and the `*Outcome` enums that keep "membership denied" distinct from the op's own
//! not-found/business-logic outcomes.
//!
//! ## R2-1 (15th IDOR) — carried verbatim, not weakened
//! Op 3's INSERT folds the FK-ownership check INTO the statement (menu-availability.ts:113-136):
//! the DEFINER-bypassing FK-existence check only validates a product/category EXISTS, not that it
//! belongs to THIS location, so a body `product_id`/`category_id` from a different tenant would
//! otherwise insert a live schedule that hides/rewrites that tenant's storefront availability.
//! `EXISTS (... AND location_id = $1)` on both branches (mirrors `products.rs` #5/#9/#11's
//! ownership-fold-in pattern) makes a cross-tenant reference insert 0 rows → 404, not a silent
//! success. This is the R2-1 fix from the council packet; it is IN SCOPE for this build (confirmed
//! disposition "PORT") and must never be relaxed to a plain existence check.
//!
//! ## Writes: `db::with_user`, never `db::with_tenant`
//! See `crate::db` module doc (Q-GUC-FAMILY) — every write here seats `app.user_id` via
//! `crate::db::with_user`, keyed on the OWNER's `user_id` (never `with_tenant`'s
//! `app.current_tenant`, reserved for the courier/service GUC family). Every query also carries an
//! explicit `WHERE location_id = $n` predicate (or the ownership-fold-in `INSERT ... SELECT ...
//! WHERE location_id = $n` above) independent of RLS enforcement.
//!
//! ## `dead_code` allowance (dark-surface scope, same posture as `crate::auth`)
use axum::extract::{Extension, Path};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use tower_http::request_id::RequestId;
use utoipa::ToSchema;
use uuid::Uuid;

use domain::ErrorCode;

use crate::error::ApiError;
use crate::repo::RepoError;
use crate::routes::correlation_id_string;

// ─────────────────────────────── DTOs (wire shapes) ───────────────────────────────

/// `{ busy_until: z.string().datetime().nullable() }.strict()` (menu-availability.ts:29): the KEY
/// is REQUIRED (`.strict()` without `.optional()` on the field), its VALUE may be `null` to clear
/// the busy window. A bare `Option<T>` field can NOT express this: serde's derive treats ANY
/// `Option<T>`-typed struct field as implicitly optional (silently `None` if the key is absent) —
/// there is no per-field way to opt out of that once the field's type is syntactically `Option<..>`
/// (confirmed empirically here, not assumed — an earlier version of this file used a bare
/// `Option<DateTime<Utc>>` and `set_kitchen_busy_request_rejects_a_missing_busy_until_key` caught
/// it deserializing a missing key to `None` instead of erroring). The standard workaround: an
/// OUTER `Option` combined with `#[serde(deserialize_with = "deserialize_some")]` — adding
/// `deserialize_with` suppresses serde's implicit default-on-missing-key behavior (serde can no
/// longer assume what an absent key should produce once a custom deserializer is in play), so the
/// KEY becomes required again; the INNER `Option<DateTime<Utc>>` is what the value itself
/// deserializes to (JSON `null` -> `None`, a datetime string -> `Some`).
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SetKitchenBusyRequest {
    // `utoipa`'s `chrono` feature isn't enabled on this crate's `Cargo.toml` (out of scope for
    // this file to change), so `DateTime<Utc>` doesn't implement `ToSchema` — `value_type`
    // overrides ONLY the generated OpenAPI schema (an ISO datetime string), the actual Rust field
    // type/serde behavior below is untouched.
    #[serde(deserialize_with = "deserialize_some")]
    #[schema(value_type = Option<String>)]
    pub busy_until: Option<Option<chrono::DateTime<chrono::Utc>>>,
}

/// See `SetKitchenBusyRequest.busy_until`'s doc for why this exists: the OUTER `Option` still gets
/// serde's implicit "missing key -> None" treatment (letting us tell "key absent" apart from "key
/// present"); `deserialize_with` on the field is what makes the key required in the first place.
fn deserialize_some<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(Some)
}

/// `{ id, kitchenBusyUntil }` (menu-availability.ts:44) — built explicitly rather than a
/// passthrough of the DB row, so the wire shape stays camelCase independent of column names.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct KitchenBusyResponse {
    pub id: Uuid,
    #[serde(rename = "kitchenBusyUntil")]
    #[schema(value_type = Option<String>)]
    pub kitchen_busy_until: Option<chrono::DateTime<chrono::Utc>>,
}

/// `mode: z.enum(['daily', 'recurring', 'period']).default('daily')` (menu-availability.ts:52).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ScheduleMode {
    #[default]
    Daily,
    Recurring,
    Period,
}

impl ScheduleMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            ScheduleMode::Daily => "daily",
            ScheduleMode::Recurring => "recurring",
            ScheduleMode::Period => "period",
        }
    }
}

/// `ScheduleBody` (menu-availability.ts:49-59, `.strict()`). Every "nullable().optional()" TS field
/// is modeled as a plain `Option<T>` with an explicit `#[serde(default)]` — unlike
/// `SetKitchenBusyRequest.busy_until` above, these fields are genuinely optional (absent and
/// explicit-`null` are both "not provided" for the exactly-one-of check below, matching JS
/// truthiness — menu-availability.ts:107-108), so defaulting a missing key to `None` is correct
/// here, not a divergence from the TS schema.
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateScheduleRequest {
    #[serde(default)]
    pub product_id: Option<Uuid>,
    #[serde(default)]
    pub category_id: Option<Uuid>,
    #[serde(default)]
    pub mode: ScheduleMode,
    #[serde(default)]
    pub start_minute: Option<i32>,
    #[serde(default)]
    pub end_minute: Option<i32>,
    #[serde(default)]
    pub days_of_week: Option<Vec<i32>>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub starts_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub ends_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default = "default_available")]
    pub available: bool,
}

fn default_available() -> bool {
    true
}

/// `rowToShape` (menu-availability.ts:61-74) — the camelCase response shape shared by op 2 (list)
/// and op 3 (create).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScheduleShape {
    pub id: Uuid,
    #[serde(rename = "productId")]
    pub product_id: Option<Uuid>,
    #[serde(rename = "categoryId")]
    pub category_id: Option<Uuid>,
    pub mode: String,
    #[serde(rename = "startMinute")]
    pub start_minute: Option<i32>,
    #[serde(rename = "endMinute")]
    pub end_minute: Option<i32>,
    #[serde(rename = "daysOfWeek")]
    pub days_of_week: Option<Vec<i32>>,
    #[serde(rename = "startsAt")]
    #[schema(value_type = Option<String>)]
    pub starts_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "endsAt")]
    #[schema(value_type = Option<String>)]
    pub ends_at: Option<chrono::DateTime<chrono::Utc>>,
    pub available: bool,
}

impl From<ScheduleRow> for ScheduleShape {
    fn from(row: ScheduleRow) -> Self {
        ScheduleShape {
            id: row.id,
            product_id: row.product_id,
            category_id: row.category_id,
            mode: row.mode,
            start_minute: row.start_minute,
            end_minute: row.end_minute,
            days_of_week: row.days_of_week,
            starts_at: row.starts_at,
            ends_at: row.ends_at,
            available: row.available,
        }
    }
}

/// `{ data: [...] }` (menu-availability.ts:91) — op 2's list envelope.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScheduleListResponse {
    pub data: Vec<ScheduleShape>,
}

// ─────────────────────────────── Repo-layer row/input types ───────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct KitchenBusyRow {
    pub id: Uuid,
    pub kitchen_busy_until: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScheduleRow {
    pub id: Uuid,
    pub product_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub mode: String,
    pub start_minute: Option<i32>,
    pub end_minute: Option<i32>,
    pub days_of_week: Option<Vec<i32>>,
    pub starts_at: Option<chrono::DateTime<chrono::Utc>>,
    pub ends_at: Option<chrono::DateTime<chrono::Utc>>,
    pub available: bool,
}

/// The 10-column tuple shape `menu_schedules` queries decode into — named so clippy's
/// `type_complexity` lint (workspace `deny`) doesn't fire on the repeated inline tuple (mirrors
/// `crate::repo::fake::ThemeLocationEntry`'s reason for existing).
#[allow(clippy::type_complexity)]
type ScheduleTuple = (
    Uuid,
    Option<Uuid>,
    Option<Uuid>,
    String,
    Option<i32>,
    Option<i32>,
    Option<Vec<i32>>,
    Option<chrono::DateTime<chrono::Utc>>,
    Option<chrono::DateTime<chrono::Utc>>,
    bool,
);

impl From<ScheduleTuple> for ScheduleRow {
    fn from(t: ScheduleTuple) -> Self {
        let (
            id,
            product_id,
            category_id,
            mode,
            start_minute,
            end_minute,
            days_of_week,
            starts_at,
            ends_at,
            available,
        ) = t;
        ScheduleRow {
            id,
            product_id,
            category_id,
            mode,
            start_minute,
            end_minute,
            days_of_week,
            starts_at,
            ends_at,
            available,
        }
    }
}

/// Op 3's create input, bundled into one struct (rather than 8 loose args) so `create_schedule`
/// doesn't trip clippy's `too_many_arguments` (workspace `deny`).
#[derive(Debug, Clone)]
pub struct NewSchedule {
    pub product_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub mode: ScheduleMode,
    pub start_minute: Option<i32>,
    pub end_minute: Option<i32>,
    pub days_of_week: Option<Vec<i32>>,
    pub starts_at: Option<chrono::DateTime<chrono::Utc>>,
    pub ends_at: Option<chrono::DateTime<chrono::Utc>>,
    pub available: bool,
}

// ─────────────────────────────── Outcome enums ───────────────────────────────
//
// Every op's `with_user` closure now gates on `super::assert_active_owner_membership` FIRST (S3
// breaker finding C1+H4), before its own SQL. These enums keep "the in-transaction membership
// re-check failed" structurally distinct from "the check passed but the op's own business logic
// says not-found/invalid" — for op 3 in particular these are THREE separate failure modes
// (membership / R2-1 FK-ownership fold-in / the handler's own exactly-one-of 400), and conflating
// membership-denied with the FK-ownership miss would blur two different security properties even
// though (today) both happen to render as an HTTP 404 to the caller.

#[derive(Debug, Clone, PartialEq)]
pub enum KitchenBusyOutcome {
    MembershipDenied,
    LocationNotFound,
    Ok(KitchenBusyRow),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScheduleListOutcome {
    MembershipDenied,
    Ok(Vec<ScheduleRow>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum CreateScheduleOutcome {
    MembershipDenied,
    /// R2-1: the referenced `product_id`/`category_id` doesn't belong to this `location_id`.
    FkOwnershipMiss,
    Created(ScheduleRow),
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeleteScheduleOutcome {
    MembershipDenied,
    ScheduleNotFound,
    Deleted,
}

// ─────────────────────────────── Repo trait + state ───────────────────────────────

#[async_trait::async_trait]
pub trait MenuAvailabilityRepo: Send + Sync {
    /// `UPDATE locations SET kitchen_busy_until = $2 WHERE id = $1 RETURNING id,
    /// kitchen_busy_until` (menu-availability.ts:38-42). No extra tenant predicate beyond `id`:
    /// unlike `menu_schedules`, `locations` is keyed by the location's OWN id, already scoped by
    /// the membership checks above — carried verbatim, not "hardened" with an invented predicate
    /// that doesn't exist in the TS.
    async fn set_kitchen_busy(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        busy_until: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<KitchenBusyOutcome, RepoError>;

    /// `SELECT ... FROM menu_schedules WHERE location_id = $1 ORDER BY created_at ASC`
    /// (menu-availability.ts:86-89).
    async fn list_schedules(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
    ) -> Result<ScheduleListOutcome, RepoError>;

    /// The R2-1-hardened `INSERT ... SELECT ... WHERE EXISTS (...)` (menu-availability.ts:118-130).
    async fn create_schedule(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        new: NewSchedule,
    ) -> Result<CreateScheduleOutcome, RepoError>;

    /// `DELETE FROM menu_schedules WHERE location_id = $1 AND id = $2 RETURNING id`
    /// (menu-availability.ts:152).
    async fn delete_schedule(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        id: Uuid,
    ) -> Result<DeleteScheduleOutcome, RepoError>;
}

#[derive(Clone)]
pub struct MenuAvailabilityState {
    pub auth: crate::auth::AuthState,
    pub repo: std::sync::Arc<dyn MenuAvailabilityRepo>,
}

/// Maps `crate::db::TenantTxnError` (the `with_user` failure type) onto `RepoError` — a plain
/// function, deliberately NOT a `From`/`Into` trait impl: `RepoError`/`TenantTxnError` are both
/// foreign to this module but local to the `api` crate, so a `impl From<TenantTxnError> for
/// RepoError` here would be orphan-rule-legal yet collide crate-wide the moment a SIBLING S3
/// submodule (built concurrently in the same tree) added the identical blanket impl — two `impl
/// From<X> for Y` for the same `(X, Y)` pair anywhere in one crate is a hard compile error
/// (E0119), independent of which module wrote which one first. `RepoError`'s inner `sqlx::Error`
/// field is `pub`, so constructing `RepoError(sqlx_err)` directly needs no impl at all.
fn map_txn_err(err: crate::db::TenantTxnError) -> RepoError {
    use crate::db::TenantTxnError as E;
    let sqlx_err = match err {
        E::Begin(e) | E::SetTenant(e) | E::Work(e) | E::Commit(e) => e,
        E::WorkThenRollbackFailed { work, .. } => work,
    };
    RepoError(sqlx_err)
}

/// The real `sqlx`-backed implementation. Every method's FIRST in-transaction statement is
/// `super::assert_active_owner_membership` (S3 breaker C1+H4) — see module doc.
pub struct PgMenuAvailabilityRepo {
    pool: sqlx::PgPool,
}

impl PgMenuAvailabilityRepo {
    pub fn new(pool: sqlx::PgPool) -> Self {
        PgMenuAvailabilityRepo { pool }
    }
}

#[async_trait::async_trait]
impl MenuAvailabilityRepo for PgMenuAvailabilityRepo {
    async fn set_kitchen_busy(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        busy_until: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<KitchenBusyOutcome, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !super::assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(KitchenBusyOutcome::MembershipDenied);
                }
                let row: Option<(Uuid, Option<chrono::DateTime<chrono::Utc>>)> = sqlx::query_as(
                    "UPDATE locations SET kitchen_busy_until = $2 WHERE id = $1 RETURNING id, kitchen_busy_until",
                )
                .bind(location_id)
                .bind(busy_until)
                .fetch_optional(&mut **txn)
                .await?;
                Ok(match row {
                    Some((id, kitchen_busy_until)) => {
                        KitchenBusyOutcome::Ok(KitchenBusyRow { id, kitchen_busy_until })
                    }
                    None => KitchenBusyOutcome::LocationNotFound,
                })
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn list_schedules(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
    ) -> Result<ScheduleListOutcome, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !super::assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(ScheduleListOutcome::MembershipDenied);
                }
                let rows: Vec<ScheduleTuple> = sqlx::query_as(
                    "SELECT id, product_id, category_id, mode, start_minute, end_minute, days_of_week, starts_at, ends_at, available
                       FROM menu_schedules WHERE location_id = $1 ORDER BY created_at ASC",
                )
                .bind(location_id)
                .fetch_all(&mut **txn)
                .await?;
                Ok(ScheduleListOutcome::Ok(
                    rows.into_iter().map(ScheduleRow::from).collect(),
                ))
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn create_schedule(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        new: NewSchedule,
    ) -> Result<CreateScheduleOutcome, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !super::assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(CreateScheduleOutcome::MembershipDenied);
                }
                let row: Option<ScheduleTuple> = sqlx::query_as(
                    "INSERT INTO menu_schedules
                       (location_id, product_id, category_id, mode, start_minute, end_minute, days_of_week, starts_at, ends_at, available)
                     SELECT $1,$2,$3,$4,$5,$6,$7,$8,$9,$10
                     WHERE ($2::uuid IS NULL OR EXISTS (SELECT 1 FROM products p WHERE p.id = $2 AND p.location_id = $1))
                       AND ($3::uuid IS NULL OR EXISTS (SELECT 1 FROM categories c WHERE c.id = $3 AND c.location_id = $1))
                     RETURNING id, product_id, category_id, mode, start_minute, end_minute, days_of_week, starts_at, ends_at, available",
                )
                .bind(location_id)
                .bind(new.product_id)
                .bind(new.category_id)
                .bind(new.mode.as_str())
                .bind(new.start_minute)
                .bind(new.end_minute)
                .bind(new.days_of_week)
                .bind(new.starts_at)
                .bind(new.ends_at)
                .bind(new.available)
                .fetch_optional(&mut **txn)
                .await?;
                Ok(match row {
                    Some(row) => CreateScheduleOutcome::Created(ScheduleRow::from(row)),
                    None => CreateScheduleOutcome::FkOwnershipMiss,
                })
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn delete_schedule(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        id: Uuid,
    ) -> Result<DeleteScheduleOutcome, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                if !super::assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(DeleteScheduleOutcome::MembershipDenied);
                }
                let row: Option<(Uuid,)> = sqlx::query_as(
                    "DELETE FROM menu_schedules WHERE location_id = $1 AND id = $2 RETURNING id",
                )
                .bind(location_id)
                .bind(id)
                .fetch_optional(&mut **txn)
                .await?;
                Ok(if row.is_some() {
                    DeleteScheduleOutcome::Deleted
                } else {
                    DeleteScheduleOutcome::ScheduleNotFound
                })
            })
        })
        .await
        .map_err(map_txn_err)
    }
}

// ─────────────────────────────── Handlers ───────────────────────────────

/// `PATCH /api/owner/locations/{locationId}/kitchen-busy` — source: `menu-availability.ts:22-46`.
#[utoipa::path(
    patch,
    path = "/api/owner/locations/{locationId}/kitchen-busy",
    params(("locationId" = Uuid, Path)),
    request_body = SetKitchenBusyRequest,
    responses(
        (status = 200, description = "Busy window set/cleared", body = KitchenBusyResponse),
        (status = 404, description = "Location not found or not owned by caller", body = domain::ErrorEnvelope),
    ),
    tag = "owner-catalog"
)]
pub async fn set_kitchen_busy(
    Extension(state): Extension<MenuAvailabilityState>,
    crate::auth::extractors::OwnerClaimsExt(owner): crate::auth::extractors::OwnerClaimsExt,
    Path(location_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
    axum::Json(body): axum::Json<SetKitchenBusyRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    super::require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    // menu-availability.ts:29: `busy_until` is required-but-nullable — the outer `Option` is
    // `None` only when the KEY itself was absent from the body (see `SetKitchenBusyRequest`'s doc).
    let Some(busy_until) = body.busy_until else {
        return Err(ApiError::validation_failed_400(
            "busy_until is required",
            correlation_id,
        ));
    };

    let outcome = state
        .repo
        .set_kitchen_busy(owner.user_id, location_id, busy_until)
        .await
        .map_err(|_err| {
            ApiError::new(
                ErrorCode::Internal,
                "internal_error",
                correlation_id.clone(),
            )
        })?;

    let row = match outcome {
        KitchenBusyOutcome::Ok(row) => row,
        KitchenBusyOutcome::MembershipDenied | KitchenBusyOutcome::LocationNotFound => {
            return Err(ApiError::new(
                ErrorCode::NotFound,
                "Not found",
                correlation_id,
            ));
        }
    };

    Ok(axum::Json(KitchenBusyResponse {
        id: row.id,
        kitchen_busy_until: row.kitchen_busy_until,
    }))
}

/// `GET /api/owner/locations/{locationId}/menu-schedules` — source: `menu-availability.ts:76-93`.
#[utoipa::path(
    get,
    path = "/api/owner/locations/{locationId}/menu-schedules",
    params(("locationId" = Uuid, Path)),
    responses(
        (status = 200, description = "Schedules for this location", body = ScheduleListResponse),
        (status = 404, description = "Location not found or not owned by caller", body = domain::ErrorEnvelope),
    ),
    tag = "owner-catalog"
)]
pub async fn list_schedules(
    Extension(state): Extension<MenuAvailabilityState>,
    crate::auth::extractors::OwnerClaimsExt(owner): crate::auth::extractors::OwnerClaimsExt,
    Path(location_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    super::require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    let outcome = state
        .repo
        .list_schedules(owner.user_id, location_id)
        .await
        .map_err(|_err| {
            ApiError::new(
                ErrorCode::Internal,
                "internal_error",
                correlation_id.clone(),
            )
        })?;

    let rows = match outcome {
        ScheduleListOutcome::Ok(rows) => rows,
        ScheduleListOutcome::MembershipDenied => {
            return Err(ApiError::new(
                ErrorCode::NotFound,
                "Not found",
                correlation_id,
            ));
        }
    };

    Ok(axum::Json(ScheduleListResponse {
        data: rows.into_iter().map(ScheduleShape::from).collect(),
    }))
}

/// `POST /api/owner/locations/{locationId}/menu-schedules` — source: `menu-availability.ts:95-140`.
#[utoipa::path(
    post,
    path = "/api/owner/locations/{locationId}/menu-schedules",
    params(("locationId" = Uuid, Path)),
    request_body = CreateScheduleRequest,
    responses(
        (status = 201, description = "Schedule created", body = ScheduleShape),
        (status = 400, description = "Neither or both of product_id/category_id provided", body = domain::ErrorEnvelope),
        (status = 404, description = "Location not owned by caller, or product/category not in this tenant (R2-1)", body = domain::ErrorEnvelope),
    ),
    tag = "owner-catalog"
)]
pub async fn create_schedule(
    Extension(state): Extension<MenuAvailabilityState>,
    crate::auth::extractors::OwnerClaimsExt(owner): crate::auth::extractors::OwnerClaimsExt,
    Path(location_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
    axum::Json(body): axum::Json<CreateScheduleRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    super::require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    // Exactly one target — mirrors the DB CHECK so we 400 instead of a constraint violation
    // (menu-availability.ts:106-111). NEITHER present is ALSO a 400, not just BOTH present: JS
    // truthiness treats `null`/absent identically, so `hasProduct === hasCategory` catches both
    // "zero" and "two" targets with one comparison — ported as `has_product == has_category`.
    let has_product = body.product_id.is_some();
    let has_category = body.category_id.is_some();
    if has_product == has_category {
        return Err(ApiError::validation_failed_400(
            "Provide exactly one of product_id or category_id",
            correlation_id,
        ));
    }

    let new = NewSchedule {
        product_id: body.product_id,
        category_id: body.category_id,
        mode: body.mode,
        start_minute: body.start_minute,
        end_minute: body.end_minute,
        days_of_week: body.days_of_week,
        starts_at: body.starts_at,
        ends_at: body.ends_at,
        available: body.available,
    };

    let outcome = state
        .repo
        .create_schedule(owner.user_id, location_id, new)
        .await
        .map_err(|_err| {
            ApiError::new(
                ErrorCode::Internal,
                "internal_error",
                correlation_id.clone(),
            )
        })?;

    let row = match outcome {
        CreateScheduleOutcome::Created(row) => row,
        CreateScheduleOutcome::MembershipDenied => {
            return Err(ApiError::new(
                ErrorCode::NotFound,
                "Not found",
                correlation_id,
            ));
        }
        CreateScheduleOutcome::FkOwnershipMiss => {
            // menu-availability.ts:132-135: structured warn — a cross-tenant attempt or a
            // nonexistent product/category, kept operationally visible (not required for
            // test-passing, a good-faith port of the security-relevant log line).
            tracing::warn!(
                %location_id,
                product_id = ?body.product_id,
                category_id = ?body.category_id,
                user_id = %owner.user_id,
                "POST menu-schedules FK-ownership miss — product/category not in this tenant (cross-tenant attempt or nonexistent)"
            );
            return Err(ApiError::new(
                ErrorCode::NotFound,
                "Product or category not found",
                correlation_id,
            ));
        }
    };

    Ok((StatusCode::CREATED, axum::Json(ScheduleShape::from(row))))
}

/// `DELETE /api/owner/locations/{locationId}/menu-schedules/{id}` — source: `menu-availability.ts:142-157`.
#[utoipa::path(
    delete,
    path = "/api/owner/locations/{locationId}/menu-schedules/{id}",
    params(("locationId" = Uuid, Path), ("id" = Uuid, Path)),
    responses(
        (status = 204, description = "Schedule deleted"),
        (status = 404, description = "Location not owned by caller, or schedule not found", body = domain::ErrorEnvelope),
    ),
    tag = "owner-catalog"
)]
pub async fn delete_schedule(
    Extension(state): Extension<MenuAvailabilityState>,
    crate::auth::extractors::OwnerClaimsExt(owner): crate::auth::extractors::OwnerClaimsExt,
    Path((location_id, id)): Path<(Uuid, Uuid)>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    super::require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    let outcome = state
        .repo
        .delete_schedule(owner.user_id, location_id, id)
        .await
        .map_err(|_err| {
            ApiError::new(
                ErrorCode::Internal,
                "internal_error",
                correlation_id.clone(),
            )
        })?;

    match outcome {
        DeleteScheduleOutcome::Deleted => Ok(StatusCode::NO_CONTENT),
        DeleteScheduleOutcome::MembershipDenied | DeleteScheduleOutcome::ScheduleNotFound => Err(
            ApiError::new(ErrorCode::NotFound, "Not found", correlation_id),
        ),
    }
}

// ─────────────────────────────── Fake repo (cfg(test)) ───────────────────────────────

#[cfg(test)]
pub mod fake {
    //! `FakeMenuAvailabilityRepo` — the `cfg(test)` stub so handler tests never need a live
    //! Postgres. Models JUST enough state to exercise the R2-1 IDOR fold-in (a `products`/
    //! `categories` map from id -> owning location) and the schedules themselves (a
    //! `(location_id, ScheduleRow)` list — insertion order stands in for `ORDER BY created_at`).
    //!
    //! This fake has no independent membership concept, so it never produces `MembershipDenied` —
    //! per the S3 breaker note, a fake repo can't exercise a real-Postgres RLS/in-transaction gap;
    //! that check is structurally pinned in `PgMenuAvailabilityRepo` above (first statement in
    //! every `with_user` closure), and the extractor-level `require_location_access` cross-location
    //! 404 is what these tests exercise instead (same observable behavior at the handler level).
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    pub struct FakeMenuAvailabilityRepo {
        /// `location_id -> kitchen_busy_until`. Presence of the key = "the location row exists".
        pub locations:
            Mutex<std::collections::HashMap<Uuid, Option<chrono::DateTime<chrono::Utc>>>>,
        /// `product_id -> its owning location_id`.
        pub products: Mutex<std::collections::HashMap<Uuid, Uuid>>,
        /// `category_id -> its owning location_id`.
        pub categories: Mutex<std::collections::HashMap<Uuid, Uuid>>,
        /// `(location_id, row)` in insertion order.
        pub schedules: Mutex<Vec<(Uuid, ScheduleRow)>>,
    }

    #[async_trait::async_trait]
    impl MenuAvailabilityRepo for FakeMenuAvailabilityRepo {
        async fn set_kitchen_busy(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            busy_until: Option<chrono::DateTime<chrono::Utc>>,
        ) -> Result<KitchenBusyOutcome, RepoError> {
            let mut locations = self.locations.lock().unwrap();
            match locations.get_mut(&location_id) {
                Some(existing) => {
                    *existing = busy_until;
                    Ok(KitchenBusyOutcome::Ok(KitchenBusyRow {
                        id: location_id,
                        kitchen_busy_until: busy_until,
                    }))
                }
                None => Ok(KitchenBusyOutcome::LocationNotFound),
            }
        }

        async fn list_schedules(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
        ) -> Result<ScheduleListOutcome, RepoError> {
            let rows = self
                .schedules
                .lock()
                .unwrap()
                .iter()
                .filter(|(loc, _)| *loc == location_id)
                .map(|(_, row)| row.clone())
                .collect();
            Ok(ScheduleListOutcome::Ok(rows))
        }

        async fn create_schedule(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            new: NewSchedule,
        ) -> Result<CreateScheduleOutcome, RepoError> {
            if let Some(product_id) = new.product_id {
                let owns =
                    self.products.lock().unwrap().get(&product_id).copied() == Some(location_id);
                if !owns {
                    return Ok(CreateScheduleOutcome::FkOwnershipMiss);
                }
            }
            if let Some(category_id) = new.category_id {
                let owns =
                    self.categories.lock().unwrap().get(&category_id).copied() == Some(location_id);
                if !owns {
                    return Ok(CreateScheduleOutcome::FkOwnershipMiss);
                }
            }

            let row = ScheduleRow {
                id: Uuid::new_v4(),
                product_id: new.product_id,
                category_id: new.category_id,
                mode: new.mode.as_str().to_string(),
                start_minute: new.start_minute,
                end_minute: new.end_minute,
                days_of_week: new.days_of_week,
                starts_at: new.starts_at,
                ends_at: new.ends_at,
                available: new.available,
            };
            self.schedules
                .lock()
                .unwrap()
                .push((location_id, row.clone()));
            Ok(CreateScheduleOutcome::Created(row))
        }

        async fn delete_schedule(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            id: Uuid,
        ) -> Result<DeleteScheduleOutcome, RepoError> {
            let mut schedules = self.schedules.lock().unwrap();
            match schedules
                .iter()
                .position(|(loc, row)| *loc == location_id && row.id == id)
            {
                Some(pos) => {
                    schedules.remove(pos);
                    Ok(DeleteScheduleOutcome::Deleted)
                }
                None => Ok(DeleteScheduleOutcome::ScheduleNotFound),
            }
        }
    }
}

// ─────────────────────────────── Tests ───────────────────────────────

#[cfg(test)]
mod tests {
    use super::fake::FakeMenuAvailabilityRepo;
    use super::*;
    use crate::auth::AuthState;
    use crate::auth::claims::OwnerClaims;
    use crate::auth::repo::fake::FakeAuthRepo;
    use std::sync::{Arc, Mutex};

    fn test_state(
        repo: FakeMenuAvailabilityRepo,
        user_id: Uuid,
        active_locations: Vec<Uuid>,
    ) -> MenuAvailabilityState {
        let auth_repo = Arc::new(FakeAuthRepo {
            active_owner_locations: Mutex::new([(user_id, active_locations)].into_iter().collect()),
            ..Default::default()
        });
        MenuAvailabilityState {
            auth: AuthState::test_state(auth_repo),
            repo: Arc::new(repo),
        }
    }

    fn owner(user_id: Uuid) -> crate::auth::extractors::OwnerClaimsExt {
        crate::auth::extractors::OwnerClaimsExt(OwnerClaims::new(user_id, None))
    }

    fn request_id() -> Extension<RequestId> {
        Extension(RequestId::new(axum::http::HeaderValue::from_static(
            "corr-1",
        )))
    }

    // ── op 1: kitchen-busy ──

    #[tokio::test]
    async fn set_kitchen_busy_sets_the_window_and_returns_camel_case_body() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeMenuAvailabilityRepo::default();
        repo.locations.lock().unwrap().insert(loc, None);
        let state = test_state(repo, user_id, vec![loc]);
        let busy_until = chrono::Utc::now() + chrono::Duration::minutes(30);

        let response = set_kitchen_busy(
            Extension(state),
            owner(user_id),
            Path(loc),
            request_id(),
            axum::Json(SetKitchenBusyRequest {
                busy_until: Some(Some(busy_until)),
            }),
        )
        .await
        .unwrap()
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: KitchenBusyResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body.id, loc);
        assert_eq!(
            body.kitchen_busy_until.unwrap().timestamp(),
            busy_until.timestamp()
        );
    }

    #[tokio::test]
    async fn set_kitchen_busy_null_clears_the_window() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeMenuAvailabilityRepo::default();
        repo.locations
            .lock()
            .unwrap()
            .insert(loc, Some(chrono::Utc::now()));
        let state = test_state(repo, user_id, vec![loc]);

        let response = set_kitchen_busy(
            Extension(state),
            owner(user_id),
            Path(loc),
            request_id(),
            axum::Json(SetKitchenBusyRequest {
                busy_until: Some(None),
            }),
        )
        .await
        .unwrap()
        .into_response();

        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: KitchenBusyResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body.kitchen_busy_until, None);
    }

    #[tokio::test]
    async fn set_kitchen_busy_404_when_the_location_row_is_missing() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        // access is granted (active membership) but the repo has no such location row.
        let state = test_state(FakeMenuAvailabilityRepo::default(), user_id, vec![loc]);

        let err = crate::error::expect_err(
            set_kitchen_busy(
                Extension(state),
                owner(user_id),
                Path(loc),
                request_id(),
                axum::Json(SetKitchenBusyRequest {
                    busy_until: Some(None),
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn set_kitchen_busy_404_for_a_cross_location_owner() {
        let user_id = Uuid::new_v4();
        let mine = Uuid::new_v4();
        let theirs = Uuid::new_v4();
        let repo = FakeMenuAvailabilityRepo::default();
        repo.locations.lock().unwrap().insert(theirs, None);
        let state = test_state(repo, user_id, vec![mine]);

        let err = crate::error::expect_err(
            set_kitchen_busy(
                Extension(state),
                owner(user_id),
                Path(theirs),
                request_id(),
                axum::Json(SetKitchenBusyRequest {
                    busy_until: Some(None),
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    #[test]
    fn set_kitchen_busy_request_rejects_a_missing_busy_until_key() {
        // menu-availability.ts:29: `.nullable()` WITHOUT `.optional()` still requires the key
        // present — pins that a bare `{}` body is a deserialize error, not a silent `None`.
        let err = serde_json::from_value::<SetKitchenBusyRequest>(serde_json::json!({}))
            .expect_err("a missing busy_until key must fail to deserialize");
        assert!(err.to_string().contains("busy_until"));
    }

    #[test]
    fn set_kitchen_busy_request_accepts_an_explicit_null() {
        let parsed: SetKitchenBusyRequest =
            serde_json::from_value(serde_json::json!({ "busy_until": null })).unwrap();
        // key present, value null -> outer Some (key was seen), inner None (the value itself).
        assert_eq!(parsed.busy_until, Some(None));
    }

    #[test]
    fn set_kitchen_busy_request_accepts_an_explicit_datetime() {
        let dt = chrono::Utc::now();
        let parsed: SetKitchenBusyRequest =
            serde_json::from_value(serde_json::json!({ "busy_until": dt.to_rfc3339() })).unwrap();
        assert_eq!(
            parsed.busy_until.flatten().unwrap().timestamp(),
            dt.timestamp()
        );
    }

    // ── op 2: list schedules ──

    #[tokio::test]
    async fn list_schedules_returns_the_camel_case_shape() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeMenuAvailabilityRepo::default();
        let product_id = Uuid::new_v4();
        repo.products.lock().unwrap().insert(product_id, loc);
        let state = test_state(repo, user_id, vec![loc]);

        create_schedule(
            Extension(state.clone()),
            owner(user_id),
            Path(loc),
            request_id(),
            axum::Json(CreateScheduleRequest {
                product_id: Some(product_id),
                category_id: None,
                mode: ScheduleMode::Daily,
                start_minute: Some(600),
                end_minute: Some(900),
                days_of_week: Some(vec![1, 2, 3]),
                starts_at: None,
                ends_at: None,
                available: true,
            }),
        )
        .await
        .unwrap();

        let response = list_schedules(Extension(state), owner(user_id), Path(loc), request_id())
            .await
            .unwrap()
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: ScheduleListResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body.data.len(), 1);
        assert_eq!(body.data[0].product_id, Some(product_id));
        assert_eq!(body.data[0].start_minute, Some(600));
        assert_eq!(body.data[0].days_of_week, Some(vec![1, 2, 3]));
    }

    #[tokio::test]
    async fn list_schedules_empty_is_200_not_404() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = test_state(FakeMenuAvailabilityRepo::default(), user_id, vec![loc]);

        let response = list_schedules(Extension(state), owner(user_id), Path(loc), request_id())
            .await
            .unwrap()
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: ScheduleListResponse = serde_json::from_slice(&bytes).unwrap();
        assert!(body.data.is_empty());
    }

    #[tokio::test]
    async fn list_schedules_404_for_a_cross_location_owner() {
        let user_id = Uuid::new_v4();
        let mine = Uuid::new_v4();
        let theirs = Uuid::new_v4();
        let state = test_state(FakeMenuAvailabilityRepo::default(), user_id, vec![mine]);

        let err = crate::error::expect_err(
            list_schedules(Extension(state), owner(user_id), Path(theirs), request_id()).await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    // ── op 3: create schedule ──

    fn create_body(product_id: Option<Uuid>, category_id: Option<Uuid>) -> CreateScheduleRequest {
        CreateScheduleRequest {
            product_id,
            category_id,
            mode: ScheduleMode::Daily,
            start_minute: None,
            end_minute: None,
            days_of_week: None,
            starts_at: None,
            ends_at: None,
            available: true,
        }
    }

    #[tokio::test]
    async fn create_schedule_201_with_product_id() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let product_id = Uuid::new_v4();
        let repo = FakeMenuAvailabilityRepo::default();
        repo.products.lock().unwrap().insert(product_id, loc);
        let state = test_state(repo, user_id, vec![loc]);

        let response = create_schedule(
            Extension(state),
            owner(user_id),
            Path(loc),
            request_id(),
            axum::Json(create_body(Some(product_id), None)),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::CREATED);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: ScheduleShape = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body.product_id, Some(product_id));
        assert_eq!(body.category_id, None);
    }

    #[tokio::test]
    async fn create_schedule_201_with_category_id_defaults_mode_and_available() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let category_id = Uuid::new_v4();
        let repo = FakeMenuAvailabilityRepo::default();
        repo.categories.lock().unwrap().insert(category_id, loc);
        let state = test_state(repo, user_id, vec![loc]);

        // `mode`/`available` omitted from the JSON body entirely — proves the zod `.default(...)`
        // parity (menu-availability.ts:52,58) via CreateScheduleRequest's `#[serde(default...)]`.
        let raw_body = serde_json::json!({ "category_id": category_id });
        let body: CreateScheduleRequest = serde_json::from_value(raw_body).unwrap();
        assert_eq!(body.mode, ScheduleMode::Daily);
        assert!(body.available);

        let response = create_schedule(
            Extension(state),
            owner(user_id),
            Path(loc),
            request_id(),
            axum::Json(body),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::CREATED);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let shape: ScheduleShape = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(shape.mode, "daily");
        assert!(shape.available);
    }

    #[tokio::test]
    async fn create_schedule_400_when_both_product_and_category_present() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = test_state(FakeMenuAvailabilityRepo::default(), user_id, vec![loc]);

        let err = crate::error::expect_err(
            create_schedule(
                Extension(state),
                owner(user_id),
                Path(loc),
                request_id(),
                axum::Json(create_body(Some(Uuid::new_v4()), Some(Uuid::new_v4()))),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::ValidationFailed);
        assert_eq!(err.envelope.status, 400);
    }

    #[tokio::test]
    async fn create_schedule_400_when_neither_product_nor_category_present() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = test_state(FakeMenuAvailabilityRepo::default(), user_id, vec![loc]);

        let err = crate::error::expect_err(
            create_schedule(
                Extension(state),
                owner(user_id),
                Path(loc),
                request_id(),
                axum::Json(create_body(None, None)),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::ValidationFailed);
        assert_eq!(err.envelope.status, 400);
    }

    #[tokio::test]
    async fn create_schedule_404_when_product_belongs_to_a_different_location() {
        // R2-1 (15th IDOR) — the load-bearing test: a product_id that exists but is owned by a
        // DIFFERENT location must be rejected (0-row fold-in miss), not silently inserted.
        let user_id = Uuid::new_v4();
        let mine = Uuid::new_v4();
        let someone_elses_location = Uuid::new_v4();
        let their_product = Uuid::new_v4();
        let repo = FakeMenuAvailabilityRepo::default();
        repo.products
            .lock()
            .unwrap()
            .insert(their_product, someone_elses_location);
        let state = test_state(repo, user_id, vec![mine]);

        let err = crate::error::expect_err(
            create_schedule(
                Extension(state),
                owner(user_id),
                Path(mine),
                request_id(),
                axum::Json(create_body(Some(their_product), None)),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
        assert_eq!(err.envelope.message, "Product or category not found");
    }

    #[tokio::test]
    async fn create_schedule_404_when_category_belongs_to_a_different_location() {
        let user_id = Uuid::new_v4();
        let mine = Uuid::new_v4();
        let someone_elses_location = Uuid::new_v4();
        let their_category = Uuid::new_v4();
        let repo = FakeMenuAvailabilityRepo::default();
        repo.categories
            .lock()
            .unwrap()
            .insert(their_category, someone_elses_location);
        let state = test_state(repo, user_id, vec![mine]);

        let err = crate::error::expect_err(
            create_schedule(
                Extension(state),
                owner(user_id),
                Path(mine),
                request_id(),
                axum::Json(create_body(None, Some(their_category))),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
        assert_eq!(err.envelope.message, "Product or category not found");
    }

    #[tokio::test]
    async fn create_schedule_404_for_a_cross_location_owner() {
        let user_id = Uuid::new_v4();
        let mine = Uuid::new_v4();
        let theirs = Uuid::new_v4();
        let state = test_state(FakeMenuAvailabilityRepo::default(), user_id, vec![mine]);

        let err = crate::error::expect_err(
            create_schedule(
                Extension(state),
                owner(user_id),
                Path(theirs),
                request_id(),
                axum::Json(create_body(Some(Uuid::new_v4()), None)),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
        assert_eq!(err.envelope.message, "Not found");
    }

    // ── op 4: delete schedule ──

    #[tokio::test]
    async fn delete_schedule_204_when_found() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeMenuAvailabilityRepo::default();
        let schedule_id = Uuid::new_v4();
        repo.schedules.lock().unwrap().push((
            loc,
            ScheduleRow {
                id: schedule_id,
                product_id: None,
                category_id: None,
                mode: "daily".to_string(),
                start_minute: None,
                end_minute: None,
                days_of_week: None,
                starts_at: None,
                ends_at: None,
                available: true,
            },
        ));
        let state = test_state(repo, user_id, vec![loc]);

        let response = delete_schedule(
            Extension(state),
            owner(user_id),
            Path((loc, schedule_id)),
            request_id(),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn delete_schedule_404_when_not_found() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = test_state(FakeMenuAvailabilityRepo::default(), user_id, vec![loc]);

        let err = crate::error::expect_err(
            delete_schedule(
                Extension(state),
                owner(user_id),
                Path((loc, Uuid::new_v4())),
                request_id(),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn delete_schedule_404_for_a_cross_location_owner() {
        let user_id = Uuid::new_v4();
        let mine = Uuid::new_v4();
        let theirs = Uuid::new_v4();
        let state = test_state(FakeMenuAvailabilityRepo::default(), user_id, vec![mine]);

        let err = crate::error::expect_err(
            delete_schedule(
                Extension(state),
                owner(user_id),
                Path((theirs, Uuid::new_v4())),
                request_id(),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }
}
