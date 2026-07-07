//! Postgres implementation of ChannelsRepo for S3 channel registry (Phase 1.1).

use async_trait::async_trait;
use uuid::Uuid;
use sqlx::{PgPool, Row};

use crate::repo::RepoError;
use super::{ChannelsRepo, ChannelRow, ChannelWithAttribution};
use crate::routes::owner::assert_active_owner_membership;

pub struct PgChannelsRepo {
    pool: PgPool,
}

impl PgChannelsRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ChannelsRepo for PgChannelsRepo {
    async fn list(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
    ) -> Result<Vec<ChannelRow>, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, move |txn| {
            Box::pin(async move {
                // Seat the RLS context: membership read first.
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(Vec::new());
                }
                sqlx::query_as::<_, ChannelRow>(
                    "SELECT id, location_id, kind, name, token, active, created_at
                     FROM sales_channels
                     WHERE location_id = $1
                     ORDER BY created_at DESC",
                )
                .bind(location_id)
                .fetch_all(&mut **txn)
                .await
                .map_err(|e| e.into())
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn list_with_attribution(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
    ) -> Result<Vec<ChannelWithAttribution>, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, move |txn| {
            Box::pin(async move {
                // Seat the RLS context: membership read first.
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(Vec::new());
                }
                let rows = sqlx::query(
                    "SELECT sc.id, sc.location_id, sc.kind, sc.name, sc.token, sc.active, sc.created_at,
                            COALESCE(COUNT(o.id), 0)::bigint as order_count
                     FROM sales_channels sc
                     LEFT JOIN orders o ON o.location_id = sc.location_id
                                       AND (o.metadata->>'channel') = sc.kind
                     WHERE sc.location_id = $1
                     GROUP BY sc.id, sc.location_id, sc.kind, sc.name, sc.token, sc.active, sc.created_at
                     ORDER BY sc.created_at DESC",
                )
                .bind(location_id)
                .fetch_all(&mut **txn)
                .await?;

                let results = rows
                    .into_iter()
                    .map(|row| {
                        let channel = ChannelRow {
                            id: row.get("id"),
                            location_id: row.get("location_id"),
                            kind: row.get("kind"),
                            name: row.get("name"),
                            token: row.get("token"),
                            active: row.get("active"),
                            created_at: row.get("created_at"),
                        };
                        let order_count = row.get("order_count");
                        ChannelWithAttribution { channel, order_count }
                    })
                    .collect();
                Ok(results)
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn create(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        kind: String,
        name: String,
    ) -> Result<ChannelRow, RepoError> {
        // Validate kind against allowlist (mirroring the DB CHECK constraint).
        let valid_kinds = vec![
            "web-direct", "qr", "nfc", "gbp", "apple-maps",
            "instagram", "facebook", "whatsapp", "telegram-tma",
            "kiosk", "widget", "agent", "other"
        ];
        if !valid_kinds.contains(&kind.as_str()) {
            return Err(RepoError(
                sqlx::Error::ColumnNotFound(
                    format!("Invalid channel kind: {}", kind),
                ),
            ));
        }

        // Generate a unique token: base64-url-safe random bytes.
        let token = base64_urlsafe_token();

        crate::db::with_user(&self.pool, owner_user_id, move |txn| {
            Box::pin(async move {
                // Seat the RLS context.
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Err(sqlx::Error::RowNotFound.into());
                }
                sqlx::query_as::<_, ChannelRow>(
                    "INSERT INTO sales_channels (location_id, kind, name, token, active, created_at)
                     VALUES ($1, $2, $3, $4, true, now())
                     RETURNING id, location_id, kind, name, token, active, created_at",
                )
                .bind(location_id)
                .bind(&kind)
                .bind(&name)
                .bind(&token)
                .fetch_one(&mut **txn)
                .await
                .map_err(|e| e.into())
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn update(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        channel_id: Uuid,
        name: Option<String>,
        active: Option<bool>,
    ) -> Result<Option<ChannelRow>, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, move |txn| {
            Box::pin(async move {
                // Seat the RLS context.
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(None);
                }

                // Build query based on which fields are provided
                if let Some(n) = name {
                    if let Some(a) = active {
                        // Both name and active
                        return sqlx::query_as::<_, ChannelRow>(
                            "UPDATE sales_channels SET name = $1, active = $2
                             WHERE id = $3 AND location_id = $4
                             RETURNING id, location_id, kind, name, token, active, created_at",
                        )
                        .bind(&n)
                        .bind(a)
                        .bind(channel_id)
                        .bind(location_id)
                        .fetch_optional(&mut **txn)
                        .await
                        .map_err(|e| e.into());
                    } else {
                        // Only name
                        return sqlx::query_as::<_, ChannelRow>(
                            "UPDATE sales_channels SET name = $1
                             WHERE id = $2 AND location_id = $3
                             RETURNING id, location_id, kind, name, token, active, created_at",
                        )
                        .bind(&n)
                        .bind(channel_id)
                        .bind(location_id)
                        .fetch_optional(&mut **txn)
                        .await
                        .map_err(|e| e.into());
                    }
                } else if let Some(a) = active {
                    // Only active
                    return sqlx::query_as::<_, ChannelRow>(
                        "UPDATE sales_channels SET active = $1
                         WHERE id = $2 AND location_id = $3
                         RETURNING id, location_id, kind, name, token, active, created_at",
                    )
                    .bind(a)
                    .bind(channel_id)
                    .bind(location_id)
                    .fetch_optional(&mut **txn)
                    .await
                    .map_err(|e| e.into());
                } else {
                    // No updates — return current row
                    return sqlx::query_as::<_, ChannelRow>(
                        "SELECT id, location_id, kind, name, token, active, created_at
                         FROM sales_channels
                         WHERE id = $1 AND location_id = $2",
                    )
                    .bind(channel_id)
                    .bind(location_id)
                    .fetch_optional(&mut **txn)
                    .await
                    .map_err(|e| e.into());
                }
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn delete(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        channel_id: Uuid,
    ) -> Result<bool, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, move |txn| {
            Box::pin(async move {
                // Seat the RLS context.
                if !assert_active_owner_membership(txn, owner_user_id, location_id).await? {
                    return Ok(false);
                }
                let result = sqlx::query(
                    "DELETE FROM sales_channels WHERE id = $1 AND location_id = $2",
                )
                .bind(channel_id)
                .bind(location_id)
                .execute(&mut **txn)
                .await?;
                Ok(result.rows_affected() > 0)
            })
        })
        .await
        .map_err(map_txn_err)
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────────────────────

/// Generate a URL-safe base64 random token (32 bytes = 256 bits).
fn base64_urlsafe_token() -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    let bytes = rand::random::<[u8; 32]>();
    URL_SAFE_NO_PAD.encode(&bytes)
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
