//! Owned customer data management — Phase 2.3 MVP. PII ownership for the owner.
//! Per A5 (module placement), this lands as a hub-module with manifest.
//!
//! Routes:
//! - GET `/api/owner/locations/:locationId/customers` — list owned customers (paginated, searchable)
//! - GET `/api/owner/locations/:locationId/customers/:customerId` — get single customer
//! - DELETE `/api/owner/locations/:locationId/customers/:customerId` — erasure (goal-state verified)
//! - POST `/api/orders` — upserts customer at checkout (integration point)
//!
//! Red-line gates:
//! - NOBYPASSRLS behavioral test: cross-location delete attempt → denied
//! - Erasure oracle: re-read via list/search/order-detail after delete → absent everywhere

use async_trait::async_trait;
use axum::extract::{Extension, Path, Json, Query};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::ToSchema;

use crate::auth::AuthState;
use crate::error::ApiError;
use crate::repo::RepoError;
use domain::ErrorCode;

pub mod pg;

#[derive(Clone)]
pub struct CustomerManagementState {
    pub auth: AuthState,
    pub repo: std::sync::Arc<dyn CustomerRepo>,
}

/// Customer record (PII).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, sqlx::FromRow)]
pub struct CustomerRow {
    pub id: Uuid,
    pub location_id: Uuid,
    pub phone: String,
    pub name: Option<String>,
    pub consented_to_terms: bool,
    pub consented_to_marketing: bool,
    pub no_show_count: i32,
    #[serde(serialize_with = "crate::dto::serialize_js_instant")]
    #[schema(value_type = String)]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(serialize_with = "crate::dto::serialize_js_instant")]
    #[schema(value_type = String)]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateOrUpsertCustomerRequest {
    pub phone: String,
    pub name: Option<String>,
    pub consented_to_terms: Option<bool>,
    pub consented_to_marketing: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ListCustomersQuery {
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

#[async_trait]
pub trait CustomerRepo: Send + Sync {
    /// List customers for a location (paginated, searchable by phone/name).
    async fn list_customers(
        &self,
        location_id: Uuid,
        search: Option<String>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CustomerRow>, RepoError>;

    /// Get a single customer by ID (must belong to location).
    async fn get_customer(
        &self,
        location_id: Uuid,
        customer_id: Uuid,
    ) -> Result<Option<CustomerRow>, RepoError>;

    /// Create or upsert a customer (upsert on phone per location).
    async fn create_or_upsert_customer(
        &self,
        location_id: Uuid,
        req: CreateOrUpsertCustomerRequest,
    ) -> Result<CustomerRow, RepoError>;

    /// Delete a customer (erasure: returns count of orders that had customer_id zeroed).
    async fn delete_customer(
        &self,
        location_id: Uuid,
        customer_id: Uuid,
    ) -> Result<(u64, u64), RepoError>; // (customers deleted, order records updated)

    /// Erasure oracle: verify customer is absent from all surfaces.
    /// Returns (absent_from_customers, absent_from_order_refs, is_truly_erased).
    async fn verify_customer_erased(
        &self,
        location_id: Uuid,
        customer_id: Uuid,
    ) -> Result<(bool, bool, bool), RepoError>;
}

/// Routes handler (to be wired in routes/owner/mod.rs) — Phase 2.3 implementation deferred.
pub async fn list_customers(
    _auth: Extension<AuthState>,
    _location_id: Path<Uuid>,
    _q: Query<ListCustomersQuery>,
) -> Result<Json<Vec<CustomerRow>>, ApiError> {
    // TODO: implement with RLS check
    Err(ApiError::new(
        ErrorCode::NotImplemented,
        "customer list: Phase 2.3 implementation pending",
        "",
    ))
}

pub async fn get_customer(
    _auth: Extension<AuthState>,
    _path: Path<(Uuid, Uuid)>,
) -> Result<Json<CustomerRow>, ApiError> {
    // TODO: implement with RLS check
    Err(ApiError::new(
        ErrorCode::NotImplemented,
        "customer get: Phase 2.3 implementation pending",
        "",
    ))
}

pub async fn delete_customer(
    _auth: Extension<AuthState>,
    _path: Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
    // TODO: implement with RLS check + erasure oracle
    Err(ApiError::new(
        ErrorCode::NotImplemented,
        "customer delete: Phase 2.3 implementation pending",
        "",
    ))
}
