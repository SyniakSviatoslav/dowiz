//! `anonymizer.gdpr` cron plumbing — §2 "NOT S8" register: "the GDPR erasure LOGIC
//! (`gdpr_erase_customer` DEFINER draft, `anonymizeOrder`) — S9. S8 owns the
//! `anonymizer.gdpr`/`*.retention-sweep` cron plumbing + the batch-of-10 `FOR UPDATE SKIP LOCKED`
//! loop, never the erasure semantics." Q-GDPR-GLOBAL-SINGLETON (§11): the Node original serializes
//! ALL erasure work behind one global `singletonKey` (at most one erasure in-flight system-wide);
//! this port carries that same single-flight posture via the advisory lock (never the
//! per-erasure-request CAS the quirk register recommends investigating — REV-S9-2 adopts the CAS
//! at the CLAIM statement itself instead, see `crate::jobs::gdpr_erasure::CLAIM_PENDING_SQL`).
//!
//! ## S9 landed — this module now WIRES the real erasure semantics (was a documented no-op)
//! [`run_once`] used to be a SKELETON (`GdprSweepOutcome::NotYetWired`) because the request
//! table/DEFINER function this calls into was S9's to design. S9 has now landed
//! (`crate::jobs::gdpr_erasure`, `docs/design/rebuild-gdpr-s9-council/`) — this cron's job is
//! UNCHANGED (acquire the single-flight lock, drive the batch loop on a fixed interval); it now
//! calls [`crate::jobs::gdpr_erasure::run_once_batch`] for the actual claim + erase + completion-
//! gate work instead of doing nothing. S8 still owns ONLY the timing/single-flight; the erasure
//! semantics live entirely in `jobs::gdpr_erasure` (S9), per the scope line above.

use crate::jobs::advisory_lock::ANONYMIZER_RETENTION;
use crate::jobs::cron::try_with_lock;
use crate::jobs::gdpr_erasure::{self, BatchOutcome};

/// Matches the Node worker's `LIMIT 10` batch size (`anonymizer-gdpr.ts:31`).
const BATCH_SIZE: i64 = 10;

pub async fn run_once(pool: &sqlx::PgPool) -> Result<Option<BatchOutcome>, sqlx::Error> {
    try_with_lock(pool, ANONYMIZER_RETENTION, || async {
        let outcome = gdpr_erasure::run_once_batch(pool, BATCH_SIZE).await?;
        if outcome.claimed > 0 {
            tracing::info!(
                cron = "anonymizer.gdpr",
                claimed = outcome.claimed,
                completed = outcome.completed,
                failed = outcome.failed,
                retried = outcome.retried,
                "gdpr erasure batch processed"
            );
        }
        Ok(outcome)
    })
    .await
}

/// Spawned dark (mounted, scheduling-only) alongside every other S8 cron — S8 owns the timing +
/// single-flight plumbing per §2 scope; the erasure semantics it now calls into are S9's
/// (`crate::jobs::gdpr_erasure`, module doc). Per-minute cadence matches the other batch-oriented
/// sweeps in this fleet.
pub fn spawn(pool: sqlx::PgPool) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Err(err) = run_once(&pool).await {
                tracing::error!(%err, cron = "anonymizer.gdpr", "gdpr sweep tick failed");
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── live-Postgres proof (requires DATABASE_URL_OPERATIONAL; not run in this sandbox) ──

    #[tokio::test]
    #[ignore = "requires a live Postgres — set DATABASE_URL_OPERATIONAL and run with --ignored"]
    async fn run_once_acquires_and_releases_its_lock_cleanly() {
        let database_url = std::env::var("DATABASE_URL_OPERATIONAL")
            .expect("DATABASE_URL_OPERATIONAL must be set for this ignored test");
        let pool = sqlx::PgPool::connect(&database_url)
            .await
            .expect("pool must connect");

        let first = run_once(&pool).await.expect("must not error");
        assert!(
            first.is_some(),
            "the lock must be acquired in a single-runner test"
        );
        // A second call must ALSO succeed (proving the first call released the lock, not held it).
        let second = run_once(&pool).await.expect("must not error");
        assert!(second.is_some());
    }
}
