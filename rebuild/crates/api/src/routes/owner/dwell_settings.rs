//! S3 owner dwell-thresholds settings (`apps/api/src/routes/owner/dwell-settings.ts`) — GET/PUT of
//! `locations.dwell_thresholds` (jsonb). Two ops, both owner+location-scoped, seated via
//! `db::with_user` with `assert_active_owner_membership` as the first in-tx statement (the S3
//! owner-write seam — the same discipline as `categories`/`products`). Node used a raw
//! `withTenant(db, userId)` read/write; the port routes it through `with_user` for post-B3
//! correctness (the S3 council's owner-write ruling), behavior-identical under BYPASSRLS today.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use axum::Json;
use axum::extract::{Extension, Path};
use axum::response::IntoResponse;

use domain::ErrorCode;

use crate::auth::AuthState;
use crate::auth::extractors::OwnerClaimsExt;
use crate::error::ApiError;
use crate::repo::RepoError;
use crate::routes::correlation_id_string;
use tower_http::request_id::RequestId;

use super::{assert_active_owner_membership, require_location_access};

// ── State + repo trait ──────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct DwellSettingsState {
    pub auth: AuthState,
    pub repo: std::sync::Arc<dyn DwellSettingsRepo>,
}

#[async_trait::async_trait]
pub trait DwellSettingsRepo: Send + Sync {
    /// `SELECT dwell_thresholds FROM locations WHERE id=$1` under `with_user`. `Ok(None)` = the
    /// location is not an active-owner membership of this user (→ 404); `Ok(Some(None))` = the row
    /// exists but the column is null (→ caller substitutes DEFAULT).
    async fn get(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
    ) -> Result<Option<Option<serde_json::Value>>, RepoError>;

    /// `UPDATE locations SET dwell_thresholds=$1 WHERE id=$2 RETURNING id`. `Ok(None)` → 404.
    async fn put(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        stored: serde_json::Value,
    ) -> Result<Option<()>, RepoError>;
}

// ── DTOs ────────────────────────────────────────────────────────────────────────────────────

/// `dwell-thresholds.ts` `inputSchema` — every field an integer-seconds bound. `v` is NOT accepted
/// on input (the server stamps `v:1`), matching Node's `{ v: 1, ...dwellThresholds }` store shape.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct DwellThresholdsInput {
    pub pending_s: i64,
    pub confirmed_s: i64,
    pub preparing_s: i64,
    pub en_route_s: i64,
}

impl DwellThresholdsInput {
    /// Node's zod bounds (`dwell-settings.ts:7-12`): pending/confirmed 10..=3600,
    /// preparing/en_route 10..=7200. Returns the offending field name on the first violation.
    fn validate(&self) -> Result<(), &'static str> {
        let in_range = |v: i64, lo: i64, hi: i64| v >= lo && v <= hi;
        if !in_range(self.pending_s, 10, 3600) {
            return Err("pending_s");
        }
        if !in_range(self.confirmed_s, 10, 3600) {
            return Err("confirmed_s");
        }
        if !in_range(self.preparing_s, 10, 7200) {
            return Err("preparing_s");
        }
        if !in_range(self.en_route_s, 10, 7200) {
            return Err("en_route_s");
        }
        Ok(())
    }

    /// The `{ v: 1, ...input }` stored jsonb (`dwell-settings.ts:52`).
    fn stored(&self) -> serde_json::Value {
        serde_json::json!({
            "v": 1,
            "pending_s": self.pending_s,
            "confirmed_s": self.confirmed_s,
            "preparing_s": self.preparing_s,
            "en_route_s": self.en_route_s,
        })
    }
}

/// `DEFAULT_DWELL_THRESHOLDS` (`lib/dwell-thresholds.ts:14-20`) — served when the column is null.
fn default_thresholds() -> serde_json::Value {
    serde_json::json!({
        "v": 1, "pending_s": 60, "confirmed_s": 300, "preparing_s": 600, "en_route_s": 900
    })
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DwellSettingsResponse {
    #[serde(rename = "dwellThresholds")]
    pub dwell_thresholds: serde_json::Value,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PutDwellRequest {
    #[serde(rename = "dwellThresholds")]
    pub dwell_thresholds: DwellThresholdsInput,
}

// ── Handlers ────────────────────────────────────────────────────────────────────────────────

/// `GET /api/owner/locations/{locationId}/settings/dwell` (`dwell-settings.ts:24`).
#[utoipa::path(
    get,
    path = "/api/owner/locations/{locationId}/settings/dwell",
    params(("locationId" = Uuid, Path)),
    responses(
        (status = 200, description = "Dwell thresholds", body = DwellSettingsResponse),
        (status = 404, description = "Not found", body = domain::ErrorEnvelope),
    ),
    tag = "owner-catalog"
)]
pub async fn get_dwell_settings(
    Extension(state): Extension<DwellSettingsState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(location_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    let row = state
        .repo
        .get(owner.user_id, location_id)
        .await
        .map_err(|_e| internal_error(correlation_id.clone()))?
        .ok_or_else(|| not_found(correlation_id))?;

    // `res.rows[0]?.dwell_thresholds || DEFAULT` (dwell-settings.ts:39): null/absent → default.
    let dwell_thresholds = row
        .filter(|v| !v.is_null())
        .unwrap_or_else(default_thresholds);
    Ok(Json(DwellSettingsResponse { dwell_thresholds }))
}

/// `PUT /api/owner/locations/{locationId}/settings/dwell` (`dwell-settings.ts:45`).
#[utoipa::path(
    put,
    path = "/api/owner/locations/{locationId}/settings/dwell",
    params(("locationId" = Uuid, Path)),
    request_body = PutDwellRequest,
    responses(
        (status = 200, description = "Stored thresholds", body = DwellSettingsResponse),
        (status = 400, description = "Validation error", body = domain::ErrorEnvelope),
        (status = 404, description = "Not found", body = domain::ErrorEnvelope),
    ),
    tag = "owner-catalog"
)]
pub async fn put_dwell_settings(
    Extension(state): Extension<DwellSettingsState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(location_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<PutDwellRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    if let Err(field) = body.dwell_thresholds.validate() {
        // 400 (not the ValidationFailed default 422) — Node's zod body validation returns 400.
        return Err(ApiError::validation_failed_400(
            format!("Validation error: {field} out of range"),
            correlation_id,
        ));
    }

    let stored = body.dwell_thresholds.stored();
    state
        .repo
        .put(owner.user_id, location_id, stored.clone())
        .await
        .map_err(|_e| internal_error(correlation_id.clone()))?
        .ok_or_else(|| not_found(correlation_id))?;

    Ok(Json(DwellSettingsResponse {
        dwell_thresholds: stored,
    }))
}

fn internal_error(correlation_id: String) -> ApiError {
    ApiError::new(ErrorCode::Internal, "internal_error", correlation_id)
}
fn not_found(correlation_id: String) -> ApiError {
    ApiError::new(ErrorCode::NotFound, "Not found", correlation_id)
}

// ── PgDwellSettingsRepo ─────────────────────────────────────────────────────────────────────

pub struct PgDwellSettingsRepo {
    pool: sqlx::PgPool,
}

impl PgDwellSettingsRepo {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl DwellSettingsRepo for PgDwellSettingsRepo {
    async fn get(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
    ) -> Result<Option<Option<serde_json::Value>>, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, move |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(None);
                }
                let row: Option<(Option<serde_json::Value>,)> =
                    sqlx::query_as("SELECT dwell_thresholds FROM locations WHERE id = $1")
                        .bind(location_id)
                        .fetch_optional(&mut **txn)
                        .await?;
                Ok(row.map(|(v,)| v))
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn put(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        stored: serde_json::Value,
    ) -> Result<Option<()>, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, move |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(None);
                }
                let row: Option<(Uuid,)> = sqlx::query_as(
                    "UPDATE locations SET dwell_thresholds = $1 WHERE id = $2 RETURNING id",
                )
                .bind(stored)
                .bind(location_id)
                .fetch_optional(&mut **txn)
                .await?;
                Ok(row.map(|_| ()))
            })
        })
        .await
        .map_err(map_txn_err)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_enforces_node_zod_bounds() {
        let ok = DwellThresholdsInput {
            pending_s: 60,
            confirmed_s: 300,
            preparing_s: 600,
            en_route_s: 900,
        };
        assert!(ok.validate().is_ok());
        assert_eq!(
            DwellThresholdsInput {
                pending_s: 9,
                ..clone_of(&ok)
            }
            .validate(),
            Err("pending_s")
        );
        assert_eq!(
            DwellThresholdsInput {
                preparing_s: 7201,
                ..clone_of(&ok)
            }
            .validate(),
            Err("preparing_s")
        );
    }

    #[test]
    fn stored_stamps_v1_and_default_matches_node() {
        let stored = DwellThresholdsInput {
            pending_s: 60,
            confirmed_s: 300,
            preparing_s: 600,
            en_route_s: 900,
        }
        .stored();
        assert_eq!(stored["v"], 1);
        assert_eq!(
            stored,
            default_thresholds(),
            "default == the canonical stored shape"
        );
    }

    fn clone_of(x: &DwellThresholdsInput) -> DwellThresholdsInput {
        DwellThresholdsInput {
            pending_s: x.pending_s,
            confirmed_s: x.confirmed_s,
            preparing_s: x.preparing_s,
            en_route_s: x.en_route_s,
        }
    }
}

/// Mutex-backed stub for handler tests (mirrors `categories::fake`) — no live Postgres needed.
#[cfg(test)]
pub mod fake {
    use super::{DwellSettingsRepo, RepoError};
    use std::collections::HashMap;
    use std::sync::Mutex;
    use uuid::Uuid;

    #[derive(Default)]
    pub struct FakeDwellSettingsRepo {
        /// location_id -> stored dwell_thresholds jsonb (absent = a location the owner cannot see → 404).
        pub rows: Mutex<HashMap<Uuid, Option<serde_json::Value>>>,
    }

    #[async_trait::async_trait]
    impl DwellSettingsRepo for FakeDwellSettingsRepo {
        async fn get(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
        ) -> Result<Option<Option<serde_json::Value>>, RepoError> {
            Ok(self.rows.lock().unwrap().get(&location_id).cloned())
        }

        async fn put(
            &self,
            _owner_user_id: Uuid,
            location_id: Uuid,
            stored: serde_json::Value,
        ) -> Result<Option<()>, RepoError> {
            let mut rows = self.rows.lock().unwrap();
            if let std::collections::hash_map::Entry::Occupied(mut e) = rows.entry(location_id) {
                e.insert(Some(stored));
                Ok(Some(()))
            } else {
                Ok(None)
            }
        }
    }
}
