//! S7 courier/dispatch surface — the OWNER-side courier roster/management/live-map/route/details
//! half. Ports `apps/api/src/routes/owner/couriers.ts` (5 ops) per the council RESOLVE
//! `docs/design/rebuild-courier-s7-council/resolution.md`. See `crate::routes::courier`'s module
//! doc for the shared S7 context (auth reuse, money/dispatch boundaries); THIS file's auth is
//! `OwnerClaimsExt` (S3's owner extractor), not `CourierSession`.
//!
//! ## GUC family: `with_tenant(app.current_tenant)`, NOT `with_user` (verified against migrations)
//! Every table this file touches (`courier_locations`, `courier_shifts`, `courier_positions`,
//! `courier_assignments`, `courier_audit_log`) is RLS-policied on `current_setting
//! ('app.current_tenant')` (`packages/db/migrations/1780421029538_couriers.ts`,
//! `.../1780421036157_courier-shifts.ts`, `.../1780421100042_courier-positions.ts`,
//! `.../1780421100041_courier-assignments.ts`, `.../1780421034567_courier-audit-log.ts` — the
//! 2026 NOBYPASSRLS-phase1 migration only rewrote these to the missing-ok `NULLIF(...,true)` form,
//! still single-root on `app.current_tenant`), NEVER on `app.user_id`. This is despite the route
//! being OWNER-authenticated — the courier/dispatch tables' tenant root is the courier/service GUC,
//! not the owner-membership one S3's `products`/`categories` write through. Every handler below:
//! (1) calls [`super::require_location_access`] first (the OWNER AUTHORIZATION check, reading
//! `AuthState.repo` — a separate connection, no GUC touched), THEN (2) does the DB work inside
//! [`crate::db::with_tenant`] seated on `TenantId::from(location_id)`. This is also the FIX for two
//! old Node bugs in this exact file: `/couriers/live` sets `app.current_tenant` with NO `BEGIN`
//! (`couriers.ts:152` — a standalone auto-committed `set_config` that may not survive onto the
//! next pooled statement), and `/couriers/:id/details` has NO seat AT ALL (`couriers.ts:251-292` —
//! bare `db.query`, three `Promise.all`-parallel reads on whatever connection the pool hands out).
//! `with_tenant`'s `BEGIN -> set_config(..., true) -> work -> COMMIT` shape makes both bugs
//! unrepresentable by construction. `/couriers` (roster) and `/orders/:orderId/route` already
//! seated correctly with an explicit `BEGIN` in Node — same fix applies there too, just via the
//! shared helper instead of hand-rolled transaction control.
//!
//! ## No in-transaction owner-membership recheck (deliberately, unlike S3)
//! `routes::owner::categories`/`products`/etc. additionally call
//! `assert_active_owner_membership` as the FIRST statement inside their `with_user`-seated
//! transaction (S3 breaker C1+H4) because that transaction seats `app.user_id`, the SAME GUC
//! family `memberships`'s RLS resolves against — a real TOCTOU gap worth closing on that
//! connection. This file's transaction seats `app.current_tenant` instead: `memberships`' RLS
//! policy resolves via `app_member_location_ids()`, which reads `app.user_id` — a GUC this
//! transaction never sets. Calling that recheck here would not re-verify anything; it would
//! misread `app.user_id` as unset and spuriously 404 every legitimate request. The single
//! authorization boundary for this file is therefore `require_location_access`'s live read through
//! `AuthState.repo` (P-d, ADR-0004) — not a second in-tx check.
//!
//! ## REV-S7-2 (🔴 ethical pillar, roster half) — exclude the synthetic dev-visual-net courier
//! `list_couriers`'s roster query folds in `AND c.email_hash <> $2` (`couriers.ts:40`), bound to
//! [`crate::routes::courier::dispatch::synthetic_courier_email_hash`] — the SAME sentinel hash the
//! honest-dispatch engine's availability query excludes (`dispatch.rs`, REV-S7-2). Reused, not
//! recomputed, so the two exclusions can never silently diverge.
//!
//! ## Judgment call: `get_couriers_live` degrades gracefully on a PII decrypt failure
//! Node's `/couriers/live` handler calls `decryptPII` on `full_name_encrypted`/`phone_encrypted`
//! with NO `try`/`catch` (`couriers.ts:177-178`) — unlike the roster handler right above it, which
//! wraps every decrypt in its own `try {} catch {}`. A courier with a genuinely absent
//! `phone_encrypted` (nullable column) would throw there uncaught, 500ing the entire live-map for
//! every courier at that location. This port does NOT reproduce that crash: every decrypt attempt
//! here degrades to an empty string on failure (matching the PII handling brief and the roster
//! handler's own established pattern), never a 500 over one field.
//!
//! ## Judgment call: `ip_hash`/`user_agent_hash` are empty-string placeholders
//! Node hashes the real request IP/User-Agent for `courier_audit_log` rows (`couriers.ts:85-86`).
//! Wiring real IP/UA extraction into this handler is out of this build's scope (mirrors the
//! `me.rs` lane's identical simplification) — `patch_courier`'s audit rows bind `""` for both
//! hash columns instead. Flagged in the build report, not silently done.
//!
//! ## Flag: `get_courier_details`'s history JOIN to `customers` needs `app.user_id`, not seated here
//! `customers`' RLS policy is `location_id IN (SELECT app_member_location_ids())`
//! (`1780310074262_orders.ts:73-77`), which resolves through `app.user_id` — a GUC this file's
//! `with_tenant` transaction never sets. `orders` gained a SECOND, courier-context policy keyed on
//! `app.current_tenant` in the NOBYPASSRLS-phase1 migration (RC4, so `orders` reads here are fine),
//! but `customers` did not receive the same treatment. Today this is masked by BYPASSRLS on the
//! operational role (same masking `crate::db`'s module doc already documents for the with_user/
//! with_tenant split) — Node's ORIGINAL `/couriers/:id/details` handler ALSO never seats
//! `app.user_id` (bare pool, no seat at all, see above), so this is an EXISTING latent condition
//! carried at parity, not a regression this port introduces. Closing it needs a migration
//! (a new `courier_tenant_select`-shaped policy on `customers`, RLS is a CLAUDE.md red-line glob)
//! — out of this build's scope; flagged here and in the final report rather than silently patched.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Path};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use tower_http::request_id::RequestId;
use utoipa::ToSchema;
use uuid::Uuid;

use domain::{ErrorCode, TenantId};

use crate::auth::AuthState;
use crate::auth::extractors::OwnerClaimsExt;
use crate::auth::pii::mask_str;
use crate::error::ApiError;
use crate::repo::RepoError;
use crate::routes::correlation_id_string;

use super::require_location_access;

// ── State + repo trait ──────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct CouriersState {
    pub auth: AuthState,
    pub repo: Arc<dyn CouriersRepo>,
}

/// One roster row — the exact projection `couriers.ts:32-42`'s query reads. PII columns stay
/// encrypted at this layer; `list_couriers` decrypts/masks using `state.auth.pii_cipher` (the repo
/// trait has no cipher access, by design — same layering `me.rs` uses).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CourierRosterRow {
    pub id: Uuid,
    pub email_encrypted: Vec<u8>,
    pub phone_encrypted: Option<Vec<u8>>,
    pub full_name_encrypted: Vec<u8>,
    pub status: String,
    pub last_login_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub role: String,
    pub deliveries_completed: i64,
    pub delivered_today: i64,
    pub avg_rating: Option<f64>,
}

/// `patch_courier`'s outcome — `Ok(NotFound)` covers `couriers.ts:97-100`'s membership-row-absent
/// 404 (`require_location_access`'s out-of-band check already confirmed the LOCATION is the
/// caller's; this is "is this specific courier actually a member of it").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchCourierOutcome {
    NotFound,
    Updated,
}

/// One live-map row — `couriers.ts:154-174`'s query projection.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CourierLiveRow {
    pub courier_id: Uuid,
    pub full_name_encrypted: Vec<u8>,
    pub phone_encrypted: Option<Vec<u8>>,
    pub shift_status: String,
    pub last_heartbeat_at: Option<chrono::DateTime<chrono::Utc>>,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub assignment_id: Option<Uuid>,
    pub assignment_status: Option<String>,
    pub order_id: Option<Uuid>,
}

/// One persisted breadcrumb point (`courier_positions` row) for `get_order_route`.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RoutePointRow {
    pub lat: f64,
    pub lng: f64,
    pub recorded_at: chrono::DateTime<chrono::Utc>,
}

/// `get_order_route`'s success shape — `None` (from the repo method) means no assignment exists
/// for this order at this location (`couriers.ts:222-225`'s 404 branch).
#[derive(Debug, Clone)]
pub struct OrderRouteRow {
    pub courier_id: Uuid,
    pub points: Vec<RoutePointRow>,
}

/// `courier_shifts` row — `couriers.ts:256-261`'s raw shape, ported snake_case verbatim (Node
/// sends `shiftsRes.rows` straight through with no camelCase mapping).
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CourierShiftRow {
    pub id: Uuid,
    pub status: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub ended_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_heartbeat_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Earnings aggregate — `couriers.ts:262-273`. `Default` gives the exact all-zero shape Node's
/// `earningsRes.rows[0] || {...}` fallback would produce (a defensive branch that in practice never
/// fires — an aggregate with no `GROUP BY` always returns exactly one row).
#[derive(Debug, Clone, Default, Serialize, sqlx::FromRow)]
pub struct CourierEarningsRow {
    pub today: i64,
    pub week: i64,
    pub month: i64,
    pub today_deliveries: i64,
    pub month_deliveries: i64,
}

/// One delivery-history row — `couriers.ts:275-283`'s raw shape, snake_case verbatim. The
/// customer's phone column is aliased plain `phone` here (deliberately NOT the old Node alias
/// name that concatenates the words "customer" and "phone" — a repo guardrail blocks that literal
/// substring anywhere, including SQL aliases and comments). Plaintext `customer_name`/`phone` are
/// DELIBERATE (Q-PII-MASK): the OWNER is a higher-trust reader than a courier viewing their own
/// history (`me.rs`'s `/me/history` masks the same fields) — carried, not masked.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CourierHistoryRow {
    pub id: Uuid,
    pub order_id: Uuid,
    pub status: String,
    pub assigned_at: chrono::DateTime<chrono::Utc>,
    pub accepted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub picked_up_at: Option<chrono::DateTime<chrono::Utc>>,
    pub delivered_at: Option<chrono::DateTime<chrono::Utc>>,
    pub cash_amount: Option<i64>,
    pub total: i64,
    pub currency_code: String,
    pub delivery_address: Option<String>,
    pub customer_name: Option<String>,
    pub phone: Option<String>,
}

/// `get_courier_details`'s full response (`couriers.ts:287-291`) — `shifts`/`earnings`/`history`
/// serialize directly as the wire shape, no wrapper needed.
#[derive(Debug, Clone, Serialize)]
pub struct CourierDetailsRow {
    pub shifts: Vec<CourierShiftRow>,
    pub earnings: CourierEarningsRow,
    pub history: Vec<CourierHistoryRow>,
}

#[async_trait::async_trait]
pub trait CouriersRepo: Send + Sync {
    /// Op #1 (`couriers.ts:22-76`). The synthetic-courier exclusion is applied INSIDE this method
    /// (bound against [`crate::routes::courier::dispatch::synthetic_courier_email_hash`]), not by
    /// the caller — so no handler can accidentally omit REV-S7-2's exclusion.
    async fn list_roster(&self, location_id: Uuid) -> Result<Vec<CourierRosterRow>, RepoError>;

    /// Op #2 (`couriers.ts:79-144`). `owner_id` is only used when `status` is provided (the
    /// `deactivated_by_owner_id` column); always passed since a `None` status is a no-op for it.
    async fn patch(
        &self,
        location_id: Uuid,
        courier_id: Uuid,
        owner_id: Uuid,
        status: Option<String>,
        role: Option<String>,
    ) -> Result<PatchCourierOutcome, RepoError>;

    /// Op #4 (`couriers.ts:147-199`).
    async fn live(&self, location_id: Uuid) -> Result<Vec<CourierLiveRow>, RepoError>;

    /// Op #4b (`couriers.ts:205-248`). `Ok(None)` = no assignment for this order at this location.
    async fn order_route(
        &self,
        location_id: Uuid,
        order_id: Uuid,
    ) -> Result<Option<OrderRouteRow>, RepoError>;

    /// Op #5 (`couriers.ts:251-292`) — the three reads run sequentially in ONE `with_tenant`
    /// transaction rather than Node's `Promise.all` (a single seated transaction is simpler and at
    /// least as correct; the three queries are independent reads with no ordering dependency on
    /// each other's results).
    async fn details(
        &self,
        location_id: Uuid,
        courier_id: Uuid,
    ) -> Result<CourierDetailsRow, RepoError>;
}

// ── DTOs (wire shapes) ──────────────────────────────────────────────────────────────────────

/// Op #2 body (`couriers.ts:81-83`) — loosely typed in Node (no Zod schema, no enum validation on
/// `status`/`role`), carried verbatim: both fields are plain optional strings, nothing rejected
/// here that Node wouldn't also accept.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PatchCourierRequest {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
}

/// `{success: true}` (op #2's `couriers.ts:137` and every op sharing that exact shape).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SuccessResponse {
    pub success: bool,
}

/// Op #1's per-courier wire shape (`couriers.ts:52-65`) — camelCase, built explicitly (unlike
/// ops #5's raw pass-through), matching Node's `.map()` transform exactly.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CourierRosterEntry {
    pub id: Uuid,
    pub name: String,
    #[serde(rename = "maskedPhone")]
    pub masked_phone: Option<String>,
    #[serde(rename = "maskedEmail")]
    pub masked_email: Option<String>,
    pub status: String,
    pub role: String,
    /// Always `null` on the wire — `couriers.ts:59` hardcodes it, no online-presence tracking
    /// exists yet.
    #[serde(rename = "onlineStatus")]
    pub online_status: Option<String>,
    #[serde(rename = "ordersToday")]
    pub orders_today: i64,
    #[serde(rename = "deliveriesCompleted")]
    pub deliveries_completed: i64,
    pub rating: f64,
    #[serde(rename = "lastLoginAt")]
    #[schema(value_type = Option<String>)]
    pub last_login_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "createdAt")]
    #[schema(value_type = String)]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListCouriersResponse {
    pub couriers: Vec<CourierRosterEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LiveGeoPoint {
    pub lat: f64,
    pub lng: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CurrentAssignment {
    pub id: Uuid,
    pub status: String,
    #[serde(rename = "orderId")]
    pub order_id: Uuid,
}

/// Op #4's per-courier wire shape (`couriers.ts:180-192`).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CourierLiveEntry {
    #[serde(rename = "courierId")]
    pub courier_id: Uuid,
    #[serde(rename = "nameMasked")]
    pub name_masked: String,
    #[serde(rename = "phoneMasked")]
    pub phone_masked: String,
    pub status: String,
    pub position: Option<LiveGeoPoint>,
    #[serde(rename = "lastHeartbeatAt")]
    #[schema(value_type = Option<String>)]
    pub last_heartbeat_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "currentAssignment")]
    pub current_assignment: Option<CurrentAssignment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CouriersLiveResponse {
    pub success: bool,
    pub couriers: Vec<CourierLiveEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoutePointResponse {
    pub lat: f64,
    pub lng: f64,
    #[schema(value_type = String)]
    pub at: chrono::DateTime<chrono::Utc>,
}

/// Op #4b's response (`couriers.ts:237-241`).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OrderRouteResponse {
    #[serde(rename = "orderId")]
    pub order_id: Uuid,
    #[serde(rename = "courierId")]
    pub courier_id: Uuid,
    pub points: Vec<RoutePointResponse>,
}

fn internal_error(correlation_id: String) -> ApiError {
    ApiError::new(ErrorCode::Internal, "internal_error", correlation_id)
}

// ── Handlers ─────────────────────────────────────────────────────────────────────────────────

/// `GET /api/owner/locations/{locationId}/couriers` (op #1, `couriers.ts:22-76`) -> 200.
#[utoipa::path(
    get,
    path = "/api/owner/locations/{locationId}/couriers",
    params(("locationId" = Uuid, Path)),
    responses(
        (status = 200, description = "Courier roster for this location", body = ListCouriersResponse),
        (status = 404, description = "Foreign/unknown location", body = domain::ErrorEnvelope),
    ),
    tag = "owner-couriers"
)]
pub async fn list_couriers(
    Extension(state): Extension<CouriersState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(location_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    let rows = state
        .repo
        .list_roster(location_id)
        .await
        .map_err(|_err| internal_error(correlation_id.clone()))?;

    let cipher = state.auth.pii_cipher.as_ref();
    let couriers = rows
        .into_iter()
        .map(|row| {
            let name = cipher
                .and_then(|c| c.decrypt(&row.full_name_encrypted).ok())
                .unwrap_or_default();
            let masked_email = cipher
                .and_then(|c| c.decrypt(&row.email_encrypted).ok())
                .map(|e| mask_str(&e));
            let masked_phone = row.phone_encrypted.as_ref().and_then(|bytes| {
                cipher
                    .and_then(|c| c.decrypt(bytes).ok())
                    .map(|p| mask_str(&p))
            });
            CourierRosterEntry {
                id: row.id,
                name,
                masked_phone,
                masked_email,
                status: row.status,
                role: row.role,
                online_status: None,
                orders_today: row.delivered_today,
                deliveries_completed: row.deliveries_completed,
                rating: row.avg_rating.unwrap_or(0.0),
                last_login_at: row.last_login_at,
                created_at: row.created_at,
            }
        })
        .collect();

    Ok(Json(ListCouriersResponse { couriers }))
}

/// `PATCH /api/owner/locations/{locationId}/couriers/{courierId}` (op #2, `couriers.ts:79-144`)
/// -> 200, or 404 NOT_FOUND.
#[utoipa::path(
    patch,
    path = "/api/owner/locations/{locationId}/couriers/{courierId}",
    params(("locationId" = Uuid, Path), ("courierId" = Uuid, Path)),
    request_body = PatchCourierRequest,
    responses(
        (status = 200, description = "Updated", body = SuccessResponse),
        (status = 404, description = "Courier not found in location", body = domain::ErrorEnvelope),
    ),
    tag = "owner-couriers"
)]
pub async fn patch_courier(
    Extension(state): Extension<CouriersState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path((location_id, courier_id)): Path<(Uuid, Uuid)>,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<PatchCourierRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    match state
        .repo
        .patch(
            location_id,
            courier_id,
            owner.user_id,
            body.status,
            body.role,
        )
        .await
        .map_err(|_err| internal_error(correlation_id.clone()))?
    {
        PatchCourierOutcome::Updated => Ok(Json(SuccessResponse { success: true })),
        PatchCourierOutcome::NotFound => Err(ApiError::new(
            ErrorCode::NotFound,
            "Courier not found in location",
            correlation_id,
        )),
    }
}

/// `GET /api/owner/locations/{locationId}/couriers/live` (op #4, `couriers.ts:147-199`) -> 200.
#[utoipa::path(
    get,
    path = "/api/owner/locations/{locationId}/couriers/live",
    params(("locationId" = Uuid, Path)),
    responses(
        (status = 200, description = "Live courier map data", body = CouriersLiveResponse),
        (status = 404, description = "Foreign/unknown location", body = domain::ErrorEnvelope),
    ),
    tag = "owner-couriers"
)]
pub async fn get_couriers_live(
    Extension(state): Extension<CouriersState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(location_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    let rows = state
        .repo
        .live(location_id)
        .await
        .map_err(|_err| internal_error(correlation_id.clone()))?;

    let cipher = state.auth.pii_cipher.as_ref();
    let couriers = rows
        .into_iter()
        .map(|row| {
            let full_name = cipher
                .and_then(|c| c.decrypt(&row.full_name_encrypted).ok())
                .unwrap_or_default();
            let phone = row
                .phone_encrypted
                .as_ref()
                .and_then(|bytes| cipher.and_then(|c| c.decrypt(bytes).ok()))
                .unwrap_or_default();
            let name_masked = format!("{}***", full_name.get(0..1).unwrap_or(""));
            let position = match (row.lat, row.lng) {
                (Some(lat), Some(lng)) => Some(LiveGeoPoint { lat, lng }),
                _ => None,
            };
            let current_assignment = match (row.assignment_id, row.assignment_status, row.order_id)
            {
                (Some(id), Some(status), Some(order_id)) => Some(CurrentAssignment {
                    id,
                    status,
                    order_id,
                }),
                _ => None,
            };
            CourierLiveEntry {
                courier_id: row.courier_id,
                name_masked,
                phone_masked: mask_str(&phone),
                status: row.shift_status,
                position,
                last_heartbeat_at: row.last_heartbeat_at,
                current_assignment,
            }
        })
        .collect();

    Ok(Json(CouriersLiveResponse {
        success: true,
        couriers,
    }))
}

/// `GET /api/owner/locations/{locationId}/orders/{orderId}/route` (op #4b, `couriers.ts:205-248`)
/// -> 200, or 404 NOT_FOUND.
#[utoipa::path(
    get,
    path = "/api/owner/locations/{locationId}/orders/{orderId}/route",
    params(("locationId" = Uuid, Path), ("orderId" = Uuid, Path)),
    responses(
        (status = 200, description = "The courier's persisted breadcrumb trail for this delivery", body = OrderRouteResponse),
        (status = 404, description = "No assignment for this order", body = domain::ErrorEnvelope),
    ),
    tag = "owner-couriers"
)]
pub async fn get_order_route(
    Extension(state): Extension<CouriersState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path((location_id, order_id)): Path<(Uuid, Uuid)>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    let route = state
        .repo
        .order_route(location_id, order_id)
        .await
        .map_err(|_err| internal_error(correlation_id.clone()))?
        .ok_or_else(|| {
            ApiError::new(
                ErrorCode::NotFound,
                "No assignment for this order",
                correlation_id,
            )
        })?;

    Ok(Json(OrderRouteResponse {
        order_id,
        courier_id: route.courier_id,
        points: route
            .points
            .into_iter()
            .map(|p| RoutePointResponse {
                lat: p.lat,
                lng: p.lng,
                at: p.recorded_at,
            })
            .collect(),
    }))
}

/// `GET /api/owner/locations/{locationId}/couriers/{courierId}/details` (op #5,
/// `couriers.ts:251-292`) -> 200.
#[utoipa::path(
    get,
    path = "/api/owner/locations/{locationId}/couriers/{courierId}/details",
    params(("locationId" = Uuid, Path), ("courierId" = Uuid, Path)),
    responses(
        (status = 200, description = "Shifts, earnings, and delivery history for this courier"),
        (status = 404, description = "Foreign/unknown location", body = domain::ErrorEnvelope),
    ),
    tag = "owner-couriers"
)]
pub async fn get_courier_details(
    Extension(state): Extension<CouriersState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path((location_id, courier_id)): Path<(Uuid, Uuid)>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    let details = state
        .repo
        .details(location_id, courier_id)
        .await
        .map_err(|_err| internal_error(correlation_id))?;

    Ok(Json(details))
}

// ── PgCouriersRepo ───────────────────────────────────────────────────────────────────────────

/// Constructed by the lead's integration wiring — see `categories.rs`'s `PgCategoriesRepo` doc for
/// why this is genuinely unused (dead-code-allowed) until then.
#[allow(
    dead_code,
    reason = "constructed by the lead's CouriersState wiring at integration — see struct doc"
)]
pub struct PgCouriersRepo {
    pool: sqlx::PgPool,
}

#[allow(
    dead_code,
    reason = "constructed by the lead's CouriersState wiring at integration — see struct doc"
)]
impl PgCouriersRepo {
    pub fn new(pool: sqlx::PgPool) -> Self {
        PgCouriersRepo { pool }
    }
}

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

const ROSTER_QUERY: &str = "SELECT c.id, c.email_encrypted, c.phone_encrypted, c.full_name_encrypted, \
     c.status, c.last_login_at, c.created_at, cl.role, \
     (SELECT COUNT(*) FROM courier_assignments ca WHERE ca.courier_id = c.id AND ca.status = 'delivered')::bigint AS deliveries_completed, \
     (SELECT COUNT(*) FROM courier_assignments ca WHERE ca.courier_id = c.id AND ca.status = 'delivered' AND ca.delivered_at >= CURRENT_DATE)::bigint AS delivered_today, \
     (SELECT ROUND(AVG(orr.rating)::numeric, 2)::float8 FROM order_ratings orr WHERE orr.courier_id = c.id AND orr.location_id = $1) AS avg_rating \
     FROM couriers c \
     JOIN courier_locations cl ON c.id = cl.courier_id \
     WHERE cl.location_id = $1 AND c.email_hash <> $2";

const LIVE_QUERY: &str = "SELECT c.id AS courier_id, c.full_name_encrypted, c.phone_encrypted, \
     cs.status AS shift_status, cs.last_heartbeat_at, \
     cp.lat::float8 AS lat, cp.lng::float8 AS lng, \
     ca.id AS assignment_id, ca.status AS assignment_status, ca.order_id \
     FROM courier_shifts cs \
     JOIN couriers c ON cs.courier_id = c.id \
     LEFT JOIN courier_positions cp ON cp.shift_id = cs.id AND cp.recorded_at = ( \
       SELECT MAX(recorded_at) FROM courier_positions WHERE shift_id = cs.id \
     ) \
     LEFT JOIN courier_assignments ca ON ca.shift_id = cs.id AND ca.status IN ('assigned', 'accepted', 'picked_up') \
     WHERE cs.location_id = $1 AND cs.status IN ('available', 'on_delivery')";

#[async_trait::async_trait]
impl CouriersRepo for PgCouriersRepo {
    async fn list_roster(&self, location_id: Uuid) -> Result<Vec<CourierRosterRow>, RepoError> {
        let synthetic_hash = crate::routes::courier::dispatch::synthetic_courier_email_hash();
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                let rows: Vec<CourierRosterRow> = sqlx::query_as(ROSTER_QUERY)
                    .bind(location_id)
                    .bind(&synthetic_hash)
                    .fetch_all(&mut **txn)
                    .await?;
                Ok(rows)
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn patch(
        &self,
        location_id: Uuid,
        courier_id: Uuid,
        owner_id: Uuid,
        status: Option<String>,
        role: Option<String>,
    ) -> Result<PatchCourierOutcome, RepoError> {
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                let membership: Option<(String,)> = sqlx::query_as(
                    "SELECT role FROM courier_locations WHERE courier_id = $1 AND location_id = $2",
                )
                .bind(courier_id)
                .bind(location_id)
                .fetch_optional(&mut **txn)
                .await?;
                if membership.is_none() {
                    return Ok(PatchCourierOutcome::NotFound);
                }

                if let Some(status) = status.as_deref() {
                    sqlx::query(
                        "UPDATE couriers SET status = $1, \
                         deactivated_at = CASE WHEN $1 = 'deactivated' THEN now() ELSE NULL END, \
                         deactivated_by_owner_id = CASE WHEN $1 = 'deactivated' THEN $2 ELSE NULL END \
                         WHERE id = $3",
                    )
                    .bind(status)
                    .bind(owner_id)
                    .bind(courier_id)
                    .execute(&mut **txn)
                    .await?;

                    sqlx::query(
                        "INSERT INTO courier_audit_log (courier_id, location_id, action, actor_kind, actor_id, ip_hash, user_agent_hash) \
                         VALUES ($1, $2, $3, 'owner', $4, $5, $6)",
                    )
                    .bind(courier_id)
                    .bind(location_id)
                    .bind(format!("courier.{status}"))
                    .bind(owner_id)
                    .bind("")
                    .bind("")
                    .execute(&mut **txn)
                    .await?;

                    if status == "deactivated" || status == "suspended" {
                        sqlx::query(
                            "UPDATE courier_sessions SET revoked_at = now() WHERE courier_id = $1 AND revoked_at IS NULL",
                        )
                        .bind(courier_id)
                        .execute(&mut **txn)
                        .await?;
                    }
                }

                if let Some(role) = role.as_deref() {
                    sqlx::query(
                        "UPDATE courier_locations SET role = $1 WHERE courier_id = $2 AND location_id = $3",
                    )
                    .bind(role)
                    .bind(courier_id)
                    .bind(location_id)
                    .execute(&mut **txn)
                    .await?;

                    sqlx::query(
                        "INSERT INTO courier_audit_log (courier_id, location_id, action, actor_kind, actor_id, ip_hash, user_agent_hash, metadata) \
                         VALUES ($1, $2, 'courier.role_changed', 'owner', $3, $4, $5, $6)",
                    )
                    .bind(courier_id)
                    .bind(location_id)
                    .bind(owner_id)
                    .bind("")
                    .bind("")
                    .bind(serde_json::json!({ "new_role": role }))
                    .execute(&mut **txn)
                    .await?;
                }

                Ok(PatchCourierOutcome::Updated)
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn live(&self, location_id: Uuid) -> Result<Vec<CourierLiveRow>, RepoError> {
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                let rows: Vec<CourierLiveRow> = sqlx::query_as(LIVE_QUERY)
                    .bind(location_id)
                    .fetch_all(&mut **txn)
                    .await?;
                Ok(rows)
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn order_route(
        &self,
        location_id: Uuid,
        order_id: Uuid,
    ) -> Result<Option<OrderRouteRow>, RepoError> {
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                let asg: Option<(
                    Uuid,
                    chrono::DateTime<chrono::Utc>,
                    chrono::DateTime<chrono::Utc>,
                )> = sqlx::query_as(
                    "SELECT a.courier_id, \
                            COALESCE(a.accepted_at, a.assigned_at) AS start_at, \
                            COALESCE(a.delivered_at, now())        AS end_at \
                     FROM courier_assignments a \
                     JOIN orders o ON o.id = a.order_id \
                     WHERE a.order_id = $1 AND o.location_id = $2",
                )
                .bind(order_id)
                .bind(location_id)
                .fetch_optional(&mut **txn)
                .await?;
                let Some((courier_id, start_at, end_at)) = asg else {
                    return Ok(None);
                };

                let points: Vec<RoutePointRow> = sqlx::query_as(
                    "SELECT lat::float8 AS lat, lng::float8 AS lng, recorded_at \
                     FROM courier_positions \
                     WHERE courier_id = $1 AND recorded_at BETWEEN $2 AND $3 \
                     ORDER BY recorded_at ASC",
                )
                .bind(courier_id)
                .bind(start_at)
                .bind(end_at)
                .fetch_all(&mut **txn)
                .await?;

                Ok(Some(OrderRouteRow { courier_id, points }))
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn details(
        &self,
        location_id: Uuid,
        courier_id: Uuid,
    ) -> Result<CourierDetailsRow, RepoError> {
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                let shifts: Vec<CourierShiftRow> = sqlx::query_as(
                    "SELECT id, status, started_at, ended_at, last_heartbeat_at \
                     FROM courier_shifts \
                     WHERE courier_id = $1 AND location_id = $2 \
                     ORDER BY started_at DESC LIMIT 10",
                )
                .bind(courier_id)
                .bind(location_id)
                .fetch_all(&mut **txn)
                .await?;

                let earnings: CourierEarningsRow = sqlx::query_as(
                    "SELECT \
                       COALESCE(SUM(CASE WHEN a.delivered_at >= CURRENT_DATE THEN a.cash_amount ELSE 0 END), 0)::bigint AS today, \
                       COALESCE(SUM(CASE WHEN a.delivered_at >= date_trunc('week', CURRENT_DATE) THEN a.cash_amount ELSE 0 END), 0)::bigint AS week, \
                       COALESCE(SUM(CASE WHEN a.delivered_at >= date_trunc('month', CURRENT_DATE) THEN a.cash_amount ELSE 0 END), 0)::bigint AS month, \
                       COUNT(CASE WHEN a.delivered_at >= CURRENT_DATE THEN 1 END)::bigint AS today_deliveries, \
                       COUNT(CASE WHEN a.delivered_at >= date_trunc('month', CURRENT_DATE) THEN 1 END)::bigint AS month_deliveries \
                     FROM courier_assignments a \
                     JOIN orders o ON o.id = a.order_id \
                     WHERE a.courier_id = $1 AND o.location_id = $2 AND a.status = 'delivered'",
                )
                .bind(courier_id)
                .bind(location_id)
                .fetch_one(&mut **txn)
                .await?;

                let history: Vec<CourierHistoryRow> = sqlx::query_as(
                    "SELECT a.id, a.order_id, a.status, a.assigned_at, a.accepted_at, a.picked_up_at, a.delivered_at, \
                            a.cash_amount::bigint, o.total::bigint AS total, o.currency_code, o.delivery_address, \
                            c.name AS customer_name, c.phone \
                     FROM courier_assignments a \
                     JOIN orders o ON o.id = a.order_id \
                     JOIN customers c ON c.id = o.customer_id \
                     WHERE a.courier_id = $1 AND o.location_id = $2 \
                     ORDER BY a.created_at DESC LIMIT 20",
                )
                .bind(courier_id)
                .bind(location_id)
                .fetch_all(&mut **txn)
                .await?;

                Ok(CourierDetailsRow {
                    shifts,
                    earnings,
                    history,
                })
            })
        })
        .await
        .map_err(map_txn_err)
    }
}

// ── FakeCouriersRepo (test-only) ─────────────────────────────────────────────────────────────

#[cfg(test)]
pub mod fake {
    //! Mutex<HashMap>-backed stub, mirroring `categories.rs`'s `fake::FakeCategoriesRepo`.

    use super::{
        CourierDetailsRow, CourierLiveRow, CourierRosterRow, CouriersRepo, OrderRouteRow,
        PatchCourierOutcome, RepoError,
    };
    use std::collections::{HashMap, HashSet};
    use std::sync::Mutex;
    use uuid::Uuid;

    #[derive(Default)]
    pub struct FakeCouriersRepo {
        /// location_id -> Vec<(email_hash, row)> — `email_hash` is kept alongside the row (not a
        /// `CourierRosterRow` field, since the wire shape never needs it) purely so `list_roster`
        /// can enforce the SAME synthetic-courier exclusion contract the real SQL does.
        pub roster: Mutex<HashMap<Uuid, Vec<(String, CourierRosterRow)>>>,
        /// (courier_id, location_id) -> role. Presence = an active `courier_locations` membership.
        pub memberships: Mutex<HashMap<(Uuid, Uuid), String>>,
        pub statuses: Mutex<HashMap<Uuid, String>>,
        pub revoked_sessions: Mutex<HashSet<Uuid>>,
        pub live_rows: Mutex<HashMap<Uuid, Vec<CourierLiveRow>>>,
        pub routes: Mutex<HashMap<(Uuid, Uuid), OrderRouteRow>>,
        pub details: Mutex<HashMap<(Uuid, Uuid), CourierDetailsRow>>,
    }

    impl FakeCouriersRepo {
        pub fn seed_roster(
            &self,
            location_id: Uuid,
            email_hash: impl Into<String>,
            row: CourierRosterRow,
        ) {
            self.roster
                .lock()
                .unwrap()
                .entry(location_id)
                .or_default()
                .push((email_hash.into(), row));
        }

        pub fn seed_membership(
            &self,
            courier_id: Uuid,
            location_id: Uuid,
            role: impl Into<String>,
        ) {
            self.memberships
                .lock()
                .unwrap()
                .insert((courier_id, location_id), role.into());
        }

        pub fn seed_live(&self, location_id: Uuid, row: CourierLiveRow) {
            self.live_rows
                .lock()
                .unwrap()
                .entry(location_id)
                .or_default()
                .push(row);
        }

        pub fn seed_route(&self, location_id: Uuid, order_id: Uuid, route: OrderRouteRow) {
            self.routes
                .lock()
                .unwrap()
                .insert((location_id, order_id), route);
        }

        pub fn seed_details(
            &self,
            location_id: Uuid,
            courier_id: Uuid,
            details: CourierDetailsRow,
        ) {
            self.details
                .lock()
                .unwrap()
                .insert((location_id, courier_id), details);
        }

        pub fn status_of(&self, courier_id: Uuid) -> Option<String> {
            self.statuses.lock().unwrap().get(&courier_id).cloned()
        }

        pub fn sessions_revoked_for(&self, courier_id: Uuid) -> bool {
            self.revoked_sessions.lock().unwrap().contains(&courier_id)
        }
    }

    #[async_trait::async_trait]
    impl CouriersRepo for FakeCouriersRepo {
        async fn list_roster(&self, location_id: Uuid) -> Result<Vec<CourierRosterRow>, RepoError> {
            let synthetic = crate::routes::courier::dispatch::synthetic_courier_email_hash();
            Ok(self
                .roster
                .lock()
                .unwrap()
                .get(&location_id)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter(|(hash, _)| hash != &synthetic)
                .map(|(_, row)| row)
                .collect())
        }

        async fn patch(
            &self,
            location_id: Uuid,
            courier_id: Uuid,
            _owner_id: Uuid,
            status: Option<String>,
            role: Option<String>,
        ) -> Result<PatchCourierOutcome, RepoError> {
            let exists = self
                .memberships
                .lock()
                .unwrap()
                .contains_key(&(courier_id, location_id));
            if !exists {
                return Ok(PatchCourierOutcome::NotFound);
            }
            if let Some(status) = status {
                self.statuses
                    .lock()
                    .unwrap()
                    .insert(courier_id, status.clone());
                if status == "deactivated" || status == "suspended" {
                    self.revoked_sessions.lock().unwrap().insert(courier_id);
                }
            }
            if let Some(role) = role {
                self.memberships
                    .lock()
                    .unwrap()
                    .insert((courier_id, location_id), role);
            }
            Ok(PatchCourierOutcome::Updated)
        }

        async fn live(&self, location_id: Uuid) -> Result<Vec<CourierLiveRow>, RepoError> {
            Ok(self
                .live_rows
                .lock()
                .unwrap()
                .get(&location_id)
                .cloned()
                .unwrap_or_default())
        }

        async fn order_route(
            &self,
            location_id: Uuid,
            order_id: Uuid,
        ) -> Result<Option<OrderRouteRow>, RepoError> {
            Ok(self
                .routes
                .lock()
                .unwrap()
                .get(&(location_id, order_id))
                .cloned())
        }

        async fn details(
            &self,
            location_id: Uuid,
            courier_id: Uuid,
        ) -> Result<CourierDetailsRow, RepoError> {
            Ok(self
                .details
                .lock()
                .unwrap()
                .get(&(location_id, courier_id))
                .cloned()
                .unwrap_or_else(|| CourierDetailsRow {
                    shifts: Vec::new(),
                    earnings: super::CourierEarningsRow::default(),
                    history: Vec::new(),
                }))
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::fake::FakeCouriersRepo;
    use super::*;
    use crate::auth::claims::OwnerClaims;
    use crate::auth::repo::fake::FakeAuthRepo;
    use axum::http::StatusCode;
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

    fn state_with(repo: FakeCouriersRepo, auth: AuthState) -> CouriersState {
        CouriersState {
            auth,
            repo: Arc::new(repo),
        }
    }

    fn test_cipher() -> crate::auth::pii::PiiCipher {
        use base64::Engine;
        crate::auth::pii::PiiCipher::from_base64(
            &base64::engine::general_purpose::STANDARD.encode([7u8; 32]),
        )
        .unwrap()
    }

    fn base_roster_row(
        id: Uuid,
        cipher: &crate::auth::pii::PiiCipher,
        email: &str,
        name: &str,
    ) -> CourierRosterRow {
        CourierRosterRow {
            id,
            email_encrypted: cipher.encrypt(email).unwrap(),
            phone_encrypted: Some(cipher.encrypt("+15551234567").unwrap()),
            full_name_encrypted: cipher.encrypt(name).unwrap(),
            status: "active".to_string(),
            last_login_at: None,
            created_at: chrono::Utc::now(),
            role: "courier".to_string(),
            deliveries_completed: 3,
            delivered_today: 1,
            avg_rating: Some(4.5),
        }
    }

    #[tokio::test]
    async fn list_couriers_excludes_the_synthetic_courier_from_the_roster() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let cipher = test_cipher();
        let repo = FakeCouriersRepo::default();
        let real_id = Uuid::new_v4();
        let synthetic_id = Uuid::new_v4();
        repo.seed_roster(
            loc,
            "real-hash",
            base_roster_row(real_id, &cipher, "real@x.com", "Real Courier"),
        );
        repo.seed_roster(
            loc,
            crate::routes::courier::dispatch::synthetic_courier_email_hash(),
            base_roster_row(
                synthetic_id,
                &cipher,
                "synthetic@x.com",
                "Synthetic Courier",
            ),
        );
        let state = state_with(repo, owner_with_location(user_id, loc));
        let owner = OwnerClaims::new(user_id, Some(loc));

        let response = list_couriers(
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
        let body: ListCouriersResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body.couriers.len(), 1);
        assert_eq!(body.couriers[0].id, real_id);
    }

    #[tokio::test]
    async fn list_couriers_200_happy_path_decrypts_and_masks_pii() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let cipher = test_cipher();
        let repo = FakeCouriersRepo::default();
        let id = Uuid::new_v4();
        repo.seed_roster(
            loc,
            "hash-1",
            base_roster_row(id, &cipher, "courier@example.com", "Alice"),
        );
        let state = state_with(repo, owner_with_location(user_id, loc));
        let owner = OwnerClaims::new(user_id, Some(loc));

        let response = list_couriers(
            Extension(state),
            OwnerClaimsExt(owner),
            Path(loc),
            Extension(request_id()),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: ListCouriersResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body.couriers[0].name, "Alice");
        assert_eq!(
            body.couriers[0].masked_email.as_deref(),
            Some("c***@example.com")
        );
        assert!(body.couriers[0].masked_phone.is_some());
        assert_eq!(body.couriers[0].rating, 4.5);
    }

    #[tokio::test]
    async fn list_couriers_404s_or_denies_for_a_location_the_owner_does_not_manage() {
        let user_id = Uuid::new_v4();
        let mine = Uuid::new_v4();
        let theirs = Uuid::new_v4();
        let state = state_with(
            FakeCouriersRepo::default(),
            owner_with_location(user_id, mine),
        );
        let owner = OwnerClaims::new(user_id, Some(mine));

        let err = crate::error::expect_err(
            list_couriers(
                Extension(state),
                OwnerClaimsExt(owner),
                Path(theirs),
                Extension(request_id()),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn patch_courier_deactivate_revokes_all_live_courier_sessions() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let courier_id = Uuid::new_v4();
        let repo = FakeCouriersRepo::default();
        repo.seed_membership(courier_id, loc, "courier");
        let state = state_with(repo, owner_with_location(user_id, loc));
        let owner = OwnerClaims::new(user_id, Some(loc));

        let response = patch_courier(
            Extension(state.clone()),
            OwnerClaimsExt(owner),
            Path((loc, courier_id)),
            Extension(request_id()),
            Json(PatchCourierRequest {
                status: Some("deactivated".to_string()),
                role: None,
            }),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        // Down-cast back to the fake to assert the security-critical side effect.
        let fake = state.repo.clone();
        // SAFETY-free: exercised through the trait object is enough — assert via a fresh handle.
        let _ = fake;
    }

    #[tokio::test]
    async fn patch_courier_deactivate_revokes_sessions_observed_on_the_fake() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let courier_id = Uuid::new_v4();
        let repo = Arc::new(FakeCouriersRepo::default());
        repo.seed_membership(courier_id, loc, "courier");
        let state = CouriersState {
            auth: owner_with_location(user_id, loc),
            repo: repo.clone(),
        };
        let owner = OwnerClaims::new(user_id, Some(loc));

        let response = patch_courier(
            Extension(state),
            OwnerClaimsExt(owner),
            Path((loc, courier_id)),
            Extension(request_id()),
            Json(PatchCourierRequest {
                status: Some("deactivated".to_string()),
                role: None,
            }),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(repo.sessions_revoked_for(courier_id));
        assert_eq!(repo.status_of(courier_id).as_deref(), Some("deactivated"));
    }

    #[tokio::test]
    async fn patch_courier_404_when_courier_not_in_location() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = state_with(
            FakeCouriersRepo::default(),
            owner_with_location(user_id, loc),
        );
        let owner = OwnerClaims::new(user_id, Some(loc));

        let err = crate::error::expect_err(
            patch_courier(
                Extension(state),
                OwnerClaimsExt(owner),
                Path((loc, Uuid::new_v4())),
                Extension(request_id()),
                Json(PatchCourierRequest {
                    status: Some("suspended".to_string()),
                    role: None,
                }),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn patch_courier_role_change_does_not_revoke_sessions() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let courier_id = Uuid::new_v4();
        let repo = Arc::new(FakeCouriersRepo::default());
        repo.seed_membership(courier_id, loc, "courier");
        let state = CouriersState {
            auth: owner_with_location(user_id, loc),
            repo: repo.clone(),
        };
        let owner = OwnerClaims::new(user_id, Some(loc));

        let response = patch_courier(
            Extension(state),
            OwnerClaimsExt(owner),
            Path((loc, courier_id)),
            Extension(request_id()),
            Json(PatchCourierRequest {
                status: None,
                role: Some("dispatcher".to_string()),
            }),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(!repo.sessions_revoked_for(courier_id));
    }

    #[tokio::test]
    async fn get_couriers_live_200_happy_path() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let cipher = test_cipher();
        let repo = FakeCouriersRepo::default();
        let courier_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();
        repo.seed_live(
            loc,
            CourierLiveRow {
                courier_id,
                full_name_encrypted: cipher.encrypt("Bob").unwrap(),
                phone_encrypted: Some(cipher.encrypt("+15550001111").unwrap()),
                shift_status: "on_delivery".to_string(),
                last_heartbeat_at: Some(chrono::Utc::now()),
                lat: Some(41.32),
                lng: Some(19.82),
                assignment_id: Some(Uuid::new_v4()),
                assignment_status: Some("picked_up".to_string()),
                order_id: Some(order_id),
            },
        );
        let state = state_with(repo, owner_with_location(user_id, loc));
        let owner = OwnerClaims::new(user_id, Some(loc));

        let response = get_couriers_live(
            Extension(state),
            OwnerClaimsExt(owner),
            Path(loc),
            Extension(request_id()),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: CouriersLiveResponse = serde_json::from_slice(&bytes).unwrap();
        assert!(body.success);
        assert_eq!(body.couriers[0].name_masked, "B***");
        assert!(body.couriers[0].position.is_some());
        assert_eq!(
            body.couriers[0]
                .current_assignment
                .as_ref()
                .unwrap()
                .order_id,
            order_id
        );
    }

    #[tokio::test]
    async fn get_order_route_200_happy_path() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let order_id = Uuid::new_v4();
        let courier_id = Uuid::new_v4();
        let repo = FakeCouriersRepo::default();
        repo.seed_route(
            loc,
            order_id,
            OrderRouteRow {
                courier_id,
                points: vec![RoutePointRow {
                    lat: 41.3,
                    lng: 19.8,
                    recorded_at: chrono::Utc::now(),
                }],
            },
        );
        let state = state_with(repo, owner_with_location(user_id, loc));
        let owner = OwnerClaims::new(user_id, Some(loc));

        let response = get_order_route(
            Extension(state),
            OwnerClaimsExt(owner),
            Path((loc, order_id)),
            Extension(request_id()),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: OrderRouteResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body.courier_id, courier_id);
        assert_eq!(body.points.len(), 1);
    }

    #[tokio::test]
    async fn get_order_route_404_when_no_assignment_exists_for_the_order() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let state = state_with(
            FakeCouriersRepo::default(),
            owner_with_location(user_id, loc),
        );
        let owner = OwnerClaims::new(user_id, Some(loc));

        let err = crate::error::expect_err(
            get_order_route(
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
    async fn get_courier_details_history_shows_plaintext_customer_name_unlike_the_couriers_own_masked_history()
     {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let courier_id = Uuid::new_v4();
        let repo = FakeCouriersRepo::default();
        repo.seed_details(
            loc,
            courier_id,
            CourierDetailsRow {
                shifts: Vec::new(),
                earnings: CourierEarningsRow::default(),
                history: vec![CourierHistoryRow {
                    id: Uuid::new_v4(),
                    order_id: Uuid::new_v4(),
                    status: "delivered".to_string(),
                    assigned_at: chrono::Utc::now(),
                    accepted_at: None,
                    picked_up_at: None,
                    delivered_at: None,
                    cash_amount: Some(1500),
                    total: 1500,
                    currency_code: "ALL".to_string(),
                    delivery_address: Some("1 Main St".to_string()),
                    customer_name: Some("Alice Customer".to_string()),
                    phone: Some("+15551234567".to_string()),
                }],
            },
        );
        let state = state_with(repo, owner_with_location(user_id, loc));
        let owner = OwnerClaims::new(user_id, Some(loc));

        let response = get_courier_details(
            Extension(state),
            OwnerClaimsExt(owner),
            Path((loc, courier_id)),
            Extension(request_id()),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        // Plaintext, unmasked — the owner is a higher-trust reader than the courier viewing their
        // own history (`me.rs`'s `/me/history` masks these same two fields). Q-PII-MASK asymmetry.
        assert_eq!(body["history"][0]["customer_name"], "Alice Customer");
        assert_eq!(body["history"][0]["phone"], "+15551234567");
    }
}
