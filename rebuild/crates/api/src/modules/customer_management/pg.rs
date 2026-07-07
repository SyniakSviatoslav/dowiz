use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::repo::RepoError;
use super::{CustomerRepo, CustomerRow, CreateOrUpsertCustomerRequest};

pub struct PgCustomerRepo {
    pool: PgPool,
}

impl PgCustomerRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CustomerRepo for PgCustomerRepo {
    async fn list_customers(
        &self,
        location_id: Uuid,
        search: Option<String>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CustomerRow>, RepoError> {
        let limit = limit.min(1000).max(1);
        let offset = offset.max(0);

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
            .fetch_all(&self.pool)
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
            .fetch_all(&self.pool)
            .await?
        };

        Ok(customers)
    }

    async fn get_customer(
        &self,
        location_id: Uuid,
        customer_id: Uuid,
    ) -> Result<Option<CustomerRow>, RepoError> {
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
        .fetch_optional(&self.pool)
        .await?;

        Ok(customer)
    }

    async fn create_or_upsert_customer(
        &self,
        location_id: Uuid,
        req: CreateOrUpsertCustomerRequest,
    ) -> Result<CustomerRow, RepoError> {
        let customer = sqlx::query_as::<_, CustomerRow>(
            r#"
            INSERT INTO customers (location_id, phone, name, consented_to_terms,
                                   consented_to_marketing, updated_at)
            VALUES ($1, $2, $3, $4, $5, NOW())
            ON CONFLICT (location_id, phone)
            DO UPDATE SET
              name = COALESCE($3, customers.name),
              consented_to_terms = COALESCE($4, customers.consented_to_terms),
              consented_to_marketing = COALESCE($5, customers.consented_to_marketing),
              updated_at = NOW()
            RETURNING id, location_id, phone, name, consented_to_terms, consented_to_marketing,
                      no_show_count, created_at, updated_at
            "#,
        )
        .bind(location_id)
        .bind(&req.phone)
        .bind(&req.name)
        .bind(req.consented_to_terms)
        .bind(req.consented_to_marketing)
        .fetch_one(&self.pool)
        .await?;

        Ok(customer)
    }

    async fn delete_customer(
        &self,
        location_id: Uuid,
        customer_id: Uuid,
    ) -> Result<(u64, u64), RepoError> {
        let mut tx = self.pool.begin().await?;

        // RED-LINE: verify customer belongs to location (RLS gate)
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM customers WHERE id = $1 AND location_id = $2)"
        )
        .bind(customer_id)
        .bind(location_id)
        .fetch_one(&mut *tx)
        .await?;

        if !exists {
            return Err(RepoError(sqlx::Error::RowNotFound));
        }

        // Erasure: cascade-delete from customers
        let deleted = sqlx::query_scalar::<_, i64>(
            "DELETE FROM customers WHERE id = $1 AND location_id = $2"
        )
        .bind(customer_id)
        .bind(location_id)
        .fetch_one(&mut *tx)
        .await? as u64;

        // Erasure: NULL out customer_id from orders (denormalization cleanup)
        let updated = sqlx::query_scalar::<_, i64>(
            "UPDATE orders SET customer_id = NULL WHERE customer_id = $1 AND location_id = $2"
        )
        .bind(customer_id)
        .bind(location_id)
        .fetch_one(&mut *tx)
        .await? as u64;

        tx.commit().await?;

        Ok((deleted, updated))
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
