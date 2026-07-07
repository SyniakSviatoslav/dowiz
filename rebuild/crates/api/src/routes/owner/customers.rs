//! Owner customer management — Phase 2.3 MVP. PII ownership (NOBYPASSRLS + erasure oracle).
//!
//! Routes:
//! - GET `/api/owner/locations/:locationId/customers` — list owned customers (paginated, searchable)
//! - GET `/api/owner/locations/:locationId/customers/:customerId` — get single customer
//! - DELETE `/api/owner/locations/:locationId/customers/:customerId` — erasure (goal-state verified)

use axum::extract::{Extension, Path, Json, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::ToSchema;

use crate::auth::AuthState;
use crate::auth::extractors::OwnerClaimsExt;
use crate::error::ApiError;
use crate::routes::correlation_id_string;
use tower_http::request_id::RequestId;
use domain::ErrorCode;

use super::require_location_access;
use crate::modules::customer_management::{CustomerRepo, CustomerRow};

#[derive(Clone)]
pub struct CustomersState {
    pub auth: AuthState,
    pub repo: std::sync::Arc<dyn CustomerRepo>,
}

/// Query params for list customers endpoint.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ListCustomersQuery {
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

/// GET /api/owner/locations/:locationId/customers — list owned customers (paginated, searchable).
pub async fn list_customers(
    Extension(state): Extension<CustomersState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(location_id): Path<Uuid>,
    Query(q): Query<ListCustomersQuery>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);

    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    let limit = q.limit.unwrap_or(50).clamp(1, 1000);
    let offset = q.offset.unwrap_or(0).max(0);

    let customers = state
        .repo
        .list_customers(owner.user_id, location_id, q.search, limit, offset)
        .await
        .map_err(|_| ApiError::new(ErrorCode::Internal, "internal_error", &correlation_id))?
        .unwrap_or_default();

    Ok(Json(customers))
}

/// GET /api/owner/locations/:locationId/customers/:customerId — get single customer.
pub async fn get_customer(
    Extension(state): Extension<CustomersState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path((location_id, customer_id)): Path<(Uuid, Uuid)>,
    Extension(request_id): Extension<RequestId>,
) -> Result<impl IntoResponse, ApiError> {
    let correlation_id = correlation_id_string(&request_id);

    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    let customer = state
        .repo
        .get_customer(owner.user_id, location_id, customer_id)
        .await
        .map_err(|_| ApiError::new(ErrorCode::Internal, "internal_error", &correlation_id))?
        .ok_or_else(|| ApiError::new(ErrorCode::NotFound, "Not found", &correlation_id))?;

    Ok(Json(customer))
}

/// DELETE /api/owner/locations/:locationId/customers/:customerId — erasure (goal-state verified).
pub async fn delete_customer(
    Extension(state): Extension<CustomersState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path((location_id, customer_id)): Path<(Uuid, Uuid)>,
    Extension(request_id): Extension<RequestId>,
) -> Result<StatusCode, ApiError> {
    let correlation_id = correlation_id_string(&request_id);

    require_location_access(&state.auth, &owner, location_id, &correlation_id).await?;

    let _counts = state
        .repo
        .delete_customer(owner.user_id, location_id, customer_id)
        .await
        .map_err(|_| ApiError::new(ErrorCode::Internal, "internal_error", &correlation_id))?
        .ok_or_else(|| ApiError::new(ErrorCode::NotFound, "Not found", &correlation_id))?;

    // GATE: erasure oracle — goal-state re-read from all surfaces
    let (_absent_customers, _absent_orders, is_truly_erased) = state
        .repo
        .verify_customer_erased(location_id, customer_id)
        .await
        .map_err(|_| ApiError::new(ErrorCode::Internal, "internal_error", &correlation_id))?;

    if !is_truly_erased {
        return Err(ApiError::new(
            ErrorCode::Internal,
            "erasure_verification_failed",
            &correlation_id,
        ));
    }

    Ok(StatusCode::NO_CONTENT)
}
