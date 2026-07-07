use sqlx::PgPool;
use uuid::Uuid;

use crate::repo::RepoError;
use crate::routes::owner::assert_active_owner_membership;
use crate::db::TenantTxnError;
use super::{CustomerRepo, CustomerRow};

fn map_txn_err(err: TenantTxnError) -> RepoError {
    match err {
        TenantTxnError::Begin(e)
        | TenantTxnError::SetTenant(e)
        | TenantTxnError::Work(e)
        | TenantTxnError::Commit(e) => RepoError(e),
        TenantTxnError::WorkThenRollbackFailed { work, .. } => RepoError(work),
    }
}

pub struct PgCustomerRepo {
    pub pool: PgPool,
}

impl PgCustomerRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl CustomerRepo for PgCustomerRepo {
    async fn list_customers(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        search: Option<String>,
        limit: i64,
        offset: i64,
    ) -> Result<Option<Vec<CustomerRow>>, RepoError> {
        let limit = limit.min(1000).max(1);
        let offset = offset.max(0);

        crate::db::with_user(&self.pool, owner_user_id, move |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(None);
                }

                let customers = if let Some(search_term) = search {
                    let search_pattern = format!("%{}%", search_term.to_lowercase());
                    sqlx::query_as::<_, CustomerRow>(
                        r#"
                        SELECT c.id, c.location_id, c.phone, c.name, c.consented_to_terms,
                               c.consented_to_marketing, c.no_show_count, c.created_at, c.updated_at
                        FROM customers c
                        WHERE c.location_id = $1
                          AND (LOWER(c.phone) LIKE $2 OR LOWER(c.name) LIKE $2)
                        ORDER BY c.created_at DESC
                        LIMIT $3 OFFSET $4
                        "#,
                    )
                    .bind(location_id)
                    .bind(search_pattern)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&mut **txn)
                    .await?
                } else {
                    sqlx::query_as::<_, CustomerRow>(
                        r#"
                        SELECT c.id, c.location_id, c.phone, c.name, c.consented_to_terms,
                               c.consented_to_marketing, c.no_show_count, c.created_at, c.updated_at
                        FROM customers c
                        WHERE c.location_id = $1
                        ORDER BY c.created_at DESC
                        LIMIT $2 OFFSET $3
                        "#,
                    )
                    .bind(location_id)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&mut **txn)
                    .await?
                };

                Ok(Some(customers))
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn get_customer(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        customer_id: Uuid,
    ) -> Result<Option<CustomerRow>, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, move |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(None);
                }

                let customer = sqlx::query_as::<_, CustomerRow>(
                    r#"
                    SELECT c.id, c.location_id, c.phone, c.name, c.consented_to_terms,
                           c.consented_to_marketing, c.no_show_count, c.created_at, c.updated_at
                    FROM customers c
                    WHERE c.id = $1 AND c.location_id = $2
                    "#,
                )
                .bind(customer_id)
                .bind(location_id)
                .fetch_optional(&mut **txn)
                .await?;

                Ok(customer)
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn delete_customer(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        customer_id: Uuid,
    ) -> Result<Option<(u64, u64)>, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, move |txn| {
            Box::pin(async move {
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(None);
                }

                // Erasure: cascade-delete from customers
                let deleted = sqlx::query_scalar::<_, i64>(
                    "DELETE FROM customers WHERE id = $1 AND location_id = $2"
                )
                .bind(customer_id)
                .bind(location_id)
                .fetch_one(&mut **txn)
                .await? as u64;

                // Erasure: NULL out customer_id from orders (denormalization cleanup)
                let updated = sqlx::query_scalar::<_, i64>(
                    "UPDATE orders SET customer_id = NULL WHERE customer_id = $1 AND location_id = $2"
                )
                .bind(customer_id)
                .bind(location_id)
                .fetch_one(&mut **txn)
                .await? as u64;

                Ok(Some((deleted, updated)))
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn verify_customer_erased(
        &self,
        location_id: Uuid,
        customer_id: Uuid,
    ) -> Result<(bool, bool, bool), RepoError> {
        // Erasure oracle: goal-state re-read from all surfaces

        // Check 1: customer row gone from customers table
        let absent_from_customers = sqlx::query_scalar::<_, bool>(
            "SELECT NOT EXISTS(SELECT 1 FROM customers WHERE id = $1 AND location_id = $2)"
        )
        .bind(customer_id)
        .bind(location_id)
        .fetch_one(&self.pool)
        .await?;

        // Check 2: no orders still reference this customer
        let absent_from_orders = sqlx::query_scalar::<_, bool>(
            "SELECT NOT EXISTS(SELECT 1 FROM orders WHERE customer_id = $1 AND location_id = $2)"
        )
        .bind(customer_id)
        .bind(location_id)
        .fetch_one(&self.pool)
        .await?;

        // Check 3: fully erased (both checks pass)
        let is_truly_erased = absent_from_customers && absent_from_orders;

        Ok((absent_from_customers, absent_from_orders, is_truly_erased))
    }
}
