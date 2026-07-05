//! Worker liveness roster — Q-WORKER-ROSTER-DUP (`docs/design/rebuild-jobs-s8-council/proposal.md`
//! §7, threat S8-T10). The Node source carries TWO hardcoded, independently-maintained arrays for
//! what is conceptually one roster: `CRITICAL_WORKERS` (5 ids, `liveness-checker.ts:13`,
//! env-overridable via `WORKER_CRITICAL_LIST`) is the real-time subset that pages immediately on a
//! stale heartbeat; `EXPECTED_WORKERS` (8 ids, `reconciliation.ts:212-213`) is the nightly
//! completeness check. The census confirmed this is an INTENTIONAL two-tier design, not drift —
//! but two parallel arrays with no shared source of truth is exactly how a future add-a-worker
//! change forgets one of them. FIX-IN-PORT: one roster, one `critical: bool` flag per entry.
//!
//! `critical_workers()`/`all_workers()` below are the two projections the old two arrays gave —
//! same information, one source.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkerRosterEntry {
    pub name: &'static str,
    /// `true` -> also in the old `CRITICAL_WORKERS` real-time-paging subset; `false` -> only in
    /// the old `EXPECTED_WORKERS` nightly-completeness set.
    pub critical: bool,
}

/// The unified roster. Every name that was in EITHER old array appears here exactly once, with
/// `critical` set from the OLD `CRITICAL_WORKERS` membership (verified: `dispatcher`,
/// `settlement-cron`, `dwell-monitor`, `anonymizer-retention`, `backup-hourly` — the 5-id subset).
pub const WORKER_ROSTER: &[WorkerRosterEntry] = &[
    WorkerRosterEntry {
        name: "dispatcher",
        critical: true,
    },
    WorkerRosterEntry {
        name: "settlement-cron",
        critical: true,
    },
    WorkerRosterEntry {
        name: "dwell-monitor",
        critical: true,
    },
    WorkerRosterEntry {
        name: "anonymizer-retention",
        critical: true,
    },
    WorkerRosterEntry {
        name: "backup-hourly",
        critical: true,
    },
    WorkerRosterEntry {
        name: "signal-raiser",
        critical: false,
    },
    WorkerRosterEntry {
        name: "liveness-checker",
        critical: false,
    },
    WorkerRosterEntry {
        name: "courier-stale_check",
        critical: false,
    },
];

/// The `liveness.check` "watcher of the watcher" projection — pages immediately on any of these
/// going stale (was `CRITICAL_WORKERS`).
pub fn critical_workers() -> Vec<&'static str> {
    WORKER_ROSTER
        .iter()
        .filter(|w| w.critical)
        .map(|w| w.name)
        .collect()
}

/// The `reconciliation.nightly` completeness-check projection — every worker that must have
/// heartbeated at least once in the window, critical or not (was `EXPECTED_WORKERS`). Not yet
/// consumed by `crate::jobs::crons::reconciliation` — that cron's nightly worker-completeness
/// check needs the same heartbeat-table schema verification flagged in
/// `crate::jobs::crons::liveness`'s module doc before it can read real data; this projection is
/// ready for that wiring.
#[allow(
    dead_code,
    reason = "awaiting the same heartbeat-schema verification as crons::liveness"
)]
pub fn all_workers() -> Vec<&'static str> {
    WORKER_ROSTER.iter().map(|w| w.name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_worker_name_appears_exactly_once() {
        let names: Vec<_> = WORKER_ROSTER.iter().map(|w| w.name).collect();
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(
            unique.len(),
            names.len(),
            "the whole point of unifying the two arrays is one entry per worker, never a duplicate"
        );
    }

    #[test]
    fn critical_workers_matches_the_old_five_id_roster() {
        let mut critical = critical_workers();
        critical.sort_unstable();
        let mut expected = vec![
            "anonymizer-retention",
            "backup-hourly",
            "dispatcher",
            "dwell-monitor",
            "settlement-cron",
        ];
        expected.sort_unstable();
        assert_eq!(critical, expected);
    }

    #[test]
    fn all_workers_is_a_superset_of_critical_workers() {
        let all: std::collections::HashSet<_> = all_workers().into_iter().collect();
        for name in critical_workers() {
            assert!(
                all.contains(name),
                "{name} must appear in the unified roster too"
            );
        }
        assert_eq!(all.len(), 8, "matches the old EXPECTED_WORKERS count");
    }
}
