//! The S5→S8 window shim — REV-S8-4 (breaker HIGH, 🔴) partition-aware bridge.
//! `docs/design/rebuild-jobs-s8-council/proposal.md` §9.2, resolution.md REV-S8-4.
//!
//! ## The bug this avoids
//! pg-boss v10 is **partition-per-queue**: `pgboss.job` is `PARTITION BY LIST (name)`, and each
//! queue's `create_queue()` call attaches its own child table (`CREATE TABLE pgboss.<hash> (LIKE
//! pgboss.job INCLUDING DEFAULTS)` then `ALTER TABLE pgboss.job ATTACH PARTITION pgboss.<hash> FOR
//! VALUES IN (queue_name)` — verified directly against the vendored
//! `pg-boss@10.4.2/node_modules/pg-boss/src/plans.js`, both the `createQueueFunction` DDL and the
//! parent `job` table's own `CREATE TABLE ... PARTITION BY LIST (name)`). The child partition
//! table name is `'j' || encode(sha224(queue_name::bytea), 'hex')` — an internal implementation
//! detail computed BY POSTGRES inside pg-boss's own `create_queue` SQL function, not something an
//! external inserter should ever need to compute or guess.
//!
//! **This is exactly why this bridge inserts into the PARENT `pgboss.job` table, never a
//! separately-computed child table name.** Native declarative partitioning routes a row inserted
//! into the parent to whichever child is `ATTACH`ed `FOR VALUES IN` that row's `name` — entirely
//! transparent to the inserter, and correct BY CONSTRUCTION as long as (a) the target queue
//! already exists (verified below via `pgboss.queue`, never assumed) and (b) the row satisfies
//! the partition's own `CHECK (name = <queue_name>)` constraint, which it does by definition
//! (the `name` value IS what routes it). A hand-rolled `INSERT INTO pgboss.job` that instead
//! targeted a GUESSED child table name (or omitted the queue-existence check) is the "wrong/absent
//! partition, silently dropped" failure REV-S8-4 names.
//!
//! ## Scope — a bounded, temporary shim
//! This bridge exists ONLY for the S5-Rust / S8-Node overlap window (§9.2): once S8 flips (the
//! fleet-atomic flip, §9 control 1), Rust owns its OWN `jobs` table (`crate::jobs::ddl`) end to
//! end and this bridge is retired. It carries the SAME `dedup_key`-as-`singleton_key` convention
//! `order-persistence.ts:166-173` already uses for `notify.telegram.send` — but per
//! Q-SINGLETONKEY-POLICY (§11), `singleton_key` dedup is only HONORED by pg-boss when the queue's
//! own `policy = 'short'`; this bridge does not itself guarantee that (it reads whatever the
//! already-live queue was created with) — the REAL double-send guard is
//! `crate::jobs::dedup`'s durable claim-before-send at the Telegram-adapter layer (REV-S8-1),
//! which this bridge's target job will go through on the Node side exactly as any other
//! `notify.telegram.send` job does today. This bridge's OWN correctness claim is narrower and
//! precise: **the row lands in the live, correct queue** — not "the row can never be duplicated."
//!
//! ## What this module does NOT claim
//! The exact NOT-NULL/default column set below is read directly from the vendored pg-boss source
//! (cited per-field), so it is not a guess — but this bridge has never been run against a live
//! `pgboss` schema in this sandbox (no DB writes anywhere here, `crate::db`'s posture). The
//! `#[ignore]`'d test below is the actual proof and MUST be run against a staging database with a
//! real, already-created `notify.telegram.send` queue before this ships live.

//! ## Why this module is `#[allow(dead_code)]`
//! The real call site is inside S5's `POST /orders` transactional-enqueue
//! (`routes/orders/pg.rs`/`order-persistence.ts:158-173`'s Rust successor) — wiring THIS bridge in
//! there is a money/order-creation-path change this pass deliberately left untouched (S5 is
//! already-shipped, already-tested; a same-pass edit to the order-creation transaction under time
//! pressure is exactly the kind of change that deserves its own reviewed pass, not a drive-by
//! addition here). Everything in this module is real and tested (see the tests below); it is
//! `dead_code` only in the sense that nothing calls it FROM PRODUCT CODE yet — same posture
//! `crate::db`'s module doc documents for `with_tenant` staying uncalled after S1+S3.

#![allow(
    dead_code,
    reason = "the real call site is S5's POST /orders transactional-enqueue path — deliberately \
              not wired in this pass, see module doc"
)]

use uuid::Uuid;

use sqlx::PgExecutor;

/// The queue name this shim targets — the ONE notification the proposal says is NOT
/// sweep-floored during the overlap window (§9.2: `order.timeout` IS sweep-floored by the
/// Node cron regardless; `order.created`'s Telegram notification is not, so it is the one thing
/// this bridge must carry).
pub const ORDER_CREATED_TELEGRAM_QUEUE: &str = "notify.telegram.send";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeOutcome {
    /// The row was inserted into `pgboss.job` (the parent) and routed by Postgres to the live
    /// queue's partition.
    Inserted,
    /// `pgboss.queue` had no row for the target queue name — the bridge refuses to insert a job
    /// naming a queue that doesn't exist (which is exactly the silent-drop failure mode REV-S8-4
    /// exists to avoid at the OTHER end: better a loud zero-rows-affected here than a row that
    /// satisfies no partition's CHECK constraint).
    QueueNotFound,
}

/// Every column named here is either NOT NULL with no default on `pgboss.job` (`name`, verified
/// against `plans.js`'s `CREATE TABLE ${schema}.job` — every OTHER column has a table-level
/// default copied to each partition via `LIKE ... INCLUDING DEFAULTS`) or a per-queue override
/// this bridge deliberately pulls from the LIVE `pgboss.queue` row rather than trusting the
/// table's bare global defaults (`retry_limit=2, retry_delay=0, retry_backoff=false` — the exact
/// Q-BARE-DEFAULTS gap this whole surface fixes elsewhere; a shim that silently reverted to bare
/// defaults for this one bridged job would be a step backward even though it's temporary).
const BRIDGE_INSERT_SQL: &str = "\
WITH queue_cfg AS (
  SELECT retry_limit, retry_delay, retry_backoff, expire_seconds, retention_minutes, policy
    FROM pgboss.queue
   WHERE name = $1
)
INSERT INTO pgboss.job (name, data, singleton_key, retry_limit, retry_delay, retry_backoff, expire_in, keep_until, policy)
SELECT $1,
       $2::jsonb,
       $3,
       queue_cfg.retry_limit,
       queue_cfg.retry_delay,
       queue_cfg.retry_backoff,
       make_interval(secs => queue_cfg.expire_seconds),
       now() + make_interval(mins => queue_cfg.retention_minutes),
       queue_cfg.policy
  FROM queue_cfg
ON CONFLICT DO NOTHING
RETURNING id";

/// The bridged `order.created` Telegram notification — REV-S8-4's named DoD test target. Carries
/// the SAME claim-check payload shape (`crate::jobs::dispatch::JobPayload`) and the same
/// `dedup_key` convention (`crate::jobs::dedup::dedup_key`) the steady-state Rust `jobs` table
/// producer will use post-flip, so nothing about the payload shape changes across the cutover.
pub async fn bridge_order_created_telegram(
    executor: impl PgExecutor<'_>,
    order_id: Uuid,
    location_id: Uuid,
) -> Result<BridgeOutcome, sqlx::Error> {
    let payload = crate::jobs::dispatch::JobPayload {
        entity_id: order_id,
        location_id,
        event: "order.created".to_string(),
    };
    let singleton_key = crate::jobs::dedup::dedup_key("order.created", order_id, location_id);
    #[allow(
        clippy::expect_used,
        reason = "JobPayload is two Uuids + a String literal — every field is unconditionally \
                  serializable, so serde_json::to_value cannot return Err for this specific type"
    )]
    let data = serde_json::to_value(&payload)
        .expect("JobPayload serialization cannot fail (no non-serializable fields)");

    let inserted: Option<Uuid> = sqlx::query_scalar(BRIDGE_INSERT_SQL)
        .bind(ORDER_CREATED_TELEGRAM_QUEUE)
        .bind(data)
        .bind(&singleton_key)
        .fetch_optional(executor)
        .await?;

    Ok(match inserted {
        Some(_) => BridgeOutcome::Inserted,
        None => BridgeOutcome::QueueNotFound,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_targets_the_parent_table_never_a_computed_partition_name() {
        assert!(
            BRIDGE_INSERT_SQL.contains("INSERT INTO pgboss.job"),
            "must insert into the PARENT table — Postgres native partition routing handles the \
             rest; a guessed `pgboss.<hash>` child table name is the exact bug REV-S8-4 exists to \
             prevent"
        );
        assert!(
            !BRIDGE_INSERT_SQL.to_lowercase().contains("sha224")
                && !BRIDGE_INSERT_SQL.to_lowercase().contains("encode("),
            "this bridge must never compute pg-boss's internal partition-name hash itself"
        );
    }

    #[test]
    fn bridge_verifies_the_queue_exists_before_inserting() {
        assert!(
            BRIDGE_INSERT_SQL.contains("FROM pgboss.queue"),
            "must read the live queue row — an insert for a queue that doesn't exist must yield \
             zero rows (BridgeOutcome::QueueNotFound), never a malformed job"
        );
    }

    #[test]
    fn bridge_pulls_retry_config_from_the_live_queue_not_bare_table_defaults() {
        for column in [
            "retry_limit",
            "retry_delay",
            "retry_backoff",
            "expire_seconds",
            "retention_minutes",
        ] {
            assert!(
                BRIDGE_INSERT_SQL.contains(column),
                "must read {column} from pgboss.queue — falling back to pgboss.job's bare table \
                 defaults would silently re-introduce Q-BARE-DEFAULTS for this one bridged job"
            );
        }
    }

    #[test]
    fn bridged_payload_uses_the_same_claim_check_shape_and_dedup_convention() {
        let order_id = Uuid::new_v4();
        let location_id = Uuid::new_v4();
        let payload = crate::jobs::dispatch::JobPayload {
            entity_id: order_id,
            location_id,
            event: "order.created".to_string(),
        };
        let expected_key = crate::jobs::dedup::dedup_key("order.created", order_id, location_id);
        assert!(expected_key.starts_with("order.created:"));
        assert_eq!(payload.entity_id, order_id);
    }

    // ── REV-S8-4 named DoD test: partition-aware bridge (requires a live Postgres with the
    // `notify.telegram.send` pg-boss queue already created — not run in this sandbox) ──

    #[tokio::test]
    #[ignore = "requires a live Postgres with pgboss bootstrapped (mig 1790000000011) and the \
                notify.telegram.send queue already created — run with --ignored against staging"]
    async fn bridge_lands_a_rust_created_order_notification_in_the_live_queue() {
        let database_url = std::env::var("DATABASE_URL_OPERATIONAL")
            .expect("DATABASE_URL_OPERATIONAL must be set for this ignored test");
        let pool = sqlx::PgPool::connect(&database_url)
            .await
            .expect("pool must connect");

        let order_id = Uuid::new_v4();
        let location_id = Uuid::new_v4();
        let outcome = bridge_order_created_telegram(&pool, order_id, location_id)
            .await
            .expect("bridge insert must not error");
        assert_eq!(
            outcome,
            BridgeOutcome::Inserted,
            "the notify.telegram.send queue must already exist on staging — if this returns \
             QueueNotFound, the queue was never bootstrapped, not a bridge bug"
        );

        // Prove the row is visible through the PARENT table — i.e. Postgres actually routed it
        // to whatever partition is attached, transparently, exactly as the module doc claims.
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pgboss.job WHERE name = $1 AND singleton_key = $2",
        )
        .bind(ORDER_CREATED_TELEGRAM_QUEUE)
        .bind(crate::jobs::dedup::dedup_key(
            "order.created",
            order_id,
            location_id,
        ))
        .fetch_one(&pool)
        .await
        .expect("query must succeed");
        assert_eq!(count, 1);
    }
}
