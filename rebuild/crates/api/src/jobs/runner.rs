//! The hand-rolled `SELECT ... FOR UPDATE SKIP LOCKED` job runner — REBUILD-MAP §2's decision
//! register verdict, made buildable (`docs/design/rebuild-jobs-s8-council/proposal.md` §3, Q1 🔴,
//! load-bearing). Replaces pg-boss for the ~9 queues that genuinely need a durable cross-process
//! queue (§3.7); the other 21 (pure cron sweeps) shed the queue abstraction entirely
//! (`crate::jobs::cron`).
//!
//! ## At-least-once by construction (no at-most-once exists, anywhere)
//! [`claim`] is the ONLY way a job transitions `queued`/reclaimable-`active` → `active`. Whatever
//! the handler does after that (send a Telegram message, call a DEFINER money function) is a
//! side effect that happens BEFORE the separate write that marks the job `completed`
//! ([`complete`]). If the process crashes between the side effect and that write, the job's
//! `locked_until` eventually lapses and [`claim`]'s own predicate reclaims it — the SAME row runs
//! again. There is no code path that makes this at-most-once; every handler that does something
//! non-idempotent (a Telegram send, a money-adjacent DEFINER call) MUST guard itself
//! (`crate::jobs::dedup` for notifications; the DEFINER functions' own `WHERE status='PENDING'` /
//! `NOT EXISTS` guards for money crons, §6/§8) — this module supplies at-least-once delivery and
//! visibility-timeout reclaim, nothing more.
//!
//! ## Visibility timeout doubles as the reclaim mechanism — no separate reaper
//! [`CLAIM_SQL`]'s `WHERE` clause matches BOTH fresh `queued` work AND any `active` job whose
//! `locked_until` has already passed — so a dead worker's claimed-but-abandoned job is picked up
//! by the very next claim tick, with no separate sweeper thread. `locked_until` MUST exceed the
//! handler's max bounded runtime (every external call in this surface is timeout-bounded — the
//! Telegram/push/email adapters each cap their own HTTP call, `crate::jobs::channels`) or a job
//! could be reclaimed WHILE its previous attempt is still legitimately running; the idempotency
//! guards above are what make that survivable even if `vt` is set too tight, but too-tight is
//! still a real cost (a live double-run) that must be tuned intentionally, not discovered.

use sqlx::PgExecutor;
use sqlx::postgres::PgRow;
use sqlx::{FromRow, Row};

use crate::jobs::backoff::backoff_for_attempt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Queued,
    Active,
    Completed,
    Failed,
}

impl JobState {
    fn from_db_str(raw: &str) -> Option<Self> {
        match raw {
            "queued" => Some(JobState::Queued),
            "active" => Some(JobState::Active),
            "completed" => Some(JobState::Completed),
            "failed" => Some(JobState::Failed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Job {
    pub id: i64,
    pub queue_name: String,
    pub payload: serde_json::Value,
    /// Always `Active` immediately after a successful `claim` (the claim SQL sets it) — carried
    /// on the struct for completeness/future handlers that branch on it, not read by
    /// `crate::jobs::worker` today (every claimed job is, by definition, `Active`).
    #[allow(
        dead_code,
        reason = "populated for completeness; no handler needs to branch on it yet"
    )]
    pub state: JobState,
    pub attempts: i32,
    pub max_attempts: i32,
}

impl FromRow<'_, PgRow> for Job {
    fn from_row(row: &PgRow) -> Result<Self, sqlx::Error> {
        let state_raw: String = row.try_get("state")?;
        let state = JobState::from_db_str(&state_raw).ok_or_else(|| {
            sqlx::Error::Decode(format!("jobs.state: unrecognized value {state_raw:?}").into())
        })?;
        Ok(Job {
            id: row.try_get("id")?,
            queue_name: row.try_get("queue_name")?,
            payload: row.try_get("payload")?,
            state,
            attempts: row.try_get("attempts")?,
            max_attempts: row.try_get("max_attempts")?,
        })
    }
}

/// The claim statement — `docs/design/rebuild-jobs-s8-council/proposal.md` §3.1, adapted to this
/// port's column names (`crate::jobs::ddl::JOBS_TABLE_DDL`). `FOR UPDATE SKIP LOCKED` is the
/// entire multi-consumer safety property: two runners racing this same query never claim the same
/// row (one gets it, the other's `SKIP LOCKED` silently steps over it and claims the next).
/// `attempts = attempts + 1` happens AT CLAIM TIME (not at failure time) — so `attempts` always
/// reflects "how many times has this row been picked up," including the current, still-running
/// one; [`fail`] compares against this post-increment value.
const CLAIM_SQL: &str = "\
UPDATE jobs
   SET state = 'active',
       locked_until = now() + make_interval(secs => $1),
       attempts = attempts + 1,
       started_at = now()
 WHERE id IN (
   SELECT id FROM jobs
    WHERE (state = 'queued' AND run_after <= now())
       OR (state = 'active' AND locked_until < now())
    ORDER BY priority DESC, run_after
    FOR UPDATE SKIP LOCKED
    LIMIT $2
 )
RETURNING id, queue_name, payload, state, attempts, max_attempts";

/// Claims up to `batch_size` ready/reclaimable jobs, setting `locked_until` `vt_seconds` from now.
/// `vt_seconds` must exceed the slowest handler's bounded runtime (module doc).
pub async fn claim(
    executor: impl PgExecutor<'_>,
    vt_seconds: i64,
    batch_size: i64,
) -> Result<Vec<Job>, sqlx::Error> {
    sqlx::query_as(CLAIM_SQL)
        .bind(vt_seconds)
        .bind(batch_size)
        .fetch_all(executor)
        .await
}

const COMPLETE_SQL: &str =
    "UPDATE jobs SET state = 'completed', completed_at = now() WHERE id = $1";

/// The separate write that closes the at-least-once window (module doc). Called ONLY after the
/// handler's side effect has fully happened — a crash before this runs is exactly the re-run case
/// [`claim`]'s predicate exists to recover from.
pub async fn complete(executor: impl PgExecutor<'_>, job_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query(COMPLETE_SQL)
        .bind(job_id)
        .execute(executor)
        .await?;
    Ok(())
}

#[allow(
    dead_code,
    reason = "the real caller is S5's POST /orders transaction — not wired this pass"
)]
const ENQUEUE_SQL: &str = "\
INSERT INTO jobs (queue_name, payload, run_after, idempotency_key, max_attempts)
VALUES ($1, $2, COALESCE($3, now()), $4, $5)
ON CONFLICT (idempotency_key) WHERE idempotency_key IS NOT NULL DO NOTHING
RETURNING id";

/// §3.5 (Q-TXN-ENQUEUE) — the ONE hard requirement: `order-persistence.ts:158-173` inserts
/// `order.timeout` + `notify.telegram.send` into the queue INSIDE the `POST /orders` transaction,
/// so a ROLLBACK of the order also rolls back the enqueue. Takes `impl PgExecutor<'_>` (not a
/// bare `&PgPool`) specifically so a caller CAN pass `&mut *txn` (a `sqlx::Transaction` — see
/// `crate::db::with_tenant`'s callback shape) and get that same-transaction atomicity natively;
/// every other producer (§3.5: "fire-and-forget-after-commit... none needs transactional
/// enqueue") passes a bare pool instead and gets ordinary auto-commit semantics. `max_attempts`
/// defaults to `crate::jobs::backoff::DEFAULT_MAX_ATTEMPTS` when the caller doesn't need a
/// per-job override.
///
/// Returns `None` when `idempotency_key` is `Some` and already claimed by a prior enqueue (§3.4:
/// "a duplicate producer-enqueue is a no-op insert") — this is the OTHER half of idempotency from
/// `crate::jobs::dedup` (that module guards the SEND, this one guards the ENQUEUE; both matter,
/// neither substitutes for the other — see `crate::jobs::ddl`'s module doc).
///
/// `#[allow(dead_code)]`: the ONE real transactional-enqueue call site is S5's `POST /orders`
/// transaction (`order-persistence.ts:158-173`'s Rust successor) — not wired this pass, same
/// posture as `crate::jobs::bridge` (see that module's doc for why an order-creation-path edit
/// under time pressure was deliberately left for its own reviewed pass).
#[allow(
    dead_code,
    reason = "the real caller is S5's POST /orders transaction — not wired this pass"
)]
pub async fn enqueue(
    executor: impl PgExecutor<'_>,
    queue_name: &str,
    payload: &serde_json::Value,
    run_after: Option<chrono::DateTime<chrono::Utc>>,
    idempotency_key: Option<&str>,
) -> Result<Option<i64>, sqlx::Error> {
    sqlx::query_scalar(ENQUEUE_SQL)
        .bind(queue_name)
        .bind(payload)
        .bind(run_after)
        .bind(idempotency_key)
        .bind(crate::jobs::backoff::DEFAULT_MAX_ATTEMPTS)
        .fetch_optional(executor)
        .await
}

const REQUEUE_SQL: &str = "UPDATE jobs SET state = 'queued', run_after = now() + make_interval(secs => $2), last_error = $3 WHERE id = $1";
const FAIL_TO_DLQ_SQL: &str = "UPDATE jobs SET state = 'failed', last_error = $2 WHERE id = $1";

/// What a failed handler run does next — Q-BARE-DEFAULTS/Q-DLQ-NOCONSUMER (§3.3): every queue
/// gets backoff+DLQ by default now, not the Node original's 6/30 opt-in. `redacted_error` must
/// already have gone through `crate::jobs::dispatch::redact_error` (or an equivalent) BEFORE it
/// reaches this function — this module does not itself redact, so a PII leak here is a caller
/// bug, not something this function can catch structurally; see that module's no-PII-assert test
/// for the actual guardrail.
pub async fn fail(
    executor: impl PgExecutor<'_>,
    job: &Job,
    redacted_error: &str,
) -> Result<FailOutcome, sqlx::Error> {
    if should_move_to_dlq(job.attempts, job.max_attempts) {
        sqlx::query(FAIL_TO_DLQ_SQL)
            .bind(job.id)
            .bind(redacted_error)
            .execute(executor)
            .await?;
        Ok(FailOutcome::MovedToDlq)
    } else {
        let delay = backoff_for_attempt(job.attempts);
        #[allow(
            clippy::as_conversions,
            reason = "delay is bounded by backoff::CAP (900s) — always fits i64 seconds"
        )]
        let delay_secs = delay.as_secs() as i64;
        sqlx::query(REQUEUE_SQL)
            .bind(job.id)
            .bind(delay_secs)
            .bind(redacted_error)
            .execute(executor)
            .await?;
        Ok(FailOutcome::Requeued { delay })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FailOutcome {
    Requeued { delay: std::time::Duration },
    MovedToDlq,
}

/// Pure decision mirroring what [`fail`] does against the DB — split out so the poison-job /
/// max-attempts boundary is a plain unit test, not something only provable with a live claim loop.
pub fn should_move_to_dlq(attempts: i32, max_attempts: i32) -> bool {
    attempts >= max_attempts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claim_sql_uses_for_update_skip_locked() {
        assert!(CLAIM_SQL.contains("FOR UPDATE SKIP LOCKED"));
    }

    #[test]
    fn enqueue_sql_dedups_on_idempotency_key_when_present() {
        assert!(ENQUEUE_SQL.contains(
            "ON CONFLICT (idempotency_key) WHERE idempotency_key IS NOT NULL DO NOTHING"
        ));
        assert!(ENQUEUE_SQL.contains("RETURNING id"));
    }

    #[test]
    fn enqueue_sql_defaults_run_after_to_now_when_not_given() {
        assert!(ENQUEUE_SQL.contains("COALESCE($3, now())"));
    }

    #[test]
    fn claim_sql_matches_ready_queued_and_reclaimable_active_rows() {
        assert!(CLAIM_SQL.contains("state = 'queued' AND run_after <= now()"));
        assert!(
            CLAIM_SQL.contains("state = 'active' AND locked_until < now()"),
            "visibility-timeout reclaim IS the claim predicate — no separate reaper (module doc)"
        );
    }

    #[test]
    fn claim_sql_increments_attempts_at_claim_time_not_at_failure_time() {
        assert!(CLAIM_SQL.contains("attempts = attempts + 1"));
    }

    #[test]
    fn complete_sql_writes_a_separate_terminal_state_after_the_claim() {
        assert!(COMPLETE_SQL.contains("state = 'completed'"));
    }

    #[test]
    fn poison_job_quarantine_boundary() {
        assert!(should_move_to_dlq(8, 8));
        assert!(should_move_to_dlq(9, 8));
        assert!(!should_move_to_dlq(7, 8));
        assert!(!should_move_to_dlq(
            0,
            crate::jobs::backoff::DEFAULT_MAX_ATTEMPTS
        ));
    }

    #[test]
    fn fail_to_dlq_sql_never_silently_drops_the_error() {
        assert!(FAIL_TO_DLQ_SQL.contains("last_error = $2"));
        assert!(FAIL_TO_DLQ_SQL.contains("state = 'failed'"));
    }

    #[test]
    fn requeue_sql_carries_a_computed_delay_not_a_fixed_retry() {
        assert!(REQUEUE_SQL.contains("run_after = now() + make_interval(secs => $2)"));
        assert!(
            REQUEUE_SQL.contains("state = 'queued'"),
            "a requeued job goes back to 'queued', not 'active' — it must re-enter the claim \
             predicate's fresh-work branch, not the reclaim branch"
        );
    }

    // ── live-Postgres proof (requires the jobs table applied — crate::jobs::ddl; not run in
    // this sandbox) ──

    /// Cutover DoD (proposal §12): "claim under FOR UPDATE SKIP LOCKED (two concurrent runners
    /// never claim the same row)." Two concurrent `claim` calls against the same small batch of
    /// ready rows must partition them, never double-claim.
    #[tokio::test]
    #[ignore = "requires a live Postgres with the jobs table (crate::jobs::ddl) applied — run with --ignored"]
    async fn two_concurrent_claims_never_claim_the_same_row() {
        let database_url = std::env::var("DATABASE_URL_OPERATIONAL")
            .expect("DATABASE_URL_OPERATIONAL must be set for this ignored test");
        let pool = sqlx::PgPool::connect(&database_url)
            .await
            .expect("pool must connect");

        // Seed two ready jobs.
        for _ in 0..2 {
            sqlx::query(
                "INSERT INTO jobs (queue_name, payload) VALUES ('test.skip_locked', '{}'::jsonb)",
            )
            .execute(&pool)
            .await
            .expect("seed insert must succeed");
        }

        let (a, b) = tokio::join!(claim(&pool, 30, 1), claim(&pool, 30, 1));
        let a = a.expect("claim A must succeed");
        let b = b.expect("claim B must succeed");
        let ids: std::collections::HashSet<i64> = a.iter().chain(b.iter()).map(|j| j.id).collect();
        assert_eq!(
            ids.len(),
            a.len() + b.len(),
            "no id may appear in both claim results — FOR UPDATE SKIP LOCKED must partition, not duplicate"
        );
    }
}
