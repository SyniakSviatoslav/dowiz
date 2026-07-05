//! S9 GDPR/compliance — the owner-authenticated gdpr-requests + retention-settings surface.
//! Ports `apps/api/src/routes/owner/gdpr.ts` verbatim (route-surface-map rows 70-74). Council:
//! `docs/design/rebuild-gdpr-s9-council/{proposal.md,resolution.md,breaker-findings.md}` —
//! **the reddest surface in the rebuild** (an irreversible erasure). See `crate::jobs::gdpr_erasure`
//! for the erasure ENGINE semantics this surface only enqueues into (customer/order fan-out, the
//! completion gate, the DEFINER call) — this file owns only the request lifecycle + reads.
//!
//! ## Auth + write pattern (reuses `crate::routes::owner` verbatim — see that module's doc)
//! Every op binds `OwnerClaimsExt` + [`super::require_location_access`] (OWNER+LOC, 404
//! existence-hiding) + [`super::assert_active_owner_membership`] as the first in-transaction
//! statement (S3 breaker C1+H4 belt-and-suspenders) + `db::with_user` — no new auth invented.
//!
//! ## Q2 🔴 — the cross-tenant erasure IDOR + status masking (CARRY VERBATIM, ledger #57)
//! A client-supplied `customerId` is UNVERIFIED — it must prove same-tenant membership before it
//! can drive an irreversible erasure (`gdpr.ts:63-86`): 0 rows at `(id, location_id)` -> a plain
//! masked **404** (never distinguishing nonexistent from cross-tenant to the CALLER); a
//! cross-tenant attempt (the id exists at ANOTHER location) is `tracing::warn!(event =
//! "cross_tenant_attempt", ...)` FIRST so it stays detectable server-side. Status reads
//! (list/get) mask the subject id via [`mask_name`] (`maskName`, `pii-mask.ts:6-9`: first char +
//! `"***"`) — the request/audit surface must never itself be a PII disclosure. Note the BRK-8
//! residual (NOT a REV-S9 fix, carried as documented behavior): under NOBYPASSRLS, `customers`'
//! own member-only RLS means a TRULY foreign-tenant id is invisible to the existence probe too
//! (0 rows either way) — the cross-tenant log fires only when the id exists at ANOTHER of the
//! *same* owner's active memberships (a multi-location owner). This is an observability
//! regression the breaker flagged as LOW/not-a-leak, not build-blocking here.
//!
//! ## BRK-6 (carry-fix) — re-request-after-cooldown is 409, never an unhandled 500
//! `gdpr_dedup_per_customer` (`(location_id, customer_id) WHERE status IN ('pending',
//! 'in_progress', 'completed')`) covers a `completed` row PERMANENTLY — the 24h cooldown check
//! only looks back 24h, so a re-request *after* 24h passes the cooldown gate then collides with
//! the still-present `completed` row on INSERT (`unique_violation`, `23505`). [`PgGdprRepo::create`]
//! catches that violation and maps it to the SAME `AlreadyActive` 409 outcome the active-request
//! guard already produces — never an unhandled 500.
//!
//! ## The anonymizer scope is never self-derived (ledger #57, carry verbatim)
//! Every call into `crate::jobs::gdpr_erasure` takes an explicit `location_id` — there is no
//! `|| row.location_id` fallback anywhere in this surface (Q-SCOPE-FAILCLOSED).

use axum::Json;
use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use chrono::{DateTime, Utc};
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

use super::{assert_active_owner_membership, require_location_access};

// ── State + repo trait ──────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct GdprState {
    pub auth: AuthState,
    pub repo: std::sync::Arc<dyn GdprRepo>,
}

/// Op #1's three-way non-happy outcome (`gdpr.ts:63-107`) PLUS the created-id happy path. Named
/// distinctly from `Option<T>` (unlike most S3 repo methods) because there are three DISTINCT
/// non-2xx branches with three DISTINCT status codes, mirroring `categories::DeleteOutcome`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateOutcome {
    /// Membership re-check failed, OR the customerId is nonexistent/cross-tenant — the caller
    /// always sees the SAME masked 404 (`gdpr.ts:84,119-121`).
    NotOwned,
    /// An active (`pending`/`in_progress`) request already exists for this customer, OR (BRK-6)
    /// the INSERT collided with a still-present `completed` row past the 24h cooldown window —
    /// both are the SAME 409 `CONFLICT` to the caller.
    AlreadyActive,
    /// A request for this customer was completed within the last 24h (`gdpr.ts:99-107`).
    TooSoon,
    Created {
        request_id: Uuid,
    },
}

/// A `gdpr_erasure_requests` row projected to exactly what the list/detail reads need.
#[derive(Debug, Clone)]
pub struct GdprRequestRow {
    pub id: Uuid,
    pub customer_id: Option<Uuid>,
    pub status: String,
    pub error_message: Option<String>,
    pub metadata: serde_json::Value,
    pub requested_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// An `anonymization_audit_log` row for the detail read (`gdpr.ts:220-227`).
#[derive(Debug, Clone)]
pub struct AuditLogRow {
    pub id: i64,
    pub scope: String,
    pub subject_kind: String,
    pub subject_id: Uuid,
    pub actor_kind: String,
    pub actor_id: Option<Uuid>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[async_trait::async_trait]
pub trait GdprRepo: Send + Sync {
    /// Op #1 (`gdpr.ts:33-136`). See [`CreateOutcome`] for the four-way result.
    #[allow(clippy::too_many_arguments)]
    async fn create(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        customer_id: Option<Uuid>,
        phone: Option<String>,
        reason: Option<String>,
    ) -> Result<CreateOutcome, RepoError>;

    /// Op #2 (`gdpr.ts:139-196`), cursor-paged DESC by `requested_at`, `limit+1` rows so the
    /// handler can compute `hasMore`/`nextCursor` exactly like `gdpr.ts:181-193`. `Ok(None)` =
    /// the in-transaction membership re-check failed.
    async fn list(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        status: Option<String>,
        cursor: Option<DateTime<Utc>>,
        limit_plus_one: i64,
    ) -> Result<Option<Vec<GdprRequestRow>>, RepoError>;

    /// Op #3 (`gdpr.ts:199-254`). `Ok(None)` covers BOTH the membership re-check failing AND the
    /// row not existing at this tenant (`gdpr.ts:232`'s identical 404 for both).
    async fn get(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        request_id: Uuid,
    ) -> Result<Option<(GdprRequestRow, Vec<AuditLogRow>)>, RepoError>;

    /// Op #4 (`gdpr.ts:257-269`). `Ok(None)` = membership re-check failed or location not found.
    async fn get_retention(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
    ) -> Result<Option<i32>, RepoError>;

    /// Op #5 (`gdpr.ts:272-287`). `Ok(None)` = membership re-check failed or location not found.
    /// Zod's `min(30).max(2555)` bound is NOT re-enforced here (same judgment call as
    /// `categories.rs`'s un-enforced string-length bounds — no validation-attribute crate in this
    /// build's dependency graph); an out-of-range value surfaces as the DB `CHECK` constraint
    /// violation (`ErrorCode::Internal`), which is the S9 council's Q5 to disposition, not a
    /// build-blocking gap for this port.
    async fn set_retention(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        retention_days: i32,
    ) -> Result<Option<i32>, RepoError>;
}

/// `maskName` (`pii-mask.ts:6-9`): first char + `"***"`. Kept local to this file (not shared with
/// `auth::pii::mask_str`, which is a DIFFERENT email-shaped mask) — the only caller is this
/// surface's status reads.
fn mask_name(id: &Uuid) -> String {
    let s = id.to_string();
    format!("{}***", &s[..1])
}

// ── DTOs ─────────────────────────────────────────────────────────────────────────────────────

/// Op #1 body (`gdpr.ts:7-13`, `.strict().refine(...)`). The refine ("either customerId or phone")
/// is enforced in the handler, not `serde` — see [`create_gdpr_request`].
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateGdprRequestBody {
    #[serde(default)]
    pub customer_id: Option<Uuid>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateGdprRequestResponse {
    pub request_id: Uuid,
    pub status: String,
}

/// Op #2 querystring (`gdpr.ts:15-19`).
#[derive(Debug, Deserialize)]
pub struct ListGdprRequestsQuery {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub cursor: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GdprRequestSummary {
    pub id: Uuid,
    /// `maskName(row.customer_id)` — masked, NEVER the raw subject id (`gdpr.ts:184`).
    pub customer_id: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    /// `#[schema(value_type = String)]`: `utoipa`'s `chrono` feature isn't enabled on this crate
    /// (`Cargo.toml` out of scope — same posture as `themes.rs`/`menu_availability.rs`'s
    /// `DateTime<Utc>` fields); the wire shape is already an RFC3339 string via chrono's `serde`
    /// feature, this only overrides the generated OpenAPI schema.
    #[schema(value_type = String)]
    pub requested_at: DateTime<Utc>,
    #[schema(value_type = Option<String>)]
    pub completed_at: Option<DateTime<Utc>>,
}

impl From<GdprRequestRow> for GdprRequestSummary {
    fn from(row: GdprRequestRow) -> Self {
        GdprRequestSummary {
            id: row.id,
            customer_id: row.customer_id.as_ref().map(mask_name),
            status: row.status,
            error_message: row.error_message,
            requested_at: row.requested_at,
            completed_at: row.completed_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListGdprRequestsResponse {
    pub requests: Vec<GdprRequestSummary>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogEntry {
    pub id: i64,
    pub scope: String,
    pub subject_kind: String,
    /// `maskName(a.subject_id)` — ALWAYS masked (`gdpr.ts:247`).
    pub subject_id: String,
    pub actor_kind: String,
    /// `maskName(a.actor_id)` when present (`gdpr.ts:249`).
    pub actor_id: Option<String>,
    pub metadata: serde_json::Value,
    #[schema(value_type = String)]
    pub created_at: DateTime<Utc>,
}

impl From<AuditLogRow> for AuditLogEntry {
    fn from(row: AuditLogRow) -> Self {
        AuditLogEntry {
            id: row.id,
            scope: row.scope,
            subject_kind: row.subject_kind,
            subject_id: mask_name(&row.subject_id),
            actor_kind: row.actor_kind,
            actor_id: row.actor_id.as_ref().map(mask_name),
            metadata: row.metadata,
            created_at: row.created_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GdprRequestDetail {
    pub id: Uuid,
    pub customer_id: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub metadata: serde_json::Value,
    #[schema(value_type = String)]
    pub requested_at: DateTime<Utc>,
    #[schema(value_type = Option<String>)]
    pub completed_at: Option<DateTime<Utc>>,
    pub audit_logs: Vec<AuditLogEntry>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RetentionResponse {
    pub retention_days: i32,
}

/// Op #5 body (`gdpr.ts:21-23`, `.strict()`).
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateRetentionRequest {
    pub retention_days: i32,
}

fn internal_error(correlation_id: String) -> ApiError {
    ApiError::new(ErrorCode::Internal, "internal_error", correlation_id)
}

fn not_found(correlation_id: String) -> ApiError {
    ApiError::new(ErrorCode::NotFound, "Not found", correlation_id)
}

// ── Handlers ─────────────────────────────────────────────────────────────────────────────────

/// `POST /api/owner/locations/{locationId}/gdpr-requests` (op #1, `gdpr.ts:33-136`) -> 201, 404,
/// 409, 429, or 400 (missing both `customerId`/`phone`). Rate-limited 30/min (`gdpr.ts:34-36`) —
/// mounted on this route in `super::owner_catalog_router` via `RateLimitLayer`.
#[utoipa::path(
    post,
    path = "/api/owner/locations/{locationId}/gdpr-requests",
    params(("locationId" = Uuid, Path)),
    request_body = CreateGdprRequestBody,
    responses(
        (status = 201, description = "Created", body = CreateGdprRequestResponse),
        (status = 400, description = "Neither customerId nor phone provided", body = domain::ErrorEnvelope),
        (status = 404, description = "customerId not owned by this tenant", body = domain::ErrorEnvelope),
        (status = 409, description = "An active or dedup-colliding request already exists", body = domain::ErrorEnvelope),
        (status = 429, description = "Completed within the last 24h", body = domain::ErrorEnvelope),
    ),
    tag = "owner-gdpr"
)]
pub async fn create_gdpr_request(
    Extension(state): Extension<GdprState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(location_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<CreateGdprRequestBody>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    // Zod's `.refine(data => data.customerId || data.phone, ...)` (gdpr.ts:11-13).
    if body.customer_id.is_none() && body.phone.is_none() {
        return Err(ApiError::validation_failed_400(
            "Either customerId or phone is required",
            correlation_id,
        ));
    }

    let outcome = state
        .repo
        .create(
            owner.user_id,
            location_id,
            body.customer_id,
            body.phone,
            body.reason,
        )
        .await
        .map_err(|_err| internal_error(correlation_id.clone()))?;

    match outcome {
        CreateOutcome::NotOwned => Err(not_found(correlation_id)),
        CreateOutcome::AlreadyActive => Err(ApiError::new(
            ErrorCode::Conflict,
            "An erasure request for this customer is already pending, in progress, or was recently completed",
            correlation_id,
        )),
        CreateOutcome::TooSoon => Err(ApiError::new(
            ErrorCode::RateLimit,
            "A request for this customer was already completed in the last 24 hours",
            correlation_id,
        )),
        CreateOutcome::Created { request_id } => Ok((
            StatusCode::CREATED,
            Json(CreateGdprRequestResponse {
                request_id,
                status: "pending".to_string(),
            }),
        )),
    }
}

/// `GET /api/owner/locations/{locationId}/gdpr-requests` (op #2, `gdpr.ts:139-196`) -> 200
/// `{requests:[], nextCursor}`, cursor-paged DESC by `requestedAt`. The cursor is an opaque
/// base64url-encoded `{"requestedAt": "..."}` JSON blob (`gdpr.ts:159,192`) — an invalid cursor is
/// TOLERATED (logged, ignored), never a 400 (`gdpr.ts:164-166`'s `catch`).
#[utoipa::path(
    get,
    path = "/api/owner/locations/{locationId}/gdpr-requests",
    params(
        ("locationId" = Uuid, Path),
        ("status" = Option<String>, Query),
        ("limit" = Option<i64>, Query),
        ("cursor" = Option<String>, Query),
    ),
    responses(
        (status = 200, description = "Cursor-paged, subject-id masked", body = ListGdprRequestsResponse),
        (status = 404, description = "Foreign/unknown location", body = domain::ErrorEnvelope),
    ),
    tag = "owner-gdpr"
)]
pub async fn list_gdpr_requests(
    Extension(state): Extension<GdprState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(location_id): Path<Uuid>,
    Query(params): Query<ListGdprRequestsQuery>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    // z.coerce.number().int().min(1).max(100).default(50) (gdpr.ts:17).
    let limit = params.limit.unwrap_or(50).clamp(1, 100);
    let cursor = params.cursor.as_deref().and_then(decode_cursor);

    let rows = state
        .repo
        .list(owner.user_id, location_id, params.status, cursor, limit + 1)
        .await
        .map_err(|_err| internal_error(correlation_id.clone()))?
        .ok_or_else(|| not_found(correlation_id))?;

    // limit is clamped to [1,100] above, always non-negative and small.
    let limit_usize = usize::try_from(limit).unwrap_or(0);
    let has_more = rows.len() > limit_usize;
    let mut rows = rows;
    if has_more {
        rows.truncate(limit_usize);
    }
    let next_cursor = if has_more {
        rows.last().map(|r| encode_cursor(r.requested_at))
    } else {
        None
    };

    Ok(Json(ListGdprRequestsResponse {
        requests: rows.into_iter().map(GdprRequestSummary::from).collect(),
        next_cursor,
    }))
}

/// `GET /api/owner/locations/{locationId}/gdpr-requests/{requestId}` (op #3, `gdpr.ts:199-254`)
/// -> 200 or 404.
#[utoipa::path(
    get,
    path = "/api/owner/locations/{locationId}/gdpr-requests/{requestId}",
    params(("locationId" = Uuid, Path), ("requestId" = Uuid, Path)),
    responses(
        (status = 200, description = "Request detail + masked audit trail", body = GdprRequestDetail),
        (status = 404, description = "Not found", body = domain::ErrorEnvelope),
    ),
    tag = "owner-gdpr"
)]
pub async fn get_gdpr_request(
    Extension(state): Extension<GdprState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path((location_id, request_id)): Path<(Uuid, Uuid)>,
    Extension(request_id_ext): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id_ext);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    let (row, audit_rows) = state
        .repo
        .get(owner.user_id, location_id, request_id)
        .await
        .map_err(|_err| internal_error(correlation_id.clone()))?
        .ok_or_else(|| not_found(correlation_id))?;

    Ok(Json(GdprRequestDetail {
        id: row.id,
        customer_id: row.customer_id.as_ref().map(mask_name),
        status: row.status,
        error_message: row.error_message,
        metadata: row.metadata,
        requested_at: row.requested_at,
        completed_at: row.completed_at,
        audit_logs: audit_rows.into_iter().map(AuditLogEntry::from).collect(),
    }))
}

/// `GET /api/owner/locations/{locationId}/settings/retention` (op #4, `gdpr.ts:257-269`) -> 200 or
/// 404.
#[utoipa::path(
    get,
    path = "/api/owner/locations/{locationId}/settings/retention",
    params(("locationId" = Uuid, Path)),
    responses(
        (status = 200, description = "Current retention_days", body = RetentionResponse),
        (status = 404, description = "Foreign/unknown location", body = domain::ErrorEnvelope),
    ),
    tag = "owner-gdpr"
)]
pub async fn get_retention_settings(
    Extension(state): Extension<GdprState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(location_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    let retention_days = state
        .repo
        .get_retention(owner.user_id, location_id)
        .await
        .map_err(|_err| internal_error(correlation_id.clone()))?
        .ok_or_else(|| not_found(correlation_id))?;

    Ok(Json(RetentionResponse { retention_days }))
}

/// `PUT /api/owner/locations/{locationId}/settings/retention` (op #5, `gdpr.ts:272-287`) -> 200 or
/// 404. Q5 (retention legal basis, `docs/design/rebuild-gdpr-s9-council/resolution.md` REV-S9-7)
/// is a controller-policy/DPA disposition, not a code gate here — see `GdprRepo::set_retention`'s
/// doc for why the 30-2555 bound is not re-enforced app-side.
#[utoipa::path(
    put,
    path = "/api/owner/locations/{locationId}/settings/retention",
    params(("locationId" = Uuid, Path)),
    request_body = UpdateRetentionRequest,
    responses(
        (status = 200, description = "Updated retention_days", body = RetentionResponse),
        (status = 404, description = "Foreign/unknown location", body = domain::ErrorEnvelope),
    ),
    tag = "owner-gdpr"
)]
pub async fn put_retention_settings(
    Extension(state): Extension<GdprState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(location_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<UpdateRetentionRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    let retention_days = state
        .repo
        .set_retention(owner.user_id, location_id, body.retention_days)
        .await
        .map_err(|_err| internal_error(correlation_id.clone()))?
        .ok_or_else(|| not_found(correlation_id))?;

    Ok(Json(RetentionResponse { retention_days }))
}

// ── cursor codec (gdpr.ts:157-166,191-193) ──────────────────────────────────────────────────

/// Encodes `{"requestedAt": "<rfc3339>"}` as base64url — matches `Buffer.from(JSON.stringify(...))
/// .toString('base64url')` (`gdpr.ts:192`).
fn encode_cursor(requested_at: DateTime<Utc>) -> String {
    use base64::Engine;
    let json = serde_json::json!({ "requestedAt": requested_at.to_rfc3339() });
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(json.to_string())
}

/// Decodes a cursor; ANY failure (bad base64, bad JSON, missing/unparseable `requestedAt`) returns
/// `None` (ignore the cursor, proceed unfiltered) — carries `gdpr.ts:164-166`'s tolerant `catch`
/// verbatim, never a 400.
fn decode_cursor(raw: &str) -> Option<DateTime<Utc>> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(raw)
        .ok()?;
    let text = String::from_utf8(bytes).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let requested_at = value.get("requestedAt")?.as_str()?;
    DateTime::parse_from_rfc3339(requested_at)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

// ── PgGdprRepo ───────────────────────────────────────────────────────────────────────────────

/// Constructed by the lead's integration wiring (`main.rs`'s `GdprState` assembly point) — same
/// "unused until integration" posture `PgCategoriesRepo` documents.
#[allow(
    dead_code,
    reason = "constructed by the lead's GdprState wiring at integration — see struct doc"
)]
pub struct PgGdprRepo {
    pool: sqlx::PgPool,
}

#[allow(
    dead_code,
    reason = "constructed by the lead's GdprState wiring at integration — see struct doc"
)]
impl PgGdprRepo {
    pub fn new(pool: sqlx::PgPool) -> Self {
        PgGdprRepo { pool }
    }
}

/// Same posture as `categories.rs`'s `map_txn_err` — only reachable through `PgGdprRepo`, unused
/// until integration.
#[allow(
    dead_code,
    reason = "only reachable through PgGdprRepo, unused until the lead's integration wiring"
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
impl GdprRepo for PgGdprRepo {
    async fn create(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        customer_id: Option<Uuid>,
        phone: Option<String>,
        reason: Option<String>,
    ) -> Result<CreateOutcome, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, move |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(CreateOutcome::NotOwned);
                }

                // Resolve customerId from phone if not provided directly (gdpr.ts:47-57).
                let mut resolved_customer_id = customer_id;
                if customer_id.is_none() {
                    if let Some(phone) = phone.as_ref() {
                        let row: Option<(Uuid,)> = sqlx::query_as(
                            "SELECT id FROM customers WHERE location_id = $1 AND phone = $2 LIMIT 1",
                        )
                        .bind(location_id)
                        .bind(phone)
                        .fetch_optional(&mut **txn)
                        .await?;
                        resolved_customer_id = row.map(|(id,)| id);
                    }
                }

                // Q2 the cross-tenant erasure IDOR (ledger #57, carry verbatim): a client-supplied
                // customerId is unverified until proven same-tenant (gdpr.ts:63-86).
                if let Some(cid) = customer_id {
                    let owned: Option<(i32,)> = sqlx::query_as(
                        "SELECT 1 FROM customers WHERE id = $1 AND location_id = $2",
                    )
                    .bind(cid)
                    .bind(location_id)
                    .fetch_optional(&mut **txn)
                    .await?;
                    if owned.is_none() {
                        let exists: Option<(Uuid,)> =
                            sqlx::query_as("SELECT location_id FROM customers WHERE id = $1")
                                .bind(cid)
                                .fetch_optional(&mut **txn)
                                .await?;
                        if let Some((subject_location_id,)) = exists {
                            tracing::warn!(
                                event = "cross_tenant_attempt",
                                resource = "gdpr_erasure_requests",
                                actor_user_id = %owner_user_id,
                                actor_location_id = %location_id,
                                target_customer_id = %cid,
                                subject_location_id = %subject_location_id,
                                "cross-tenant erasure request blocked"
                            );
                        }
                        return Ok(CreateOutcome::NotOwned);
                    }
                }

                if let Some(cid) = resolved_customer_id {
                    let active: Option<(Uuid,)> = sqlx::query_as(
                        "SELECT id FROM gdpr_erasure_requests
                         WHERE location_id = $1 AND customer_id = $2
                           AND status IN ('pending', 'in_progress')
                         LIMIT 1",
                    )
                    .bind(location_id)
                    .bind(cid)
                    .fetch_optional(&mut **txn)
                    .await?;
                    if active.is_some() {
                        return Ok(CreateOutcome::AlreadyActive);
                    }

                    let recent: Option<(Uuid,)> = sqlx::query_as(
                        "SELECT id FROM gdpr_erasure_requests
                         WHERE location_id = $1 AND customer_id = $2 AND status = 'completed'
                           AND completed_at > now() - interval '24 hours'
                         LIMIT 1",
                    )
                    .bind(location_id)
                    .bind(cid)
                    .fetch_optional(&mut **txn)
                    .await?;
                    if recent.is_some() {
                        return Ok(CreateOutcome::TooSoon);
                    }
                }

                let insert: Result<(Uuid,), sqlx::Error> = sqlx::query_as(
                    "INSERT INTO gdpr_erasure_requests
                       (location_id, customer_id, subject_phone, reason, requested_by_owner_id, status)
                     VALUES ($1, $2, $3, $4, $5, 'pending')
                     RETURNING id",
                )
                .bind(location_id)
                .bind(resolved_customer_id)
                .bind(phone)
                .bind(reason)
                .bind(owner_user_id)
                .fetch_one(&mut **txn)
                .await;

                match insert {
                    Ok((id,)) => Ok(CreateOutcome::Created { request_id: id }),
                    // BRK-6 (carry-fix): the dedup unique index covers `completed` PERMANENTLY —
                    // a re-request after the 24h cooldown can still collide. Map the unique
                    // violation to the SAME 409 the active-request guard already produces, never
                    // an unhandled 500.
                    Err(sqlx::Error::Database(db_err))
                        if db_err.code().as_deref() == Some("23505") =>
                    {
                        Ok(CreateOutcome::AlreadyActive)
                    }
                    Err(other) => Err(other),
                }
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn list(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        status: Option<String>,
        cursor: Option<DateTime<Utc>>,
        limit_plus_one: i64,
    ) -> Result<Option<Vec<GdprRequestRow>>, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, move |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(None);
                }
                let rows: Vec<(
                    Uuid,
                    Option<Uuid>,
                    String,
                    Option<String>,
                    DateTime<Utc>,
                    Option<DateTime<Utc>>,
                )> = sqlx::query_as(
                    "SELECT id, customer_id, status, error_message, requested_at, completed_at
                         FROM gdpr_erasure_requests
                         WHERE location_id = $1
                           AND ($2::text IS NULL OR status = $2)
                           AND ($3::timestamptz IS NULL OR requested_at < $3)
                         ORDER BY requested_at DESC
                         LIMIT $4",
                )
                .bind(location_id)
                .bind(status)
                .bind(cursor)
                .bind(limit_plus_one)
                .fetch_all(&mut **txn)
                .await?;
                Ok(Some(rows))
            })
        })
        .await
        .map(|opt| {
            opt.map(|rows| {
                rows.into_iter()
                    .map(
                        |(id, customer_id, status, error_message, requested_at, completed_at)| {
                            GdprRequestRow {
                                id,
                                customer_id,
                                status,
                                error_message,
                                metadata: serde_json::Value::Null,
                                requested_at,
                                completed_at,
                            }
                        },
                    )
                    .collect()
            })
        })
        .map_err(map_txn_err)
    }

    async fn get(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        request_id: Uuid,
    ) -> Result<Option<(GdprRequestRow, Vec<AuditLogRow>)>, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, move |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(None);
                }
                let req: Option<(
                    Uuid,
                    Option<Uuid>,
                    String,
                    Option<String>,
                    serde_json::Value,
                    DateTime<Utc>,
                    Option<DateTime<Utc>>,
                )> = sqlx::query_as(
                    "SELECT id, customer_id, status, error_message, metadata, requested_at, completed_at
                     FROM gdpr_erasure_requests
                     WHERE id = $1 AND location_id = $2",
                )
                .bind(request_id)
                .bind(location_id)
                .fetch_optional(&mut **txn)
                .await?;
                let Some((id, customer_id, status, error_message, metadata, requested_at, completed_at)) =
                    req
                else {
                    return Ok(None);
                };

                let mut audit_rows = Vec::new();
                if let Some(cid) = customer_id {
                    let rows: Vec<(
                        i64,
                        String,
                        String,
                        Uuid,
                        String,
                        Option<Uuid>,
                        serde_json::Value,
                        DateTime<Utc>,
                    )> = sqlx::query_as(
                        "SELECT id, scope, subject_kind, subject_id, actor_kind, actor_id, metadata, created_at
                         FROM anonymization_audit_log
                         WHERE subject_id = $1 AND location_id = $2
                         ORDER BY created_at DESC",
                    )
                    .bind(cid)
                    .bind(location_id)
                    .fetch_all(&mut **txn)
                    .await?;
                    audit_rows = rows
                        .into_iter()
                        .map(
                            |(id, scope, subject_kind, subject_id, actor_kind, actor_id, metadata, created_at)| {
                                AuditLogRow {
                                    id,
                                    scope,
                                    subject_kind,
                                    subject_id,
                                    actor_kind,
                                    actor_id,
                                    metadata,
                                    created_at,
                                }
                            },
                        )
                        .collect();
                }

                Ok(Some((
                    GdprRequestRow {
                        id,
                        customer_id,
                        status,
                        error_message,
                        metadata,
                        requested_at,
                        completed_at,
                    },
                    audit_rows,
                )))
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn get_retention(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
    ) -> Result<Option<i32>, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, move |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(None);
                }
                let row: Option<(i32,)> =
                    sqlx::query_as("SELECT retention_days FROM locations WHERE id = $1")
                        .bind(location_id)
                        .fetch_optional(&mut **txn)
                        .await?;
                Ok(row.map(|(d,)| d))
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn set_retention(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        retention_days: i32,
    ) -> Result<Option<i32>, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, move |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(None);
                }
                let row: Option<(i32,)> = sqlx::query_as(
                    "UPDATE locations SET retention_days = $1 WHERE id = $2 RETURNING retention_days",
                )
                .bind(retention_days)
                .bind(location_id)
                .fetch_optional(&mut **txn)
                .await?;
                Ok(row.map(|(d,)| d))
            })
        })
        .await
        .map_err(map_txn_err)
    }
}

// ── FakeGdprRepo (test-only) ─────────────────────────────────────────────────────────────────

#[cfg(test)]
pub mod fake {
    use super::{AuditLogRow, CreateOutcome, GdprRepo, GdprRequestRow, RepoError};
    use chrono::{DateTime, Utc};
    use std::collections::{HashMap, HashSet};
    use std::sync::Mutex;
    use uuid::Uuid;

    #[derive(Debug, Clone)]
    pub struct FakeCustomer {
        pub location_id: Uuid,
        pub phone: Option<String>,
    }

    #[derive(Debug, Clone)]
    pub struct FakeRequest {
        pub location_id: Uuid,
        pub customer_id: Option<Uuid>,
        pub status: String,
        pub completed_at: Option<DateTime<Utc>>,
        pub requested_at: DateTime<Utc>,
    }

    #[derive(Default)]
    pub struct FakeGdprRepo {
        pub customers: Mutex<HashMap<Uuid, FakeCustomer>>,
        pub requests: Mutex<HashMap<Uuid, FakeRequest>>,
        pub retention_days: Mutex<HashMap<Uuid, i32>>,
        pub membership_denied: Mutex<HashSet<Uuid>>,
        /// Records every cross-tenant-attempt "log line" this fake would have emitted — lets a
        /// test assert BRK's Q2 security-log fires without capturing real `tracing` output.
        pub cross_tenant_attempts: Mutex<Vec<Uuid>>,
        /// Forces the very next `create` INSERT to behave as a `23505` unique-violation collision
        /// (BRK-6 probe) instead of actually inserting.
        pub force_dedup_collision: Mutex<bool>,
    }

    impl FakeGdprRepo {
        pub fn seed_customer(&self, id: Uuid, location_id: Uuid, phone: Option<&str>) {
            self.customers.lock().unwrap().insert(
                id,
                FakeCustomer {
                    location_id,
                    phone: phone.map(str::to_string),
                },
            );
        }

        pub fn seed_request(&self, id: Uuid, req: FakeRequest) {
            self.requests.lock().unwrap().insert(id, req);
        }

        pub fn deny_membership(&self, location_id: Uuid) {
            self.membership_denied.lock().unwrap().insert(location_id);
        }

        pub fn force_next_insert_to_collide(&self) {
            *self.force_dedup_collision.lock().unwrap() = true;
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
    impl GdprRepo for FakeGdprRepo {
        async fn create(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            customer_id: Option<Uuid>,
            phone: Option<String>,
            _reason: Option<String>,
        ) -> Result<CreateOutcome, RepoError> {
            if !self.membership_ok(location_id) {
                return Ok(CreateOutcome::NotOwned);
            }

            let mut resolved = customer_id;
            if customer_id.is_none() {
                if let Some(phone) = phone.as_ref() {
                    resolved = self
                        .customers
                        .lock()
                        .unwrap()
                        .iter()
                        .find(|(_, c)| {
                            c.location_id == location_id
                                && c.phone.as_deref() == Some(phone.as_str())
                        })
                        .map(|(id, _)| *id);
                }
            }

            if let Some(cid) = customer_id {
                let customers = self.customers.lock().unwrap();
                match customers.get(&cid) {
                    Some(c) if c.location_id == location_id => {}
                    Some(_) => {
                        self.cross_tenant_attempts.lock().unwrap().push(cid);
                        return Ok(CreateOutcome::NotOwned);
                    }
                    None => return Ok(CreateOutcome::NotOwned),
                }
            }

            if let Some(cid) = resolved {
                let requests = self.requests.lock().unwrap();
                let has_active = requests.values().any(|r| {
                    r.location_id == location_id
                        && r.customer_id == Some(cid)
                        && matches!(r.status.as_str(), "pending" | "in_progress")
                });
                if has_active {
                    return Ok(CreateOutcome::AlreadyActive);
                }
                let recently_completed = requests.values().any(|r| {
                    r.location_id == location_id
                        && r.customer_id == Some(cid)
                        && r.status == "completed"
                        && r.completed_at
                            .is_some_and(|c| c > Utc::now() - chrono::Duration::hours(24))
                });
                if recently_completed {
                    return Ok(CreateOutcome::TooSoon);
                }
            }

            let mut force = self.force_dedup_collision.lock().unwrap();
            if *force {
                *force = false;
                return Ok(CreateOutcome::AlreadyActive);
            }
            drop(force);

            let id = Uuid::new_v4();
            self.requests.lock().unwrap().insert(
                id,
                FakeRequest {
                    location_id,
                    customer_id: resolved,
                    status: "pending".to_string(),
                    completed_at: None,
                    requested_at: Utc::now(),
                },
            );
            Ok(CreateOutcome::Created { request_id: id })
        }

        async fn list(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            status: Option<String>,
            cursor: Option<DateTime<Utc>>,
            limit_plus_one: i64,
        ) -> Result<Option<Vec<GdprRequestRow>>, RepoError> {
            if !self.membership_ok(location_id) {
                return Ok(None);
            }
            let requests = self.requests.lock().unwrap();
            let mut rows: Vec<GdprRequestRow> = requests
                .iter()
                .filter(|(_, r)| r.location_id == location_id)
                .filter(|(_, r)| status.as_ref().is_none_or(|s| *s == r.status))
                .filter(|(_, r)| cursor.is_none_or(|c| r.requested_at < c))
                .map(|(id, r)| GdprRequestRow {
                    id: *id,
                    customer_id: r.customer_id,
                    status: r.status.clone(),
                    error_message: None,
                    metadata: serde_json::Value::Null,
                    requested_at: r.requested_at,
                    completed_at: r.completed_at,
                })
                .collect();
            rows.sort_by_key(|r| std::cmp::Reverse(r.requested_at));
            rows.truncate(usize::try_from(limit_plus_one).unwrap_or(0));
            Ok(Some(rows))
        }

        async fn get(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            request_id: Uuid,
        ) -> Result<Option<(GdprRequestRow, Vec<AuditLogRow>)>, RepoError> {
            if !self.membership_ok(location_id) {
                return Ok(None);
            }
            let requests = self.requests.lock().unwrap();
            let row = requests
                .get(&request_id)
                .filter(|r| r.location_id == location_id)
                .map(|r| GdprRequestRow {
                    id: request_id,
                    customer_id: r.customer_id,
                    status: r.status.clone(),
                    error_message: None,
                    metadata: serde_json::Value::Null,
                    requested_at: r.requested_at,
                    completed_at: r.completed_at,
                });
            Ok(row.map(|r| (r, Vec::<AuditLogRow>::new())))
        }

        async fn get_retention(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
        ) -> Result<Option<i32>, RepoError> {
            if !self.membership_ok(location_id) {
                return Ok(None);
            }
            Ok(Some(
                *self
                    .retention_days
                    .lock()
                    .unwrap()
                    .get(&location_id)
                    .unwrap_or(&365),
            ))
        }

        async fn set_retention(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            retention_days: i32,
        ) -> Result<Option<i32>, RepoError> {
            if !self.membership_ok(location_id) {
                return Ok(None);
            }
            self.retention_days
                .lock()
                .unwrap()
                .insert(location_id, retention_days);
            Ok(Some(retention_days))
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::fake::FakeGdprRepo;
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

    fn state_with(repo: FakeGdprRepo, auth: AuthState) -> GdprState {
        GdprState {
            auth,
            repo: Arc::new(repo),
        }
    }

    // ── op #1 create_gdpr_request ──

    #[tokio::test]
    async fn create_201_happy_path_by_customer_id() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let customer_id = Uuid::new_v4();
        let repo = FakeGdprRepo::default();
        repo.seed_customer(customer_id, loc, None);
        let state = state_with(repo, owner_with_location(user_id, loc));
        let owner = OwnerClaims::new(user_id, Some(loc));

        let response = create_gdpr_request(
            Extension(state),
            OwnerClaimsExt(owner),
            Path(loc),
            Extension(request_id()),
            Json(CreateGdprRequestBody {
                customer_id: Some(customer_id),
                phone: None,
                reason: None,
            }),
        )
        .await
        .unwrap()
        .into_response();

        assert_eq!(response.status(), StatusCode::CREATED);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: CreateGdprRequestResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body.status, "pending");
    }

    #[tokio::test]
    async fn create_400_when_neither_customer_id_nor_phone_given() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = state_with(FakeGdprRepo::default(), owner_with_location(user_id, loc));
        let owner = OwnerClaims::new(user_id, Some(loc));

        let err = crate::error::expect_err(
            create_gdpr_request(
                Extension(state),
                OwnerClaimsExt(owner),
                Path(loc),
                Extension(request_id()),
                Json(CreateGdprRequestBody {
                    customer_id: None,
                    phone: None,
                    reason: None,
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::ValidationFailed);
        assert_eq!(err.envelope.status, 400);
    }

    /// Q2 🔴 the cross-tenant erasure IDOR (ledger #57): a customerId that exists at ANOTHER
    /// tenant must be a masked 404, never a leak, AND the attempt is security-logged.
    #[tokio::test]
    async fn create_404_masked_for_a_cross_tenant_customer_id_and_logs_the_attempt() {
        let user_id = Uuid::new_v4();
        let mine = Uuid::new_v4();
        let theirs = Uuid::new_v4();
        let foreign_customer = Uuid::new_v4();
        let repo = FakeGdprRepo::default();
        repo.seed_customer(foreign_customer, theirs, None);
        let state_ref = Arc::new(repo);
        let state = GdprState {
            auth: owner_with_location(user_id, mine),
            repo: state_ref.clone(),
        };
        let owner = OwnerClaims::new(user_id, Some(mine));

        let err = crate::error::expect_err(
            create_gdpr_request(
                Extension(state),
                OwnerClaimsExt(owner),
                Path(mine),
                Extension(request_id()),
                Json(CreateGdprRequestBody {
                    customer_id: Some(foreign_customer),
                    phone: None,
                    reason: None,
                }),
            )
            .await,
        );
        assert_eq!(
            err.envelope.code,
            ErrorCode::NotFound,
            "cross-tenant must be the SAME masked 404 as nonexistent — never distinguishable"
        );
        assert_eq!(
            state_ref.cross_tenant_attempts.lock().unwrap().as_slice(),
            &[foreign_customer],
            "the cross-tenant attempt must be security-logged before the 404"
        );
    }

    #[tokio::test]
    async fn create_404_for_a_nonexistent_customer_id_without_logging() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = Arc::new(FakeGdprRepo::default());
        let state = GdprState {
            auth: owner_with_location(user_id, loc),
            repo: repo.clone(),
        };
        let owner = OwnerClaims::new(user_id, Some(loc));

        let err = crate::error::expect_err(
            create_gdpr_request(
                Extension(state),
                OwnerClaimsExt(owner),
                Path(loc),
                Extension(request_id()),
                Json(CreateGdprRequestBody {
                    customer_id: Some(Uuid::new_v4()),
                    phone: None,
                    reason: None,
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
        assert!(
            repo.cross_tenant_attempts.lock().unwrap().is_empty(),
            "a genuinely nonexistent id must NOT be logged as a cross-tenant attempt"
        );
    }

    #[tokio::test]
    async fn create_404_when_in_transaction_membership_recheck_fails() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeGdprRepo::default();
        repo.deny_membership(loc);
        let state = state_with(repo, owner_with_location(user_id, loc));
        let owner = OwnerClaims::new(user_id, Some(loc));

        let err = crate::error::expect_err(
            create_gdpr_request(
                Extension(state),
                OwnerClaimsExt(owner),
                Path(loc),
                Extension(request_id()),
                Json(CreateGdprRequestBody {
                    customer_id: None,
                    phone: Some("+15551230000".to_string()),
                    reason: None,
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn create_409_when_an_active_request_already_exists() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let customer_id = Uuid::new_v4();
        let repo = FakeGdprRepo::default();
        repo.seed_customer(customer_id, loc, None);
        repo.seed_request(
            Uuid::new_v4(),
            fake::FakeRequest {
                location_id: loc,
                customer_id: Some(customer_id),
                status: "pending".to_string(),
                completed_at: None,
                requested_at: Utc::now(),
            },
        );
        let state = state_with(repo, owner_with_location(user_id, loc));
        let owner = OwnerClaims::new(user_id, Some(loc));

        let err = crate::error::expect_err(
            create_gdpr_request(
                Extension(state),
                OwnerClaimsExt(owner),
                Path(loc),
                Extension(request_id()),
                Json(CreateGdprRequestBody {
                    customer_id: Some(customer_id),
                    phone: None,
                    reason: None,
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::Conflict);
    }

    #[tokio::test]
    async fn create_429_when_completed_within_24h() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let customer_id = Uuid::new_v4();
        let repo = FakeGdprRepo::default();
        repo.seed_customer(customer_id, loc, None);
        repo.seed_request(
            Uuid::new_v4(),
            fake::FakeRequest {
                location_id: loc,
                customer_id: Some(customer_id),
                status: "completed".to_string(),
                completed_at: Some(Utc::now() - chrono::Duration::hours(1)),
                requested_at: Utc::now() - chrono::Duration::hours(2),
            },
        );
        let state = state_with(repo, owner_with_location(user_id, loc));
        let owner = OwnerClaims::new(user_id, Some(loc));

        let err = crate::error::expect_err(
            create_gdpr_request(
                Extension(state),
                OwnerClaimsExt(owner),
                Path(loc),
                Extension(request_id()),
                Json(CreateGdprRequestBody {
                    customer_id: Some(customer_id),
                    phone: None,
                    reason: None,
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::RateLimit);
    }

    /// BRK-6 (carry-fix): a re-request colliding with the dedup unique index (a still-present
    /// `completed` row past the 24h cooldown) must be a clean 409, never an unhandled 500.
    #[tokio::test]
    async fn create_409_not_500_on_a_dedup_index_collision_past_cooldown() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let customer_id = Uuid::new_v4();
        let repo = FakeGdprRepo::default();
        repo.seed_customer(customer_id, loc, None);
        // No active/recent-completed request seeded (cooldown check passes) — but the fake
        // simulates the underlying unique-index collision the INSERT would hit anyway.
        repo.force_next_insert_to_collide();
        let state = state_with(repo, owner_with_location(user_id, loc));
        let owner = OwnerClaims::new(user_id, Some(loc));

        let err = crate::error::expect_err(
            create_gdpr_request(
                Extension(state),
                OwnerClaimsExt(owner),
                Path(loc),
                Extension(request_id()),
                Json(CreateGdprRequestBody {
                    customer_id: Some(customer_id),
                    phone: None,
                    reason: None,
                }),
            )
            .await,
        );
        assert_eq!(
            err.envelope.code,
            ErrorCode::Conflict,
            "a dedup-index collision must surface as 409 CONFLICT, never a raw 500"
        );
    }

    #[test]
    fn create_gdpr_request_body_rejects_an_unknown_field() {
        let json = serde_json::json!({ "customerId": Uuid::new_v4(), "extra": "nope" });
        assert!(serde_json::from_value::<CreateGdprRequestBody>(json).is_err());
    }

    // ── op #2 list_gdpr_requests / masking ──

    #[tokio::test]
    async fn list_masks_the_customer_id_never_returns_it_raw() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let customer_id = Uuid::new_v4();
        let repo = FakeGdprRepo::default();
        repo.seed_request(
            Uuid::new_v4(),
            fake::FakeRequest {
                location_id: loc,
                customer_id: Some(customer_id),
                status: "pending".to_string(),
                completed_at: None,
                requested_at: Utc::now(),
            },
        );
        let state = state_with(repo, owner_with_location(user_id, loc));
        let owner = OwnerClaims::new(user_id, Some(loc));

        let response = list_gdpr_requests(
            Extension(state),
            OwnerClaimsExt(owner),
            Path(loc),
            Query(ListGdprRequestsQuery {
                status: None,
                limit: None,
                cursor: None,
            }),
            Extension(request_id()),
        )
        .await
        .unwrap()
        .into_response();

        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: ListGdprRequestsResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body.requests.len(), 1);
        let masked = body.requests[0].customer_id.as_ref().unwrap();
        assert_ne!(
            masked,
            &customer_id.to_string(),
            "the raw customer id must never appear on the wire"
        );
        assert!(masked.ends_with("***"));
        assert_eq!(masked.len(), 4, "maskName: first char + \"***\"");
    }

    #[tokio::test]
    async fn list_404_for_a_foreign_location() {
        let user_id = Uuid::new_v4();
        let mine = Uuid::new_v4();
        let theirs = Uuid::new_v4();
        let state = state_with(FakeGdprRepo::default(), owner_with_location(user_id, mine));
        let owner = OwnerClaims::new(user_id, Some(mine));

        let err = crate::error::expect_err(
            list_gdpr_requests(
                Extension(state),
                OwnerClaimsExt(owner),
                Path(theirs),
                Query(ListGdprRequestsQuery {
                    status: None,
                    limit: None,
                    cursor: None,
                }),
                Extension(request_id()),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    // ── op #3 get_gdpr_request ──

    #[tokio::test]
    async fn get_404_when_row_does_not_exist() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = state_with(FakeGdprRepo::default(), owner_with_location(user_id, loc));
        let owner = OwnerClaims::new(user_id, Some(loc));

        let err = crate::error::expect_err(
            get_gdpr_request(
                Extension(state),
                OwnerClaimsExt(owner),
                Path((loc, Uuid::new_v4())),
                Extension(request_id()),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    // ── ops #4/#5 retention ──

    #[tokio::test]
    async fn retention_get_then_put_round_trips() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = state_with(FakeGdprRepo::default(), owner_with_location(user_id, loc));
        let owner = OwnerClaims::new(user_id, Some(loc));

        let put_response = put_retention_settings(
            Extension(state.clone()),
            OwnerClaimsExt(owner.clone()),
            Path(loc),
            Extension(request_id()),
            Json(UpdateRetentionRequest { retention_days: 90 }),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(put_response.status(), StatusCode::OK);

        let get_response = get_retention_settings(
            Extension(state),
            OwnerClaimsExt(owner),
            Path(loc),
            Extension(request_id()),
        )
        .await
        .unwrap()
        .into_response();
        let bytes = axum::body::to_bytes(get_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: RetentionResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body.retention_days, 90);
    }

    #[tokio::test]
    async fn retention_defaults_to_365_when_unset() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = state_with(FakeGdprRepo::default(), owner_with_location(user_id, loc));
        let owner = OwnerClaims::new(user_id, Some(loc));

        let response = get_retention_settings(
            Extension(state),
            OwnerClaimsExt(owner),
            Path(loc),
            Extension(request_id()),
        )
        .await
        .unwrap()
        .into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: RetentionResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            body.retention_days, 365,
            "REV-S9-7/Q5: the defensible SMB default, never silently the 2555 max"
        );
    }

    #[test]
    fn update_retention_request_rejects_an_unknown_field() {
        let json = serde_json::json!({ "retentionDays": 90, "extra": "nope" });
        assert!(serde_json::from_value::<UpdateRetentionRequest>(json).is_err());
    }

    // ── cursor codec ──

    #[test]
    fn cursor_round_trips() {
        let now = Utc::now();
        let encoded = encode_cursor(now);
        let decoded = decode_cursor(&encoded).unwrap();
        // rfc3339 round-trip loses sub-microsecond precision on some platforms; compare at the
        // millisecond grain, matching what the wire format actually carries.
        assert_eq!(decoded.timestamp_millis(), now.timestamp_millis());
    }

    #[test]
    fn decode_cursor_tolerates_garbage_by_returning_none() {
        assert_eq!(decode_cursor("not-valid-base64url!!!"), None);
        assert_eq!(
            decode_cursor(&base64::Engine::encode(
                &base64::engine::general_purpose::URL_SAFE_NO_PAD,
                "{}"
            )),
            None,
            "valid base64/JSON but missing requestedAt must still tolerate, not panic"
        );
    }

    #[test]
    fn mask_name_is_first_char_plus_stars() {
        let id = Uuid::parse_str("3f2b1a90-0000-0000-0000-000000000000").unwrap();
        assert_eq!(mask_name(&id), "3***");
    }
}
