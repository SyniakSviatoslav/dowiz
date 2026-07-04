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
//! directly from ARBITRARY route code — `with_tenant` (and, later, a session-pool equivalent for
//! LISTEN/NOTIFY) is meant to be the ONLY way an AUTHENTICATED-TENANT-scoped table gets touched.
//! Fields stay `pub(crate)` so `main.rs`/`crates/api/src/repo.rs` can wire them.
//!
//! ## S1 storefront-read resolves this module's "open question" — verified, not assumed
//! The S1 port (`crates/api/src/repo.rs::PgRepo`) reads `pools.operational` DIRECTLY, bypassing
//! `with_tenant` entirely. This is not a violation of the design above: every S1 route is
//! PUBLIC/unauthenticated and, verified against the live Node source
//! (`apps/api/src/routes/public/*.ts`, `spa-proxy.ts` lines before 300), NONE of them ever call
//! `withTenant` either — they filter by an explicit `slug`/`id` predicate against a BYPASSRLS-role
//! pool (`menu.ts:280-282`'s comment confirms the pool role bypasses RLS), not by an
//! authenticated tenant's session GUC. There is no "authenticated tenant" in these requests to
//! scope by. `withTenant` in `spa-proxy.ts` is used ONLY by the owner routes below line 300 (S3+,
//! out of S1 scope). So `with_tenant` correctly remains uncalled after this build too — this
//! resolves (rather than defers) `rebuild/README.md`'s "dual tenant GUC" open question for the
//! S1 surface specifically; S2 (auth) and later authenticated surfaces still need it.
//!
//! ## S3 catalog/admin CRUD resolves this module's "open question" — do NOT reuse `with_tenant`
//! The S3 council packet (`docs/design/rebuild-catalog-s3-council/proposal.md` §3, Q-GUC-FAMILY)
//! traced the live owner-write path to `packages/platform/src/auth/tenant.ts`'s `withTenant(pool,
//! userId, fn)`, which seats **`app.user_id`** (the owner's user id) — a DIFFERENT GUC, keyed by a
//! DIFFERENT value, than this module's `with_tenant` (`app.current_tenant`, a location id — the
//! courier/service root, ~102 RLS-policy sites). Owner-path RLS policies resolve the tenant from
//! `app.user_id` -> `memberships` via `app_member_location_ids()` (~34 sites). Reusing
//! `with_tenant` for an owner catalog write would seat the wrong GUC family with the wrong value:
//! masked today by BYPASSRLS, a total silent owner-write outage the moment NOBYPASSRLS goes live
//! (the exact anonymizer-N1/GDPR-worker wrong-GUC-family class this repo already hit once). So
//! `with_tenant`/`TenantId` stay reserved for the S6/S7 courier/service surfaces (still correctly
//! uncalled after S3), and `with_user` below is the owner-write combinator — same `BEGIN ->
//! set_config(..., true) -> f -> COMMIT/ROLLBACK` discipline, different GUC name and value.
//! PROVISIONAL: council Q1 (`docs/design/rebuild-catalog-s3-council/proposal.md`) is still
//! pending human RESOLVE on the broader write-pattern packet; this combinator itself is the
//! narrow, already-directed fix (course correction 2026-07-04) and does not wait on the rest of
//! that packet (locations.ts PATCH / menu-confirm.ts / menu-import.ts stay OUT of S3 scope).
//!
//! ## Why this whole module is STILL `#[allow(dead_code)]`
//! `with_tenant`/`TenantTxnError`/`SET_TENANT_STATEMENT` and `Pools.session` remain genuinely
//! unused after the S1+S3 ports (see above — no route built so far needs a `current_tenant`-scoped
//! transaction or a session-scoped LISTEN/NOTIFY connection; S3 uses `with_user` instead).
//! `Pools.operational` itself is NOT dead anymore (`PgRepo` and `with_user` both read it), but a
//! per-field allow isn't expressible at the struct-field granularity sqlx/serde-free code like
//! this needs, so the module-level allow stays until a courier/service surface gives `with_tenant`
//! its first real caller.
#![allow(
    dead_code,
    reason = "with_tenant/TenantId/Pools.session remain uncalled after S1+S3 — reserved for the courier/service GUC family, see module doc"
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

/// The exact statement text `with_user` executes — the S3 owner-write GUC (`app.user_id`), distinct
/// from `SET_TENANT_STATEMENT`'s `app.current_tenant`. Ports `packages/platform/src/auth/tenant.ts`'s
/// `withTenant(pool, userId, fn)` verbatim (that TS name is misleading — it seats `app.user_id`, not
/// a tenant/location id; the Rust name `with_user` is deliberately distinct from `with_tenant` so the
/// two GUC families can never be confused at a call site again — see module doc, Q-GUC-FAMILY).
pub const SET_USER_STATEMENT: &str = "SELECT set_config('app.user_id', $1, true)";

/// `BEGIN` -> `SET LOCAL app.user_id` (via `set_config(..., true)`) -> `f(&mut txn)` -> `COMMIT` on
/// `Ok` / `ROLLBACK` on `Err`. This is the ONLY sanctioned way an owner-authenticated route touches
/// a catalog table (`products`, `categories`, `modifier_groups`, `modifiers`, `menu_schedules`,
/// `locations.kitchen_busy_until`, `location_themes`, `theme_versions`, ...) — S3 routes must never
/// `pool.acquire()`/query the raw pool directly (`Pools` fields stay `pub(crate)`, see module doc).
///
/// `user_id` is the OWNER's user id (`OwnerClaims.user_id`/`.sub` — the two are always equal on an
/// owner token, S2 §5), never a location id: RLS resolves the visible location set from
/// `app.user_id` -> `memberships` via `app_member_location_ids()`, it is not itself a location
/// scope. Every call site must ALSO carry an explicit `WHERE location_id = $n` (or an
/// ownership-fold-in `INSERT ... SELECT ... WHERE ... location_id = $n`) — belt-and-suspenders that
/// holds independent of whether RLS is bypassed or enforced (council packet §3 clause 4).
///
/// Same shape as `with_tenant` (boxed-future callback — see that function's doc for why); kept as a
/// sibling function rather than a generic "which GUC name" parameter so the GUC family is a
/// call-site TYPE choice (`with_user` vs `with_tenant`), not a stringly-typed argument a caller
/// could pass wrong.
pub async fn with_user<T, F>(pool: &PgPool, user_id: uuid::Uuid, f: F) -> Result<T, TenantTxnError>
where
    for<'t> F: FnOnce(
        &'t mut Transaction<'_, Postgres>,
    ) -> Pin<Box<dyn Future<Output = Result<T, sqlx::Error>> + Send + 't>>,
{
    let mut txn = pool.begin().await.map_err(TenantTxnError::Begin)?;

    sqlx::query(SET_USER_STATEMENT)
        .bind(user_id.to_string())
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

    // ── S3 `with_user` (Q-GUC-FAMILY) — the owner-write GUC is `app.user_id`, NOT
    // `app.current_tenant`, and is keyed by the owner's user id, not a location id. ──

    #[test]
    fn set_user_statement_is_pinned_and_distinct_from_set_tenant_statement() {
        assert_eq!(
            SET_USER_STATEMENT,
            "SELECT set_config('app.user_id', $1, true)"
        );
        assert_ne!(
            SET_USER_STATEMENT, SET_TENANT_STATEMENT,
            "owner writes must seat a DIFFERENT GUC than the courier/service `with_tenant` path \
             (council packet Q-GUC-FAMILY) — a shared statement string here would be the exact \
             wrong-GUC-family bug this combinator exists to prevent"
        );
        assert_eq!(
            SET_USER_STATEMENT.matches('$').count(),
            1,
            "exactly one bind parameter"
        );
        assert!(
            SET_USER_STATEMENT.contains(", true)"),
            "is_local must be true — a session-scoped (false) GUC on a transaction-pooled \
             connection leaks across reuse (latent-GUC-bug class 1, same rule as with_tenant)"
        );
    }

    #[test]
    fn set_user_statement_binds_user_id_as_text() {
        let user_id = uuid::Uuid::new_v4();
        let bound: String = user_id.to_string();
        assert_eq!(
            bound.len(),
            36,
            "a hyphenated UUID string, not a bytes/uuid bind"
        );
    }

    /// Requires a live Postgres — same posture as `with_tenant_scopes_and_resets_the_guc` above.
    #[tokio::test]
    #[ignore = "requires a live Postgres — set DATABASE_URL_OPERATIONAL/DATABASE_URL_SESSION and run with --ignored"]
    async fn with_user_scopes_and_resets_the_guc() {
        let config = Config::from_env().expect("env must be valid to run this ignored test");
        let pools = Pools::connect(&config).await.expect("pools must connect");
        let user_id = uuid::Uuid::new_v4();

        let seen: String = with_user(&pools.operational, user_id, |txn| {
            Box::pin(async move {
                sqlx::query_scalar("SELECT current_setting('app.user_id', true)")
                    .fetch_one(&mut **txn)
                    .await
            })
        })
        .await
        .expect("with_user should succeed");

        assert_eq!(seen, user_id.to_string());

        let reset: Option<String> =
            sqlx::query_scalar("SELECT NULLIF(current_setting('app.user_id', true), '')")
                .fetch_one(&pools.operational)
                .await
                .expect("query must succeed");
        assert_eq!(
            reset, None,
            "app.user_id must not leak past the transaction"
        );
    }
}
