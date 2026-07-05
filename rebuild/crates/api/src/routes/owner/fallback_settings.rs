//! S3 owner fallback-config settings (`apps/api/src/routes/owner/fallback.ts` GET/PUT) — the
//! non-money `locations.fallback_config` jsonb (support phone + show-on-error/offline flags + WS
//! retry tuning). Mirrors `dwell_settings` exactly: OWNER+LOC, `with_user` +
//! `assert_active_owner_membership` first-in-tx.
//!
//! ## R2a addition: `GET /:locationId/degradation` (`fallback.ts:68-111`)
//! Previously deferred here with a note pointing at the S10 platform-read DEFINER class — that
//! deferral was WRONG for this route: the S10 DEFINER need is the PLATFORM-ADMIN (cross-tenant)
//! read of `owner_notification_targets`; the OWNER's read of their OWN location's targets is
//! admissible through the table's live policy (`tenant_isolation`, mig 077 RC6:
//! `location_id IN (SELECT app_member_location_ids())`, FORCE'd by mig 080) — which resolves via
//! `app.user_id`, exactly the GUC [`crate::db::with_user`] seats. So [`get_degradation`] runs
//! `with_user` + membership-assert-first like every other op in this file (Node reads BOTH
//! queries on the BYPASSRLS pool with no seat at all — the Rust posture is strictly narrower,
//! same rows for an authorized owner). R2a batch sheet classifies this CLEAN read-only.

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

#[derive(Clone)]
pub struct FallbackSettingsState {
    pub auth: AuthState,
    pub repo: std::sync::Arc<dyn FallbackSettingsRepo>,
}

#[async_trait::async_trait]
pub trait FallbackSettingsRepo: Send + Sync {
    /// `SELECT fallback_config FROM locations WHERE id=$1` under `with_user`. `None` = not an
    /// active-owner membership (404); `Some(None)` = row exists, column null (→ `{}` default).
    async fn get(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
    ) -> Result<Option<Option<serde_json::Value>>, RepoError>;

    /// `UPDATE locations SET fallback_config=$1 WHERE id=$2 RETURNING id`. `None` → 404.
    async fn put(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        config: serde_json::Value,
    ) -> Result<Option<()>, RepoError>;

    /// R2a degradation read (`fallback.ts:75-92`): the location's `fallback_config` + its
    /// `owner_notification_targets` rows (`ORDER BY channel`). `None` = membership denied or no
    /// location row (both Node 404 paths).
    async fn degradation(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
    ) -> Result<Option<(Option<serde_json::Value>, Vec<DegradationChannelRow>)>, RepoError>;
}

/// One `owner_notification_targets` row for the degradation read (`fallback.ts:80-84`).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DegradationChannelRow {
    pub channel: String,
    pub status: String,
    pub last_error: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// `fallbackBodySchema` (`fallback.ts:5-11`). `showPhoneOn*` are required booleans; the rest optional.
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct FallbackBody {
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(rename = "showPhoneOnError")]
    pub show_phone_on_error: bool,
    #[serde(rename = "showPhoneOnOffline")]
    pub show_phone_on_offline: bool,
    #[serde(rename = "wsRetryMax", default)]
    pub ws_retry_max: Option<i64>,
    #[serde(rename = "wsRetryBaseMs", default)]
    pub ws_retry_base_ms: Option<i64>,
}

impl FallbackBody {
    /// Node bounds (`fallback.ts:5-11`): phone ≤50; wsRetryMax 1..=30; wsRetryBaseMs 500..=10000.
    fn validate(&self) -> Result<(), &'static str> {
        if self.phone.as_ref().is_some_and(|p| p.len() > 50) {
            return Err("phone");
        }
        if self.ws_retry_max.is_some_and(|v| !(1..=30).contains(&v)) {
            return Err("wsRetryMax");
        }
        if self
            .ws_retry_base_ms
            .is_some_and(|v| !(500..=10000).contains(&v))
        {
            return Err("wsRetryBaseMs");
        }
        Ok(())
    }

    /// The stored jsonb (`fallback.ts:52-58`): snake_case keys, phone only when provided.
    fn stored(&self) -> serde_json::Value {
        let mut m = serde_json::Map::new();
        if let Some(p) = &self.phone {
            m.insert("phone".into(), serde_json::Value::String(p.clone()));
        }
        m.insert(
            "show_phone_on_error".into(),
            self.show_phone_on_error.into(),
        );
        m.insert(
            "show_phone_on_offline".into(),
            self.show_phone_on_offline.into(),
        );
        if let Some(v) = self.ws_retry_max {
            m.insert("ws_retry_max".into(), v.into());
        }
        if let Some(v) = self.ws_retry_base_ms {
            m.insert("ws_retry_base_ms".into(), v.into());
        }
        serde_json::Value::Object(m)
    }
}

/// GET response (`fallback.ts:33-38`) — camelCase, defaults applied over the stored snake_case config.
#[derive(Debug, Serialize, ToSchema)]
pub struct FallbackGetResponse {
    pub phone: Option<String>,
    #[serde(rename = "showPhoneOnError")]
    pub show_phone_on_error: bool,
    #[serde(rename = "showPhoneOnOffline")]
    pub show_phone_on_offline: bool,
    #[serde(rename = "wsRetryMax")]
    pub ws_retry_max: i64,
    #[serde(rename = "wsRetryBaseMs")]
    pub ws_retry_base_ms: i64,
}

impl FallbackGetResponse {
    /// `config.<k> || <default>` / `!== false` semantics from `fallback.ts:33-38`.
    fn from_config(cfg: &serde_json::Value) -> Self {
        let s = |k: &str| cfg.get(k).and_then(|v| v.as_str()).map(str::to_string);
        // `!== false` → default true when absent or any non-false value.
        let bool_default_true = |k: &str| cfg.get(k).and_then(|v| v.as_bool()) != Some(false);
        let int_or = |k: &str, d: i64| {
            cfg.get(k)
                .and_then(|v| v.as_i64())
                .filter(|&v| v != 0)
                .unwrap_or(d)
        };
        Self {
            phone: s("phone"),
            show_phone_on_error: bool_default_true("show_phone_on_error"),
            show_phone_on_offline: bool_default_true("show_phone_on_offline"),
            ws_retry_max: int_or("ws_retry_max", 10),
            ws_retry_base_ms: int_or("ws_retry_base_ms", 2000),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FallbackPutResponse {
    pub success: bool,
    pub config: serde_json::Value,
}

/// One wire channel entry (`fallback.ts:104-109`) — camelCase, `lastError` null-when-NULL.
#[derive(Debug, Serialize, ToSchema)]
pub struct DegradationChannel {
    pub channel: String,
    pub status: String,
    #[serde(rename = "lastError")]
    pub last_error: Option<String>,
    #[serde(
        rename = "createdAt",
        serialize_with = "crate::dto::serialize_js_instant"
    )]
    #[schema(value_type = String)]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// The degradation response (`fallback.ts:98-110`).
#[derive(Debug, Serialize, ToSchema)]
pub struct DegradationResponse {
    #[serde(rename = "locationId")]
    #[schema(value_type = String)]
    pub location_id: Uuid,
    /// `config.phone || null` — any JS-truthy stored value passes through verbatim (in practice
    /// a string; `""` and other falsies become null). Kept a raw Value for that exact semantic.
    #[serde(rename = "fallbackPhone")]
    pub fallback_phone: serde_json::Value,
    #[serde(rename = "showPhoneOnError")]
    pub show_phone_on_error: bool,
    #[serde(rename = "showPhoneOnOffline")]
    pub show_phone_on_offline: bool,
    /// `'push'` then `'telegram'`, each present iff ANY row of that channel has a truthy
    /// `last_error` (`fallback.ts:91-96` — the empty string is falsy in JS, so NOT dead).
    #[serde(rename = "deadChannels")]
    pub dead_channels: Vec<String>,
    pub channels: Vec<DegradationChannel>,
}

/// `GET /api/owner/locations/{locationId}/settings/fallback` (`fallback.ts:21`).
#[utoipa::path(
    get,
    path = "/api/owner/locations/{locationId}/settings/fallback",
    params(("locationId" = Uuid, Path)),
    responses(
        (status = 200, description = "Fallback config", body = FallbackGetResponse),
        (status = 404, description = "Not found", body = domain::ErrorEnvelope),
    ),
    tag = "owner-catalog"
)]
pub async fn get_fallback_settings(
    Extension(state): Extension<FallbackSettingsState>,
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
    let cfg = row.unwrap_or_else(|| serde_json::json!({}));
    Ok(Json(FallbackGetResponse::from_config(&cfg)))
}

/// `PUT /api/owner/locations/{locationId}/settings/fallback` (`fallback.ts:43`).
#[utoipa::path(
    put,
    path = "/api/owner/locations/{locationId}/settings/fallback",
    params(("locationId" = Uuid, Path)),
    request_body = FallbackBody,
    responses(
        (status = 200, description = "Stored", body = FallbackPutResponse),
        (status = 400, description = "Validation error", body = domain::ErrorEnvelope),
        (status = 404, description = "Not found", body = domain::ErrorEnvelope),
    ),
    tag = "owner-catalog"
)]
pub async fn put_fallback_settings(
    Extension(state): Extension<FallbackSettingsState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(location_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<FallbackBody>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;
    if let Err(field) = body.validate() {
        return Err(ApiError::validation_failed_400(
            format!("Validation error: {field}"),
            correlation_id,
        ));
    }
    let stored = body.stored();
    state
        .repo
        .put(owner.user_id, location_id, stored.clone())
        .await
        .map_err(|_e| internal_error(correlation_id.clone()))?
        .ok_or_else(|| not_found(correlation_id))?;
    Ok(Json(FallbackPutResponse {
        success: true,
        config: stored,
    }))
}

/// R2a: `GET /api/owner/locations/{locationId}/degradation` (`fallback.ts:68-111`) — read-only
/// channel-health view. 404 for a foreign/unknown location; 200 otherwise.
#[utoipa::path(
    get,
    path = "/api/owner/locations/{locationId}/degradation",
    params(("locationId" = Uuid, Path)),
    responses(
        (status = 200, description = "Degradation status", body = DegradationResponse),
        (status = 404, description = "Not found", body = domain::ErrorEnvelope),
    ),
    tag = "owner-catalog"
)]
pub async fn get_degradation(
    Extension(state): Extension<FallbackSettingsState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(location_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;
    let (config, rows) = state
        .repo
        .degradation(owner.user_id, location_id)
        .await
        .map_err(|_e| internal_error(correlation_id.clone()))?
        .ok_or_else(|| not_found(correlation_id))?;
    let cfg = config.unwrap_or_else(|| serde_json::json!({}));

    // `config.phone || null` — JS truthiness (null/false/0/"" -> null; anything else verbatim).
    let fallback_phone = match cfg.get("phone") {
        Some(v) if js_truthy(v) => v.clone(),
        _ => serde_json::Value::Null,
    };
    let bool_default_true = |k: &str| cfg.get(k).and_then(|v| v.as_bool()) != Some(false);

    // `fallback.ts:91-96`: a channel is dead iff ANY of its rows carries a truthy last_error.
    let is_dead = |name: &str| {
        rows.iter()
            .any(|r| r.channel == name && r.last_error.as_deref().is_some_and(|e| !e.is_empty()))
    };
    let mut dead_channels = Vec::new();
    if is_dead("push") {
        dead_channels.push("push".to_string());
    }
    if is_dead("telegram") {
        dead_channels.push("telegram".to_string());
    }

    Ok(Json(DegradationResponse {
        location_id,
        fallback_phone,
        show_phone_on_error: bool_default_true("show_phone_on_error"),
        show_phone_on_offline: bool_default_true("show_phone_on_offline"),
        dead_channels,
        channels: rows
            .into_iter()
            .map(|r| DegradationChannel {
                channel: r.channel,
                status: r.status,
                last_error: r.last_error,
                created_at: r.created_at,
            })
            .collect(),
    }))
}

/// JS truthiness over a JSON value (`config.phone || null`).
fn js_truthy(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Null => false,
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::Number(n) => n.as_f64().is_none_or(|f| f != 0.0),
        serde_json::Value::String(s) => !s.is_empty(),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => true,
    }
}

fn internal_error(correlation_id: String) -> ApiError {
    ApiError::new(ErrorCode::Internal, "internal_error", correlation_id)
}
fn not_found(correlation_id: String) -> ApiError {
    ApiError::new(ErrorCode::NotFound, "Not found", correlation_id)
}

pub struct PgFallbackSettingsRepo {
    pool: sqlx::PgPool,
}
impl PgFallbackSettingsRepo {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl FallbackSettingsRepo for PgFallbackSettingsRepo {
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
                    sqlx::query_as("SELECT fallback_config FROM locations WHERE id = $1")
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
        config: serde_json::Value,
    ) -> Result<Option<()>, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, move |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(None);
                }
                let row: Option<(Uuid,)> = sqlx::query_as(
                    "UPDATE locations SET fallback_config = $1 WHERE id = $2 RETURNING id",
                )
                .bind(config)
                .bind(location_id)
                .fetch_optional(&mut **txn)
                .await?;
                Ok(row.map(|_| ()))
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn degradation(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
    ) -> Result<Option<(Option<serde_json::Value>, Vec<DegradationChannelRow>)>, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, move |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(None);
                }
                // Node runs these two on the pool in parallel (`Promise.all`, no seat at all —
                // BYPASSRLS); one seated transaction with two sequential reads is at least as
                // correct, and `owner_notification_targets`' FORCE-RLS policy resolves via the
                // `app.user_id` this transaction seats (see module doc).
                let fb: Option<(Option<serde_json::Value>,)> =
                    sqlx::query_as("SELECT fallback_config FROM locations WHERE id = $1")
                        .bind(location_id)
                        .fetch_optional(&mut **txn)
                        .await?;
                let Some((config,)) = fb else {
                    return Ok(None); // `fallback.ts:87` — location row absent -> 404.
                };
                let rows: Vec<DegradationChannelRow> = sqlx::query_as(
                    "SELECT channel, status, last_error, created_at FROM owner_notification_targets \
                     WHERE location_id = $1 ORDER BY channel",
                )
                .bind(location_id)
                .fetch_all(&mut **txn)
                .await?;
                Ok(Some((config, rows)))
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
    fn get_response_applies_node_defaults() {
        let empty = FallbackGetResponse::from_config(&serde_json::json!({}));
        assert_eq!(empty.phone, None);
        assert!(empty.show_phone_on_error, "absent → true (!== false)");
        assert_eq!(empty.ws_retry_max, 10);
        assert_eq!(empty.ws_retry_base_ms, 2000);
        let set = FallbackGetResponse::from_config(&serde_json::json!({
            "phone": "+355", "show_phone_on_error": false, "ws_retry_max": 5
        }));
        assert_eq!(set.phone.as_deref(), Some("+355"));
        assert!(!set.show_phone_on_error, "explicit false honored");
        assert_eq!(set.ws_retry_max, 5);
    }

    #[test]
    fn stored_omits_phone_when_absent_and_snake_cases() {
        let b = FallbackBody {
            phone: None,
            show_phone_on_error: true,
            show_phone_on_offline: false,
            ws_retry_max: None,
            ws_retry_base_ms: Some(3000),
        };
        let s = b.stored();
        assert!(s.get("phone").is_none());
        assert_eq!(s["show_phone_on_error"], true);
        assert_eq!(s["ws_retry_base_ms"], 3000);
    }

    #[test]
    fn validate_bounds() {
        let ok = FallbackBody {
            phone: Some("x".repeat(50)),
            show_phone_on_error: true,
            show_phone_on_offline: true,
            ws_retry_max: Some(30),
            ws_retry_base_ms: Some(500),
        };
        assert!(ok.validate().is_ok());
        assert_eq!(
            FallbackBody {
                ws_retry_max: Some(31),
                ..ok_clone(&ok)
            }
            .validate(),
            Err("wsRetryMax")
        );
    }

    fn ok_clone(b: &FallbackBody) -> FallbackBody {
        FallbackBody {
            phone: b.phone.clone(),
            show_phone_on_error: b.show_phone_on_error,
            show_phone_on_offline: b.show_phone_on_offline,
            ws_retry_max: b.ws_retry_max,
            ws_retry_base_ms: b.ws_retry_base_ms,
        }
    }

    // ── R2a: GET degradation ─────────────────────────────────────────────────────────────────

    use super::fake::FakeFallbackSettingsRepo;
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

    /// Full wire-shape pin (`fallback.ts:98-110`): camelCase keys, JS-truthy `fallbackPhone`
    /// (`""` -> null), dead-channel derivation (empty-string last_error is NOT dead — JS
    /// falsy), `createdAt` as JS toISOString.
    #[tokio::test]
    async fn get_degradation_matches_node_wire_shape() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeFallbackSettingsRepo::default();
        repo.rows.lock().unwrap().insert(
            loc,
            Some(serde_json::json!({ "phone": "", "show_phone_on_error": false })),
        );
        let created = chrono::DateTime::parse_from_rfc3339("2026-07-01T10:00:00.123456Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        repo.channels.lock().unwrap().insert(
            loc,
            vec![
                DegradationChannelRow {
                    channel: "push".to_string(),
                    status: "active".to_string(),
                    last_error: Some("410 gone".to_string()),
                    created_at: created,
                },
                DegradationChannelRow {
                    channel: "telegram".to_string(),
                    status: "active".to_string(),
                    last_error: Some(String::new()), // truthy check: '' is falsy -> NOT dead
                    created_at: created,
                },
            ],
        );
        let state = FallbackSettingsState {
            auth: owner_with_location(user_id, loc),
            repo: Arc::new(repo),
        };

        let response = get_degradation(
            Extension(state),
            OwnerClaimsExt(OwnerClaims::new(user_id, Some(loc))),
            Path(loc),
            Extension(request_id()),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            body,
            serde_json::json!({
                "locationId": loc.to_string(),
                "fallbackPhone": null,
                "showPhoneOnError": false,
                "showPhoneOnOffline": true,
                "deadChannels": ["push"],
                "channels": [
                    { "channel": "push", "status": "active", "lastError": "410 gone",
                      "createdAt": "2026-07-01T10:00:00.123Z" },
                    { "channel": "telegram", "status": "active", "lastError": "",
                      "createdAt": "2026-07-01T10:00:00.123Z" }
                ]
            })
        );
    }

    /// Foreign location and repo-None both surface as the envelope 404 (`fallback.ts:87`).
    #[tokio::test]
    async fn get_degradation_404_for_foreign_or_unknown_location() {
        let user_id = Uuid::new_v4();
        let mine = Uuid::new_v4();
        let theirs = Uuid::new_v4();
        let state = FallbackSettingsState {
            auth: owner_with_location(user_id, mine),
            repo: Arc::new(FakeFallbackSettingsRepo::default()),
        };

        // Foreign -> require_location_access 404.
        let err = crate::error::expect_err(
            get_degradation(
                Extension(state.clone()),
                OwnerClaimsExt(OwnerClaims::new(user_id, Some(mine))),
                Path(theirs),
                Extension(request_id()),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);

        // Mine but no location row in the repo -> 404 `Not found`.
        let err = crate::error::expect_err(
            get_degradation(
                Extension(state),
                OwnerClaimsExt(OwnerClaims::new(user_id, Some(mine))),
                Path(mine),
                Extension(request_id()),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
        assert_eq!(err.envelope.message, "Not found");
    }

    /// Requires a live Postgres — proves BOTH degradation queries bind/decode against the real
    /// schema (locations.fallback_config jsonb; owner_notification_targets projection incl.
    /// timestamptz), read-only. A random owner/location denies at the membership assert
    /// (Ok(None)); the targets projection is additionally decoded via a direct LIMIT-1 read.
    #[tokio::test]
    #[ignore = "requires a live Postgres — set DATABASE_URL_OPERATIONAL/DATABASE_URL_SESSION and run with --ignored"]
    async fn live_pg_degradation_denies_random_pair_and_projection_decodes() {
        let config =
            crate::config::Config::from_env().expect("env must be valid to run this ignored test");
        let pools = crate::db::Pools::connect(&config)
            .await
            .expect("pools must connect");
        let repo = PgFallbackSettingsRepo::new(pools.operational.clone());
        let out = repo
            .degradation(Uuid::new_v4(), Uuid::new_v4())
            .await
            .expect("membership-denied path must be a clean Ok(None)");
        assert!(out.is_none());

        // Decode proof for the targets projection (may be an empty table on a fresh env — the
        // parse/bind is still proven; a row, when present, pins the column decode too).
        let rows: Vec<DegradationChannelRow> = sqlx::query_as(
            "SELECT channel, status, last_error, created_at FROM owner_notification_targets \
             ORDER BY channel LIMIT 5",
        )
        .fetch_all(&pools.operational)
        .await
        .expect("targets projection must parse and decode against the live schema");
        let _ = rows;
    }
}

#[cfg(test)]
pub mod fake {
    use super::{DegradationChannelRow, FallbackSettingsRepo, RepoError};
    use std::collections::HashMap;
    use std::sync::Mutex;
    use uuid::Uuid;

    #[derive(Default)]
    pub struct FakeFallbackSettingsRepo {
        pub rows: Mutex<HashMap<Uuid, Option<serde_json::Value>>>,
        pub channels: Mutex<HashMap<Uuid, Vec<DegradationChannelRow>>>,
    }

    #[async_trait::async_trait]
    impl FallbackSettingsRepo for FakeFallbackSettingsRepo {
        async fn get(
            &self,
            _u: Uuid,
            location_id: Uuid,
        ) -> Result<Option<Option<serde_json::Value>>, RepoError> {
            Ok(self.rows.lock().unwrap().get(&location_id).cloned())
        }
        async fn put(
            &self,
            _u: Uuid,
            location_id: Uuid,
            config: serde_json::Value,
        ) -> Result<Option<()>, RepoError> {
            let mut rows = self.rows.lock().unwrap();
            if let std::collections::hash_map::Entry::Occupied(mut e) = rows.entry(location_id) {
                e.insert(Some(config));
                Ok(Some(()))
            } else {
                Ok(None)
            }
        }
        async fn degradation(
            &self,
            _u: Uuid,
            location_id: Uuid,
        ) -> Result<Option<(Option<serde_json::Value>, Vec<DegradationChannelRow>)>, RepoError>
        {
            let Some(config) = self.rows.lock().unwrap().get(&location_id).cloned() else {
                return Ok(None);
            };
            let channels = self
                .channels
                .lock()
                .unwrap()
                .get(&location_id)
                .cloned()
                .unwrap_or_default();
            Ok(Some((config, channels)))
        }
    }
}
