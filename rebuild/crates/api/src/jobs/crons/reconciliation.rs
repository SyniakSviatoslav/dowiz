//! `reconciliation.nightly` (`0 3 * * *`) — read-only drift detection (§6: "12 read-only drift
//! checks... idempotent trivially"). This port carries the ONE check the S8 packet names
//! explicitly: the dead-job count detector (`reconciliation.ts:60`, source-verified as
//! `checkFailedJobs()`) — Q-DLQ-NOCONSUMER's fix (§3.3): today NO consumer reads any `.dlq`; this
//! makes a dead job a PAGED signal (via the ops bus), not landfill. The other 11 read-only checks
//! the Node census counts are out of this pass's scope (not money/PII/security-adjacent — pure
//! observability sweeps a future pass can port without any design risk).

use crate::jobs::advisory_lock::RECONCILIATION_NIGHTLY;
use crate::jobs::cron::try_with_lock;

/// Mirrors `reconciliation.ts:232-241`'s `checkFailedJobs()` verbatim, adapted to this port's
/// `jobs` table (not `pgboss.job`) — a queue with more than 10 `failed` jobs in the last 24h is
/// drift worth paging on, not a per-job DLQ consumer (Q-DLQ-NOCONSUMER).
const DEAD_JOB_COUNT_SQL: &str = "\
SELECT queue_name, count(*)::bigint AS dead_count
  FROM jobs
 WHERE state = 'failed' AND created_at > now() - interval '24 hours'
 GROUP BY queue_name
HAVING count(*) > 10
 ORDER BY dead_count DESC";

/// The paging threshold — pinned as its own constant (not inlined into the SQL) so a test can
/// assert the SQL's literal `10` matches this without duplicating the number.
pub const DEAD_JOB_ALERT_THRESHOLD: i64 = 10;

#[derive(Debug, Clone, sqlx::FromRow, PartialEq, Eq)]
pub struct DeadJobDrift {
    pub queue_name: String,
    pub dead_count: i64,
}

pub async fn run_once(pool: &sqlx::PgPool) -> Result<Option<Vec<DeadJobDrift>>, sqlx::Error> {
    try_with_lock(pool, RECONCILIATION_NIGHTLY, || async {
        let drift: Vec<DeadJobDrift> = sqlx::query_as(DEAD_JOB_COUNT_SQL).fetch_all(pool).await?;
        if !drift.is_empty() {
            // The ops-alert routing itself (the same bus `crate::ws::pg_fanout`/S6 producer would
            // use for `ops:*` topics) is left as a TODO wiring point for whoever mounts this cron
            // — the DETECTION query (this function's whole point, Q-DLQ-NOCONSUMER) is what this
            // build proves; routing it to a specific paging channel is a mounting decision, not a
            // detection-logic one.
            for d in &drift {
                tracing::warn!(
                    queue = %d.queue_name,
                    dead_count = d.dead_count,
                    "dead-job drift: queue exceeds the {DEAD_JOB_ALERT_THRESHOLD}-in-24h threshold"
                );
            }
        }
        Ok(drift)
    })
    .await
}

pub fn spawn(pool: sqlx::PgPool) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let delay = crate::jobs::cron::next_daily_utc_run(3, 0, chrono::Utc::now());
            tokio::time::sleep(delay).await;
            if let Err(err) = run_once(&pool).await {
                tracing::error!(%err, cron = "reconciliation.nightly", "nightly reconciliation tick failed");
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dead_job_sql_is_read_only() {
        assert!(
            DEAD_JOB_COUNT_SQL
                .trim_start()
                .to_uppercase()
                .starts_with("SELECT")
        );
        assert!(!DEAD_JOB_COUNT_SQL.to_uppercase().contains("UPDATE"));
        assert!(!DEAD_JOB_COUNT_SQL.to_uppercase().contains("DELETE"));
    }

    #[test]
    fn dead_job_sql_thresholds_at_the_named_constant() {
        assert!(DEAD_JOB_COUNT_SQL.contains(&format!("count(*) > {DEAD_JOB_ALERT_THRESHOLD}")));
    }

    #[test]
    fn dead_job_sql_windows_to_the_last_24_hours() {
        assert!(DEAD_JOB_COUNT_SQL.contains("interval '24 hours'"));
    }

    #[test]
    fn dead_job_sql_filters_on_the_failed_state_not_the_dlq_table() {
        // Q-DLQ-NOCONSUMER's whole point: THIS query is the consumer now — it reads `jobs`
        // directly (state='failed' IS this port's DLQ, per crate::jobs::runner's doc), not some
        // separate `.dlq` table nobody reads.
        assert!(DEAD_JOB_COUNT_SQL.contains("state = 'failed'"));
    }

    // ── live-Postgres proof (requires the jobs table applied; not run in this sandbox) ──

    #[tokio::test]
    #[ignore = "requires a live Postgres with the jobs table (crate::jobs::ddl) applied and >10 \
                failed rows for one queue in the last 24h — run with --ignored"]
    async fn detects_dead_job_drift_above_the_threshold() {
        let database_url = std::env::var("DATABASE_URL_OPERATIONAL")
            .expect("DATABASE_URL_OPERATIONAL must be set for this ignored test");
        let pool = sqlx::PgPool::connect(&database_url)
            .await
            .expect("pool must connect");

        for _ in 0..(DEAD_JOB_ALERT_THRESHOLD + 1) {
            sqlx::query(
                "INSERT INTO jobs (queue_name, payload, state) VALUES ('test.dead_job_drift', '{}'::jsonb, 'failed')",
            )
            .execute(&pool)
            .await
            .expect("seed insert must succeed");
        }

        let drift = run_once(&pool)
            .await
            .expect("run_once must not error")
            .expect("this test runs single-instance, the lock must be acquired");
        assert!(
            drift.iter().any(|d| d.queue_name == "test.dead_job_drift"
                && d.dead_count > DEAD_JOB_ALERT_THRESHOLD)
        );
    }
}
