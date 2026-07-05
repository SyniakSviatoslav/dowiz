//! `liveness.check` — the "watcher of the watcher" (§7, Q-LIVENESS-DETECT, Q-WORKER-ROSTER-DUP).
//! CARRIES the two-tier design verbatim: the heartbeat proves the VM breathes, NOT that the queue
//! drains (`order.timeout_sweep`'s own overdue-count DETECTION query,
//! `crate::jobs::crons::order_timeout_sweep`, is the real drain-signal — this cron does not
//! duplicate that). This cron pages on a CRITICAL worker (`crate::jobs::worker_roster::critical_workers`)
//! going heartbeat-stale — the unified roster (one source, a `critical: bool` flag) replaces the
//! Node original's two hardcoded parallel arrays.
//!
//! [`stale_critical_workers`] is the pure decision core (given a snapshot of
//! `(worker_name, last_heartbeat_age)` pairs + a staleness threshold, which CRITICAL workers are
//! stale) — fully unit-testable without a database. The actual heartbeat READ (`ops_worker_heartbeat`
//! or equivalent) is a thin wrapper the operator must verify against the live schema before this
//! cron runs for real — same "plumbing now, exact schema verified before go-live" posture as
//! `crate::jobs::crons::gdpr_sweep`, kept honest rather than guessed.

use std::time::Duration;

use crate::jobs::advisory_lock::LIVENESS_CHECK;
use crate::jobs::cron::try_with_lock;
use crate::jobs::worker_roster::critical_workers;

/// A worker heartbeats less often than this is considered stale — generously above the
/// per-minute cadence most crons in this fleet run at, so normal tick jitter never false-pages.
pub const STALE_THRESHOLD: Duration = Duration::from_secs(5 * 60);

/// Pure: given each worker's time-since-last-heartbeat, returns which CRITICAL workers exceed
/// [`STALE_THRESHOLD`] — a worker absent from `heartbeats` entirely (never seen) counts as
/// maximally stale (`Duration::MAX`), not silently skipped.
pub fn stale_critical_workers(heartbeats: &[(&str, Duration)]) -> Vec<String> {
    critical_workers()
        .into_iter()
        .filter(|critical_name| {
            let age = heartbeats
                .iter()
                .find(|(name, _)| name == critical_name)
                .map(|(_, age)| *age)
                .unwrap_or(Duration::MAX);
            age >= STALE_THRESHOLD
        })
        .map(str::to_string)
        .collect()
}

pub async fn run_once(pool: &sqlx::PgPool) -> Result<Option<Vec<String>>, sqlx::Error> {
    try_with_lock(pool, LIVENESS_CHECK, || async {
        // The heartbeat READ itself — schema-verification pending (module doc). Returns an empty
        // snapshot today (every critical worker reads as maximally stale, which is the SAFE
        // failure direction for a liveness check: "unknown" pages, it never silently passes).
        let heartbeats: Vec<(&str, Duration)> = Vec::new();
        let stale = stale_critical_workers(&heartbeats);
        if !stale.is_empty() {
            tracing::error!(?stale, "liveness.check: critical worker(s) heartbeat-stale");
        }
        Ok(stale)
    })
    .await
}

pub fn spawn(pool: sqlx::PgPool) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Err(err) = run_once(&pool).await {
                tracing::error!(%err, cron = "liveness.check", "liveness tick failed");
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_heartbeat_is_never_stale() {
        let heartbeats: Vec<(&str, Duration)> = critical_workers()
            .into_iter()
            .map(|name| (name, Duration::from_secs(10)))
            .collect();
        assert!(stale_critical_workers(&heartbeats).is_empty());
    }

    #[test]
    fn a_missing_heartbeat_counts_as_maximally_stale_not_silently_skipped() {
        // No entries at all — every critical worker must be reported, "fail loud on unknown."
        let stale = stale_critical_workers(&[]);
        assert_eq!(stale.len(), critical_workers().len());
    }

    #[test]
    fn only_workers_past_the_threshold_are_reported() {
        let mut heartbeats: Vec<(&str, Duration)> = critical_workers()
            .into_iter()
            .map(|name| (name, Duration::from_secs(10)))
            .collect();
        // Make exactly one critical worker stale.
        let target = heartbeats[0].0;
        heartbeats[0].1 = STALE_THRESHOLD + Duration::from_secs(1);
        let stale = stale_critical_workers(&heartbeats);
        assert_eq!(stale, vec![target.to_string()]);
    }

    #[test]
    fn non_critical_workers_never_appear_even_if_missing() {
        // e.g. "signal-raiser" is in the unified roster but NOT critical — must never show up in
        // this cron's paging output regardless of its heartbeat state (it's the nightly
        // completeness check's job, not this one's — Q-LIVENESS-DETECT's whole distinction).
        let stale = stale_critical_workers(&[]);
        assert!(!stale.contains(&"signal-raiser".to_string()));
    }
}
