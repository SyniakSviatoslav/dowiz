//! S7 courier settlements — the courier's OWN read of their `courier_payouts`/`settlement_items`
//! rows. Ports `apps/api/src/routes/courier/settlements.ts`. See `crate::routes::courier` module
//! doc for the shared auth/tenancy contract (`CourierSession` extractor +
//! `with_tenant(active_location_id)`), and `courier::me`/`courier::assignments` for sibling S7
//! files using the same conventions.
//!
//! ## Money is NOT generated here (REV-S7-3, S7-T6)
//! Settlement GENERATION (`app_generate_settlements`, S5/S8-owned) is never called or
//! re-implemented by this file — every method here is a pure READ of rows some OTHER surface
//! already wrote. No settlement math, no `total_earned`/`amount` computation, anywhere below.
//!
//! ## Tenancy fix-by-port (REV-S7-1 / S7-T10)
//! Old Node's `settlements.ts` seats `app.current_tenant` on the BARE pool with NO `BEGIN`
//! (`SELECT set_config('app.current_tenant', $1, true)` as a standalone auto-committed statement,
//! `settlements.ts:25,59,74`) — the seat can land on a different pooled connection than the
//! subsequent SELECT, so it only "works" today because the operational pool's role still bypasses
//! RLS. `settlement_items`'s RLS policy is additionally the THROWING bare `current_setting(...)`
//! form (no `, true` missing-ok arg, `settlement-items.ts` migration) — under real NOBYPASSRLS
//! enforcement a missing seat would 500, not silently return zero rows. Every method below runs
//! inside [`crate::db::with_tenant`] (a real `BEGIN -> set_config -> ... -> COMMIT` transaction),
//! closing both gaps by construction.
//!
//! ## Q-PAYOUT-READ-SHARED — stricter courier redaction on payout items
//! `get_payout_detail`'s items list is a SEPARATE, narrower query than what an owner-facing
//! settlement read would return: "strictly no orderId, no assignmentId, no customer phone"
//! (`settlements.ts:89`, carried verbatim as this file's own comment below). [`PayoutItemRow`]'s
//! fields are `delivered_at`/`amount`/`currency` ONLY — `payout_items_never_include_order_id_or_
//! assignment_id` is the structural DoD test pinning this shape.
//!
//! ## Wire shape: RAW snake_case passthrough, no camelCase mapper (unlike `courier::me`)
//! Node's `settlements.ts` sends `res.rows` straight through with no `mapXRow` transform at all —
//! [`PayoutRow`]/[`PayoutItemRow`] double as BOTH the repo row type and the wire DTO (their field
//! names literally ARE the SQL column aliases), matching that absence of a mapping layer exactly.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Path, Query};
use axum::response::IntoResponse;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tower_http::request_id::RequestId;
use uuid::Uuid;

use domain::{ErrorCode, TenantId};

use crate::auth::AuthState;
use crate::auth::extractors::CourierSession;
use crate::error::ApiError;
use crate::repo::RepoError;
use crate::routes::correlation_id_string;

// ── State + repo trait ──────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct SettlementsState {
    /// Not read here — payout handlers authenticate via the `CourierSession` extractor (which reads
    /// `AuthState` from request extensions, layered by `courier_router`). Kept for State-shape
    /// uniformity with the other courier submodules; see `assignments::AssignmentsState.auth`.
    #[allow(
        dead_code,
        reason = "CourierSession extractor reads AuthState from request extensions, not this field — kept for State-shape uniformity"
    )]
    pub auth: AuthState,
    pub repo: Arc<dyn SettlementsRepo>,
}

/// `courier_payouts JOIN locations` (`settlements.ts:28-34`, `:62-68`) — RAW column-aliased shape,
/// no camelCase transform; doubles as the wire DTO (see module doc).
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PayoutRow {
    pub id: Uuid,
    pub location_id: Uuid,
    pub location_name: String,
    pub deliveries_count: i32,
    pub total_earned: i64,
    #[serde(serialize_with = "crate::dto::serialize_js_instant")]
    pub period_start: DateTime<Utc>,
    #[serde(serialize_with = "crate::dto::serialize_js_instant")]
    pub period_end: DateTime<Utc>,
    pub status: String,
    #[serde(serialize_with = "crate::dto::serialize_js_instant")]
    pub created_at: DateTime<Utc>,
    #[serde(serialize_with = "crate::dto::serialize_js_instant_opt")]
    pub approved_at: Option<DateTime<Utc>>,
    pub currency: String,
}

/// `settlement_items JOIN courier_assignments` (`settlements.ts:79-85`) — Q-PAYOUT-READ-SHARED:
/// strictly no orderId, no assignmentId, no customer phone. These three fields are the WHOLE
/// shape by design; the structural test pins that no more ever get added silently.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PayoutItemRow {
    #[serde(serialize_with = "crate::dto::serialize_js_instant_opt")]
    pub delivered_at: Option<DateTime<Utc>>,
    pub amount: i64,
    pub currency: String,
}

#[async_trait::async_trait]
pub trait SettlementsRepo: Send + Sync {
    /// `GET /me/payouts` (`settlements.ts:12-48`) — optional `status` filter.
    async fn list_payouts(
        &self,
        courier_id: Uuid,
        location_id: Uuid,
        status: Option<String>,
    ) -> Result<Vec<PayoutRow>, RepoError>;

    /// `GET /me/payouts/:id`'s payout half (`settlements.ts:62-71`) — scoped to
    /// `(id, courier_id)`. `Ok(None)` covers BOTH "no such payout" AND "belongs to another
    /// courier" (the actor-gate's existence-hiding posture, same reasoning as S3/S7's other
    /// not-found collapses) — 404 either way.
    async fn get_payout(
        &self,
        id: Uuid,
        courier_id: Uuid,
        location_id: Uuid,
    ) -> Result<Option<PayoutRow>, RepoError>;

    /// `GET /me/payouts/:id`'s items half (`settlements.ts:79-85`).
    async fn payout_items(
        &self,
        payout_id: Uuid,
        location_id: Uuid,
    ) -> Result<Vec<PayoutItemRow>, RepoError>;
}

// ── DTOs ─────────────────────────────────────────────────────────────────────────────────────

/// `?status=` querystring (`settlements.ts:14-16`).
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PayoutStatus {
    Pending,
    Approved,
    Paid,
    Disputed,
}

impl PayoutStatus {
    fn as_db_str(self) -> &'static str {
        match self {
            PayoutStatus::Pending => "pending",
            PayoutStatus::Approved => "approved",
            PayoutStatus::Paid => "paid",
            PayoutStatus::Disputed => "disputed",
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ListPayoutsQuery {
    #[serde(default)]
    pub status: Option<PayoutStatus>,
}

// `Serialize`-only (no `ToSchema`): `PayoutRow`/`PayoutItemRow` carry raw `DateTime<Utc>` fields,
// which don't implement utoipa's `PartialSchema` without a workspace-wide Cargo.toml "chrono"
// feature this two-file port doesn't add (see `courier::me`'s identical note, which cites the
// established `owner::products::ProductRow` precedent for this same workaround).
#[derive(Debug, Serialize)]
pub struct PayoutsListResponse {
    pub payouts: Vec<PayoutRow>,
}

#[derive(Debug, Serialize)]
pub struct PayoutDetailResponse {
    pub payout: PayoutRow,
    pub items: Vec<PayoutItemRow>,
}

fn internal_error(correlation_id: String) -> ApiError {
    ApiError::new(ErrorCode::Internal, "internal_error", correlation_id)
}

fn not_found(correlation_id: String) -> ApiError {
    ApiError::new(ErrorCode::NotFound, "Not found", correlation_id)
}

// ── Handlers ─────────────────────────────────────────────────────────────────────────────────

/// `GET /api/courier/me/payouts` (`settlements.ts:12-48`).
#[utoipa::path(get, path = "/api/courier/me/payouts", tag = "courier",
    params(("status" = Option<String>, Query, description = "pending|approved|paid|disputed")),
    responses((status = 200, description = "This courier's settlement payouts")))]
pub async fn get_payouts(
    Extension(state): Extension<SettlementsState>,
    CourierSession(courier): CourierSession,
    Query(params): Query<ListPayoutsQuery>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let status = params.status.map(|s| s.as_db_str().to_string());
    let rows = state
        .repo
        .list_payouts(courier.sub, courier.active_location_id, status)
        .await
        .map_err(|_e| internal_error(correlation_id))?;
    Ok(Json(PayoutsListResponse { payouts: rows }))
}

/// `GET /api/courier/me/payouts/{id}` (`settlements.ts:51-91`) — Q-PAYOUT-READ-SHARED redaction
/// on `items`.
#[utoipa::path(get, path = "/api/courier/me/payouts/{id}", tag = "courier",
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "Payout + redacted settlement items"),
        (status = 404, description = "Not found or belongs to another courier", body = domain::ErrorEnvelope),
    ))]
pub async fn get_payout_detail(
    Extension(state): Extension<SettlementsState>,
    CourierSession(courier): CourierSession,
    Path(id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    let payout = state
        .repo
        .get_payout(id, courier.sub, courier.active_location_id)
        .await
        .map_err(|_e| internal_error(correlation_id.clone()))?
        .ok_or_else(|| not_found(correlation_id.clone()))?;

    let items = state
        .repo
        .payout_items(id, courier.active_location_id)
        .await
        .map_err(|_e| internal_error(correlation_id))?;

    Ok(Json(PayoutDetailResponse { payout, items }))
}

// ── PgSettlementsRepo ────────────────────────────────────────────────────────────────────────

const PAYOUT_SELECT: &str = "SELECT p.id, p.location_id, l.name AS location_name, p.deliveries_count, \
     p.total_earned::bigint AS total_earned, p.period_start, p.period_end, p.status, p.created_at, \
     p.approved_at, l.currency_code AS currency \
     FROM courier_payouts p JOIN locations l ON l.id = p.location_id ";

pub struct PgSettlementsRepo {
    pool: sqlx::PgPool,
}

impl PgSettlementsRepo {
    pub fn new(pool: sqlx::PgPool) -> Self {
        PgSettlementsRepo { pool }
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
impl SettlementsRepo for PgSettlementsRepo {
    async fn list_payouts(
        &self,
        courier_id: Uuid,
        location_id: Uuid,
        status: Option<String>,
    ) -> Result<Vec<PayoutRow>, RepoError> {
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                let rows: Vec<PayoutRow> = if let Some(status) = status {
                    let sql = format!("{PAYOUT_SELECT}WHERE p.courier_id = $1 AND p.status = $2 ORDER BY p.created_at DESC");
                    sqlx::query_as(&sql)
                        .bind(courier_id)
                        .bind(status)
                        .fetch_all(&mut **txn)
                        .await?
                } else {
                    let sql = format!("{PAYOUT_SELECT}WHERE p.courier_id = $1 ORDER BY p.created_at DESC");
                    sqlx::query_as(&sql)
                        .bind(courier_id)
                        .fetch_all(&mut **txn)
                        .await?
                };
                Ok(rows)
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn get_payout(
        &self,
        id: Uuid,
        courier_id: Uuid,
        location_id: Uuid,
    ) -> Result<Option<PayoutRow>, RepoError> {
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                let sql = format!("{PAYOUT_SELECT}WHERE p.id = $1 AND p.courier_id = $2");
                let row: Option<PayoutRow> = sqlx::query_as(&sql)
                    .bind(id)
                    .bind(courier_id)
                    .fetch_optional(&mut **txn)
                    .await?;
                Ok(row)
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn payout_items(
        &self,
        payout_id: Uuid,
        location_id: Uuid,
    ) -> Result<Vec<PayoutItemRow>, RepoError> {
        crate::db::with_tenant(&self.pool, TenantId::from(location_id), move |txn| {
            Box::pin(async move {
                // Strictly no orderId, no assignmentId, no customer phone (settlements.ts:89) —
                // the SELECT list itself is the redaction, not a post-hoc field strip.
                let rows: Vec<PayoutItemRow> = sqlx::query_as(
                    "SELECT ca.delivered_at, si.amount::bigint AS amount, si.currency_code AS currency \
                     FROM settlement_items si JOIN courier_assignments ca ON ca.id = si.assignment_id \
                     WHERE si.payout_id = $1 ORDER BY ca.delivered_at DESC",
                )
                .bind(payout_id)
                .fetch_all(&mut **txn)
                .await?;
                Ok(rows)
            })
        })
        .await
        .map_err(map_txn_err)
    }
}

// ── FakeSettlementsRepo (test-only) ──────────────────────────────────────────────────────────

#[cfg(test)]
pub mod fake {
    use super::{PayoutItemRow, PayoutRow, RepoError, SettlementsRepo};
    use std::collections::HashMap;
    use std::sync::Mutex;
    use uuid::Uuid;

    #[derive(Default)]
    pub struct FakeSettlementsRepo {
        /// payout_id -> (owning courier_id, row). The courier scoping lives here (not on
        /// `PayoutRow`, which never carries a `courier_id` column — matching Node's own SELECT).
        pub payouts: Mutex<HashMap<Uuid, (Uuid, PayoutRow)>>,
        pub items: Mutex<HashMap<Uuid, Vec<PayoutItemRow>>>,
    }

    impl FakeSettlementsRepo {
        pub fn seed_payout(&self, courier_id: Uuid, row: PayoutRow) {
            self.payouts
                .lock()
                .unwrap()
                .insert(row.id, (courier_id, row));
        }
        pub fn seed_items(&self, payout_id: Uuid, items: Vec<PayoutItemRow>) {
            self.items.lock().unwrap().insert(payout_id, items);
        }
    }

    #[async_trait::async_trait]
    impl SettlementsRepo for FakeSettlementsRepo {
        async fn list_payouts(
            &self,
            courier_id: Uuid,
            _location_id: Uuid,
            status: Option<String>,
        ) -> Result<Vec<PayoutRow>, RepoError> {
            Ok(self
                .payouts
                .lock()
                .unwrap()
                .values()
                .filter(|(cid, row)| {
                    *cid == courier_id && status.as_deref().is_none_or(|s| row.status == s)
                })
                .map(|(_, row)| row.clone())
                .collect())
        }

        async fn get_payout(
            &self,
            id: Uuid,
            courier_id: Uuid,
            _location_id: Uuid,
        ) -> Result<Option<PayoutRow>, RepoError> {
            Ok(self
                .payouts
                .lock()
                .unwrap()
                .get(&id)
                .filter(|(cid, _)| *cid == courier_id)
                .map(|(_, row)| row.clone()))
        }

        async fn payout_items(
            &self,
            payout_id: Uuid,
            _location_id: Uuid,
        ) -> Result<Vec<PayoutItemRow>, RepoError> {
            Ok(self
                .items
                .lock()
                .unwrap()
                .get(&payout_id)
                .cloned()
                .unwrap_or_default())
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::fake::FakeSettlementsRepo;
    use super::*;
    use crate::auth::claims::CourierClaims;
    use crate::auth::repo::fake::FakeAuthRepo;
    use axum::body::to_bytes;
    use axum::http::StatusCode;
    use axum::response::Response;
    use std::sync::Arc;

    fn request_id() -> RequestId {
        RequestId::new(axum::http::HeaderValue::from_static("corr-1"))
    }

    fn courier_session(courier_id: Uuid, location_id: Uuid) -> CourierSession {
        CourierSession(CourierClaims::new(courier_id, location_id, None))
    }

    fn state_with(repo: FakeSettlementsRepo) -> SettlementsState {
        SettlementsState {
            auth: AuthState::test_state(Arc::new(FakeAuthRepo::default())),
            repo: Arc::new(repo),
        }
    }

    async fn json_body(resp: Response) -> (StatusCode, serde_json::Value) {
        let status = resp.status();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    fn fixture_payout(id: Uuid, location_id: Uuid, status: &str) -> PayoutRow {
        PayoutRow {
            id,
            location_id,
            location_name: "Eljo's Pizza".to_string(),
            deliveries_count: 12,
            total_earned: 4500,
            period_start: Utc::now(),
            period_end: Utc::now(),
            status: status.to_string(),
            created_at: Utc::now(),
            approved_at: None,
            currency: "ALL".to_string(),
        }
    }

    // ── get_payouts ──

    #[tokio::test]
    async fn get_payouts_200_happy_path() {
        let courier_id = Uuid::new_v4();
        let location = Uuid::new_v4();
        let repo = FakeSettlementsRepo::default();
        repo.seed_payout(
            courier_id,
            fixture_payout(Uuid::new_v4(), location, "pending"),
        );
        let state = state_with(repo);

        let resp = get_payouts(
            Extension(state),
            courier_session(courier_id, location),
            Query(ListPayoutsQuery { status: None }),
            Extension(request_id()),
        )
        .await
        .unwrap()
        .into_response();
        let (status, json) = json_body(resp).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["payouts"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn get_payouts_filters_by_status_query_param() {
        let courier_id = Uuid::new_v4();
        let location = Uuid::new_v4();
        let repo = FakeSettlementsRepo::default();
        repo.seed_payout(
            courier_id,
            fixture_payout(Uuid::new_v4(), location, "pending"),
        );
        repo.seed_payout(courier_id, fixture_payout(Uuid::new_v4(), location, "paid"));
        // A payout belonging to ANOTHER courier must never leak into this courier's filtered list.
        repo.seed_payout(
            Uuid::new_v4(),
            fixture_payout(Uuid::new_v4(), location, "paid"),
        );
        let state = state_with(repo);

        let resp = get_payouts(
            Extension(state),
            courier_session(courier_id, location),
            Query(ListPayoutsQuery {
                status: Some(PayoutStatus::Paid),
            }),
            Extension(request_id()),
        )
        .await
        .unwrap()
        .into_response();
        let (_, json) = json_body(resp).await;
        let payouts = json["payouts"].as_array().unwrap();
        assert_eq!(payouts.len(), 1);
        assert_eq!(payouts[0]["status"], "paid");
    }

    // ── get_payout_detail ──

    #[tokio::test]
    async fn get_payout_detail_200_happy_path_returns_payout_and_items() {
        let courier_id = Uuid::new_v4();
        let location = Uuid::new_v4();
        let payout_id = Uuid::new_v4();
        let repo = FakeSettlementsRepo::default();
        repo.seed_payout(courier_id, fixture_payout(payout_id, location, "paid"));
        repo.seed_items(
            payout_id,
            vec![PayoutItemRow {
                delivered_at: Some(Utc::now()),
                amount: 900,
                currency: "ALL".to_string(),
            }],
        );
        let state = state_with(repo);

        let resp = get_payout_detail(
            Extension(state),
            courier_session(courier_id, location),
            Path(payout_id),
            Extension(request_id()),
        )
        .await
        .unwrap()
        .into_response();
        let (status, json) = json_body(resp).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["payout"]["id"], payout_id.to_string());
        assert_eq!(json["items"].as_array().unwrap().len(), 1);
        assert_eq!(json["items"][0]["amount"], 900);
    }

    #[tokio::test]
    async fn get_payout_detail_404_for_a_foreign_payout() {
        let owner_courier = Uuid::new_v4();
        let attacker_courier = Uuid::new_v4();
        let location = Uuid::new_v4();
        let payout_id = Uuid::new_v4();
        let repo = FakeSettlementsRepo::default();
        repo.seed_payout(owner_courier, fixture_payout(payout_id, location, "paid"));
        let state = state_with(repo);

        let err = crate::error::expect_err(
            get_payout_detail(
                Extension(state),
                courier_session(attacker_courier, location),
                Path(payout_id),
                Extension(request_id()),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
    }

    /// Structural DoD test (Q-PAYOUT-READ-SHARED): the payout-items DTO must literally have no
    /// `orderId`/`assignmentId`/customer-phone field — pin the serialized key set exactly.
    #[test]
    fn get_payout_detail_items_never_include_order_id_or_assignment_id() {
        let item = PayoutItemRow {
            delivered_at: Some(Utc::now()),
            amount: 500,
            currency: "ALL".to_string(),
        };
        let value = serde_json::to_value(&item).unwrap();
        let keys: std::collections::BTreeSet<String> =
            value.as_object().unwrap().keys().cloned().collect();
        let expected: std::collections::BTreeSet<String> = ["delivered_at", "amount", "currency"]
            .into_iter()
            .map(str::to_string)
            .collect();
        assert_eq!(
            keys, expected,
            "payout items must strictly exclude orderId/assignmentId/customer phone"
        );
    }
}
