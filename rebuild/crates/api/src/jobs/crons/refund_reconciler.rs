//! `refund_due.reconcile` tick — calls `app_reconcile_refund_due()` (§6). **That function does
//! not exist on any applied migration today** — verified: it exists only as an UN-APPLIED DRAFT
//! at `docs/design/audit-fix-money/migration-drafts/1790000000087_refund-due-reconciler.ts`
//! (headed "⛔ OPERATOR ACTION REQUIRED: place this file VERBATIM..."), alongside its companion
//! trigger draft `1790000000086_refund-due-trigger.ts`. The Node source ALREADY calls this
//! missing function today (`order-timeout-sweep.ts:139`, wrapped in try/catch — "a missing fn
//! degrades to a logged error, never a crashed sweep"). This port carries that exact
//! degrade-not-crash posture: a `42883` (undefined_function) Postgres error is caught and logged,
//! never propagated as a sweep failure — §6's idempotency guard (mig 086's **N5 partial unique
//! `(payment_id) WHERE type='refund_due'`**) only applies once 086/087 are actually landed; until
//! then this tick is a documented no-op, not a silently-crashing one.

use crate::jobs::advisory_lock::REFUND_DUE_RECONCILE;
use crate::jobs::cron::try_with_lock;

const RECONCILE_SQL: &str = "SELECT * FROM app_reconcile_refund_due()";

/// Postgres error code for "undefined function" — what this call reports while 086/087 remain
/// unapplied drafts.
const UNDEFINED_FUNCTION_SQLSTATE: &str = "42883";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconcileOutcome {
    Ran,
    /// `app_reconcile_refund_due()` is not yet applied (086/087 are drafts) — degraded, not an
    /// error the caller should page on.
    FunctionNotYetApplied,
}

pub async fn run_once(pool: &sqlx::PgPool) -> Result<Option<ReconcileOutcome>, sqlx::Error> {
    try_with_lock(pool, REFUND_DUE_RECONCILE, || async {
        match sqlx::query(RECONCILE_SQL).execute(pool).await {
            Ok(_) => Ok(ReconcileOutcome::Ran),
            Err(sqlx::Error::Database(db_err))
                if db_err.code().as_deref() == Some(UNDEFINED_FUNCTION_SQLSTATE) =>
            {
                tracing::warn!(
                    cron = "refund_due.reconcile",
                    "app_reconcile_refund_due() is not yet applied (086/087 are unapplied drafts) \
                     — degrading to a no-op tick, not crashing the sweep"
                );
                Ok(ReconcileOutcome::FunctionNotYetApplied)
            }
            Err(other) => Err(other),
        }
    })
    .await
}

pub fn spawn(pool: sqlx::PgPool) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Err(err) = run_once(&pool).await {
                tracing::error!(%err, cron = "refund_due.reconcile", "reconcile tick failed");
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconcile_sql_calls_the_function_verbatim() {
        assert_eq!(RECONCILE_SQL, "SELECT * FROM app_reconcile_refund_due()");
    }

    #[test]
    fn does_not_reimplement_the_partial_unique_guard_itself() {
        // The real guard (mig 086's N5 partial unique `(payment_id) WHERE type='refund_due'`)
        // must live ONLY inside the DEFINER function — this thin caller must never itself INSERT
        // into `payment_events`, only invoke the function (whose NAME legitimately contains
        // "refund_due" — that substring alone isn't the thing being guarded against).
        assert!(!RECONCILE_SQL.to_uppercase().contains("INSERT"));
        assert!(!RECONCILE_SQL.to_uppercase().contains("PAYMENT_EVENTS"));
    }

    // ── live-Postgres proof (requires DATABASE_URL_OPERATIONAL; not run in this sandbox) ──

    #[tokio::test]
    #[ignore = "requires a live Postgres — set DATABASE_URL_OPERATIONAL and run with --ignored. \
                On today's tree (086/087 unapplied) this proves the graceful-degrade path; once \
                086/087 land, re-run to confirm it proves ReconcileOutcome::Ran instead."]
    async fn reconcile_degrades_gracefully_while_the_function_is_unapplied() {
        let database_url = std::env::var("DATABASE_URL_OPERATIONAL")
            .expect("DATABASE_URL_OPERATIONAL must be set for this ignored test");
        let pool = sqlx::PgPool::connect(&database_url)
            .await
            .expect("pool must connect");

        let outcome = run_once(&pool)
            .await
            .expect("must not error even if the fn is missing");
        // Either outcome is a PASS depending on whether 086/087 have landed on the target DB —
        // the assertion that matters is that `run_once` never returns `Err` for the
        // undefined-function case, proven by `.expect` above not panicking.
        assert!(
            outcome.is_some(),
            "the lock must be acquired in a single-runner test"
        );
    }
}
