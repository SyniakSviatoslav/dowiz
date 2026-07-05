//! R2a: `PATCH /api/owner/locations/:locationId` (`apps/api/src/routes/owner/locations.ts`) —
//! the owner's partial location update. Same OWNER+LOC seam as every S3 owner-config op:
//! `require_location_access` out-of-band fast-path, then `with_user` +
//! [`super::assert_active_owner_membership`] as the FIRST in-transaction statement (session
//! lesson: a bare `UPDATE ... WHERE id=$1` under FORCE-RLS returns 0 rows without the membership
//! context that first read establishes — the assert IS the RLS-admissibility seat check).
//!
//! ## The dynamic SET (carried verbatim, allowlist-bounded)
//! Node builds `SET <k> = $n` straight from the body keys (`locations.ts:49-60`) — safe there
//! only because Zod `.strict()` bounds the key set. Here the same bound holds structurally: only
//! the 13 schema keys parse into [`LocationsPatch`], each with a FIXED column identifier — no
//! request string ever reaches the SQL text. Two Node quirks carried on purpose:
//! - `delivery_address` is accepted by the schema but `locations` has NO such column
//!   (grep the migrations: only `address` exists) — Node's UPDATE fails at Postgres and 500s.
//!   Same here: the column name goes into the SET verbatim and the statement errors -> 500.
//!   FLAGGED in the R2a report as a latent Node defect (CARRY-VERBATIM, not a port liberty).
//! - `.optional()` vs `.nullish()` is load-bearing: an explicit `null` on an `.optional()`-only
//!   field (name/phone/currency_code/delivery_fee_flat/tax_rate/default_locale/
//!   supported_locales) is a Zod 400 in Node; only the `.nullish()` fields
//!   (min_order_value/free_delivery_threshold/delivery_radius_km/lat/lng/delivery_address) may
//!   SET NULL. Parsed field-by-field below (`present-vs-null-vs-absent`), never a blanket
//!   `Option`.
//!
//! ## Response: `RETURNING *`, node-postgres rendering, byte-for-byte
//! Node replies with the RAW row (`res.rows[0]`) — snake_case column names, node-pg's default
//! type rendering (NO custom `setTypeParser` anywhere in the Node tree, grep-verified):
//! `numeric` -> STRING (Postgres text rendering, e.g. `"20.00"`), `int4` -> number,
//! `timestamptz` -> `Date` -> `toISOString()` (millisecond precision), `jsonb` -> object,
//! `text[]` -> array. [`LocationRow`] pins all 53 live columns with those exact treatments
//! (`::text` on the two numeric columns; `serialize_js_instant*` on the six timestamps).
//! ⚠️ SCHEMA-DRIFT FLAG: `RETURNING *` in Node auto-includes any future column; this port's
//! explicit list will lag a new migration until re-synced — surfaced by the parity oracle, noted
//! here so the diff is a known class, not a mystery.

use serde::Serialize;
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
pub struct LocationsState {
    pub auth: AuthState,
    pub repo: std::sync::Arc<dyn LocationsRepo>,
}

/// One parsed update — a FIXED column identifier plus its typed bind. The column string is
/// never taken from the request (see module doc).
#[derive(Debug, Clone)]
pub enum LocationUpdate {
    Text(&'static str, String),
    /// `.nullish()` text (delivery_address) — `None` = SET NULL.
    TextNull(&'static str, Option<String>),
    TextArray(&'static str, Vec<String>),
    /// `.optional()` int (`delivery_fee_flat`) — always present when listed.
    Int(&'static str, i32),
    /// `.nullish()` ints (min_order_value / free_delivery_threshold).
    IntNull(&'static str, Option<i32>),
    /// `.optional()` float (tax_rate — numeric column, PG assignment-casts the f64).
    Float(&'static str, f64),
    /// `.nullish()` floats (delivery_radius_km / lat / lng).
    FloatNull(&'static str, Option<f64>),
}

impl LocationUpdate {
    pub fn column(&self) -> &'static str {
        match self {
            LocationUpdate::Text(c, _)
            | LocationUpdate::TextNull(c, _)
            | LocationUpdate::TextArray(c, _)
            | LocationUpdate::Int(c, _)
            | LocationUpdate::IntNull(c, _)
            | LocationUpdate::Float(c, _)
            | LocationUpdate::FloatNull(c, _) => c,
        }
    }
}

#[async_trait::async_trait]
pub trait LocationsRepo: Send + Sync {
    /// The dynamic `UPDATE locations SET ... WHERE id = $1 RETURNING <all 53 columns>` under
    /// `with_user` + first-in-tx membership assert. `None` = not the caller's active-owner
    /// location OR no row matched (both are Node's 404 paths).
    async fn update(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        updates: Vec<LocationUpdate>,
    ) -> Result<Option<LocationRow>, RepoError>;
}

/// The full `locations` row (all 53 live columns, ordinal order), node-postgres rendering —
/// see module doc. Snake_case field names ARE the wire names (Node sends the raw row).
#[derive(Debug, Clone, Serialize, sqlx::FromRow, ToSchema)]
pub struct LocationRow {
    pub id: Uuid,
    pub org_id: Uuid,
    pub slug: String,
    pub name: String,
    pub phone: String,
    pub status: String,
    pub busy_mode: bool,
    pub confirm_timeout_min: i32,
    /// `numeric` -> node-pg STRING (`::text` in the projection preserves the stored scale).
    pub delivery_radius_km: Option<String>,
    #[serde(serialize_with = "crate::dto::serialize_js_number_opt")]
    pub lat: Option<f64>,
    #[serde(serialize_with = "crate::dto::serialize_js_number_opt")]
    pub lng: Option<f64>,
    pub closed_message: Option<String>,
    pub menu_version: i32,
    pub custom_domain: Option<String>,
    #[serde(serialize_with = "crate::dto::serialize_js_instant_opt")]
    #[schema(value_type = Option<String>)]
    pub domain_verified_at: Option<chrono::DateTime<chrono::Utc>>,
    pub widget_enabled: bool,
    pub customer_otp_required: bool,
    #[serde(serialize_with = "crate::dto::serialize_js_instant")]
    #[schema(value_type = String)]
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub default_locale: String,
    pub supported_locales: Vec<String>,
    pub currency_code: String,
    pub currency_minor_unit: i32,
    /// `numeric` -> node-pg STRING.
    pub tax_rate: String,
    pub price_includes_tax: bool,
    pub min_order_value: Option<i32>,
    pub free_delivery_threshold: Option<i32>,
    pub delivery_fee_flat: Option<i32>,
    pub delivery_polygon: Option<serde_json::Value>,
    pub require_phone_otp: bool,
    pub onboarding_state: serde_json::Value,
    #[serde(serialize_with = "crate::dto::serialize_js_instant_opt")]
    #[schema(value_type = Option<String>)]
    pub onboarding_completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub dwell_thresholds: serde_json::Value,
    pub retention_days: i32,
    pub fallback_config: serde_json::Value,
    pub rate_limit_overrides: serde_json::Value,
    pub address: Option<String>,
    pub public_phone: Option<String>,
    pub hours_json: Option<serde_json::Value>,
    pub geo: Option<serde_json::Value>,
    pub delivery_paused: bool,
    #[serde(serialize_with = "crate::dto::serialize_js_instant_opt")]
    #[schema(value_type = Option<String>)]
    pub published_at: Option<chrono::DateTime<chrono::Utc>>,
    pub pickup_enabled: bool,
    #[serde(serialize_with = "crate::dto::serialize_js_instant_opt")]
    #[schema(value_type = Option<String>)]
    pub menu_confirmed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub telegram_alert_detail: String,
    pub timezone: Option<String>,
    pub plan: String,
    #[serde(serialize_with = "crate::dto::serialize_js_instant_opt")]
    #[schema(value_type = Option<String>)]
    pub kitchen_busy_until: Option<chrono::DateTime<chrono::Utc>>,
    pub eta_cap_min: i32,
    pub dispatch_margin_min: i32,
    pub material_shift_min: i32,
    pub otp_target_pct: i32,
    pub min_window_width_min: i32,
    pub geofence_radius_m: i32,
}

/// The `RETURNING` projection — every live column, `::text` on the numerics (node-pg string
/// parity), ordinal order (matches Node's `*` key order).
const RETURNING_COLS: &str = "id, org_id, slug, name, phone, status, busy_mode, \
    confirm_timeout_min, delivery_radius_km::text AS delivery_radius_km, lat, lng, \
    closed_message, menu_version, custom_domain::text AS custom_domain, domain_verified_at, \
    widget_enabled, customer_otp_required, created_at, default_locale, supported_locales, \
    currency_code, currency_minor_unit, tax_rate::text AS tax_rate, price_includes_tax, \
    min_order_value, free_delivery_threshold, delivery_fee_flat, delivery_polygon, \
    require_phone_otp, onboarding_state, onboarding_completed_at, dwell_thresholds, \
    retention_days, fallback_config, rate_limit_overrides, address, public_phone, hours_json, \
    geo, delivery_paused, published_at, pickup_enabled, menu_confirmed_at, \
    telegram_alert_detail, timezone, plan, kitchen_busy_until, eta_cap_min, \
    dispatch_margin_min, material_shift_min, otp_target_pct, min_window_width_min, \
    geofence_radius_m";

// ── Body parsing (locations.ts:17-31 — the Zod schema, field-exact) ─────────────────────────

/// Zod-parity error: 400 VALIDATION_FAILED (never axum's 422 default — ledger #78 class).
fn zod_400(message: impl Into<String>, correlation_id: &str) -> ApiError {
    ApiError::validation_failed_400(message, correlation_id.to_string())
}

/// Parse + validate the PATCH body against `locations.ts:17-31` exactly: `.strict()` key set,
/// per-field bounds, and the `.optional()`-vs-`.nullish()` null-admissibility split. Key order:
/// Node iterates `Object.entries` (insertion order); serde_json's default map is a BTreeMap
/// (alphabetical) — the SET-clause order differs but is semantically inert (each column appears
/// at most once), so no behavior diverges.
fn parse_updates(
    body: &serde_json::Value,
    correlation_id: &str,
) -> Result<Vec<LocationUpdate>, ApiError> {
    use serde_json::Value;
    let obj = body
        .as_object()
        .ok_or_else(|| zod_400("body must be an object", correlation_id))?;

    let mut updates = Vec::with_capacity(obj.len());
    for (key, v) in obj {
        let bad = |m: &str| zod_400(format!("{key}: {m}"), correlation_id);
        // Helper shapes. `.optional()` fields reject explicit null; `.nullish()` accept it.
        let opt_str = |v: &Value| -> Result<String, ApiError> {
            v.as_str()
                .map(str::to_string)
                .ok_or_else(|| bad("must be a string"))
        };
        let num = |v: &Value| -> Result<f64, ApiError> {
            v.as_f64().ok_or_else(|| bad("must be a number"))
        };
        // Zod `.int()` is `Number.isInteger` — `250.0` PASSES (integral float), `250.5` fails.
        let int = |v: &Value| -> Result<i32, ApiError> {
            let f = v.as_f64().ok_or_else(|| bad("must be a number"))?;
            if f.fract() != 0.0 || !(f64::from(i32::MIN)..=f64::from(i32::MAX)).contains(&f) {
                return Err(bad("must be an integer"));
            }
            #[allow(
                clippy::as_conversions,
                reason = "guarded: fract()==0 and within i32 bounds — exact f64→i32"
            )]
            Ok(f as i32)
        };
        let update = match key.as_str() {
            "default_locale" => {
                let s = opt_str(v)?;
                if s.len() < 2 {
                    return Err(bad("min length 2"));
                }
                LocationUpdate::Text("default_locale", s)
            }
            "supported_locales" => {
                let arr = v.as_array().ok_or_else(|| bad("must be an array"))?;
                let mut out = Vec::with_capacity(arr.len());
                for item in arr {
                    let s = item
                        .as_str()
                        .filter(|s| s.len() >= 2)
                        .ok_or_else(|| bad("items must be strings of min length 2"))?;
                    out.push(s.to_string());
                }
                LocationUpdate::TextArray("supported_locales", out)
            }
            "name" => {
                let s = opt_str(v)?;
                if s.is_empty() || s.len() > 200 {
                    return Err(bad("length must be 1..=200"));
                }
                LocationUpdate::Text("name", s)
            }
            "phone" => {
                let s = opt_str(v)?;
                if s.len() < 3 || s.len() > 30 {
                    return Err(bad("length must be 3..=30"));
                }
                LocationUpdate::Text("phone", s)
            }
            "currency_code" => {
                let s = opt_str(v)?;
                if s.len() != 3 {
                    return Err(bad("length must be exactly 3"));
                }
                LocationUpdate::Text("currency_code", s)
            }
            "delivery_fee_flat" => {
                let n = int(v)?;
                if n < 0 {
                    return Err(bad("must be >= 0"));
                }
                LocationUpdate::Int("delivery_fee_flat", n)
            }
            "min_order_value" | "free_delivery_threshold" => {
                let col: &'static str = if key == "min_order_value" {
                    "min_order_value"
                } else {
                    "free_delivery_threshold"
                };
                let parsed = if v.is_null() {
                    None
                } else {
                    let n = int(v)?;
                    if n < 0 {
                        return Err(bad("must be >= 0"));
                    }
                    Some(n)
                };
                LocationUpdate::IntNull(col, parsed)
            }
            "delivery_radius_km" => {
                let parsed = if v.is_null() {
                    None
                } else {
                    let n = num(v)?;
                    if n < 0.0 {
                        return Err(bad("must be >= 0"));
                    }
                    Some(n)
                };
                LocationUpdate::FloatNull("delivery_radius_km", parsed)
            }
            "tax_rate" => {
                let n = num(v)?;
                if !(0.0..=100.0).contains(&n) {
                    return Err(bad("must be between 0 and 100"));
                }
                LocationUpdate::Float("tax_rate", n)
            }
            "lat" | "lng" => {
                let (col, lo, hi): (&'static str, f64, f64) = if key == "lat" {
                    ("lat", -90.0, 90.0)
                } else {
                    ("lng", -180.0, 180.0)
                };
                let parsed = if v.is_null() {
                    None
                } else {
                    let n = num(v)?;
                    if !(lo..=hi).contains(&n) {
                        return Err(bad("out of range"));
                    }
                    Some(n)
                };
                LocationUpdate::FloatNull(col, parsed)
            }
            "delivery_address" => {
                // Schema-accepted but the COLUMN does not exist — carried verbatim, the UPDATE
                // will fail at Postgres and 500 exactly like Node (see module doc).
                let parsed = if v.is_null() {
                    None
                } else {
                    let s = opt_str(v)?;
                    if s.len() > 500 {
                        return Err(bad("max length 500"));
                    }
                    Some(s)
                };
                LocationUpdate::TextNull("delivery_address", parsed)
            }
            // `.strict()`: any unknown key is a Zod 400 in Node.
            other => {
                return Err(zod_400(
                    format!("unrecognized key: {other}"),
                    correlation_id,
                ));
            }
        };
        updates.push(update);
    }
    Ok(updates)
}

// ── Handler ──────────────────────────────────────────────────────────────────────────────────

/// `PATCH /api/owner/locations/{locationId}` (`locations.ts:9-66`) -> 200 raw row, 400
/// VALIDATION_FAILED (empty body / locale-consistency / Zod bounds), 404 NOT_FOUND.
#[utoipa::path(
    patch,
    path = "/api/owner/locations/{locationId}",
    params(("locationId" = Uuid, Path)),
    responses(
        (status = 200, description = "The full updated locations row (raw column names, node-pg rendering)", body = LocationRow),
        (status = 400, description = "No updates / schema violation / default_locale not in supported_locales", body = domain::ErrorEnvelope),
        (status = 404, description = "Foreign/unknown location", body = domain::ErrorEnvelope),
    ),
    tag = "owner-locations"
)]
pub async fn patch_location(
    Extension(state): Extension<LocationsState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(location_id): Path<Uuid>,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);
    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    let updates = parse_updates(&body, &correlation_id)?;
    if updates.is_empty() {
        // `locations.ts:39` — Node's exact message.
        return Err(zod_400("No updates provided", &correlation_id));
    }

    // `locations.ts:42-46`: when BOTH are present, default_locale must be in supported_locales.
    let default_locale = updates.iter().find_map(|u| match u {
        LocationUpdate::Text("default_locale", s) => Some(s.clone()),
        _ => None,
    });
    let supported = updates.iter().find_map(|u| match u {
        LocationUpdate::TextArray("supported_locales", a) => Some(a.clone()),
        _ => None,
    });
    if let (Some(dl), Some(sl)) = (default_locale, supported) {
        if !sl.contains(&dl) {
            return Err(zod_400(
                "default_locale must be in supported_locales",
                &correlation_id,
            ));
        }
    }

    let row = state
        .repo
        .update(owner.user_id, location_id, updates)
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

// ── PgLocationsRepo ──────────────────────────────────────────────────────────────────────────

pub struct PgLocationsRepo {
    pool: sqlx::PgPool,
}
impl PgLocationsRepo {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl LocationsRepo for PgLocationsRepo {
    async fn update(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        updates: Vec<LocationUpdate>,
    ) -> Result<Option<LocationRow>, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, move |txn| {
            Box::pin(async move {
                // FIRST in-tx statement (session lesson / S3 C1+H4): the membership read seats
                // the RLS-admissible context; without it a FORCE-RLS UPDATE matches 0 rows.
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(None);
                }
                let set_clauses: Vec<String> = updates
                    .iter()
                    .enumerate()
                    .map(|(i, u)| format!("{} = ${}", u.column(), i + 2))
                    .collect();
                let sql = format!(
                    "UPDATE locations SET {} WHERE id = $1 RETURNING {RETURNING_COLS}",
                    set_clauses.join(", ")
                );
                let mut query = sqlx::query_as::<_, LocationRow>(&sql).bind(location_id);
                for u in updates {
                    query = match u {
                        LocationUpdate::Text(_, s) => query.bind(s),
                        LocationUpdate::TextNull(_, s) => query.bind(s),
                        LocationUpdate::TextArray(_, a) => query.bind(a),
                        LocationUpdate::Int(_, n) => query.bind(n),
                        LocationUpdate::IntNull(_, n) => query.bind(n),
                        LocationUpdate::Float(_, f) => query.bind(f),
                        LocationUpdate::FloatNull(_, f) => query.bind(f),
                    };
                }
                let row = query.fetch_optional(&mut **txn).await?;
                Ok(row)
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

// ── Fake + tests ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
pub mod fake {
    use super::{LocationRow, LocationUpdate, LocationsRepo, RepoError};
    use std::sync::Mutex;
    use uuid::Uuid;

    #[derive(Default)]
    pub struct FakeLocationsRepo {
        /// The row to answer with (any location), or `None` -> 404 path.
        pub row: Mutex<Option<LocationRow>>,
        pub seen_updates: Mutex<Vec<(Uuid, Vec<LocationUpdate>)>>,
    }

    #[async_trait::async_trait]
    impl LocationsRepo for FakeLocationsRepo {
        async fn update(
            &self,
            _owner: Uuid,
            location_id: Uuid,
            updates: Vec<LocationUpdate>,
        ) -> Result<Option<LocationRow>, RepoError> {
            self.seen_updates
                .lock()
                .unwrap()
                .push((location_id, updates));
            Ok(self.row.lock().unwrap().clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::FakeLocationsRepo;
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

    fn sample_row(id: Uuid) -> LocationRow {
        LocationRow {
            id,
            org_id: Uuid::new_v4(),
            slug: "demo".into(),
            name: "Demo".into(),
            phone: "+355".into(),
            status: "active".into(),
            busy_mode: false,
            confirm_timeout_min: 10,
            delivery_radius_km: Some("5.00".into()),
            lat: Some(41.0),
            lng: Some(19.8),
            closed_message: None,
            menu_version: 1,
            custom_domain: None,
            domain_verified_at: None,
            widget_enabled: false,
            customer_otp_required: false,
            created_at: chrono::DateTime::parse_from_rfc3339("2026-06-20T19:36:37.915496Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            default_locale: "sq".into(),
            supported_locales: vec!["sq".into(), "en".into()],
            currency_code: "ALL".into(),
            currency_minor_unit: 0,
            tax_rate: "20.00".into(),
            price_includes_tax: true,
            min_order_value: Some(0),
            free_delivery_threshold: None,
            delivery_fee_flat: Some(200),
            delivery_polygon: None,
            require_phone_otp: false,
            onboarding_state: serde_json::json!({}),
            onboarding_completed_at: None,
            dwell_thresholds: serde_json::json!({}),
            retention_days: 365,
            fallback_config: serde_json::json!({}),
            rate_limit_overrides: serde_json::json!({}),
            address: Some("1 Main St".into()),
            public_phone: None,
            hours_json: None,
            geo: None,
            delivery_paused: false,
            published_at: None,
            pickup_enabled: false,
            menu_confirmed_at: None,
            telegram_alert_detail: "full".into(),
            timezone: None,
            plan: "free".into(),
            kitchen_busy_until: None,
            eta_cap_min: 60,
            dispatch_margin_min: 5,
            material_shift_min: 5,
            otp_target_pct: 0,
            min_window_width_min: 10,
            geofence_radius_m: 150,
        }
    }

    /// Wire parity of the raw-row response: numeric-as-STRING, JS `toISOString()` milliseconds,
    /// integral floats as JSON ints — the three node-pg rendering classes.
    #[tokio::test]
    async fn patch_location_returns_the_raw_row_with_node_pg_rendering() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = FakeLocationsRepo::default();
        *repo.row.lock().unwrap() = Some(sample_row(loc));
        let state = LocationsState {
            auth: owner_with_location(user_id, loc),
            repo: Arc::new(repo),
        };
        let owner = OwnerClaims::new(user_id, Some(loc));

        let response = patch_location(
            Extension(state),
            OwnerClaimsExt(owner),
            Path(loc),
            Extension(request_id()),
            Json(serde_json::json!({ "name": "Renamed" })),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            body["tax_rate"], "20.00",
            "numeric renders as STRING (node-pg)"
        );
        assert_eq!(body["delivery_radius_km"], "5.00");
        assert_eq!(
            body["created_at"], "2026-06-20T19:36:37.915Z",
            "JS Date.toISOString() — 3-digit millis + Z"
        );
        assert_eq!(
            body["lat"], 41,
            "integral f64 -> JSON int (JSON.stringify parity)"
        );
        assert_eq!(body["min_order_value"], 0);
        assert_eq!(body["supported_locales"], serde_json::json!(["sq", "en"]));
        assert_eq!(
            body.as_object().unwrap().len(),
            53,
            "all 53 live columns present (RETURNING * parity)"
        );
    }

    /// `locations.ts:39,42-46` — Node's two handler-level 400s, exact messages, 400 wire status
    /// (#78). Plus the `.strict()` unknown-key and `.optional()`-null rejections.
    #[tokio::test]
    async fn patch_location_400s_match_node_status_and_messages() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let mk_state = || LocationsState {
            auth: owner_with_location(user_id, loc),
            repo: Arc::new(FakeLocationsRepo::default()),
        };
        let call = |body: serde_json::Value| {
            let state = mk_state();
            async move {
                crate::error::expect_err(
                    patch_location(
                        Extension(state),
                        OwnerClaimsExt(OwnerClaims::new(user_id, Some(loc))),
                        Path(loc),
                        Extension(request_id()),
                        Json(body),
                    )
                    .await,
                )
            }
        };

        let err = call(serde_json::json!({})).await;
        assert_eq!(err.envelope.status, 400);
        assert_eq!(err.envelope.message, "No updates provided");

        let err = call(serde_json::json!({
            "default_locale": "de",
            "supported_locales": ["sq", "en"]
        }))
        .await;
        assert_eq!(err.envelope.status, 400);
        assert_eq!(
            err.envelope.message,
            "default_locale must be in supported_locales"
        );

        // .strict(): unknown key.
        let err = call(serde_json::json!({ "nonsense": 1 })).await;
        assert_eq!(err.envelope.status, 400);

        // .optional() (NOT nullish) field with explicit null: Zod 400 in Node.
        let err = call(serde_json::json!({ "name": null })).await;
        assert_eq!(err.envelope.status, 400);

        // Bounds.
        let err = call(serde_json::json!({ "tax_rate": 150 })).await;
        assert_eq!(err.envelope.status, 400);
        let err = call(serde_json::json!({ "phone": "12" })).await;
        assert_eq!(err.envelope.status, 400);
    }

    /// `.nullish()` fields accept explicit null (SET NULL); update list preserves types.
    #[tokio::test]
    async fn patch_location_nullish_fields_accept_null_and_reach_the_repo() {
        let user_id = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let repo = Arc::new(FakeLocationsRepo::default());
        *repo.row.lock().unwrap() = Some(sample_row(loc));
        let state = LocationsState {
            auth: owner_with_location(user_id, loc),
            repo: repo.clone(),
        };

        let response = patch_location(
            Extension(state),
            OwnerClaimsExt(OwnerClaims::new(user_id, Some(loc))),
            Path(loc),
            Extension(request_id()),
            Json(serde_json::json!({
                "min_order_value": null,
                "lat": 41.33,
                "delivery_fee_flat": 250
            })),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let seen = repo.seen_updates.lock().unwrap();
        let (seen_loc, updates) = &seen[0];
        assert_eq!(*seen_loc, loc);
        assert_eq!(updates.len(), 3);
        // Order-independent (serde_json map iteration is alphabetical, Node's is insertion —
        // semantically inert for a SET clause; see parse_updates doc).
        assert!(
            updates
                .iter()
                .any(|u| matches!(u, LocationUpdate::IntNull("min_order_value", None)))
        );
        assert!(updates.iter().any(
            |u| matches!(u, LocationUpdate::FloatNull("lat", Some(v)) if (v - 41.33).abs() < 1e-9)
        ));
        assert!(
            updates
                .iter()
                .any(|u| matches!(u, LocationUpdate::Int("delivery_fee_flat", 250)))
        );
    }

    /// Foreign location -> 404 (existence-hiding), and repo-None -> 404 `Not found`.
    #[tokio::test]
    async fn patch_location_404_for_foreign_location_and_no_row() {
        let user_id = Uuid::new_v4();
        let mine = Uuid::new_v4();
        let theirs = Uuid::new_v4();
        let state = LocationsState {
            auth: owner_with_location(user_id, mine),
            repo: Arc::new(FakeLocationsRepo::default()),
        };

        let err = crate::error::expect_err(
            patch_location(
                Extension(state.clone()),
                OwnerClaimsExt(OwnerClaims::new(user_id, Some(mine))),
                Path(theirs),
                Extension(request_id()),
                Json(serde_json::json!({ "name": "X" })),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);

        // Membership passes but the repo answers None (RLS-invisible / vanished row).
        let err = crate::error::expect_err(
            patch_location(
                Extension(state),
                OwnerClaimsExt(OwnerClaims::new(user_id, Some(mine))),
                Path(mine),
                Extension(request_id()),
                Json(serde_json::json!({ "name": "X" })),
            )
            .await,
        );
        assert_eq!(err.envelope.code, ErrorCode::NotFound);
        assert_eq!(err.envelope.message, "Not found");
    }

    /// Requires a live Postgres — proves the 53-column RETURNING projection DECODES against the
    /// real schema (#77 class: numeric::text, int4->i32, text[], jsonb, timestamptz) via the
    /// SELECT-equivalent of the projection on any existing row. Read-only.
    #[tokio::test]
    #[ignore = "requires a live Postgres — set DATABASE_URL_OPERATIONAL/DATABASE_URL_SESSION and run with --ignored"]
    async fn live_pg_locations_projection_decodes_all_53_columns() {
        let config =
            crate::config::Config::from_env().expect("env must be valid to run this ignored test");
        let pools = crate::db::Pools::connect(&config)
            .await
            .expect("pools must connect");
        let sql = format!("SELECT {RETURNING_COLS} FROM locations LIMIT 1");
        let row: Option<LocationRow> = sqlx::query_as(&sql)
            .fetch_optional(&pools.operational)
            .await
            .expect("the 53-column projection must decode against the live schema");
        // Staging has real locations; if this env is empty the decode above still proved the
        // projection PARSES — but assert data presence so a silently-empty run can't fake green.
        let row = row.expect("staging must have at least one locations row");
        assert!(!row.slug.is_empty());
    }

    /// Requires a live Postgres — proves the membership-first UPDATE path binds/parses: a random
    /// owner/location pair fails the in-tx membership assert -> Ok(None), nothing written.
    #[tokio::test]
    #[ignore = "requires a live Postgres — set DATABASE_URL_OPERATIONAL/DATABASE_URL_SESSION and run with --ignored"]
    async fn live_pg_update_denies_a_random_owner_location_pair_without_writing() {
        let config =
            crate::config::Config::from_env().expect("env must be valid to run this ignored test");
        let pools = crate::db::Pools::connect(&config)
            .await
            .expect("pools must connect");
        let repo = PgLocationsRepo::new(pools.operational.clone());
        let row = repo
            .update(
                Uuid::new_v4(),
                Uuid::new_v4(),
                vec![LocationUpdate::Text("name", "nope".into())],
            )
            .await
            .expect("membership-denied path must be a clean Ok(None), not an error");
        assert!(row.is_none());
    }
}
