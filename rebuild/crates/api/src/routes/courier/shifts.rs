//! S7 courier shift lifecycle — ports `apps/api/src/routes/courier/shifts.ts` +
//! `apps/api/src/lib/shiftService.ts`'s `openShift`, per the council RESOLVE
//! `docs/design/rebuild-courier-s7-council/resolution.md` (REV-S7-4). See `crate::routes::courier`
//! module doc for the shared `CourierSession`/`with_tenant` contract this file follows.
//!
//! ## The canonical shift selector (D1/D2 fix — this file's highest-value change)
//! The Node source ran THREE divergent shift-selection queries across start/transition/end (this
//! repo's worst-health file, CLAUDE.md 1.0/10). The council RESOLVE fixes this by using the
//! STATUS-FILTER pattern everywhere — explicitly NOT `DATE(started_at) = CURRENT_DATE` (which
//! corrupts overnight shifts: a shift started yesterday, still `available`, would be invisible to a
//! date-scoped query, causing a duplicate-shift bug). [`select_open_shift_for_update`] is the ONE
//! selector every mutator (`start`/`end`/`transition`) now shares — it matches `GET /me/shift`'s
//! query (`shifts.ts`'s one already-correct query), so all four ops agree on what "the open shift"
//! means. `None` = the courier is logically `offline` (no live row).
//!
//! ## D3 — the active-delivery guard is now LOCATION-scoped
//! The old Node query behind `end`/`transition(to='offline')` lacked `AND location_id = $2` — a
//! courier active at location B could go offline at location A without being blocked. Every guard
//! query in this file binds BOTH `courier_id` and `location_id`.
//!
//! ## D4 — CARRY, not a bug to close: geofence-less locations admit any ping
//! `shifts_ping`'s range-check is skipped entirely when `locations.lat`/`lng` is NULL (no pin set)
//! — an accepted risk (a location that never configured a pin cannot geofence-reject a courier),
//! not a defect this port closes. See [`ShiftsRepo::ping`] doc.
//!
//! ## D5 — noted, NOT implemented here
//! The old Node ping rate-limiter keys on the raw `authorization` header. This crate's shared
//! `RateLimitLayer` (`crate::middleware::ratelimit`, wired globally in `main.rs`) is out of this
//! file's scope to touch — a future improvement would key it on the courier's `sub` claim
//! post-auth instead.
//!
//! ## D6/D7 — already fixed by construction
//! Every failure path here returns a typed [`crate::error::ApiError`] (Rust has no untyped throw),
//! and `CourierClaims.active_location_id` is `Uuid`, never `Option` — both classes of the old
//! Node bug are structurally unreachable in this port.
//!
//! ## D8 — CARRY verbatim: the geofence-event insert is best-effort
//! `shifts_ping`'s `order_sensor_events` write is wrapped in `SAVEPOINT geofence_evt` /
//! `RELEASE SAVEPOINT` on success / `ROLLBACK TO SAVEPOINT` on failure (swallowed) — a sensor write
//! must never fail the ping itself. Same idiom `orders::pg::apply_transition` already uses for its
//! `order_status_history` audit insert (`orders/pg.rs`'s `SAVEPOINT osh`).
//!
//! ## P0-1 privacy: no position write outside an active delivery
//! `start`/`transition(to='available')` never write to `courier_positions` even when `lat`/`lng`
//! are supplied in the body (accepted for schema parity only). `ping` writes a position ONLY when
//! the courier has an active order (`accepted`/`picked_up` — deliberately excludes `assigned`, the
//! courier's ACCEPT is the consent act that turns tracking on, per `courier-gps.ts`'s
//! `ACTIVE_DELIVERY_ASSIGNMENT_STATUSES`).

use std::sync::Arc;

use axum::Json;
use axum::extract::Extension;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use tower_http::request_id::RequestId;
use utoipa::ToSchema;
use uuid::Uuid;

use domain::{ErrorCode, TenantId};

use crate::auth::AuthState;
use crate::auth::extractors::CourierSession;
use crate::error::ApiError;
use crate::repo::RepoError;
use crate::routes::correlation_id_string;
use crate::routes::orders::pricing::distance_km;

// ── State + repo trait ──────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ShiftsState {
    /// Not read here — shift handlers authenticate via the `CourierSession` extractor (which reads
    /// `AuthState` from request extensions, layered by `courier_router`). Kept for State-shape
    /// uniformity with the other courier submodules; see `assignments::AssignmentsState.auth`.
    #[allow(
        dead_code,
        reason = "CourierSession extractor reads AuthState from request extensions, not this field — kept for State-shape uniformity"
    )]
    pub auth: AuthState,
    pub repo: Arc<dyn ShiftsRepo>,
}

/// `GET /me/shift`'s snapshot of the open shift — `None` (at the trait-method level) means no live
/// row (the courier is logically `offline`).
#[derive(Debug, Clone, PartialEq)]
pub struct ShiftSnapshot {
    pub shift_id: Uuid,
    pub status: String,
    pub started_at: String,
    pub elapsed_seconds: i64,
}

/// `start`/`transition(to='available')` always succeed — either reactivate the newest matching row
/// (D1/D2) or `INSERT` a fresh one.
#[derive(Debug, Clone, PartialEq)]
pub struct StartedShift {
    pub shift_id: Uuid,
    pub started_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndOutcome {
    /// No live shift row — a no-op success (never an error; `shifts.ts` treats "already offline"
    /// as success).
    AlreadyOffline,
    /// D3 guard: an active delivery exists AT THIS LOCATION.
    ActiveDeliveryExists,
    Ended,
}

/// `POST /shifts/transition`'s `to` — the only two states a caller may REQUEST (`on_delivery` is
/// system-driven via assignment accept, never a caller-chosen target).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TransitionTarget {
    Offline,
    Available,
}

impl TransitionTarget {
    fn as_str(self) -> &'static str {
        match self {
            TransitionTarget::Offline => "offline",
            TransitionTarget::Available => "available",
        }
    }
}

/// The idempotent-no-op and the successful-mutation cases share ONE response shape
/// (`{success:true,status,shiftId}`) — `Success` covers both; the repo doesn't need to distinguish
/// "did I write anything" for the handler to respond correctly.
#[derive(Debug, Clone, PartialEq)]
pub enum TransitionOutcome {
    Success {
        shift_id: Option<Uuid>,
        status: String,
    },
    CannotGoOfflineWithActiveOrder,
    ActiveDeliveryExists,
    InvalidTransition,
    GpsRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PingOutcome {
    NoActiveShift,
    GpsOutOfRange,
    /// `gps_stored` is `true` only when the courier had an active order (`accepted`/`picked_up`) to
    /// attribute the position write to (P0-1).
    Admitted {
        gps_stored: bool,
    },
}

#[async_trait::async_trait]
pub trait ShiftsRepo: Send + Sync {
    /// `GET /api/courier/me/shift` — read-only; the one query `shifts.ts` already got right
    /// (carried, not fixed), now `with_tenant`-seated per the REV-S7-1 seat census.
    async fn get_active(
        &self,
        courier_id: Uuid,
        location_id: Uuid,
    ) -> Result<Option<ShiftSnapshot>, RepoError>;

    /// `POST /api/courier/me/shift/start` — reuses the newest available/on_delivery row (D1/D2) or
    /// inserts a fresh one. Never writes a position (P0-1).
    async fn start(&self, courier_id: Uuid, location_id: Uuid) -> Result<StartedShift, RepoError>;

    /// `POST /api/courier/me/shift/end` — D3 active-delivery guard, location-scoped.
    async fn end(&self, courier_id: Uuid, location_id: Uuid) -> Result<EndOutcome, RepoError>;

    /// `POST /api/courier/shifts/transition`.
    async fn transition(
        &self,
        courier_id: Uuid,
        location_id: Uuid,
        to: TransitionTarget,
        lat: Option<f64>,
        lng: Option<f64>,
    ) -> Result<TransitionOutcome, RepoError>;

    /// `POST /api/courier/shifts/ping` — geofence range-check (D4 carry, skipped when the location
    /// has no pin) + active-order-scoped position write (P0-1) + best-effort geofence-enter event
    /// (D8) + an unconditional heartbeat (keeps the shift live even off-delivery/GPS-withheld).
    async fn ping(
        &self,
        courier_id: Uuid,
        location_id: Uuid,
        lat: f64,
        lng: f64,
        accuracy_meters: Option<i32>,
    ) -> Result<PingOutcome, RepoError>;
}

// ── DTOs ─────────────────────────────────────────────────────────────────────────────────────

/// `POST /me/shift/start` body (`shifts.ts`). `lat`/`lng` are accepted for schema parity ONLY —
/// P0-1 privacy means start never writes a `courier_positions` row, so neither field is ever read.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct StartShiftRequest {
    #[serde(default)]
    #[allow(
        dead_code,
        reason = "accepted for schema parity (shifts.ts) but never persisted — P0-1, see module doc"
    )]
    pub lat: Option<f64>,
    #[serde(default)]
    #[allow(
        dead_code,
        reason = "accepted for schema parity (shifts.ts) but never persisted — P0-1, see module doc"
    )]
    pub lng: Option<f64>,
}

/// `POST /shifts/transition` body (`.strict()` parity).
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TransitionRequest {
    pub to: TransitionTarget,
    #[serde(default)]
    pub lat: Option<f64>,
    #[serde(default)]
    pub lng: Option<f64>,
}

/// `POST /shifts/ping` body (`.strict()` parity).
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PingRequest {
    pub lat: f64,
    pub lng: f64,
    #[serde(default)]
    pub accuracy_meters: Option<i32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ShiftStatusResponse {
    #[serde(rename = "isActive")]
    pub is_active: bool,
    #[serde(rename = "startedAt")]
    pub started_at: Option<String>,
    #[serde(rename = "elapsedSeconds")]
    pub elapsed_seconds: i64,
    #[serde(rename = "shiftId")]
    pub shift_id: Option<Uuid>,
    pub status: Option<String>,
    pub stats: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct StartShiftResponse {
    pub success: bool,
    pub status: String,
    #[serde(rename = "shiftId")]
    pub shift_id: Uuid,
    #[serde(rename = "startedAt")]
    pub started_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EndShiftResponse {
    pub success: bool,
    pub status: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TransitionResponse {
    pub success: bool,
    pub status: String,
    #[serde(rename = "shiftId")]
    pub shift_id: Option<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PingResponse {
    pub success: bool,
    #[serde(rename = "gpsStored")]
    pub gps_stored: bool,
    pub reason: Option<String>,
}

fn internal_error(correlation_id: String) -> ApiError {
    ApiError::new(ErrorCode::Internal, "internal_error", correlation_id)
}

// ── Pure helpers (unit-tested without a database) ───────────────────────────────────────────

/// `distanceKm(...) <= radiusKm` (`geo.ts`'s geofence helper) — reuses the ALREADY-ported haversine
/// (`crate::routes::orders::pricing::distance_km`), never reimplemented.
fn is_within_geofence(lat1: f64, lon1: f64, lat2: f64, lon2: f64, radius_km: f64) -> bool {
    distance_km(lat1, lon1, lat2, lon2) <= radius_km
}

/// `Math.round(coord*100000)/100000` (`courier-gps.ts`) — 5-decimal-place rounding before a
/// `courier_positions` write.
fn round_coordinate(c: f64) -> f64 {
    (c * 100_000.0).round() / 100_000.0
}

/// `COURIER_GPS_MAX_DIST_KM` default (`courier-gps.ts`) — hardcoded per the build brief, not
/// re-exposed as a new env var.
const DEFAULT_GPS_MAX_DIST_KM: f64 = 50.0;
/// Default geofence radius (meters) applied to the geofence-ENTER sensor event when
/// `locations.geofence_radius_m` is NULL.
const DEFAULT_GEOFENCE_RADIUS_M: f64 = 150.0;
/// `ACTIVE_DELIVERY_ASSIGNMENT_STATUSES` (`courier-gps.ts`) — deliberately excludes `'assigned'`: a
/// courier is GPS-tracked only from ACCEPT (their consent act), never merely-assigned.
const ACTIVE_DELIVERY_ASSIGNMENT_STATUSES: [&str; 2] = ["accepted", "picked_up"];

// ── Handlers ─────────────────────────────────────────────────────────────────────────────────

/// `GET /api/courier/me/shift` (`shifts.ts` — carried, not fixed; now `with_tenant`-seated).
#[utoipa::path(
    get,
    path = "/api/courier/me/shift",
    tag = "courier",
    responses((status = 200, body = ShiftStatusResponse))
)]
pub async fn get_shift(
    Extension(state): Extension<ShiftsState>,
    CourierSession(courier): CourierSession,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let snapshot = state
        .repo
        .get_active(courier.sub, courier.active_location_id)
        .await
        .map_err(|_e| internal_error(correlation_id))?;

    Ok(Json(match snapshot {
        Some(s) => ShiftStatusResponse {
            is_active: true,
            started_at: Some(s.started_at),
            elapsed_seconds: s.elapsed_seconds,
            shift_id: Some(s.shift_id),
            status: Some(s.status),
            stats: None,
        },
        None => ShiftStatusResponse {
            is_active: false,
            started_at: None,
            elapsed_seconds: 0,
            shift_id: None,
            status: None,
            stats: None,
        },
    }))
}

/// `POST /api/courier/me/shift/start` (`shifts.ts` + `shiftService.ts::openShift`).
#[utoipa::path(
    post,
    path = "/api/courier/me/shift/start",
    tag = "courier",
    request_body = StartShiftRequest,
    responses((status = 200, body = StartShiftResponse))
)]
pub async fn start_shift(
    Extension(state): Extension<ShiftsState>,
    CourierSession(courier): CourierSession,
    Extension(request_id): Extension<RequestId>,
    Json(_body): Json<StartShiftRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let started = state
        .repo
        .start(courier.sub, courier.active_location_id)
        .await
        .map_err(|_e| internal_error(correlation_id))?;

    Ok(Json(StartShiftResponse {
        success: true,
        status: "available".to_string(),
        shift_id: started.shift_id,
        started_at: started.started_at,
    }))
}

/// `POST /api/courier/me/shift/end` (`shifts.ts`) — D3 active-delivery guard (location-scoped).
#[utoipa::path(
    post,
    path = "/api/courier/me/shift/end",
    tag = "courier",
    responses(
        (status = 200, body = EndShiftResponse),
        (status = 409, description = "ACTIVE_DELIVERY_EXISTS", body = domain::ErrorEnvelope),
    )
)]
pub async fn end_shift(
    Extension(state): Extension<ShiftsState>,
    CourierSession(courier): CourierSession,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let outcome = state
        .repo
        .end(courier.sub, courier.active_location_id)
        .await
        .map_err(|_e| internal_error(correlation_id.clone()))?;

    match outcome {
        EndOutcome::AlreadyOffline | EndOutcome::Ended => Ok(Json(EndShiftResponse {
            success: true,
            status: "offline".to_string(),
        })),
        EndOutcome::ActiveDeliveryExists => Err(ApiError::new(
            ErrorCode::ActiveDeliveryExists,
            "ACTIVE_DELIVERY_EXISTS",
            correlation_id,
        )),
    }
}

/// `POST /api/courier/shifts/transition` (`shifts.ts`).
#[utoipa::path(
    post,
    path = "/api/courier/shifts/transition",
    tag = "courier",
    request_body = TransitionRequest,
    responses(
        (status = 200, body = TransitionResponse),
        (status = 400, description = "GPS_REQUIRED", body = domain::ErrorEnvelope),
        (status = 409, description = "state-machine conflict", body = domain::ErrorEnvelope),
    )
)]
pub async fn shifts_transition(
    Extension(state): Extension<ShiftsState>,
    CourierSession(courier): CourierSession,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<TransitionRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let outcome = state
        .repo
        .transition(
            courier.sub,
            courier.active_location_id,
            body.to,
            body.lat,
            body.lng,
        )
        .await
        .map_err(|_e| internal_error(correlation_id.clone()))?;

    match outcome {
        TransitionOutcome::Success { shift_id, status } => Ok(Json(TransitionResponse {
            success: true,
            status,
            shift_id,
        })),
        TransitionOutcome::CannotGoOfflineWithActiveOrder => Err(ApiError::new(
            ErrorCode::CannotGoOfflineWithActiveOrder,
            "CANNOT_GO_OFFLINE_WITH_ACTIVE_ORDER",
            correlation_id,
        )),
        TransitionOutcome::ActiveDeliveryExists => Err(ApiError::new(
            ErrorCode::ActiveDeliveryExists,
            "ACTIVE_DELIVERY_EXISTS",
            correlation_id,
        )),
        TransitionOutcome::InvalidTransition => Err(ApiError::new(
            ErrorCode::InvalidTransition,
            "INVALID_TRANSITION",
            correlation_id,
        )),
        TransitionOutcome::GpsRequired => Err(ApiError::new(
            ErrorCode::GpsRequired,
            "GPS_REQUIRED",
            correlation_id,
        )),
    }
}

/// `POST /api/courier/shifts/ping` (`shifts.ts` + `courier-gps.ts`).
#[utoipa::path(
    post,
    path = "/api/courier/shifts/ping",
    tag = "courier",
    request_body = PingRequest,
    responses(
        (status = 200, body = PingResponse),
        (status = 400, description = "GPS_OUT_OF_RANGE", body = domain::ErrorEnvelope),
        (status = 409, description = "NO_ACTIVE_SHIFT", body = domain::ErrorEnvelope),
    )
)]
pub async fn shifts_ping(
    Extension(state): Extension<ShiftsState>,
    CourierSession(courier): CourierSession,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<PingRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let outcome = state
        .repo
        .ping(
            courier.sub,
            courier.active_location_id,
            body.lat,
            body.lng,
            body.accuracy_meters,
        )
        .await
        .map_err(|_e| internal_error(correlation_id.clone()))?;

    match outcome {
        PingOutcome::Admitted { gps_stored } => Ok(Json(PingResponse {
            success: true,
            gps_stored,
            reason: if gps_stored {
                None
            } else {
                Some("NOT_ON_ACTIVE_DELIVERY".to_string())
            },
        })),
        PingOutcome::NoActiveShift => Err(ApiError::new(
            ErrorCode::NoActiveShift,
            "NO_ACTIVE_SHIFT",
            correlation_id,
        )),
        PingOutcome::GpsOutOfRange => Err(ApiError::new(
            ErrorCode::GpsOutOfRange,
            "GPS_OUT_OF_RANGE",
            correlation_id,
        )),
    }
}

// ── The canonical shift selector (D1/D2 fix) ────────────────────────────────────────────────

/// The single canonical shift selector (D1/D2 fix) — status-filtered + deterministic + FOR UPDATE.
/// Matches `courier/shifts.ts`'s `/me/shift` query exactly (the ONE query in the old file that was
/// already correct); every mutator now uses this SAME selector instead of three divergent ones.
async fn select_open_shift_for_update(
    txn: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    courier_id: Uuid,
    location_id: Uuid,
) -> Result<Option<(Uuid, String)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, status FROM courier_shifts \
         WHERE courier_id = $1 AND location_id = $2 AND status IN ('available','on_delivery') \
         ORDER BY started_at DESC LIMIT 1 FOR UPDATE",
    )
    .bind(courier_id)
    .bind(location_id)
    .fetch_optional(&mut **txn)
    .await
}

/// D3 guard, shared by `end`/`transition(to='offline')` — location-scoped (the old Node query
/// lacked `AND location_id = $2`).
async fn has_active_delivery(
    txn: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    courier_id: Uuid,
    location_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let row: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM courier_assignments \
         WHERE courier_id = $1 AND location_id = $2 \
           AND status IN ('assigned','accepted','picked_up') LIMIT 1",
    )
    .bind(courier_id)
    .bind(location_id)
    .fetch_optional(&mut **txn)
    .await?;
    Ok(row.is_some())
}

// ── PgShiftsRepo ─────────────────────────────────────────────────────────────────────────────

pub struct PgShiftsRepo {
    pool: sqlx::PgPool,
}

impl PgShiftsRepo {
    pub fn new(pool: sqlx::PgPool) -> Self {
        PgShiftsRepo { pool }
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

#[async_trait::async_trait]
impl ShiftsRepo for PgShiftsRepo {
    async fn get_active(
        &self,
        courier_id: Uuid,
        location_id: Uuid,
    ) -> Result<Option<ShiftSnapshot>, RepoError> {
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                let row: Option<(Uuid, String, String, i64)> = sqlx::query_as(
                    "SELECT id, status, started_at::text, \
                       EXTRACT(EPOCH FROM (now() - started_at))::bigint AS elapsed_seconds \
                     FROM courier_shifts \
                     WHERE courier_id = $1 AND location_id = $2 \
                       AND status IN ('available','on_delivery') \
                     ORDER BY started_at DESC LIMIT 1",
                )
                .bind(courier_id)
                .bind(location_id)
                .fetch_optional(&mut **txn)
                .await?;
                Ok(row)
            })
        })
        .await
        .map(|opt| {
            opt.map(|(id, status, started_at, elapsed_seconds)| ShiftSnapshot {
                shift_id: id,
                status,
                started_at,
                elapsed_seconds,
            })
        })
        .map_err(map_txn_err)
    }

    async fn start(&self, courier_id: Uuid, location_id: Uuid) -> Result<StartedShift, RepoError> {
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                let existing = select_open_shift_for_update(txn, courier_id, location_id).await?;
                let row: (Uuid, String) = if let Some((id, _status)) = existing {
                    sqlx::query_as(
                        "UPDATE courier_shifts \
                         SET status = 'available', ended_at = NULL, \
                             started_at = COALESCE(started_at, now()), last_heartbeat_at = now() \
                         WHERE id = $1 \
                         RETURNING id, started_at::text",
                    )
                    .bind(id)
                    .fetch_one(&mut **txn)
                    .await?
                } else {
                    sqlx::query_as(
                        "INSERT INTO courier_shifts \
                           (courier_id, location_id, status, started_at, last_heartbeat_at) \
                         VALUES ($1, $2, 'available', now(), now()) \
                         RETURNING id, started_at::text",
                    )
                    .bind(courier_id)
                    .bind(location_id)
                    .fetch_one(&mut **txn)
                    .await?
                };
                sqlx::query(
                    "INSERT INTO courier_audit_log \
                       (courier_id, location_id, action, actor_kind, actor_id) \
                     VALUES ($1, $2, 'shift.started', 'courier', $1)",
                )
                .bind(courier_id)
                .bind(location_id)
                .execute(&mut **txn)
                .await?;
                Ok(row)
            })
        })
        .await
        .map(|(id, started_at)| StartedShift {
            shift_id: id,
            started_at,
        })
        .map_err(map_txn_err)
    }

    async fn end(&self, courier_id: Uuid, location_id: Uuid) -> Result<EndOutcome, RepoError> {
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                let Some((shift_id, _status)) =
                    select_open_shift_for_update(txn, courier_id, location_id).await?
                else {
                    return Ok(EndOutcome::AlreadyOffline);
                };

                if has_active_delivery(txn, courier_id, location_id).await? {
                    return Ok(EndOutcome::ActiveDeliveryExists);
                }

                sqlx::query(
                    "UPDATE courier_shifts SET status = 'offline', ended_at = now() WHERE id = $1",
                )
                .bind(shift_id)
                .execute(&mut **txn)
                .await?;
                sqlx::query(
                    "INSERT INTO courier_audit_log \
                       (courier_id, location_id, action, actor_kind, actor_id) \
                     VALUES ($1, $2, 'shift.ended', 'courier', $1)",
                )
                .bind(courier_id)
                .bind(location_id)
                .execute(&mut **txn)
                .await?;
                Ok(EndOutcome::Ended)
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn transition(
        &self,
        courier_id: Uuid,
        location_id: Uuid,
        to: TransitionTarget,
        lat: Option<f64>,
        lng: Option<f64>,
    ) -> Result<TransitionOutcome, RepoError> {
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                let existing = select_open_shift_for_update(txn, courier_id, location_id).await?;
                let (shift_id, current_status) = match existing {
                    Some((id, status)) => (Some(id), status),
                    None => (None, "offline".to_string()),
                };

                if to.as_str() == current_status {
                    return Ok(TransitionOutcome::Success {
                        shift_id,
                        status: to.as_str().to_string(),
                    });
                }

                match to {
                    TransitionTarget::Offline => {
                        if current_status == "on_delivery" {
                            return Ok(TransitionOutcome::CannotGoOfflineWithActiveOrder);
                        }
                        if has_active_delivery(txn, courier_id, location_id).await? {
                            return Ok(TransitionOutcome::ActiveDeliveryExists);
                        }
                        // The idempotent branch above already handles current_status=="offline"
                        // when to=="offline"; shift_id is therefore always Some here — the None
                        // arm is kept only for defensive exhaustiveness.
                        let Some(id) = shift_id else {
                            return Ok(TransitionOutcome::Success {
                                shift_id: None,
                                status: "offline".to_string(),
                            });
                        };
                        sqlx::query(
                            "UPDATE courier_shifts SET status = 'offline', ended_at = now() WHERE id = $1",
                        )
                        .bind(id)
                        .execute(&mut **txn)
                        .await?;
                        sqlx::query(
                            "INSERT INTO courier_audit_log \
                               (courier_id, location_id, action, actor_kind, actor_id) \
                             VALUES ($1, $2, 'shift.transition_offline', 'courier', $1)",
                        )
                        .bind(courier_id)
                        .bind(location_id)
                        .execute(&mut **txn)
                        .await?;
                        Ok(TransitionOutcome::Success {
                            shift_id: Some(id),
                            status: "offline".to_string(),
                        })
                    }
                    TransitionTarget::Available => {
                        if current_status == "on_delivery" {
                            return Ok(TransitionOutcome::InvalidTransition);
                        }
                        if lat.is_none() || lng.is_none() {
                            return Ok(TransitionOutcome::GpsRequired);
                        }
                        let id: Uuid = if let Some(id) = shift_id {
                            sqlx::query(
                                "UPDATE courier_shifts \
                                 SET status = 'available', ended_at = NULL, \
                                     started_at = COALESCE(started_at, now()), \
                                     last_heartbeat_at = now() \
                                 WHERE id = $1",
                            )
                            .bind(id)
                            .execute(&mut **txn)
                            .await?;
                            id
                        } else {
                            let row: (Uuid,) = sqlx::query_as(
                                "INSERT INTO courier_shifts \
                                   (courier_id, location_id, status, started_at, last_heartbeat_at) \
                                 VALUES ($1, $2, 'available', now(), now()) RETURNING id",
                            )
                            .bind(courier_id)
                            .bind(location_id)
                            .fetch_one(&mut **txn)
                            .await?;
                            row.0
                        };
                        sqlx::query(
                            "INSERT INTO courier_audit_log \
                               (courier_id, location_id, action, actor_kind, actor_id) \
                             VALUES ($1, $2, 'shift.transition_available', 'courier', $1)",
                        )
                        .bind(courier_id)
                        .bind(location_id)
                        .execute(&mut **txn)
                        .await?;
                        Ok(TransitionOutcome::Success {
                            shift_id: Some(id),
                            status: "available".to_string(),
                        })
                    }
                }
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn ping(
        &self,
        courier_id: Uuid,
        location_id: Uuid,
        lat: f64,
        lng: f64,
        accuracy_meters: Option<i32>,
    ) -> Result<PingOutcome, RepoError> {
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                // Step 1 — geofence range-check, ONLY when the location has a pin (D4 carry).
                let loc: Option<(Option<f64>, Option<f64>, Option<i32>)> = sqlx::query_as(
                    "SELECT lat::float8, lng::float8, geofence_radius_m FROM locations WHERE id = $1",
                )
                .bind(location_id)
                .fetch_optional(&mut **txn)
                .await?;
                let (loc_lat, loc_lng, radius_m) = loc.unwrap_or((None, None, None));
                if let (Some(loc_lat), Some(loc_lng)) = (loc_lat, loc_lng) {
                    if !is_within_geofence(lat, lng, loc_lat, loc_lng, DEFAULT_GPS_MAX_DIST_KM) {
                        return Ok(PingOutcome::GpsOutOfRange);
                    }
                }

                // Step 2 — an active shift must exist.
                let shift_row: Option<(Uuid,)> = sqlx::query_as(
                    "SELECT id FROM courier_shifts \
                     WHERE courier_id = $1 AND location_id = $2 \
                       AND status IN ('available','on_delivery') LIMIT 1",
                )
                .bind(courier_id)
                .bind(location_id)
                .fetch_optional(&mut **txn)
                .await?;
                let Some((shift_id,)) = shift_row else {
                    return Ok(PingOutcome::NoActiveShift);
                };

                // Step 3 — the courier's OWN active order (never trust the ping payload for this).
                // ACTIVE_DELIVERY_ASSIGNMENT_STATUSES deliberately excludes 'assigned' — a courier
                // is tracked only from ACCEPT, their consent act, never merely-assigned.
                let active_order: Option<(Uuid,)> = sqlx::query_as(
                    "SELECT order_id FROM courier_assignments \
                     WHERE courier_id = $1 AND status = ANY($2::text[]) \
                     ORDER BY picked_up_at DESC NULLS LAST, accepted_at DESC NULLS LAST, order_id \
                     LIMIT 1",
                )
                .bind(courier_id)
                .bind(&ACTIVE_DELIVERY_ASSIGNMENT_STATUSES[..])
                .fetch_optional(&mut **txn)
                .await?;

                let gps_stored = if let Some((order_id,)) = active_order {
                    let rounded_lat = round_coordinate(lat);
                    let rounded_lng = round_coordinate(lng);
                    sqlx::query(
                        "INSERT INTO courier_positions \
                           (courier_id, location_id, shift_id, lat, lng, accuracy_meters, source) \
                         VALUES ($1, $2, $3, $4, $5, $6, 'gps')",
                    )
                    .bind(courier_id)
                    .bind(location_id)
                    .bind(shift_id)
                    .bind(rounded_lat)
                    .bind(rounded_lng)
                    .bind(accuracy_meters)
                    .execute(&mut **txn)
                    .await?;

                    // Best-effort geofence-enter sensor event (D8 CARRY) — a sensor write must
                    // never fail the ping itself.
                    if let (Some(loc_lat), Some(loc_lng)) = (loc_lat, loc_lng) {
                        let dist_m = distance_km(lat, lng, loc_lat, loc_lng) * 1000.0;
                        let effective_radius_m =
                            radius_m.map(f64::from).unwrap_or(DEFAULT_GEOFENCE_RADIUS_M);
                        if dist_m <= effective_radius_m {
                            sqlx::query("SAVEPOINT geofence_evt")
                                .execute(&mut **txn)
                                .await?;
                            let payload = serde_json::json!({
                                "distance_m": dist_m.round(),
                                "radius_m": effective_radius_m,
                            });
                            let insert: Result<_, sqlx::Error> = sqlx::query(
                                "INSERT INTO order_sensor_events \
                                   (location_id, order_id, event_type, payload) \
                                 VALUES ($1, $2, 'courier_geofence_enter', $3::jsonb) \
                                 ON CONFLICT (order_id, event_type) DO NOTHING",
                            )
                            .bind(location_id)
                            .bind(order_id)
                            .bind(payload)
                            .execute(&mut **txn)
                            .await;
                            match insert {
                                Ok(_) => {
                                    sqlx::query("RELEASE SAVEPOINT geofence_evt")
                                        .execute(&mut **txn)
                                        .await?;
                                }
                                Err(_) => {
                                    sqlx::query("ROLLBACK TO SAVEPOINT geofence_evt")
                                        .execute(&mut **txn)
                                        .await?;
                                }
                            }
                        }
                    }
                    true
                } else {
                    false
                };

                // Step 5 — always heartbeat, even off-delivery / GPS-withheld (D6/D7 note: this is
                // a plain typed control-flow path, no untyped throw is possible here).
                sqlx::query("UPDATE courier_shifts SET last_heartbeat_at = now() WHERE id = $1")
                    .bind(shift_id)
                    .execute(&mut **txn)
                    .await?;

                Ok(PingOutcome::Admitted { gps_stored })
            })
        })
        .await
        .map_err(map_txn_err)
    }
}

// ── FakeShiftsRepo (test-only) ───────────────────────────────────────────────────────────────

#[cfg(test)]
pub mod fake {
    //! Mutex-backed stub mirroring `owner::categories::fake::FakeCategoriesRepo`'s style — no live
    //! Postgres needed for handler tests. Stores MULTIPLE historical shift rows per
    //! courier+location (not just the latest) so a test can prove the D1/D2 selector picks the
    //! newest matching row deterministically, never an arbitrary one.

    use super::{
        DEFAULT_GPS_MAX_DIST_KM, EndOutcome, PingOutcome, RepoError, ShiftSnapshot, ShiftsRepo,
        StartedShift, TransitionOutcome, TransitionTarget, is_within_geofence,
    };
    use std::collections::{HashMap, HashSet};
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicI64, Ordering};
    use uuid::Uuid;

    #[derive(Debug, Clone)]
    pub struct FakeShift {
        pub id: Uuid,
        pub courier_id: Uuid,
        pub location_id: Uuid,
        pub status: String,
        /// A monotonic ordering surrogate standing in for `started_at DESC` — higher is newer.
        pub started_at_rank: i64,
    }

    #[derive(Default)]
    pub struct FakeShiftsRepo {
        pub shifts: Mutex<Vec<FakeShift>>,
        /// (courier_id, location_id) pairs the D3 guard should treat as "has an active delivery".
        pub active_deliveries: Mutex<HashSet<(Uuid, Uuid)>>,
        /// location_id -> (lat, lng) pin. Absent = "no pin" (D4 carry: admit any coordinate).
        pub location_pins: Mutex<HashMap<Uuid, (f64, f64)>>,
        /// (courier_id, location_id) -> the courier's own active order_id (accepted/picked_up).
        pub active_orders: Mutex<HashMap<(Uuid, Uuid), Uuid>>,
        rank_counter: AtomicI64,
    }

    impl FakeShiftsRepo {
        pub fn seed_shift(&self, shift: FakeShift) {
            self.shifts.lock().unwrap().push(shift);
        }

        pub fn mark_active_delivery(&self, courier_id: Uuid, location_id: Uuid) {
            self.active_deliveries
                .lock()
                .unwrap()
                .insert((courier_id, location_id));
        }

        pub fn set_location_pin(&self, location_id: Uuid, lat: f64, lng: f64) {
            self.location_pins
                .lock()
                .unwrap()
                .insert(location_id, (lat, lng));
        }

        pub fn set_active_order(&self, courier_id: Uuid, location_id: Uuid, order_id: Uuid) {
            self.active_orders
                .lock()
                .unwrap()
                .insert((courier_id, location_id), order_id);
        }

        fn next_rank(&self) -> i64 {
            self.rank_counter.fetch_add(1, Ordering::SeqCst)
        }

        /// The D1/D2 selector's in-memory equivalent: newest (highest rank) row whose status is
        /// `available`/`on_delivery`, for this courier at this location.
        fn find_open(&self, courier_id: Uuid, location_id: Uuid) -> Option<FakeShift> {
            self.shifts
                .lock()
                .unwrap()
                .iter()
                .filter(|s| {
                    s.courier_id == courier_id
                        && s.location_id == location_id
                        && (s.status == "available" || s.status == "on_delivery")
                })
                .max_by_key(|s| s.started_at_rank)
                .cloned()
        }

        fn has_active_delivery(&self, courier_id: Uuid, location_id: Uuid) -> bool {
            self.active_deliveries
                .lock()
                .unwrap()
                .contains(&(courier_id, location_id))
        }

        fn set_status(&self, id: Uuid, status: &str) {
            if let Some(row) = self.shifts.lock().unwrap().iter_mut().find(|r| r.id == id) {
                row.status = status.to_string();
            }
        }
    }

    #[async_trait::async_trait]
    impl ShiftsRepo for FakeShiftsRepo {
        async fn get_active(
            &self,
            courier_id: Uuid,
            location_id: Uuid,
        ) -> Result<Option<ShiftSnapshot>, RepoError> {
            Ok(self
                .find_open(courier_id, location_id)
                .map(|s| ShiftSnapshot {
                    shift_id: s.id,
                    status: s.status,
                    started_at: "2026-01-01T00:00:00Z".to_string(),
                    elapsed_seconds: 0,
                }))
        }

        async fn start(
            &self,
            courier_id: Uuid,
            location_id: Uuid,
        ) -> Result<StartedShift, RepoError> {
            let id = if let Some(existing) = self.find_open(courier_id, location_id) {
                self.set_status(existing.id, "available");
                existing.id
            } else {
                let id = Uuid::new_v4();
                let rank = self.next_rank();
                self.seed_shift(FakeShift {
                    id,
                    courier_id,
                    location_id,
                    status: "available".to_string(),
                    started_at_rank: rank,
                });
                id
            };
            Ok(StartedShift {
                shift_id: id,
                started_at: "2026-01-01T00:00:00Z".to_string(),
            })
        }

        async fn end(&self, courier_id: Uuid, location_id: Uuid) -> Result<EndOutcome, RepoError> {
            let Some(existing) = self.find_open(courier_id, location_id) else {
                return Ok(EndOutcome::AlreadyOffline);
            };
            if self.has_active_delivery(courier_id, location_id) {
                return Ok(EndOutcome::ActiveDeliveryExists);
            }
            self.set_status(existing.id, "offline");
            Ok(EndOutcome::Ended)
        }

        async fn transition(
            &self,
            courier_id: Uuid,
            location_id: Uuid,
            to: TransitionTarget,
            lat: Option<f64>,
            lng: Option<f64>,
        ) -> Result<TransitionOutcome, RepoError> {
            let existing = self.find_open(courier_id, location_id);
            let (shift_id, current_status) = match &existing {
                Some(s) => (Some(s.id), s.status.clone()),
                None => (None, "offline".to_string()),
            };

            if to.as_str() == current_status {
                return Ok(TransitionOutcome::Success {
                    shift_id,
                    status: to.as_str().to_string(),
                });
            }

            match to {
                TransitionTarget::Offline => {
                    if current_status == "on_delivery" {
                        return Ok(TransitionOutcome::CannotGoOfflineWithActiveOrder);
                    }
                    if self.has_active_delivery(courier_id, location_id) {
                        return Ok(TransitionOutcome::ActiveDeliveryExists);
                    }
                    if let Some(id) = shift_id {
                        self.set_status(id, "offline");
                    }
                    Ok(TransitionOutcome::Success {
                        shift_id,
                        status: "offline".to_string(),
                    })
                }
                TransitionTarget::Available => {
                    if current_status == "on_delivery" {
                        return Ok(TransitionOutcome::InvalidTransition);
                    }
                    if lat.is_none() || lng.is_none() {
                        return Ok(TransitionOutcome::GpsRequired);
                    }
                    let id = if let Some(id) = shift_id {
                        self.set_status(id, "available");
                        id
                    } else {
                        let id = Uuid::new_v4();
                        let rank = self.next_rank();
                        self.seed_shift(FakeShift {
                            id,
                            courier_id,
                            location_id,
                            status: "available".to_string(),
                            started_at_rank: rank,
                        });
                        id
                    };
                    Ok(TransitionOutcome::Success {
                        shift_id: Some(id),
                        status: "available".to_string(),
                    })
                }
            }
        }

        async fn ping(
            &self,
            courier_id: Uuid,
            location_id: Uuid,
            lat: f64,
            lng: f64,
            _accuracy_meters: Option<i32>,
        ) -> Result<PingOutcome, RepoError> {
            if let Some((loc_lat, loc_lng)) = self
                .location_pins
                .lock()
                .unwrap()
                .get(&location_id)
                .copied()
            {
                if !is_within_geofence(lat, lng, loc_lat, loc_lng, DEFAULT_GPS_MAX_DIST_KM) {
                    return Ok(PingOutcome::GpsOutOfRange);
                }
            }
            if self.find_open(courier_id, location_id).is_none() {
                return Ok(PingOutcome::NoActiveShift);
            }
            let gps_stored = self
                .active_orders
                .lock()
                .unwrap()
                .contains_key(&(courier_id, location_id));
            Ok(PingOutcome::Admitted { gps_stored })
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::fake::{FakeShift, FakeShiftsRepo};
    use super::*;
    use crate::auth::claims::CourierClaims;
    use crate::auth::repo::fake::FakeAuthRepo;
    use axum::body::to_bytes;
    use axum::http::StatusCode;
    use std::sync::Arc;

    fn request_id() -> RequestId {
        RequestId::new(axum::http::HeaderValue::from_static("corr-1"))
    }

    fn courier_session(courier_id: Uuid, location_id: Uuid) -> CourierSession {
        CourierSession(CourierClaims::new(courier_id, location_id, None))
    }

    fn state_with(repo: FakeShiftsRepo) -> ShiftsState {
        ShiftsState {
            auth: AuthState::test_state(Arc::new(FakeAuthRepo::default())),
            repo: Arc::new(repo),
        }
    }

    async fn json_body(resp: axum::response::Response) -> (StatusCode, serde_json::Value) {
        let status = resp.status();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    // ── pure unit tests ──

    #[test]
    fn round_coordinate_matches_node_math_round_5dp() {
        assert!((round_coordinate(1.234_567) - 1.234_57).abs() < 1e-9);
        assert!((round_coordinate(1.234_561) - 1.234_56).abs() < 1e-9);
    }

    #[test]
    fn is_within_geofence_true_within_and_false_beyond_radius() {
        // ~1.1km apart (within a 5km radius) vs Paris ~1600km apart (beyond a 5km radius).
        assert!(is_within_geofence(41.3275, 19.8189, 41.33, 19.82, 5.0));
        assert!(!is_within_geofence(41.3275, 19.8189, 48.8566, 2.3522, 5.0));
    }

    // ── DTO `.strict()` parity ──

    #[test]
    fn transition_request_rejects_an_unknown_field() {
        let json = serde_json::json!({ "to": "available", "extra": "nope" });
        assert!(serde_json::from_value::<TransitionRequest>(json).is_err());
    }

    #[test]
    fn ping_request_rejects_an_unknown_field() {
        let json = serde_json::json!({ "lat": 1.0, "lng": 2.0, "extra": "nope" });
        assert!(serde_json::from_value::<PingRequest>(json).is_err());
    }

    // ── D1/D2: the canonical selector picks the newest row, not an arbitrary one ──

    #[tokio::test]
    async fn start_shift_reuses_the_most_recent_available_shift_row_not_an_arbitrary_one() {
        let repo = FakeShiftsRepo::default();
        let courier = Uuid::new_v4();
        let location = Uuid::new_v4();
        let older_id = Uuid::new_v4();
        let newer_id = Uuid::new_v4();
        repo.seed_shift(FakeShift {
            id: older_id,
            courier_id: courier,
            location_id: location,
            status: "available".to_string(),
            started_at_rank: 1,
        });
        repo.seed_shift(FakeShift {
            id: newer_id,
            courier_id: courier,
            location_id: location,
            status: "available".to_string(),
            started_at_rank: 2,
        });

        let started = repo.start(courier, location).await.unwrap();
        assert_eq!(
            started.shift_id, newer_id,
            "D1/D2 fix: must target the newest matching row deterministically, never an arbitrary one"
        );
    }

    // ── get_shift ──

    #[tokio::test]
    async fn get_shift_reports_inactive_when_no_shift_exists() {
        let courier = Uuid::new_v4();
        let location = Uuid::new_v4();
        let state = state_with(FakeShiftsRepo::default());

        let resp = get_shift(
            Extension(state),
            courier_session(courier, location),
            Extension(request_id()),
        )
        .await
        .unwrap()
        .into_response();
        let (status, json) = json_body(resp).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["isActive"], false);
        assert_eq!(json["shiftId"], serde_json::Value::Null);
    }

    #[tokio::test]
    async fn get_shift_reports_active_when_a_shift_exists() {
        let courier = Uuid::new_v4();
        let location = Uuid::new_v4();
        let shift_id = Uuid::new_v4();
        let repo = FakeShiftsRepo::default();
        repo.seed_shift(FakeShift {
            id: shift_id,
            courier_id: courier,
            location_id: location,
            status: "available".to_string(),
            started_at_rank: 1,
        });
        let state = state_with(repo);

        let resp = get_shift(
            Extension(state),
            courier_session(courier, location),
            Extension(request_id()),
        )
        .await
        .unwrap()
        .into_response();
        let (status, json) = json_body(resp).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["isActive"], true);
        assert_eq!(json["shiftId"], shift_id.to_string());
    }

    // ── start_shift ──

    #[tokio::test]
    async fn start_shift_200_creates_a_fresh_shift_when_none_exists() {
        let courier = Uuid::new_v4();
        let location = Uuid::new_v4();
        let state = state_with(FakeShiftsRepo::default());

        let resp = start_shift(
            Extension(state),
            courier_session(courier, location),
            Extension(request_id()),
            Json(StartShiftRequest {
                lat: None,
                lng: None,
            }),
        )
        .await
        .unwrap()
        .into_response();
        let (status, json) = json_body(resp).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["status"], "available");
    }

    // ── end_shift ──

    #[tokio::test]
    async fn end_shift_is_a_noop_success_when_already_offline() {
        let courier = Uuid::new_v4();
        let location = Uuid::new_v4();
        let state = state_with(FakeShiftsRepo::default());

        let resp = end_shift(
            Extension(state),
            courier_session(courier, location),
            Extension(request_id()),
        )
        .await
        .unwrap()
        .into_response();
        let (status, json) = json_body(resp).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["status"], "offline");
    }

    #[tokio::test]
    async fn end_shift_409s_when_an_active_delivery_exists_at_this_location() {
        let courier = Uuid::new_v4();
        let location = Uuid::new_v4();
        let repo = FakeShiftsRepo::default();
        repo.seed_shift(FakeShift {
            id: Uuid::new_v4(),
            courier_id: courier,
            location_id: location,
            status: "available".to_string(),
            started_at_rank: 1,
        });
        repo.mark_active_delivery(courier, location);
        let state = state_with(repo);

        let resp = end_shift(
            Extension(state),
            courier_session(courier, location),
            Extension(request_id()),
        )
        .await;
        let err = crate::error::expect_err(resp);
        assert_eq!(err.envelope.code, ErrorCode::ActiveDeliveryExists);
        assert_eq!(err.envelope.status, 409);
    }

    #[tokio::test]
    async fn end_shift_ignores_an_active_delivery_at_a_different_location() {
        let courier = Uuid::new_v4();
        let location_a = Uuid::new_v4();
        let location_b = Uuid::new_v4();
        let repo = FakeShiftsRepo::default();
        repo.seed_shift(FakeShift {
            id: Uuid::new_v4(),
            courier_id: courier,
            location_id: location_a,
            status: "available".to_string(),
            started_at_rank: 1,
        });
        // D3: the active delivery is at a DIFFERENT location — must NOT block ending the shift
        // at location_a.
        repo.mark_active_delivery(courier, location_b);
        let state = state_with(repo);

        let resp = end_shift(
            Extension(state),
            courier_session(courier, location_a),
            Extension(request_id()),
        )
        .await
        .unwrap()
        .into_response();
        let (status, json) = json_body(resp).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["status"], "offline");
    }

    // ── shifts_transition ──

    #[tokio::test]
    async fn shifts_transition_to_available_without_gps_is_400() {
        let courier = Uuid::new_v4();
        let location = Uuid::new_v4();
        let state = state_with(FakeShiftsRepo::default());

        let resp = shifts_transition(
            Extension(state),
            courier_session(courier, location),
            Extension(request_id()),
            Json(TransitionRequest {
                to: TransitionTarget::Available,
                lat: None,
                lng: None,
            }),
        )
        .await;
        let err = crate::error::expect_err(resp);
        assert_eq!(err.envelope.code, ErrorCode::GpsRequired);
        assert_eq!(err.envelope.status, 400);
    }

    #[tokio::test]
    async fn shifts_transition_to_offline_while_on_delivery_is_409_cannot_go_offline() {
        let courier = Uuid::new_v4();
        let location = Uuid::new_v4();
        let repo = FakeShiftsRepo::default();
        repo.seed_shift(FakeShift {
            id: Uuid::new_v4(),
            courier_id: courier,
            location_id: location,
            status: "on_delivery".to_string(),
            started_at_rank: 1,
        });
        let state = state_with(repo);

        let resp = shifts_transition(
            Extension(state),
            courier_session(courier, location),
            Extension(request_id()),
            Json(TransitionRequest {
                to: TransitionTarget::Offline,
                lat: None,
                lng: None,
            }),
        )
        .await;
        let err = crate::error::expect_err(resp);
        assert_eq!(err.envelope.code, ErrorCode::CannotGoOfflineWithActiveOrder);
        assert_eq!(err.envelope.status, 409);
    }

    #[tokio::test]
    async fn shifts_transition_available_to_available_is_idempotent_success() {
        let courier = Uuid::new_v4();
        let location = Uuid::new_v4();
        let shift_id = Uuid::new_v4();
        let repo = FakeShiftsRepo::default();
        repo.seed_shift(FakeShift {
            id: shift_id,
            courier_id: courier,
            location_id: location,
            status: "available".to_string(),
            started_at_rank: 1,
        });
        let state = state_with(repo);

        // No lat/lng supplied — proves the idempotent no-op branch is checked BEFORE the
        // GPS_REQUIRED gate.
        let resp = shifts_transition(
            Extension(state),
            courier_session(courier, location),
            Extension(request_id()),
            Json(TransitionRequest {
                to: TransitionTarget::Available,
                lat: None,
                lng: None,
            }),
        )
        .await
        .unwrap()
        .into_response();
        let (status, json) = json_body(resp).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["status"], "available");
        assert_eq!(json["shiftId"], shift_id.to_string());
    }

    // ── shifts_ping ──

    #[tokio::test]
    async fn shifts_ping_with_no_active_shift_is_409() {
        let courier = Uuid::new_v4();
        let location = Uuid::new_v4();
        let state = state_with(FakeShiftsRepo::default());

        let resp = shifts_ping(
            Extension(state),
            courier_session(courier, location),
            Extension(request_id()),
            Json(PingRequest {
                lat: 41.3275,
                lng: 19.8189,
                accuracy_meters: None,
            }),
        )
        .await;
        let err = crate::error::expect_err(resp);
        assert_eq!(err.envelope.code, ErrorCode::NoActiveShift);
        assert_eq!(err.envelope.status, 409);
    }

    #[tokio::test]
    async fn shifts_ping_out_of_geofence_range_is_400() {
        let courier = Uuid::new_v4();
        let location = Uuid::new_v4();
        let repo = FakeShiftsRepo::default();
        repo.seed_shift(FakeShift {
            id: Uuid::new_v4(),
            courier_id: courier,
            location_id: location,
            status: "available".to_string(),
            started_at_rank: 1,
        });
        // Tirana pin; the ping reports a Paris coordinate — ~1600km apart, past the 50km default.
        repo.set_location_pin(location, 41.3275, 19.8189);
        let state = state_with(repo);

        let resp = shifts_ping(
            Extension(state),
            courier_session(courier, location),
            Extension(request_id()),
            Json(PingRequest {
                lat: 48.8566,
                lng: 2.3522,
                accuracy_meters: None,
            }),
        )
        .await;
        let err = crate::error::expect_err(resp);
        assert_eq!(err.envelope.code, ErrorCode::GpsOutOfRange);
        assert_eq!(err.envelope.status, 400);
    }

    #[tokio::test]
    async fn shifts_ping_admits_any_coordinate_when_location_has_no_pin() {
        let courier = Uuid::new_v4();
        let location = Uuid::new_v4();
        let repo = FakeShiftsRepo::default();
        repo.seed_shift(FakeShift {
            id: Uuid::new_v4(),
            courier_id: courier,
            location_id: location,
            status: "available".to_string(),
            started_at_rank: 1,
        });
        // Deliberately NO location pin set — D4 carry: the range check must be skipped entirely.
        let state = state_with(repo);

        let resp = shifts_ping(
            Extension(state),
            courier_session(courier, location),
            Extension(request_id()),
            Json(PingRequest {
                lat: 48.8566,
                lng: 2.3522,
                accuracy_meters: None,
            }),
        )
        .await
        .unwrap()
        .into_response();
        let (status, json) = json_body(resp).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["success"], true);
    }

    #[tokio::test]
    async fn shifts_ping_stores_gps_when_courier_has_an_active_order() {
        let courier = Uuid::new_v4();
        let location = Uuid::new_v4();
        let order_id = Uuid::new_v4();
        let repo = FakeShiftsRepo::default();
        repo.seed_shift(FakeShift {
            id: Uuid::new_v4(),
            courier_id: courier,
            location_id: location,
            status: "on_delivery".to_string(),
            started_at_rank: 1,
        });
        repo.set_active_order(courier, location, order_id);
        let state = state_with(repo);

        let resp = shifts_ping(
            Extension(state),
            courier_session(courier, location),
            Extension(request_id()),
            Json(PingRequest {
                lat: 41.3275,
                lng: 19.8189,
                accuracy_meters: Some(10),
            }),
        )
        .await
        .unwrap()
        .into_response();
        let (status, json) = json_body(resp).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["gpsStored"], true);
        assert_eq!(json["reason"], serde_json::Value::Null);
    }
}
