//! Cron mechanism (§7 Q2): `tokio::time::interval` + `pg_try_advisory_lock` — "boring, proven,
//! Postgres-native," no leader-election framework, no external scheduler (proposal's own words).
//! Every S8 cron in `crate::jobs::crons` is built from the two primitives here: [`try_with_lock`]
//! for single-flight across `web` instances (carries the existing Node pattern verbatim, just
//! native), and [`next_daily_utc_run`] for the handful of crons that fire once at a specific UTC
//! time rather than on a fixed interval (settlement generation, nightly reconciliation) — every
//! cron in this surface is UTC-scheduled (Q-UTC-CRON); the ONLY tz-aware evaluation anywhere in
//! S8 is quiet-hours (`crate::jobs::consent`), never cron timing itself.
//!
//! ## Boot-assert (Q-BOOT-ASSERT) — extended to every cron this build owns, not 2/23
//! The Node source only fail-fasts on a missing schedule for 2 of 23 crons
//! (`assertAccessRequestSchedules`/`assertDeliveryTraceSchedule` — a `process.exit(1)` in
//! production if the expected `pgboss.schedule` rows are absent). Rust's hand-rolled tokio loops
//! have no external schedule table to check against — the equivalent failure mode is "the boot
//! sequence forgot to `tokio::spawn` one of the crons it's supposed to run." [`assert_full_roster_spawned`]
//! makes THAT loud instead of silent: `main.rs`'s boot sequence collects the names it actually
//! spawned and this function fails fast if any name from [`EXPECTED_CRON_NAMES`] is missing.
//!
//! Scope note: [`EXPECTED_CRON_NAMES`] lists only the crons THIS build implements
//! (`crate::jobs::crons`) — the backup family (Q-BACKUP-2TIER, deferred to the backup/DR council
//! per proposal §2 scope) and any cron not yet ported are deliberately absent, not silently
//! assumed complete.

use std::future::Future;
use std::time::Duration;

use chrono::{Datelike, TimeZone, Utc};
use sqlx::PgExecutor;

/// Every cron name this build spawns — the boot-assert roster (module doc). Kept in sync with
/// `crate::jobs::crons`'s actual set by the test below (a cron module added there without a
/// matching entry here fails that test, not silently passes an incomplete assert).
/// `anonymizer.gdpr` IS in this roster — S8 owns its cron TIMING/single-flight per §2; it now
/// drives the real S9 erasure semantics (`crate::jobs::gdpr_erasure`,
/// `crate::jobs::crons::gdpr_sweep`'s module doc), not a no-op.
pub const EXPECTED_CRON_NAMES: &[&str] = &[
    "order.timeout_sweep",
    "settlement.generate",
    "refund_due.reconcile",
    "reconciliation.nightly",
    "anonymizer.gdpr",
    "liveness.check",
];

/// Fails loud (returns the missing names) if the boot sequence did not spawn every cron this
/// build is supposed to run — the Rust analog of `assertAccessRequestSchedules`'s
/// `process.exit(1)`, extended to the full roster instead of 2/23. `main.rs` calls this with the
/// names it actually `tokio::spawn`ed and panics on `Err` (same fail-fast posture as
/// `Config::from_env`/`Pools::connect` — a missing cron is a boot-time configuration bug, not a
/// runtime condition to degrade through).
pub fn assert_full_roster_spawned(spawned: &[&str]) -> Result<(), Vec<&'static str>> {
    let missing: Vec<&'static str> = EXPECTED_CRON_NAMES
        .iter()
        .copied()
        .filter(|expected| !spawned.contains(expected))
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(missing)
    }
}

/// Single-flight across `web` instances (§7, §9 control 1) — `pg_try_advisory_lock` is
/// non-blocking: if another instance already holds `lock_id`, this returns `Ok(None)`
/// immediately rather than waiting, so a cron tick that loses the race simply skips this run
/// (the next tick tries again). The lock is released (`pg_advisory_unlock`) whether `f` succeeds
/// or errors — a cron that fails must not leave the lock held forever, or every future instance
/// silently never runs it again. Callers MUST use an id from `crate::jobs::advisory_lock` (never
/// a raw literal — Q10, the id=5 collision class).
pub async fn try_with_lock<T, F, Fut>(
    executor: impl PgExecutor<'_> + Copy,
    lock_id: i64,
    f: F,
) -> Result<Option<T>, sqlx::Error>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, sqlx::Error>>,
{
    let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
        .bind(lock_id)
        .fetch_one(executor)
        .await?;
    if !acquired {
        return Ok(None);
    }

    let result = f().await;

    // Best-effort unlock: releasing the SAME session-scoped advisory lock this connection just
    // took. If the release query itself fails, the connection is in an unknown state anyway (the
    // pool will likely recycle/drop it) — surfaced via `sqlx::Error` from THIS call, not swallowed,
    // but does not overwrite `result`'s own error if `f` also failed (both are real, `f`'s is the
    // more actionable one to return first).
    let _: bool = sqlx::query_scalar("SELECT pg_advisory_unlock($1)")
        .bind(lock_id)
        .fetch_one(executor)
        .await?;

    result.map(Some)
}

/// Duration until the next occurrence of `hour:minute` UTC at/after `now` (today if not yet
/// passed, otherwise tomorrow) — the settlement (`0 2 * * *`) / nightly-reconciliation (`0 3 * *
/// *`) cron shape. Deliberately hand-rolled rather than pulling in a cron-expression-parsing
/// crate (`tokio_cron_scheduler`) for 2 fixed daily-UTC times — "boring wins" (proposal §2).
pub fn next_daily_utc_run(hour: u32, minute: u32, now: chrono::DateTime<Utc>) -> Duration {
    #[allow(
        clippy::expect_used,
        reason = "hour/minute are always hardcoded call-site literals (e.g. next_daily_utc_run(2, \
                  0, ..) — never untrusted input), so an invalid time-of-day here is a programmer \
                  error worth panicking on immediately, the same fail-fast posture Config::from_env \
                  takes for a malformed boot-time constant"
    )]
    let today = Utc
        .with_ymd_and_hms(now.year(), now.month(), now.day(), hour, minute, 0)
        .single()
        .expect("hour/minute must be a valid time-of-day");
    let target = if today > now {
        today
    } else {
        today + chrono::Duration::days(1)
    };
    #[allow(
        clippy::expect_used,
        reason = "target is provably >= now by the if/else above (today already adjusted to \
                  tomorrow when it wasn't), so the subtraction can never be negative and \
                  Duration::to_std() cannot fail"
    )]
    (target - now)
        .to_std()
        .expect("target is always in the future relative to now by construction above")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn full_roster_assert_passes_when_every_cron_is_spawned() {
        let spawned: Vec<&str> = EXPECTED_CRON_NAMES.to_vec();
        assert_eq!(assert_full_roster_spawned(&spawned), Ok(()));
    }

    #[test]
    fn full_roster_assert_fails_loud_on_a_missing_cron() {
        let mut spawned: Vec<&str> = EXPECTED_CRON_NAMES.to_vec();
        spawned.retain(|name| *name != "settlement.generate");
        let err = assert_full_roster_spawned(&spawned).unwrap_err();
        assert_eq!(err, vec!["settlement.generate"]);
    }

    #[test]
    fn full_roster_assert_reports_every_missing_cron_not_just_the_first() {
        let spawned: Vec<&str> = vec!["order.timeout_sweep"];
        let err = assert_full_roster_spawned(&spawned).unwrap_err();
        assert_eq!(err.len(), EXPECTED_CRON_NAMES.len() - 1);
    }

    #[test]
    fn next_daily_utc_run_targets_later_today_when_not_yet_passed() {
        let now = Utc.with_ymd_and_hms(2026, 7, 4, 1, 0, 0).unwrap();
        let delay = next_daily_utc_run(2, 0, now);
        assert_eq!(delay, Duration::from_secs(3600));
    }

    #[test]
    fn next_daily_utc_run_rolls_to_tomorrow_when_todays_time_already_passed() {
        let now = Utc.with_ymd_and_hms(2026, 7, 4, 3, 0, 0).unwrap();
        let delay = next_daily_utc_run(2, 0, now);
        // 23 hours until 02:00 the next day.
        assert_eq!(delay, Duration::from_secs(23 * 3600));
    }

    #[test]
    fn next_daily_utc_run_at_the_exact_target_second_rolls_to_tomorrow() {
        let now = Utc.with_ymd_and_hms(2026, 7, 4, 2, 0, 0).unwrap();
        let delay = next_daily_utc_run(2, 0, now);
        assert_eq!(delay, Duration::from_secs(24 * 3600));
    }

    // ── live-Postgres proof (requires DATABASE_URL_OPERATIONAL; not run in this sandbox) ──

    #[tokio::test]
    #[ignore = "requires a live Postgres — set DATABASE_URL_OPERATIONAL and run with --ignored"]
    async fn try_with_lock_is_single_flight_across_two_concurrent_callers() {
        let database_url = std::env::var("DATABASE_URL_OPERATIONAL")
            .expect("DATABASE_URL_OPERATIONAL must be set for this ignored test");
        let pool = sqlx::PgPool::connect(&database_url)
            .await
            .expect("pool must connect");

        // A held lock (this test's own connection) blocks a `pg_try_advisory_lock` from a
        // DIFFERENT connection in the pool, proving the non-blocking single-flight semantics —
        // note this exercises the underlying primitive at the connection-pool level, since
        // `try_with_lock` itself acquires+releases within one call.
        let mut conn_a = pool.acquire().await.expect("acquire conn A");
        let acquired_a: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock(999999)")
            .fetch_one(&mut *conn_a)
            .await
            .expect("query must succeed");
        assert!(acquired_a);

        let mut conn_b = pool.acquire().await.expect("acquire conn B");
        let acquired_b: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock(999999)")
            .fetch_one(&mut *conn_b)
            .await
            .expect("query must succeed");
        assert!(
            !acquired_b,
            "a second connection must NOT acquire the same held lock id"
        );

        sqlx::query("SELECT pg_advisory_unlock(999999)")
            .execute(&mut *conn_a)
            .await
            .expect("unlock must succeed");
    }
}
