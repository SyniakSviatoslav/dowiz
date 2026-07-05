//! `order.timeout_sweep` — the cross-tenant safety-net floor (§6, §8). Per-minute (`* * * * *`),
//! single-flight via `crate::jobs::advisory_lock::ORDER_TIMEOUT_SWEEP`. Calls the EXISTING
//! DEFINER function `app_sweep_timeout_orders()` (`packages/db/migrations/
//! 1790000000078_phase2-sweep-fns.ts:13-24`, source-verified) — KEEP per REBUILD-MAP §8: S8 owns
//! the timing + single-flight, never the money-math. That function's own
//! `WHERE status='PENDING' AND timeout_at IS NOT NULL AND timeout_at < now()` guard IS the
//! idempotency: a re-run (this cron double-firing, OR the per-order `order.timeout` job ALSO
//! firing for the same row) cancels nothing already cancelled — §6's "idempotent by guard" row.
//! This sweep recovers ANY overdue order regardless of whether its per-order job ran,
//! stack-agnostic (Node or Rust) — the safety net the per-order job is not required to catch.

use crate::jobs::advisory_lock::ORDER_TIMEOUT_SWEEP;
use crate::jobs::cron::try_with_lock;

const SWEEP_SQL: &str = "SELECT * FROM app_sweep_timeout_orders()";

/// Runs one sweep tick under the single-flight lock. `Ok(None)` means another instance already
/// holds the lock this tick (correct, expected — not an error); `Ok(Some(n))` is the number of
/// rows the DEFINER function returned (orders it cancelled this pass).
pub async fn run_once(pool: &sqlx::PgPool) -> Result<Option<i64>, sqlx::Error> {
    try_with_lock(pool, ORDER_TIMEOUT_SWEEP, || async {
        let rows = sqlx::query(SWEEP_SQL).fetch_all(pool).await?;
        #[allow(
            clippy::as_conversions,
            reason = "a per-minute sweep's row count is bounded by real order volume (proposal \
                      §2: low-hundreds/day), never near i64/usize truncation risk"
        )]
        Ok(rows.len() as i64)
    })
    .await
}

/// Spawns the per-minute loop — `main.rs` wiring (dark-mounted alongside every other S8 cron,
/// gated the same way the S2+ surfaces are: only when the auth/DB env is fully configured).
pub fn spawn(pool: sqlx::PgPool) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Err(err) = run_once(&pool).await {
                tracing::error!(%err, cron = "order.timeout_sweep", "sweep tick failed");
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sweep_sql_calls_the_existing_definer_function_verbatim() {
        assert_eq!(SWEEP_SQL, "SELECT * FROM app_sweep_timeout_orders()");
    }

    #[test]
    fn sweep_sql_does_not_reimplement_the_status_guard_itself() {
        // The whole point of REBUILD-MAP §8's KEEP disposition: this cron must not contain its
        // own `WHERE status='PENDING'` — that guard lives ONLY inside the DEFINER function. A
        // future edit that inlines the cancellation logic here (instead of calling the function)
        // would duplicate a money-adjacent state transition outside its single source of truth.
        assert!(!SWEEP_SQL.to_uppercase().contains("UPDATE"));
        assert!(!SWEEP_SQL.to_uppercase().contains("PENDING"));
    }

    #[test]
    fn uses_its_own_named_advisory_lock_id_not_the_old_collision() {
        assert_ne!(
            ORDER_TIMEOUT_SWEEP, 5,
            "the exact Node collision this registry retires"
        );
    }
}
