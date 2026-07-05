//! REV-S8-1 (🔴 CRIT, breaker + counsel #3) — durable Postgres claim-before-send, replacing the
//! Node worker's Potemkin dedup.
//!
//! ## The bug this replaces
//! `notifications/workers/index.ts` dedups sends with a **process-local `HashSet<String>`**
//! (`dedupCache`, capped at 1000 entries with LRU eviction) — verified at the exact cited lines:
//! a `dedupCache.has(dedupKey)` check before send, `dedupCache.add(dedupKey)` after. This resets
//! on every restart/crash/deploy. The table the design DOCUMENTED as the durable backstop,
//! `notification_outbox_audit`, has **no unique constraint on anything** (confirmed by reading
//! migration `1790000000007_notification-outbox-audit.ts` directly — its `ON CONFLICT DO NOTHING`
//! writes are pure inserts with no arbiter, i.e. no-ops as dedup). So on an at-least-once
//! crash-AFTER-send (the hand-rolled runner's claim loop re-runs a job whose handler already
//! called the Telegram API but crashed before the row was marked `completed` —
//! `crate::jobs::runner`'s module doc), the owner's `order.created` Telegram message is sent a
//! second time. This is the exact "S8-twin of the S5 Potemkin promo" the council resolution names
//! (`docs/design/rebuild-jobs-s8-council/resolution.md` REV-S8-1).
//!
//! ## The fix — claim-before-send, not claim-before-enqueue
//! `jobs.idempotency_key` (see `crate::jobs::ddl`) already dedups a duplicate ENQUEUE — but a
//! crash-after-send retries the SAME job row, so an enqueue-time guard cannot help. The guard
//! must sit at the EFFECT, immediately before the external call:
//!
//! 1. `INSERT INTO notification_dedup (dedup_key, job_id) VALUES ($1, $2)
//!     ON CONFLICT (dedup_key) DO NOTHING RETURNING dedup_key` — committed on its own, BEFORE the
//!    handler calls Telegram/push/email.
//! 2. `0` rows returned ⇒ some prior attempt (possibly THIS job, retried after a crash) already
//!    claimed this dedup key ⇒ **skip the external call**, proceed straight to marking the job
//!    `completed` (the notification was already sent, or is being sent by a still-running
//!    attempt — either way, sending again is the bug this exists to prevent).
//! 3. `1` row returned ⇒ this attempt is the first to claim it ⇒ proceed to the external call.
//!
//! This is the proposal's own "gold standard" pattern (`access-request.notify`'s claim-before-send
//! CAS on the `access_requests` row, §3.4) generalized into a dedicated table so every notification
//! handler gets it, not just that one call site.
//!
//! ## Why the decision logic is a pure function
//! [`decide`] takes an already-resolved [`ClaimOutcome`] (what the `INSERT ... RETURNING` did) and
//! returns a [`SendDecision`] — no `sqlx`/`tokio` in this function at all, so the crash-recovery
//! scenario is a deterministic unit test (see `tests::crash_recovery_rerun_sends_at_most_once`)
//! rather than something only provable against a live database. The actual `INSERT` lives in
//! [`claim`], exercised only by the `#[ignore]` live-Postgres test (same posture as every other
//! DB-touching test in this crate — `crate::db`'s module doc explains why).

use sqlx::PgExecutor;

/// What the `INSERT ... ON CONFLICT DO NOTHING RETURNING` reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimOutcome {
    /// This call was the first to claim `dedup_key` — a row was inserted.
    Claimed,
    /// `dedup_key` was already claimed by a prior attempt — no row was inserted.
    AlreadyClaimed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendDecision {
    Send,
    Skip,
}

/// The whole crash-recovery guarantee in one pure match: `Claimed` sends, `AlreadyClaimed` never
/// does, no matter how many times this is called for the same key.
pub fn decide(outcome: ClaimOutcome) -> SendDecision {
    match outcome {
        ClaimOutcome::Claimed => SendDecision::Send,
        ClaimOutcome::AlreadyClaimed => SendDecision::Skip,
    }
}

/// The dedup key format — carries the exact convention `order-persistence.ts:166-173` already
/// uses for `notify.telegram.send` (`` `order.created:${order.id}:${locationId}` ``), generalized
/// to any `(event, entity_id, location_id)` triple so every S8 notification handler can use the
/// same claim-before-send guard, not just `order.created`.
pub fn dedup_key(event: &str, entity_id: uuid::Uuid, location_id: uuid::Uuid) -> String {
    format!("{event}:{entity_id}:{location_id}")
}

const CLAIM_SQL: &str = "INSERT INTO notification_dedup (dedup_key, job_id) VALUES ($1, $2) \
     ON CONFLICT (dedup_key) DO NOTHING RETURNING dedup_key";

/// Executes the real claim against Postgres. Requires a live database — exercised only by the
/// `#[ignore]` test below (no DB writes anywhere in this sandbox, per `crate::db`'s posture).
/// Takes any `PgExecutor` (a bare connection, a pool, or a transaction) so a caller can run this
/// either standalone (its own tiny auto-committing statement, the intended usage per the module
/// doc) or inside a larger transaction if a future caller needs that.
pub async fn claim(
    executor: impl PgExecutor<'_>,
    dedup_key: &str,
    job_id: i64,
) -> Result<ClaimOutcome, sqlx::Error> {
    let claimed: Option<String> = sqlx::query_scalar(CLAIM_SQL)
        .bind(dedup_key)
        .bind(job_id)
        .fetch_optional(executor)
        .await?;
    Ok(match claimed {
        Some(_) => ClaimOutcome::Claimed,
        None => ClaimOutcome::AlreadyClaimed,
    })
}

const RELEASE_SQL: &str = "DELETE FROM notification_dedup WHERE dedup_key = $1";

/// Releases (DELETEs) a previously-[`claim`]ed dedup row — the REV-S8-1 correctness completion the
/// guardian gate flagged as load-bearing. Because the claim is taken BEFORE the external send, a
/// send that then fails **transiently** (RateLimited / TimedOut / NetworkError — the message did
/// NOT go out) MUST release the claim, or the requeued retry sees `AlreadyClaimed` -> `Skip` ->
/// `complete` and the opted-in customer's push is silently lost forever (the exact silent-loss the
/// guardian caught). The **release-before-requeue** ordering is load-bearing and is the caller's
/// responsibility (`crate::jobs::worker::handle_customer_status`): release here FIRST, THEN let the
/// runner requeue the job — a crash in between is safe (the reclaimed job re-claims a now-absent
/// key and re-sends; at-least-once holds). A send that SUCCEEDED, or is permanently rejected, must
/// NOT release (the claim staying is exactly what makes a crash-after-send retry skip — at-most-once
/// holds). Idempotent: deleting an absent key is a no-op 0-row DELETE, never an error.
pub async fn release(executor: impl PgExecutor<'_>, dedup_key: &str) -> Result<(), sqlx::Error> {
    sqlx::query(RELEASE_SQL)
        .bind(dedup_key)
        .execute(executor)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claim_sql_targets_the_durable_table_with_a_real_arbiter() {
        assert!(CLAIM_SQL.contains("notification_dedup"));
        assert!(
            CLAIM_SQL.contains("ON CONFLICT (dedup_key) DO NOTHING"),
            "the arbiter must be the dedup_key column specifically — a bare `ON CONFLICT DO \
             NOTHING` with no column list (notification_outbox_audit's exact bug) silently \
             requires no unique constraint to exist and therefore never actually conflicts"
        );
        assert!(CLAIM_SQL.contains("RETURNING dedup_key"));
    }

    #[test]
    fn dedup_key_matches_the_order_created_convention_verbatim() {
        let order_id = uuid::Uuid::nil();
        let location_id = uuid::Uuid::nil();
        assert_eq!(
            dedup_key("order.created", order_id, location_id),
            format!("order.created:{order_id}:{location_id}"),
        );
    }

    #[test]
    fn release_sql_deletes_the_claim_row_keyed_on_dedup_key() {
        assert!(RELEASE_SQL.contains("DELETE FROM notification_dedup"));
        assert!(
            RELEASE_SQL.contains("WHERE dedup_key = $1"),
            "release must be keyed on the exact dedup_key claimed — never an unqualified DELETE"
        );
    }

    /// REV-S8-1's named DoD test: **crash-recovery re-run → single send.** Simulates the exact
    /// at-least-once scenario — job claimed, handler sends, crash before `completed`, job
    /// reclaimed and the handler runs again. The pure `decide` function proves the SECOND run
    /// never sends, independent of any database (the actual persistence of "was it claimed" is
    /// exercised by `claim_actually_persists_across_a_simulated_crash_and_retry` below, which
    /// needs a live Postgres).
    #[test]
    fn crash_recovery_rerun_sends_at_most_once() {
        // Attempt 1: first claim of this dedup key.
        let first_attempt = ClaimOutcome::Claimed;
        assert_eq!(decide(first_attempt), SendDecision::Send);

        // The worker crashes AFTER calling Telegram but BEFORE marking the job `completed`.
        // The job's `locked_until` lapses; the SAME row is reclaimed and re-run. Because the
        // claim row from attempt 1 is already committed, the retry's INSERT conflicts.
        let retry_attempt = ClaimOutcome::AlreadyClaimed;
        assert_eq!(
            decide(retry_attempt),
            SendDecision::Skip,
            "a crash-after-send retry must never re-send — this is the exact double-send REV-S8-1 exists to close"
        );

        // A THIRD reclaim (e.g. the retry itself also crashed before completing) must still skip
        // — the guard is durable across arbitrarily many re-runs, not just one retry.
        assert_eq!(decide(ClaimOutcome::AlreadyClaimed), SendDecision::Skip);
    }

    #[test]
    fn customer_push_does_not_need_this_guard_documented_here_not_silently_assumed() {
        // Proposal §4.1 / resolution.md REV-S8-1: "Customer push is safe — device coalesces by
        // tag:order-<id>." This module intentionally does not special-case that channel; the
        // claim-before-send guard is still SAFE (over-cautious, not wrong) to apply there too —
        // this test exists only so that fact is written down as an explicit assertion of intent,
        // not lost as an unstated assumption if a future reader wonders why customer push isn't
        // exempted from `claim`/`decide`.
        let customer_push_dedup_key =
            dedup_key("order.status", uuid::Uuid::nil(), uuid::Uuid::nil());
        assert!(customer_push_dedup_key.starts_with("order.status:"));
    }

    // ── live-Postgres proof (requires DATABASE_URL_OPERATIONAL; not run in this sandbox — see
    // crate::db's identical posture) ──

    #[tokio::test]
    #[ignore = "requires a live Postgres with the notification_dedup table (crate::jobs::ddl) applied — run with --ignored"]
    async fn claim_actually_persists_across_a_simulated_crash_and_retry() {
        let database_url = std::env::var("DATABASE_URL_OPERATIONAL")
            .expect("DATABASE_URL_OPERATIONAL must be set for this ignored test");
        let pool = sqlx::PgPool::connect(&database_url)
            .await
            .expect("pool must connect");

        let key = format!("test.crash-recovery:{}", uuid::Uuid::new_v4());

        let first = claim(&pool, &key, 1)
            .await
            .expect("first claim must succeed");
        assert_eq!(first, ClaimOutcome::Claimed);

        // Simulates the crash-then-retry: a FRESH call against the SAME key, exactly what the
        // reclaimed job's second attempt does. No in-process state carries over (a new `claim`
        // call, same as a new process would make) — only the committed row does.
        let second = claim(&pool, &key, 1).await.expect(
            "second claim must succeed (not error) — it must observe the conflict, not fail",
        );
        assert_eq!(
            second,
            ClaimOutcome::AlreadyClaimed,
            "the durable table must reject a second claim of the same key across separate calls"
        );
    }

    /// The guardian gate's load-bearing DoD (live half): a **transient** send-failure releases
    /// the claim so the requeued retry can actually re-claim and re-send. Proves the DB round-trip
    /// of `release` — the pure decision half (`claim_action_for` / at-most-once-across-the-sequence)
    /// lives in `crate::jobs::worker`'s tests and runs in the sandbox. Sequence:
    /// claim -> (transient fail) -> release -> re-claim MUST succeed (Claimed, not AlreadyClaimed).
    #[tokio::test]
    #[ignore = "requires a live Postgres with the notification_dedup table (crate::jobs::ddl) applied — run with --ignored"]
    async fn a_transient_failure_release_lets_the_retry_re_claim_and_resend() {
        let database_url = std::env::var("DATABASE_URL_OPERATIONAL")
            .expect("DATABASE_URL_OPERATIONAL must be set for this ignored test");
        let pool = sqlx::PgPool::connect(&database_url)
            .await
            .expect("pool must connect");

        let key = format!("test.transient-release:{}", uuid::Uuid::new_v4());

        // Attempt 1: claim, then the send fails TRANSIENTLY (RateLimited/TimedOut/NetworkError) —
        // the notification did NOT go out.
        assert_eq!(
            claim(&pool, &key, 1)
                .await
                .expect("first claim must succeed"),
            ClaimOutcome::Claimed
        );
        // The worker releases the claim BEFORE requeuing (the load-bearing ordering).
        release(&pool, &key).await.expect("release must succeed");

        // Attempt 2 (the requeued retry): must re-CLAIM (not AlreadyClaimed) so it actually
        // re-sends — this is exactly the silent-loss the guardian caught being fixed.
        assert_eq!(
            claim(&pool, &key, 1)
                .await
                .expect("re-claim after release must succeed"),
            ClaimOutcome::Claimed,
            "after a transient-failure release, the retry MUST be able to re-claim and re-send"
        );

        // Attempt 2 succeeds — the claim now STAYS (no release). A crash-after-send re-run must
        // still skip: at-most-once holds across the whole sequence.
        assert_eq!(
            claim(&pool, &key, 1)
                .await
                .expect("post-success re-claim must not error"),
            ClaimOutcome::AlreadyClaimed,
            "once the send succeeds and the claim is kept, any further reclaim must skip"
        );

        // Cleanup.
        release(&pool, &key)
            .await
            .expect("cleanup release must succeed");
    }
}
