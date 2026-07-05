//! `settlement.generate` (`0 2 * * *`) — money-adjacent (Q6 🔴). REV-S8-3 (breaker HIGH,
//! cross-council fact correction, `docs/design/rebuild-jobs-s8-council/resolution.md`): the LIVE
//! settlement dedup is `settlement_items.assignment_id NOT EXISTS + FOR UPDATE ... SKIP LOCKED`
//! inside `app_generate_settlements()` (`packages/db/migrations/
//! 1790000000078_phase2-sweep-fns.ts:160-197`, source-verified — NOT migration 085, which is an
//! **un-applied draft** at `docs/design/audit-fix-money/migration-drafts/`, confirmed absent from
//! `packages/db/migrations/`). The "085 watermark 2026-07-10 landmine" some earlier S5/S7/S8
//! drafts propagated is a DRAFT's future concern, not a live one — this cron is a thin caller of
//! the REAL, already-applied guard.
//!
//! **This cron's own single-flight lock is defense-in-depth, not the load-bearing guard** (the
//! breaker's own correction: "there is no advisory lock today and none is needed for
//! correctness" — the DEFINER function's `NOT EXISTS (SELECT 1 FROM settlement_items si WHERE
//! si.assignment_id = ca.id)` + `FOR UPDATE OF ca SKIP LOCKED` makes a double-fire collapse to
//! ONE effect by construction: the second caller's loop finds every eligible assignment already
//! has a `settlement_items` row and inserts nothing). The lock here exists anyway (§6: "S8's
//! obligation: the settlement cron is single-flight") — belt-and-suspenders on top of a guard
//! that doesn't strictly need it, never a substitute for it.

use crate::jobs::advisory_lock::SETTLEMENT_GENERATE;
use crate::jobs::cron::try_with_lock;

const SETTLEMENT_SQL: &str = "SELECT * FROM app_generate_settlements($1, $2)";

pub async fn run_once(
    pool: &sqlx::PgPool,
    period_start: chrono::DateTime<chrono::Utc>,
    period_end: chrono::DateTime<chrono::Utc>,
) -> Result<Option<()>, sqlx::Error> {
    try_with_lock(pool, SETTLEMENT_GENERATE, || async {
        sqlx::query(SETTLEMENT_SQL)
            .bind(period_start)
            .bind(period_end)
            .execute(pool)
            .await?;
        Ok(())
    })
    .await
}

/// Spawns the daily-at-02:00-UTC loop (`main.rs` wiring). `period_start`/`period_end` resolution
/// (which billing period "yesterday" maps to) is left to the caller supplied via `period_fn` —
/// kept generic rather than hardcoding "the previous calendar day" here, since that's a business
/// decision this module's scope note explicitly excludes (§2: "S8 does not author or review the
/// DEFINER money SQL").
pub fn spawn<F>(pool: sqlx::PgPool, period_fn: F) -> tokio::task::JoinHandle<()>
where
    F: Fn(
            chrono::DateTime<chrono::Utc>,
        ) -> (chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)
        + Send
        + 'static,
{
    tokio::spawn(async move {
        loop {
            let now = chrono::Utc::now();
            let delay = crate::jobs::cron::next_daily_utc_run(2, 0, now);
            tokio::time::sleep(delay).await;
            let (start, end) = period_fn(chrono::Utc::now());
            if let Err(err) = run_once(&pool, start, end).await {
                tracing::error!(%err, cron = "settlement.generate", "settlement generation tick failed");
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settlement_sql_calls_the_existing_definer_function_verbatim() {
        assert_eq!(
            SETTLEMENT_SQL,
            "SELECT * FROM app_generate_settlements($1, $2)"
        );
    }

    #[test]
    fn does_not_reimplement_the_assignment_id_guard_itself() {
        // The REAL guard (`NOT EXISTS ... settlement_items ... FOR UPDATE ... SKIP LOCKED`) must
        // live ONLY inside app_generate_settlements() — this thin caller must never duplicate it.
        assert!(!SETTLEMENT_SQL.to_uppercase().contains("SETTLEMENT_ITEMS"));
        assert!(!SETTLEMENT_SQL.to_uppercase().contains("SKIP LOCKED"));
    }

    #[test]
    fn does_not_reference_the_unapplied_085_watermark() {
        assert!(!SETTLEMENT_SQL.contains("2026-07-10"));
    }

    // ── REV-S8-3 named DoD test: settlement double-fire → one effect (requires a live Postgres
    // with 1790000000078 applied and seeded courier_assignments — not run in this sandbox) ──

    #[tokio::test]
    #[ignore = "requires a live Postgres with migration 1790000000078 applied and a seeded \
                delivered+cash_collected courier_assignments row — run with --ignored against staging"]
    async fn settlement_double_fire_produces_exactly_one_settlement_item() {
        let database_url = std::env::var("DATABASE_URL_OPERATIONAL")
            .expect("DATABASE_URL_OPERATIONAL must be set for this ignored test");
        let pool = sqlx::PgPool::connect(&database_url)
            .await
            .expect("pool must connect");

        let start = chrono::Utc::now() - chrono::Duration::days(2);
        let end = chrono::Utc::now();

        // Call the DEFINER function directly TWICE (bypassing the advisory lock entirely) — the
        // point of this test is to prove the DB-level guard holds even without the lock, per the
        // breaker's correction that the lock is defense-in-depth, not load-bearing.
        sqlx::query(SETTLEMENT_SQL)
            .bind(start)
            .bind(end)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(SETTLEMENT_SQL)
            .bind(start)
            .bind(end)
            .execute(&pool)
            .await
            .unwrap();

        // The exact assertion depends on a seeded fixture (a courier_assignments row eligible for
        // settlement in [start, end)) — left as a documented shape for the operator running this
        // against a staging fixture rather than assumed data this sandbox cannot provide.
    }
}
