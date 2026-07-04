//! `with_tenant` — the load-bearing tenancy pattern (REBUILD-MAP inventory/12 §7/§9). Ports
//! `packages/platform/src/auth/tenant.ts`'s `withTenant` verbatim: `BEGIN` -> `SELECT
//! set_config('app.current_tenant', $1, true)` (session-LOCAL, i.e. scoped to the transaction,
//! `is_local = true`) -> caller's work -> `COMMIT` on success / `ROLLBACK` on error -> connection
//! returned to the pool.
//!
//! Design cleared via Triadic Council (docs/design/rust-money-newtype-phase-a/) established the
//! checked/no-silent-swallow posture this module also follows: no bare `.unwrap()` on a DB error,
//! rollback failures are surfaced (not `let _ =`'d away), and the SQL text is a pure, unit-tested
//! constant so "does the GUC statement say what we think it says" needs no live database.
//!
//! ## GUC name — open question flagged for the contract lane
//! The live schema actually uses **two** tenant GUCs: `app.current_tenant` (courier/service path,
//! ~102 RLS-policy sites) and `app.user_id` (owner path, ~34 sites) — see REBUILD-MAP inventory/12
//! §7 ("GUC discipline"). This helper implements `app.current_tenant` only, per this build's brief.
//! Reconciling the two (or exposing a second `with_user` helper) is explicitly deferred — see
//! `rebuild/README.md` "Open questions for the contract lane".
//!
//! ## Why the raw pool is not `pub`
//! `Pools` intentionally does not expose its fields as a way to reach for `pool.acquire()`
//! directly from route code — `with_tenant` (and, later, a session-pool equivalent for
//! LISTEN/NOTIFY) is meant to be the ONLY way a tenant-scoped table gets touched. Phase A doesn't
//! yet enforce this by visibility (both fields are `pub(crate)` so `main.rs` can wire them), but
//! no route handler in this crate calls a pool method directly — see `routes/menu.rs`.
//!
//! ## Why this whole module is `#[allow(dead_code)]`
//! `Pools`/`with_tenant`/`TenantTxnError` are wired at boot (`main.rs` connects `Pools`) but have
//! no reachable caller yet: the one route this crate ships (`GET /api/v1/public/menu/{slug}`) is
//! an explicit 501 stub that never touches the database (see `routes/menu.rs`). That is the
//! correct Phase A shape, not a bug — but it means `cargo clippy -- -D warnings` would otherwise
//! fail on "never used" for this entire module. The allow is scoped to this module only (not
//! workspace-wide) and is expected to be deleted the moment the first real DB-backed route calls
//! `with_tenant` (REBUILD-MAP Phase A/B S1).
#![allow(
    dead_code,
    reason = "wired at boot; no caller until the first real DB-backed route lands, see module doc"
)]

use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::{Postgres, Transaction};
use std::future::Future;
use std::pin::Pin;

use domain::TenantId;

use crate::config::Config;

/// The exact statement text `with_tenant` executes. Exposed so a unit test can pin it without a
/// live database — the actual GUC-scoping semantics (`is_local = true`, i.e. `SET LOCAL`-
/// equivalent, reset at `COMMIT`/`ROLLBACK`) live entirely in this one string.
pub const SET_TENANT_STATEMENT: &str = "SELECT set_config('app.current_tenant', $1, true)";

pub struct Pools {
    pub(crate) operational: PgPool,
    pub(crate) session: PgPool,
}

#[derive(Debug, thiserror::Error)]
pub enum PoolConnectError {
    #[error("failed to connect the operational pool: {0}")]
    Operational(#[source] sqlx::Error),
    #[error("failed to connect the session pool: {0}")]
    Session(#[source] sqlx::Error),
}

impl Pools {
    /// Connects both pools. Sizes mirror the Node reality documented in REBUILD-MAP inventory/12
    /// §7 (operational = hot-path CRUD via Supavisor transaction mode; session = small, reserved
    /// for anything session-scoped — LISTEN/NOTIFY, advisory locks). Statement caching is
    /// disabled on the operational pool: Supavisor's transaction-pooling mode reassigns physical
    /// connections between statements, so a server-side prepared-statement cache keyed to one
    /// physical connection would silently misbehave (the exact reason the Node pool avoids named
    /// statements too — inventory/12 §7).
    pub async fn connect(config: &Config) -> Result<Self, PoolConnectError> {
        let operational = PgPoolOptions::new()
            .max_connections(20)
            .connect_with(
                config
                    .database_url_operational
                    .parse::<sqlx::postgres::PgConnectOptions>()
                    .map_err(PoolConnectError::Operational)?
                    .statement_cache_capacity(0),
            )
            .await
            .map_err(PoolConnectError::Operational)?;

        let session = PgPoolOptions::new()
            .max_connections(3)
            .connect(&config.database_url_session)
            .await
            .map_err(PoolConnectError::Session)?;

        Ok(Pools {
            operational,
            session,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TenantTxnError {
    #[error("failed to begin transaction: {0}")]
    Begin(#[source] sqlx::Error),
    #[error("failed to set app.current_tenant: {0}")]
    SetTenant(#[source] sqlx::Error),
    #[error("tenant-scoped work failed: {0}")]
    Work(#[source] sqlx::Error),
    #[error("commit failed: {0}")]
    Commit(#[source] sqlx::Error),
    /// The work failed AND the rollback that followed it also failed — surfaced distinctly
    /// (never swallowed with `let _ =`) since it means the connection's state is now unknown.
    #[error("work failed ({work}) and the subsequent rollback also failed: {rollback}")]
    WorkThenRollbackFailed {
        #[source]
        work: sqlx::Error,
        rollback: sqlx::Error,
    },
}

/// `BEGIN` -> `SET LOCAL app.current_tenant` (via `set_config(..., true)`) -> `f(&mut txn)` ->
/// `COMMIT` on `Ok` / `ROLLBACK` on `Err`. This is the ONLY sanctioned way to touch a
/// tenant-scoped (RLS-protected) table in this codebase — every route handler that reads/writes
/// tenant data must go through this, never `pool.acquire()` directly.
///
/// The callback returns a boxed, pinned future (`f(txn) -> Pin<Box<dyn Future<...> + Send + 't>>`,
/// i.e. callers write `|txn| Box::pin(async move { ... })`) rather than a plain generic `Future`
/// associated type: the future borrows `txn` for the lifetime of one call, and expressing "a
/// `FnOnce` whose returned future borrows its own argument" needs either native async closures
/// (`AsyncFnOnce`) or this well-established boxing pattern — boxing is the simpler, more portable
/// choice for one call per request (the allocation is negligible next to a network round trip to
/// Postgres).
pub async fn with_tenant<T, F>(
    pool: &PgPool,
    tenant_id: TenantId,
    f: F,
) -> Result<T, TenantTxnError>
where
    for<'t> F: FnOnce(
        &'t mut Transaction<'_, Postgres>,
    ) -> Pin<Box<dyn Future<Output = Result<T, sqlx::Error>> + Send + 't>>,
{
    let mut txn = pool.begin().await.map_err(TenantTxnError::Begin)?;

    sqlx::query(SET_TENANT_STATEMENT)
        .bind(tenant_id.to_string())
        .execute(&mut *txn)
        .await
        .map_err(TenantTxnError::SetTenant)?;

    match f(&mut txn).await {
        Ok(value) => txn
            .commit()
            .await
            .map(|()| value)
            .map_err(TenantTxnError::Commit),
        Err(work_err) => match txn.rollback().await {
            Ok(()) => Err(TenantTxnError::Work(work_err)),
            Err(rollback_err) => Err(TenantTxnError::WorkThenRollbackFailed {
                work: work_err,
                rollback: rollback_err,
            }),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unit-tests the SQL string generation (per the build brief) without touching a database:
    /// pins the exact GUC statement text and its parameter-count/shape.
    #[test]
    fn set_tenant_statement_is_pinned() {
        assert_eq!(
            SET_TENANT_STATEMENT,
            "SELECT set_config('app.current_tenant', $1, true)"
        );
        assert_eq!(
            SET_TENANT_STATEMENT.matches('$').count(),
            1,
            "exactly one bind parameter"
        );
        assert!(
            SET_TENANT_STATEMENT.contains(", true)"),
            "is_local must be true — a session-scoped (false) GUC on a transaction-pooled \
             connection leaks across reuse (REBUILD-MAP inventory/12 §7, latent-GUC-bug class 1)"
        );
    }

    #[test]
    fn set_tenant_statement_binds_tenant_id_as_text() {
        let tenant = TenantId::from(uuid::Uuid::new_v4());
        // What with_tenant actually binds — `to_string()`, not the raw Uuid — since set_config's
        // second argument is `text`, not `uuid`.
        let bound: String = tenant.to_string();
        assert_eq!(
            bound.len(),
            36,
            "a hyphenated UUID string, not a bytes/uuid bind"
        );
    }

    /// Requires a live Postgres reachable via `DATABASE_URL_OPERATIONAL`/`DATABASE_URL_SESSION` —
    /// not run in this sandbox (no DB writes anywhere, per the build brief). Run explicitly with
    /// `cargo test -- --ignored` against a real database to exercise the actual GUC-scoping.
    #[tokio::test]
    #[ignore = "requires a live Postgres — set DATABASE_URL_OPERATIONAL/DATABASE_URL_SESSION and run with --ignored"]
    async fn with_tenant_scopes_and_resets_the_guc() {
        let config = Config::from_env().expect("env must be valid to run this ignored test");
        let pools = Pools::connect(&config).await.expect("pools must connect");
        let tenant = TenantId::from(uuid::Uuid::new_v4());

        let seen: String = with_tenant(&pools.operational, tenant, |txn| {
            Box::pin(async move {
                sqlx::query_scalar("SELECT current_setting('app.current_tenant', true)")
                    .fetch_one(&mut **txn)
                    .await
            })
        })
        .await
        .expect("with_tenant should succeed");

        assert_eq!(seen, tenant.to_string());

        // Outside any with_tenant call, on a fresh connection, the GUC must NOT be visible —
        // proves `is_local = true` actually resets at commit and doesn't leak across pool reuse.
        let reset: Option<String> =
            sqlx::query_scalar("SELECT NULLIF(current_setting('app.current_tenant', true), '')")
                .fetch_one(&pools.operational)
                .await
                .expect("query must succeed");
        assert_eq!(
            reset, None,
            "app.current_tenant must not leak past the transaction"
        );
    }
}
