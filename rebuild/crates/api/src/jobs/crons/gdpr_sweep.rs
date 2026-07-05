//! `anonymizer.gdpr` cron plumbing — §2 "NOT S8" register: "the GDPR erasure LOGIC
//! (`gdpr_erase_customer` DEFINER draft, `anonymizeOrder`) — S9. S8 owns the
//! `anonymizer.gdpr`/`*.retention-sweep` cron plumbing + the batch-of-10 `FOR UPDATE SKIP LOCKED`
//! loop, never the erasure semantics." Q-GDPR-GLOBAL-SINGLETON (§11): the Node original serializes
//! ALL erasure work behind one global `singletonKey` (at most one erasure in-flight system-wide);
//! this port carries that same single-flight posture via the advisory lock (never the
//! per-erasure-request CAS the quirk register recommends investigating — that's a product
//! decision for whoever builds S9, not this scheduling shim).
//!
//! ## Scope note — plumbing only, intentionally incomplete pending S9
//! This module is deliberately a SKELETON: it acquires the single-flight lock and would drive a
//! batch-of-10 `SKIP LOCKED` claim loop over an erasure-request queue, but the exact request
//! table/DEFINER function this calls into is S9's to design (not yet built on this tree, per the
//! rebuild-map's own phase ordering — S8 lands before S9). Wiring the real claim query here NOW
//! would mean guessing at a schema this build has not verified, for a red-line PII-erasure
//! operation — the wrong kind of guess to make. [`run_once`] is therefore a documented no-op
//! today; `main.rs` does not spawn it (see that module's doc) until S9 supplies the real call.

use crate::jobs::advisory_lock::ANONYMIZER_RETENTION;
use crate::jobs::cron::try_with_lock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GdprSweepOutcome {
    /// No erasure-request claim query is wired yet (S9 scope, see module doc) — the lock was
    /// still exercised (proving single-flight plumbing works), but no rows were processed.
    NotYetWired,
}

pub async fn run_once(pool: &sqlx::PgPool) -> Result<Option<GdprSweepOutcome>, sqlx::Error> {
    try_with_lock(pool, ANONYMIZER_RETENTION, || async {
        tracing::debug!(
            cron = "anonymizer.gdpr",
            "single-flight lock acquired; erasure claim loop is S9 scope, not yet wired (see module doc)"
        );
        Ok(GdprSweepOutcome::NotYetWired)
    })
    .await
}

/// Spawned dark (mounted, scheduling-only) alongside every other S8 cron — S8 owns the timing +
/// single-flight plumbing per §2 scope even though S9 hasn't landed the erasure semantics yet
/// (module doc). Per-minute cadence matches the other batch-oriented sweeps in this fleet.
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
        assert_eq!(first, Some(GdprSweepOutcome::NotYetWired));
        // A second call must ALSO succeed (proving the first call released the lock, not held it).
        let second = run_once(&pool).await.expect("must not error");
        assert_eq!(second, Some(GdprSweepOutcome::NotYetWired));
    }
}
