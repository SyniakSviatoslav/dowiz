//! Owner settings — GET/PUT `/api/owner/settings` (`spa-proxy.ts:666-760`). The owner's OWN location
//! config (name/phone/address/geo/hours + the delivery-fee/min-order/tax **pricing inputs** that S5
//! reads). Location is DERIVED from the token (not a URL param): GET → the owner's first active
//! location, or `{id:null}` for a fresh signup with a valid token but no location yet (O1: a null-loc
//! owner is onboarding, NOT an expired session); PUT → `resolve_owner_location` (401 if none).
//!
//! Decode class (regression #77): `delivery_fee_flat`/`min_order_value`/`free_delivery_threshold` are
//! `int4` → read `::bigint`; `tax_rate`/`delivery_radius_km` are `numeric` → read `::float8`; `lat`/
//! `lng` are `double precision` (native f64). This is a faithful port of Node's exact COALESCE upsert
//! — the pricing columns are written verbatim (no re-scaling), so S5 pricing parity is preserved.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use axum::Json;
use axum::extract::Extension;
use axum::response::IntoResponse;

use crate::auth::AuthState;
use crate::auth::extractors::OwnerClaimsExt;
use crate::dto::{serialize_js_number, serialize_js_number_opt};
use crate::error::ApiError;
use crate::repo::RepoError;
use crate::routes::correlation_id_string;
use tower_http::request_id::RequestId;

use super::{assert_active_owner_membership, resolve_owner_location};

#[derive(Clone)]
pub struct SettingsState {
    pub auth: AuthState,
    pub repo: std::sync::Arc<dyn SettingsRepo>,
}

/// The `locations` config row the settings screen reads (decode-cast per #77 at the SQL).
#[derive(Debug, Clone)]
pub struct SettingsRow {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub phone: Option<String>,
    pub delivery_fee_flat: i64,
    pub min_order_value: i64,
    pub free_delivery_threshold: i64,
    pub delivery_radius_km: f64,
    pub currency_code: Option<String>,
    pub tax_rate: f64,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub address: Option<String>,
    pub hours_json: serde_json::Value,
    pub delivery_paused: bool,
}

/// The PUT patch (`settingsSchema`, `spa-proxy.ts:38`) — every field optional; `None` = keep (COALESCE).
/// `currency_code` is accepted by the schema but Node's UPDATE does NOT write it (parity: ignored here).
#[derive(Debug, Clone, Default)]
pub struct SettingsPatch {
    pub location_name: Option<String>,
    pub phone: Option<String>,
    pub delivery_fee: Option<i64>,
    pub min_order: Option<i64>,
    pub radius_km: Option<f64>,
    pub free_delivery_threshold: Option<i64>,
    pub tax_rate: Option<f64>,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub address: Option<String>,
    pub hours_json: Option<serde_json::Value>,
    pub delivery_paused: Option<bool>,
}

#[async_trait::async_trait]
pub trait SettingsRepo: Send + Sync {
    async fn get(&self, owner: Uuid, location: Uuid) -> Result<Option<SettingsRow>, RepoError>;
    async fn update(
        &self,
        owner: Uuid,
        location: Uuid,
        patch: &SettingsPatch,
    ) -> Result<Option<SettingsRow>, RepoError>;
}

// ── DTOs ──────────────────────────────────────────────────────────────────────────────────────

/// GET/PUT response (`spa-proxy.ts:690`). `currencyCode` is present on GET, absent on PUT (Node parity).
#[derive(Debug, Serialize, ToSchema)]
pub struct SettingsResponse {
    #[schema(value_type = String)]
    pub id: Uuid,
    pub slug: String,
    #[serde(rename = "locationName")]
    pub location_name: String,
    pub phone: String,
    // int4 columns → plain JSON integers (Node `Number(int)` == the same); no f64 cast needed.
    #[serde(rename = "deliveryFee")]
    pub delivery_fee: i64,
    #[serde(rename = "minOrder")]
    pub min_order: i64,
    #[serde(rename = "radiusKm", serialize_with = "serialize_js_number")]
    pub radius_km: f64,
    #[serde(rename = "freeDeliveryThreshold")]
    pub free_delivery_threshold: i64,
    #[serde(rename = "currencyCode", skip_serializing_if = "Option::is_none")]
    pub currency_code: Option<String>,
    #[serde(rename = "taxRate", serialize_with = "serialize_js_number")]
    pub tax_rate: f64,
    #[serde(serialize_with = "serialize_js_number_opt")]
    pub lat: Option<f64>,
    #[serde(serialize_with = "serialize_js_number_opt")]
    pub lng: Option<f64>,
    pub address: String,
    #[serde(rename = "hoursJson")]
    pub hours_json: serde_json::Value,
    #[serde(rename = "deliveryPaused")]
    pub delivery_paused: bool,
}

impl SettingsResponse {
    fn from_row(r: SettingsRow, include_currency: bool) -> Self {
        Self {
            id: r.id,
            slug: r.slug,
            location_name: r.name,
            phone: r.phone.unwrap_or_default(),
            delivery_fee: r.delivery_fee_flat,
            min_order: r.min_order_value,
            radius_km: r.delivery_radius_km,
            free_delivery_threshold: r.free_delivery_threshold,
            currency_code: include_currency
                .then(|| r.currency_code.unwrap_or_else(|| "ALL".to_string())),
            tax_rate: r.tax_rate,
            lat: r.lat,
            lng: r.lng,
            address: r.address.unwrap_or_default(),
            hours_json: r.hours_json,
            delivery_paused: r.delivery_paused,
        }
    }
}

// ── Handlers ──────────────────────────────────────────────────────────────────────────────────

/// `GET /api/owner/settings` — the owner's first active location, or `{id:null}` for a fresh signup.
#[utoipa::path(get, path = "/api/owner/settings", tag = "owner-settings",
    responses((status = 200, body = SettingsResponse), (status = 404, body = domain::ErrorEnvelope)))]
pub async fn get_settings(
    Extension(state): Extension<SettingsState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Extension(request_id): Extension<RequestId>,
) -> Result<axum::response::Response, ApiError> {
    let cid = correlation_id_string(&request_id);
    // A valid owner token with NO active location is a fresh signup → {id:null}, not a 401 (O1).
    let active = state
        .auth
        .repo
        .active_owner_locations(owner.user_id)
        .await
        .map_err(|_e| internal(cid.clone()))?;
    let Some(&location) = active.first() else {
        return Ok(Json(serde_json::json!({ "id": null })).into_response());
    };
    let row = state
        .repo
        .get(owner.user_id, location)
        .await
        .map_err(|_e| internal(cid.clone()))?
        .ok_or_else(|| not_found(cid))?;
    Ok(Json(SettingsResponse::from_row(row, true)).into_response())
}

/// `PUT /api/owner/settings` — 401 if the owner has no active location (Node parity).
#[utoipa::path(put, path = "/api/owner/settings", tag = "owner-settings", request_body = SettingsUpdateBody,
    responses((status = 200, body = SettingsResponse), (status = 401, body = domain::ErrorEnvelope), (status = 404, body = domain::ErrorEnvelope)))]
pub async fn put_settings(
    Extension(state): Extension<SettingsState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Extension(request_id): Extension<RequestId>,
    Json(body): Json<SettingsUpdateBody>,
) -> Result<impl IntoResponse, ApiError> {
    let cid = correlation_id_string(&request_id);
    let location = resolve_owner_location(&state.auth, &owner, &cid).await?;
    let patch = body.into_patch(&cid)?;
    let row = state
        .repo
        .update(owner.user_id, location, &patch)
        .await
        .map_err(|_e| internal(cid.clone()))?
        .ok_or_else(|| ApiError::new(domain::ErrorCode::NotFound, "Location not found", cid))?;
    Ok(Json(SettingsResponse::from_row(row, false)))
}

/// The raw JSON body (validated → `SettingsPatch`). Zod `.strip()` drops unknown keys, so NOT
/// `deny_unknown_fields`. Numeric bounds mirror `settingsSchema` (400 on violation, like Node's Zod).
#[derive(Debug, Deserialize, ToSchema)]
pub struct SettingsUpdateBody {
    #[serde(default, rename = "locationName")]
    pub location_name: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default, rename = "deliveryFee")]
    pub delivery_fee: Option<f64>,
    #[serde(default, rename = "minOrder")]
    pub min_order: Option<f64>,
    #[serde(default, rename = "radiusKm")]
    pub radius_km: Option<f64>,
    #[serde(default, rename = "freeDeliveryThreshold")]
    pub free_delivery_threshold: Option<f64>,
    #[serde(default, rename = "taxRate")]
    pub tax_rate: Option<f64>,
    #[serde(default, rename = "lat")]
    pub lat: Option<f64>,
    #[serde(default, rename = "lng")]
    pub lng: Option<f64>,
    #[serde(default, rename = "hoursJson")]
    pub hours_json: Option<serde_json::Value>,
    #[serde(default, rename = "deliveryPaused")]
    pub delivery_paused: Option<bool>,
}

impl SettingsUpdateBody {
    fn into_patch(self, cid: &str) -> Result<SettingsPatch, ApiError> {
        let bad = |m: &str| ApiError::validation_failed_400(m.to_string(), cid.to_string());
        // int fields: nonnegative integers (Zod `.int().nonnegative()`).
        #[allow(
            clippy::as_conversions,
            reason = "guarded: fract()==0 && 0<=x<=2^53 — exact f64→i64"
        )]
        let int_field = |v: Option<f64>, name: &str| -> Result<Option<i64>, ApiError> {
            match v {
                None => Ok(None),
                Some(x) if x.fract() != 0.0 || !(0.0..=9_007_199_254_740_992.0).contains(&x) => {
                    Err(bad(&format!("{name} must be a non-negative integer")))
                }
                Some(x) => Ok(Some(x as i64)),
            }
        };
        if let Some(t) = self.tax_rate {
            if !(0.0..=100.0).contains(&t) {
                return Err(bad("taxRate must be between 0 and 100"));
            }
        }
        if self.radius_km.is_some_and(|x| x < 0.0) {
            return Err(bad("radiusKm must be non-negative"));
        }
        if self.lat.is_some_and(|x| !(-90.0..=90.0).contains(&x)) {
            return Err(bad("lat out of range"));
        }
        if self.lng.is_some_and(|x| !(-180.0..=180.0).contains(&x)) {
            return Err(bad("lng out of range"));
        }
        Ok(SettingsPatch {
            location_name: self.location_name.filter(|s| !s.is_empty()),
            phone: self.phone.filter(|s| !s.is_empty()),
            delivery_fee: int_field(self.delivery_fee, "deliveryFee")?,
            min_order: int_field(self.min_order, "minOrder")?,
            radius_km: self.radius_km,
            free_delivery_threshold: int_field(
                self.free_delivery_threshold,
                "freeDeliveryThreshold",
            )?,
            tax_rate: self.tax_rate,
            lat: self.lat,
            lng: self.lng,
            address: self.address.filter(|s| !s.is_empty()),
            hours_json: self.hours_json,
            delivery_paused: self.delivery_paused,
        })
    }
}

fn internal(cid: String) -> ApiError {
    ApiError::new(domain::ErrorCode::Internal, "internal_error", cid)
}
fn not_found(cid: String) -> ApiError {
    ApiError::new(domain::ErrorCode::NotFound, "Not found", cid)
}

// ── PgSettingsRepo ────────────────────────────────────────────────────────────────────────────

pub struct PgSettingsRepo {
    pool: sqlx::PgPool,
}
impl PgSettingsRepo {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

type RowTuple = (
    Uuid,
    String,
    String,
    Option<String>,
    i64,
    i64,
    i64,
    f64,
    Option<String>,
    f64,
    Option<f64>,
    Option<f64>,
    Option<String>,
    serde_json::Value,
    bool,
);

fn tuple_to_row(t: RowTuple) -> SettingsRow {
    SettingsRow {
        id: t.0,
        slug: t.1,
        name: t.2,
        phone: t.3,
        delivery_fee_flat: t.4,
        min_order_value: t.5,
        free_delivery_threshold: t.6,
        delivery_radius_km: t.7,
        currency_code: t.8,
        tax_rate: t.9,
        lat: t.10,
        lng: t.11,
        address: t.12,
        hours_json: t.13,
        delivery_paused: t.14,
    }
}

/// The shared decode-cast projection (#77): int4 → `::bigint`, numeric → `::float8`.
const SELECT_COLS: &str = "id, slug, name, phone, \
    delivery_fee_flat::bigint, min_order_value::bigint, free_delivery_threshold::bigint, \
    delivery_radius_km::float8, currency_code, tax_rate::float8, lat, lng, address, \
    COALESCE(hours_json, '{}'::jsonb), delivery_paused";

#[async_trait::async_trait]
impl SettingsRepo for PgSettingsRepo {
    async fn get(&self, owner: Uuid, location: Uuid) -> Result<Option<SettingsRow>, RepoError> {
        crate::db::with_user(&self.pool, owner, move |txn| {
            Box::pin(async move {
                // Mirror the proven dwell/fallback seam: assert membership FIRST inside the tx (a
                // bare `WHERE id=$1` under FORCE-RLS returns 0 rows without the membership context
                // this establishes). 0 rows → None (404), matching Node's not-found.
                if !assert_active_owner_membership(txn, owner, location).await? {
                    return Ok(None);
                }
                let sql = format!("SELECT {SELECT_COLS} FROM locations WHERE id = $1");
                let row: Option<RowTuple> = sqlx::query_as(&sql)
                    .bind(location)
                    .fetch_optional(&mut **txn)
                    .await?;
                Ok(row.map(tuple_to_row))
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn update(
        &self,
        owner: Uuid,
        location: Uuid,
        patch: &SettingsPatch,
    ) -> Result<Option<SettingsRow>, RepoError> {
        let patch = patch.clone();
        crate::db::with_user(&self.pool, owner, move |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner, location).await? {
                    return Ok(None);
                }
                let sql = format!(
                    "UPDATE locations SET \
                       name = COALESCE($2, name), phone = COALESCE($3, phone), \
                       delivery_fee_flat = COALESCE($4, delivery_fee_flat), \
                       min_order_value = COALESCE($5, min_order_value), \
                       delivery_radius_km = COALESCE($6, delivery_radius_km), \
                       free_delivery_threshold = COALESCE($7, free_delivery_threshold), \
                       tax_rate = COALESCE($8, tax_rate), lat = COALESCE($9, lat), \
                       lng = COALESCE($10, lng), address = COALESCE($11, address), \
                       hours_json = COALESCE($12, hours_json), \
                       delivery_paused = COALESCE($13, delivery_paused) \
                     WHERE id = $1 RETURNING {SELECT_COLS}"
                );
                let row: Option<RowTuple> = sqlx::query_as(&sql)
                    .bind(location)
                    .bind(patch.location_name)
                    .bind(patch.phone)
                    .bind(patch.delivery_fee)
                    .bind(patch.min_order)
                    .bind(patch.radius_km)
                    .bind(patch.free_delivery_threshold)
                    .bind(patch.tax_rate)
                    .bind(patch.lat)
                    .bind(patch.lng)
                    .bind(patch.address)
                    .bind(patch.hours_json)
                    .bind(patch.delivery_paused)
                    .fetch_optional(&mut **txn)
                    .await?;
                Ok(row.map(tuple_to_row))
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
pub mod fake {
    use super::{RepoError, SettingsPatch, SettingsRepo, SettingsRow};
    use std::sync::Mutex;
    use uuid::Uuid;

    #[derive(Default)]
    pub struct FakeSettingsRepo {
        pub row: Mutex<Option<SettingsRow>>,
    }
    #[async_trait::async_trait]
    impl SettingsRepo for FakeSettingsRepo {
        async fn get(&self, _o: Uuid, _l: Uuid) -> Result<Option<SettingsRow>, RepoError> {
            Ok(self.row.lock().unwrap().clone())
        }
        async fn update(
            &self,
            _o: Uuid,
            _l: Uuid,
            _p: &SettingsPatch,
        ) -> Result<Option<SettingsRow>, RepoError> {
            Ok(self.row.lock().unwrap().clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_validation_rejects_out_of_range_and_non_int() {
        let cid = "c";
        let mut b = SettingsUpdateBody {
            location_name: None,
            phone: None,
            address: None,
            delivery_fee: None,
            min_order: None,
            radius_km: None,
            free_delivery_threshold: None,
            tax_rate: Some(150.0),
            lat: None,
            lng: None,
            hours_json: None,
            delivery_paused: None,
        };
        assert!(b.clone_for_test().into_patch(cid).is_err()); // taxRate > 100
        b.tax_rate = None;
        b.delivery_fee = Some(12.5);
        assert!(b.into_patch(cid).is_err()); // non-integer fee
    }

    impl SettingsUpdateBody {
        fn clone_for_test(&self) -> SettingsUpdateBody {
            SettingsUpdateBody {
                location_name: self.location_name.clone(),
                phone: self.phone.clone(),
                address: self.address.clone(),
                delivery_fee: self.delivery_fee,
                min_order: self.min_order,
                radius_km: self.radius_km,
                free_delivery_threshold: self.free_delivery_threshold,
                tax_rate: self.tax_rate,
                lat: self.lat,
                lng: self.lng,
                hours_json: self.hours_json.clone(),
                delivery_paused: self.delivery_paused,
            }
        }
    }

    #[test]
    fn valid_patch_maps_ints_and_clears_empty_strings() {
        let b = SettingsUpdateBody {
            location_name: Some(String::new()),
            phone: Some("+355".into()),
            address: None,
            delivery_fee: Some(200.0),
            min_order: Some(0.0),
            radius_km: Some(5.0),
            free_delivery_threshold: Some(2000.0),
            tax_rate: Some(20.0),
            lat: Some(41.3),
            lng: Some(19.8),
            hours_json: None,
            delivery_paused: Some(true),
        };
        let p = b.into_patch("c").unwrap();
        assert_eq!(p.delivery_fee, Some(200));
        assert_eq!(p.min_order, Some(0));
        assert_eq!(p.location_name, None); // empty string cleared
        assert_eq!(p.phone.as_deref(), Some("+355"));
        assert_eq!(p.delivery_paused, Some(true));
    }
}
