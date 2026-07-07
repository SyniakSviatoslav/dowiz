//! Owner customer management — Phase 2.3 MVP. PII ownership (NOBYPASSRLS + erasure oracle).
//!
//! Routes:
//! - GET `/api/owner/locations/:locationId/customers` — list owned customers (paginated, searchable)
//! - GET `/api/owner/locations/:locationId/customers/:customerId` — get single customer
//! - DELETE `/api/owner/locations/:locationId/customers/:customerId` — erasure (goal-state verified)

use axum::extract::{Extension, Path, Json, Query};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::ToSchema;
use sqlx::PgConnection;

use crate::auth::AuthState;
use crate::auth::extractors::OwnerClaimsExt;
use crate::error::ApiError;
use crate::routes::correlation_id_string;
use tower_http::request_id::RequestId;
use domain::ErrorCode;

use super::{require_location_access, assert_active_owner_membership};
use crate::modules::customer_management::{CustomerRepo, CustomerRow};
use crate::modules::customer_management::pg::PgCustomerRepo;

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

/// Response: list of customers with pagination.
#[derive(Debug, Serialize, ToSchema)]
pub struct ListCustomersResponse {
    pub customers: Vec<CustomerRow>,
    pub total: i64,
}

/// GET /api/owner/locations/:locationId/customers — list owned customers (paginated, searchable).
pub async fn list_customers(
    Extension(auth): Extension<AuthState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path(location_id): Path<Uuid>,
    Query(q): Query<ListCustomersQuery>,
    request_id: RequestId,
) -> Result<Json<Vec<CustomerRow>>, ApiError> {
    let correlation_id = correlation_id_string(&request_id);

    // OWNER+LOC: verify membership
    require_location_access(&auth, &owner, location_id, &correlation_id).await?;

    // Repo call with pagination defaults
    let limit = q.limit.unwrap_or(50).min(1000).max(1);
    let offset = q.offset.unwrap_or(0).max(0);
    let search = q.search.clone();

    let customers: Vec<CustomerRow> = crate::db::with_user(&auth.repo.pool(), owner.user_id, move |txn| {
        Box::pin(async move {
            // RED-LINE: in-transaction membership re-check (belt-and-suspenders, breaker C1+H4)
            if !assert_active_owner_membership(txn, owner.user_id, location_id).await? {
                return Ok(Vec::new()); // Empty vec on failed membership check
            }

            // Repository call (returns empty vec if no matches)
            let repo = PgCustomerRepo::new(txn.clone());
            repo.list_customers(location_id, search, limit, offset)
                .await
                .map_err(|_| ApiError::new(ErrorCode::Internal, "internal_error", &correlation_id))
        })
    })
    .await
    .map_err(|_| ApiError::new(ErrorCode::Internal, "internal_error", &correlation_id))?;

    Ok(Json(customers))
}

/// GET /api/owner/locations/:locationId/customers/:customerId — get single customer.
pub async fn get_customer(
    Extension(auth): Extension<AuthState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path((location_id, customer_id)): Path<(Uuid, Uuid)>,
    request_id: RequestId,
) -> Result<Json<CustomerRow>, ApiError> {
    let correlation_id = correlation_id_string(&request_id);

    require_location_access(&auth, &owner, location_id, &correlation_id).await?;

    let customer = auth
        .repo
        .with_user(owner.user_id, |conn| {
            Box::pin(async move {
                if !assert_active_owner_membership(conn, owner.user_id, location_id).await? {
                    return Err(ApiError::new(
                        ErrorCode::NotFound,
                        "Not found",
                        &correlation_id,
                    )
                    .into());
                }

                let repo = PgCustomerRepo::new(conn.clone());
                repo.get_customer(location_id, customer_id)
                    .await
                    .map_err(|_| {
                        ApiError::new(ErrorCode::Internal, "internal_error", &correlation_id).into()
                    })
            })
        })
        .await
        .map_err(|_| ApiError::new(ErrorCode::Internal, "internal_error", &correlation_id))?;

    customer.ok_or_else(|| ApiError::new(ErrorCode::NotFound, "Not found", &correlation_id))
        .map(Json)
}

/// DELETE /api/owner/locations/:locationId/customers/:customerId — erasure (goal-state verified).
/// RED-LINE: implements the erasure oracle gate (goal-state re-read from all surfaces).
pub async fn delete_customer(
    Extension(auth): Extension<AuthState>,
    OwnerClaimsExt(owner): OwnerClaimsExt,
    Path((location_id, customer_id)): Path<(Uuid, Uuid)>,
    request_id: RequestId,
) -> Result<StatusCode, ApiError> {
    let correlation_id = correlation_id_string(&request_id);

    require_location_access(&auth, &owner, location_id, &correlation_id).await?;

    // Perform erasure + verify goal-state
    auth
        .repo
        .with_user(owner.user_id, |conn| {
            Box::pin(async move {
                if !assert_active_owner_membership(conn, owner.user_id, location_id).await? {
                    return Err(ApiError::new(
                        ErrorCode::NotFound,
                        "Not found",
                        &correlation_id,
                    )
                    .into());
                }

                let repo = PgCustomerRepo::new(conn.clone());

                // Delete customer (cascade + denormalization cleanup)
                let (_deleted, _updated) = repo
                    .delete_customer(location_id, customer_id)
                    .await
                    .map_err(|_| {
                        ApiError::new(ErrorCode::Internal, "internal_error", &correlation_id).into()
                    })?;

                // GATE: erasure oracle — goal-state re-read from all surfaces
                let (absent_from_customers, absent_from_orders, is_truly_erased) = repo
                    .verify_customer_erased(location_id, customer_id)
                    .await
                    .map_err(|_| {
                        ApiError::new(ErrorCode::Internal, "internal_error", &correlation_id).into()
                    })?;

                // RED-LINE: if not fully erased, something went wrong
                if !is_truly_erased {
                    return Err(ApiError::new(
                        ErrorCode::Internal,
                        "erasure_verification_failed",
                        &correlation_id,
                    )
                    .into());
                }

                Ok::<(), Box<dyn std::error::Error>>(())
            })
        })
        .await
        .map_err(|_| ApiError::new(ErrorCode::Internal, "internal_error", &correlation_id))?;

    Ok(StatusCode::NO_CONTENT)
}
