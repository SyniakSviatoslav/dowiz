//! S9 GDPR/compliance — the erasure ENGINE semantics (`docs/design/rebuild-gdpr-s9-council/`).
//! Ports `apps/api/src/lib/anonymizer/index.ts` (`AnonymizerService`) +
//! `apps/api/src/workers/anonymizer-gdpr.ts` (`GdprErasureWorker`), INCLUDING the completeness fix
//! landed on Node in commit `58caf4f4` (REV-S9-1/3/5 — fan the customer-erasure out to the
//! subject's orders/ratings/phone, gate completion on the WHOLE subject-graph). `crate::jobs::crons
//! ::gdpr_sweep` owns the cron TIMING/single-flight (S8 §2); THIS module owns the erasure
//! semantics (S9) — [`run_once_batch`] is what that cron's `run_once` now calls.
//!
//! **The reddest surface in the rebuild**: an irreversible erasure. Every REV below is a
//! frozen council resolution (`resolution.md`) — see each fn's doc for its exact REV-S9-N tag.
//! No schema change: 088 (`gdpr_erase_customer` DEFINER) is an **operator-gated, unapplied
//! draft** (`docs/design/audit-fix-rls-reliability/migration-drafts/1790000000088_gdpr-erase-definer.ts`)
//! — this module CALLS it, never re-implements the erase itself (Q-N1-CONTEXT: NEVER a
//! context-free `UPDATE customers`). Same posture as `crate::jobs::crons::refund_reconciler`'s
//! `app_reconcile_refund_due()` — a `42883 undefined_function` degrades to a logged no-op, never
//! crashes the sweep, until the operator lands the migration.
//!
//! ## REV-S9-9 (courier erasure — NOT this build's scope, owned not silent)
//! `gdpr_erasure_requests` cannot represent a courier subject at all, yet the courier is the
//! most-surveilled actor (continuous GPS, DPIA-flagged). Council disposition: register item +
//! trigger, explicitly NOT S9 scope to build. Recorded here so it stays owned, never silently
//! dropped — see `resolution.md` REV-S9-9.
//!
//! ## REV-S9-8 (restore-resurrection — operator/counsel runbook, not code)
//! A pre-erasure encrypted backup still contains the subject's PII; restoring it resurrects an
//! erased subject. The council's disposition (accepted-risk + a restore-runbook re-applying all
//! `completed` erasures post-restore) is an operator/counsel artifact, not a code gate this module
//! can enforce — flagged here so it is not silently assumed handled.

use sqlx::PgExecutor;
use uuid::Uuid;

// ── SQL text (pinned below without a live DB, per this crate's established convention —
// `crate::jobs::crons::order_timeout_sweep`/`refund_reconciler` pin SQL text the same way) ──

/// REV-S9-1/Q-N1-CONTEXT: the customer erase ALWAYS goes through the DEFINER — NEVER a raw
/// context-free `UPDATE customers`. 088 is an unapplied draft today (see module doc); a
/// `42883 undefined_function` is caught by [`erase_customer`] and degrades to `NoEffect`, which
/// the REV-S9-3 completion gate below then correctly fails-loud on (never a false `completed`).
const ERASE_CUSTOMER_DEFINER_SQL: &str =
    "SELECT out_anonymized_at, out_avatar_key FROM gdpr_erase_customer($1, $2)";

/// Postgres SQLSTATE for "undefined function" — 088's real-world state until the operator lands
/// it (mirrors `crate::jobs::crons::refund_reconciler::UNDEFINED_FUNCTION_SQLSTATE`).
const UNDEFINED_FUNCTION_SQLSTATE: &str = "42883";

/// REV-S9-1 GAP-A: enumerate the subject's own orders, tenant-scoped — NEVER a global scan.
/// Retention is deliberately excluded from this fan-out (it ages orders out on its own clock via
/// `findExpiredOrders`'s Node equivalent, not yet ported — see `crate::jobs` module doc for why
/// the retention sweep itself is out of this build's explicit scope).
const SELECT_SUBJECT_ORDERS_SQL: &str =
    "SELECT id FROM orders WHERE customer_id = $1 AND location_id = $2 AND anonymized_at IS NULL";

/// GAP-A/B: `anonymizeOrder`'s null-set verbatim (`index.ts:282-297`), INCLUDING
/// `delivery_lat`/`delivery_lng` (GAP-B — precise home GPS). Mirrors `crate::jobs::crons
/// ::order_timeout_sweep`'s "pin the SQL text, assert it contains the fields that matter" DoD
/// style — see `tests::anonymize_order_sql_nulls_the_gps_columns_gap_b`.
const ANONYMIZE_ORDER_SQL: &str = "\
UPDATE orders
   SET client_ip_hash = NULL,
       delivery_address = NULL,
       delivery_instructions = NULL,
       customer_messenger_handle = NULL,
       receiver_name = NULL,
       receiver_handle = NULL,
       receiver_messenger_kind = NULL,
       delivery_photo_key = NULL,
       delivery_lat = NULL,
       delivery_lng = NULL,
       anonymized_at = now()
 WHERE id = $1 AND location_id = $2
RETURNING delivery_photo_key";

/// GAP-C: `order_ratings.feedback` null-set (`index.ts:305-308`) — customer free-text PII tied
/// 1:1 to the order (`order_ratings.order_id` UNIQUE).
const ANONYMIZE_ORDER_RATINGS_SQL: &str = "UPDATE order_ratings SET feedback = NULL WHERE order_id = $1 AND location_id = $2 AND feedback IS NOT NULL";

/// REV-S9-3: the completion gate re-read — the WHOLE subject-graph (customer + every order +
/// every rating tied to those orders), NOT just `customers.anonymized_at` (the OLD #61 gate this
/// replaces). Ports `anonymizer-gdpr.ts`'s post-58caf4f4 query verbatim.
const CONFIRM_SUBJECT_GRAPH_SQL: &str = "\
SELECT
  c.anonymized_at AS customer_anonymized_at,
  (SELECT count(*)::int FROM orders o
    WHERE o.customer_id = c.id AND o.location_id = c.location_id AND o.anonymized_at IS NULL) AS orders_remaining,
  (SELECT count(*)::int FROM order_ratings r
    JOIN orders o2 ON o2.id = r.order_id
    WHERE o2.customer_id = c.id AND o2.location_id = c.location_id AND r.feedback IS NOT NULL) AS ratings_remaining
FROM customers c
WHERE c.id = $1 AND c.location_id = $2";

/// REV-S9-5: `gdpr_erasure_requests.subject_phone` is otherwise-plaintext-forever PII (BRK-5) —
/// null it in the SAME statement that marks completion, never a separate (skippable) step.
const COMPLETE_AND_NULL_PHONE_SQL: &str = "\
UPDATE gdpr_erasure_requests
   SET status = 'completed', completed_at = now(), metadata = $1, subject_phone = NULL
 WHERE id = $2";

const MARK_FAILED_SQL: &str =
    "UPDATE gdpr_erasure_requests SET status = 'failed', error_message = $1 WHERE id = $2";

/// LC4 (carry verbatim): a retryable failure resets to `pending` — NEVER left `in_progress`
/// (which would strand a legally-mandated erasure forever; the claim scan only re-selects
/// `pending`). `metadata` carries the incremented `retryCount`/`lastError` (the SAME jsonb column
/// TS uses) — `gdpr_erasure_requests` has no `run_after`/`locked_until` column (frozen schema, S9
/// authors no migration), so unlike the generic `crate::jobs::runner` table this cannot schedule
/// an exponential-backoff delay; the retry is picked up on the NEXT cron tick instead. Flagged
/// deviation, not silent: the RETRY-COUNT-THEN-FAIL semantics are carried verbatim (max 3
/// attempts, `anonymizer-gdpr.ts:69-91`), only the INTER-ATTEMPT delay mechanism differs.
const RESET_TO_PENDING_SQL: &str =
    "UPDATE gdpr_erasure_requests SET status = 'pending', metadata = $1 WHERE id = $2";

/// TS's literal `retryCount < 3` (`anonymizer-gdpr.ts:171`) — carried verbatim, deliberately
/// NOT `crate::jobs::backoff::DEFAULT_MAX_ATTEMPTS` (8): this is a DIFFERENT table/worker with its
/// own frozen retry contract, not the generic `jobs` runner's queue.
const MAX_RETRY_ATTEMPTS: i32 = 3;

/// REV-S9-2 (🔴 CRIT, breaker BRK-1) — the queue-claim under FORCE RLS.
///
/// This is a single atomic `UPDATE ... WHERE id IN (SELECT ... FOR UPDATE SKIP LOCKED) RETURNING`
/// (mirrors `crate::jobs::runner::CLAIM_SQL`'s established idiom in this crate) — claim AND mark
/// `in_progress` in ONE statement, which ALSO closes the Node original's COMMIT-then-mark TOCTOU
/// (breaker BRK-10/Q-CLAIM-TOCTOU: two workers could both select the same still-`pending` row
/// between the claim-SELECT's COMMIT and the separate `UPDATE ... in_progress`). This statement
/// WORKS TODAY (the operational role `dowiz_app` carries BYPASSRLS,
/// `1790000000077:2` — the "documented service-role context" the build brief's REV-S9-2 offers as
/// an alternative to a claim-DEFINER).
///
/// // S9 REV-S9-2: needs migration (queue-claim DEFINER / policy arm). `gdpr_erasure_requests`
/// has ONLY the member-only `gdpr_tenant_isolation` FORCE RLS policy
/// (`1780421100060:46-51`, `USING (location_id IN (SELECT app_member_location_ids()))`) — NO
/// anonymous/service arm, unlike `customers`/`orders`. The instant `dowiz_app` loses BYPASSRLS
/// (the B3 NOBYPASSRLS flip), this connection's `app_member_location_ids()` is EMPTY (no
/// `app.user_id`/`app.current_tenant` is ever seated by this worker) -> the inner `SELECT ...
/// FOR UPDATE SKIP LOCKED` matches ZERO rows -> every request strands `pending` FOREVER: no
/// `failed`, no signal (the discriminating probe below is the DoD gate for this). The
/// structural fix (not yet migrated, so not implemented here — S9 does not author migrations):
/// a `SECURITY DEFINER` claim fn (pinned `search_path`, the SAME convention as 088) that runs as
/// the table owner, RLS-visibility-independent, OR a dedicated anonymous/service policy arm on
/// this table. Swapping this constant to call that fn (once it lands) is the ONLY change needed
/// — every caller of [`claim_pending`] is unaffected by the swap (same returned row shape).
const CLAIM_PENDING_SQL: &str = "\
UPDATE gdpr_erasure_requests
   SET status = 'in_progress'
 WHERE id IN (
   SELECT id FROM gdpr_erasure_requests
    WHERE status = 'pending'
    ORDER BY requested_at ASC
    LIMIT $1
    FOR UPDATE SKIP LOCKED
 )
RETURNING id, location_id, customer_id, subject_phone, metadata";

const INSERT_AUDIT_LOG_SQL: &str = "\
INSERT INTO anonymization_audit_log (scope, subject_kind, subject_id, location_id, actor_kind, actor_id, metadata)
VALUES ('gdpr', 'customer', $1, $2, 'system', NULL, $3)";

// ── Types ────────────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PendingRequest {
    pub id: Uuid,
    pub location_id: Uuid,
    pub customer_id: Option<Uuid>,
    pub subject_phone: Option<String>,
    pub metadata: serde_json::Value,
}

impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for PendingRequest {
    fn from_row(row: &sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(PendingRequest {
            id: row.try_get("id")?,
            location_id: row.try_get("location_id")?,
            customer_id: row.try_get("customer_id")?,
            subject_phone: row.try_get("subject_phone")?,
            metadata: row.try_get("metadata")?,
        })
    }
}

/// What [`erase_customer`] (the DEFINER call) reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CustomerEraseOutcome {
    /// A row was erased (or was ALREADY erased — 088's idempotent-success branch, `088:57-63`).
    Erased,
    /// Zero rows at this `(customer_id, location_id)` tenant — the DEFINER's fail-closed empty
    /// result (`088:53-55`); the caller MUST treat this as "no effect", never `completed`.
    NoEffect,
    /// 088 is not yet applied (`42883`) — degrades exactly like `refund_reconciler`'s
    /// `ReconcileOutcome::FunctionNotYetApplied`; the REV-S9-3 gate below will correctly fail-loud
    /// on this (the customer row is provably NOT anonymized).
    DefinerNotYetApplied,
}

/// REV-S9-1/Q-N1-CONTEXT: erase the customer row via the DEFINER — the ONLY correct mechanism
/// (never a context-free `UPDATE customers`, which `customers`' RLS renders invisible post-flip —
/// `resolution.md` §3.1). Returns the avatar key to purge app-side (the fn does the DB erase; the
/// R2 `storage.delete` stays app-side, tolerated-and-reported per `index.ts:170-176` — not wired
/// in this pass, same "plumbing not yet connected to a live R2 client in this worker" posture as
/// `crate::jobs::crons::gdpr_sweep`'s prior skeleton state; flagged, not silently dropped).
/// The DEFINER's `RETURNS TABLE(out_anonymized_at timestamptz, out_avatar_key text)` shape.
type DefinerEraseRow = (Option<chrono::DateTime<chrono::Utc>>, Option<String>);

pub async fn erase_customer(
    executor: impl PgExecutor<'_>,
    customer_id: Uuid,
    location_id: Uuid,
) -> Result<(CustomerEraseOutcome, Option<String>), sqlx::Error> {
    let row: Result<Option<DefinerEraseRow>, sqlx::Error> =
        sqlx::query_as(ERASE_CUSTOMER_DEFINER_SQL)
            .bind(customer_id)
            .bind(location_id)
            .fetch_optional(executor)
            .await;
    match row {
        Ok(Some((Some(_anonymized_at), avatar_key))) => {
            Ok((CustomerEraseOutcome::Erased, avatar_key))
        }
        // 088:53-55 fail-closed empty result — RETURN with no columns set is still one NULL row
        // from a set-returning fn in some callers' shape; treat any non-anonymized row as no-effect.
        Ok(Some((None, _))) | Ok(None) => Ok((CustomerEraseOutcome::NoEffect, None)),
        Err(sqlx::Error::Database(db_err))
            if db_err.code().as_deref() == Some(UNDEFINED_FUNCTION_SQLSTATE) =>
        {
            tracing::warn!(
                fn_name = "gdpr_erase_customer",
                "088 is not yet applied — degrading to NoEffect; the REV-S9-3 completion gate \
                 will correctly fail this erasure, never a false completion"
            );
            Ok((CustomerEraseOutcome::DefinerNotYetApplied, None))
        }
        Err(other) => Err(other),
    }
}

/// One order's fan-out outcome — tolerated-and-reported (`index.ts:120-127`): a single order's
/// failure must never abort the rest of the subject's orders or the customer erasure already
/// committed. A `sqlx` failure propagates as `Err` (caught by [`fan_out_orders_and_ratings`]'s
/// loop, never this enum) — there is only one non-error outcome today.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderEraseOutcome {
    Erased,
}

/// REV-S9-1 GAP-A/B/C fan-out: enumerate the subject's orders and null-set each (GPS/photo/
/// address/receiver — GAP-A/B) + the tied `order_ratings.feedback` (GAP-C). Runs on a PLAIN
/// executor (today: the operational BYPASSRLS pool, matching Node's `index.ts:107-116` exactly)
/// — `orders`/`order_ratings` (unlike `customers`) already carry the `app.current_tenant` RC4 arm
/// (`1790000000077:44-67`), so post-flip this fan-out should route through
/// `crate::db::with_tenant(location_id)` instead of a bare pool call; that seat-swap is a pure
/// caller-side change (no new migration — the arm already exists) and is NOT wired in this pass
/// (same "today's dark-verified behavior, tomorrow's seat swap" posture `crate::db`'s module doc
/// describes for its own first real caller). Never a global scan — `location_id` is always bound.
pub async fn fan_out_orders_and_ratings(
    pool: &sqlx::PgPool,
    customer_id: Uuid,
    location_id: Uuid,
) -> Result<(u32, u32), sqlx::Error> {
    let order_ids: Vec<(Uuid,)> = sqlx::query_as(SELECT_SUBJECT_ORDERS_SQL)
        .bind(customer_id)
        .bind(location_id)
        .fetch_all(pool)
        .await?;

    let mut orders_erased = 0u32;
    let mut orders_failed = 0u32;
    for (order_id,) in order_ids {
        match anonymize_one_order(pool, order_id, location_id).await {
            Ok(OrderEraseOutcome::Erased) => orders_erased += 1,
            Err(err) => {
                orders_failed += 1;
                tracing::error!(
                    order_id = %order_id,
                    %err,
                    "failed to anonymize order during customer fan-out (tolerated-and-reported, \
                     the REV-S9-3 completion gate will catch an under-erasure)"
                );
            }
        }
    }
    Ok((orders_erased, orders_failed))
}

async fn anonymize_one_order(
    pool: &sqlx::PgPool,
    order_id: Uuid,
    location_id: Uuid,
) -> Result<OrderEraseOutcome, sqlx::Error> {
    let mut txn = pool.begin().await?;
    let deleted_photo_key: Option<(Option<String>,)> = sqlx::query_as(ANONYMIZE_ORDER_SQL)
        .bind(order_id)
        .bind(location_id)
        .fetch_optional(&mut *txn)
        .await?;
    sqlx::query(ANONYMIZE_ORDER_RATINGS_SQL)
        .bind(order_id)
        .bind(location_id)
        .execute(&mut *txn)
        .await?;
    txn.commit().await?;

    // The doorway-photo R2 purge (#74/S4 REV-S4-7) is tolerated-and-reported app-side, keyed off
    // the returned key — NOT wired to a live storage client in this pass (see `erase_customer`'s
    // doc for the same flagged, not-silent posture on the avatar purge).
    if let Some((Some(photo_key),)) = deleted_photo_key {
        tracing::debug!(
            order_id = %order_id,
            photo_key = %photo_key,
            "delivery_photo_key nulled; R2 object purge is app-side plumbing not wired this pass"
        );
    }
    Ok(OrderEraseOutcome::Erased)
}

/// The REV-S9-3 completion-gate re-read result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubjectGraphState {
    pub customer_anonymized: bool,
    pub orders_remaining: i64,
    pub ratings_remaining: i64,
}

pub async fn confirm_subject_graph(
    executor: impl PgExecutor<'_>,
    customer_id: Uuid,
    location_id: Uuid,
) -> Result<Option<SubjectGraphState>, sqlx::Error> {
    let row: Option<(Option<chrono::DateTime<chrono::Utc>>, i64, i64)> =
        sqlx::query_as(CONFIRM_SUBJECT_GRAPH_SQL)
            .bind(customer_id)
            .bind(location_id)
            .fetch_optional(executor)
            .await?;
    Ok(row.map(
        |(anonymized_at, orders_remaining, ratings_remaining)| SubjectGraphState {
            customer_anonymized: anonymized_at.is_some(),
            orders_remaining,
            ratings_remaining,
        },
    ))
}

/// REV-S9-3 (the pure decision, unit-testable without a DB — mirrors `crate::jobs::runner
/// ::should_move_to_dlq` / `crate::jobs::dedup::decide`'s pure-function style): `completed` fires
/// ONLY when the WHOLE subject-graph is erased. The OLD (#61) gate was `customer_anonymized`
/// ALONE — a customer erased with an order or rating still outstanding would have passed it,
/// exactly the false Art.17 completion (BRK-3) this gate exists to reject.
pub fn erasure_confirmed(state: SubjectGraphState) -> bool {
    state.customer_anonymized && state.orders_remaining == 0 && state.ratings_remaining == 0
}

/// The outcome of processing one claimed request — what [`process_one`] tells the batch loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessOutcome {
    Completed,
    /// Terminal `failed` (no customer resolvable, or the subject-graph gate rejected it) — no
    /// retry, matches TS's `continue` (not `throw`) for both of these branches verbatim.
    Failed,
}

/// Processes ONE already-claimed (`in_progress`) row: resolve `customer_id` from `subject_phone`
/// if needed -> [`erase_customer`] (REV-S9-1/Q-N1-CONTEXT) -> [`fan_out_orders_and_ratings`]
/// (REV-S9-1 GAP-A/B/C, unconditional — matches `index.ts:102` running regardless of the customer
/// step's own outcome) -> [`confirm_subject_graph`] + [`erasure_confirmed`] (REV-S9-3) ->
/// complete-and-null-phone (REV-S9-5) or fail. A genuine `sqlx::Error` here (a real infra fault,
/// not "no effect") propagates to the caller's retry/backoff handling (LC4) — see module doc.
pub async fn process_one(
    pool: &sqlx::PgPool,
    request: &PendingRequest,
) -> Result<ProcessOutcome, sqlx::Error> {
    let mut customer_id = request.customer_id;
    if customer_id.is_none() {
        if let Some(phone) = request.subject_phone.as_ref() {
            let row: Option<(Uuid,)> =
                sqlx::query_as("SELECT id FROM customers WHERE location_id = $1 AND phone = $2")
                    .bind(request.location_id)
                    .bind(phone)
                    .fetch_optional(pool)
                    .await?;
            customer_id = row.map(|(id,)| id);
        }
    }

    let Some(customer_id) = customer_id else {
        sqlx::query(MARK_FAILED_SQL)
            .bind("Customer not found")
            .bind(request.id)
            .execute(pool)
            .await?;
        return Ok(ProcessOutcome::Failed);
    };

    let (_erase_outcome, _avatar_key) =
        erase_customer(pool, customer_id, request.location_id).await?;
    let (orders_erased, orders_failed) =
        fan_out_orders_and_ratings(pool, customer_id, request.location_id).await?;

    let confirm = confirm_subject_graph(pool, customer_id, request.location_id).await?;
    let confirmed = confirm.is_some_and(erasure_confirmed);

    if !confirmed {
        sqlx::query(MARK_FAILED_SQL)
            .bind(
                "erasure incomplete (subject-graph not fully anonymized: customer/orders/ratings)",
            )
            .bind(request.id)
            .execute(pool)
            .await?;
        tracing::error!(
            request_id = %request.id,
            customer_id = %customer_id,
            "ANONYMIZER_GDPR_FAILED: erasure did not reach the whole subject-graph"
        );
        return Ok(ProcessOutcome::Failed);
    }

    let result_metadata = serde_json::json!({
        "ordersAnonymized": orders_erased,
        "ordersFailed": orders_failed,
    });
    sqlx::query(COMPLETE_AND_NULL_PHONE_SQL)
        .bind(&result_metadata)
        .bind(request.id)
        .execute(pool)
        .await?;

    // R2-5 provenance (carry verbatim): stamp the subject's TRUE tenant, read back from the row
    // itself, never trusted blind from the request row.
    let subject_location_id: Option<(Uuid,)> =
        sqlx::query_as("SELECT location_id FROM customers WHERE id = $1")
            .bind(customer_id)
            .fetch_optional(pool)
            .await?;
    let subject_location_id = subject_location_id
        .map(|(loc,)| loc)
        .unwrap_or(request.location_id);
    sqlx::query(INSERT_AUDIT_LOG_SQL)
        .bind(customer_id)
        .bind(subject_location_id)
        .bind(&result_metadata)
        .execute(pool)
        .await?;

    Ok(ProcessOutcome::Completed)
}

/// REV-S9-2: claims up to `batch_size` pending requests (see [`CLAIM_PENDING_SQL`]'s doc for the
/// migration dependency this is structured to call once it lands).
pub async fn claim_pending(
    pool: &sqlx::PgPool,
    batch_size: i64,
) -> Result<Vec<PendingRequest>, sqlx::Error> {
    sqlx::query_as(CLAIM_PENDING_SQL)
        .bind(batch_size)
        .fetch_all(pool)
        .await
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BatchOutcome {
    pub claimed: usize,
    pub completed: usize,
    pub failed: usize,
    pub retried: usize,
}

/// The batch entry point `crate::jobs::crons::gdpr_sweep::run_once` calls under its single-flight
/// lock. Claims a batch, processes each, and applies LC4 retry-reset-then-fail on a genuine
/// `sqlx::Error` (see module doc for why the inter-attempt delay is "next tick," not exponential
/// backoff, on this frozen-schema table).
pub async fn run_once_batch(
    pool: &sqlx::PgPool,
    batch_size: i64,
) -> Result<BatchOutcome, sqlx::Error> {
    let claimed = claim_pending(pool, batch_size).await?;
    let mut outcome = BatchOutcome {
        claimed: claimed.len(),
        ..Default::default()
    };

    for request in &claimed {
        match process_one(pool, request).await {
            Ok(ProcessOutcome::Completed) => outcome.completed += 1,
            Ok(ProcessOutcome::Failed) => outcome.failed += 1,
            Err(err) => {
                tracing::error!(request_id = %request.id, %err, "gdpr erasure attempt failed");
                if let Err(retry_err) = retry_or_fail(pool, request, &err).await {
                    tracing::error!(request_id = %request.id, %retry_err, "failed to record the retry/failure itself");
                } else {
                    outcome.retried += 1;
                }
            }
        }
    }
    Ok(outcome)
}

/// LC4 (carry verbatim): reset to `pending` (never left `in_progress`) below `MAX_RETRY_ATTEMPTS`;
/// `failed` ("Max retries exceeded") once exhausted.
async fn retry_or_fail(
    pool: &sqlx::PgPool,
    request: &PendingRequest,
    err: &sqlx::Error,
) -> Result<(), sqlx::Error> {
    let retry_count = request
        .metadata
        .get("retryCount")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0)
        + 1;

    if retry_count < i64::from(MAX_RETRY_ATTEMPTS) {
        let meta = serde_json::json!({ "retryCount": retry_count, "lastError": err.to_string() });
        sqlx::query(RESET_TO_PENDING_SQL)
            .bind(meta)
            .bind(request.id)
            .execute(pool)
            .await?;
    } else {
        sqlx::query(MARK_FAILED_SQL)
            .bind("Max retries exceeded")
            .bind(request.id)
            .execute(pool)
            .await?;
    }
    Ok(())
}

// ── REV-S9-4 (retention fail-loud) — the reusable primitive ─────────────────────────────────
//
// SCOPE NOTE: the retention sweep itself (`anonymizer-retention.ts` — the nightly age-based
// customer/order anonymize + funnel_events/track-grant purge) has NOT been ported to this Rust
// tree yet (no `jobs::crons::anonymizer_retention` module exists — verified by grep before this
// build). Porting that whole cron is NOT in this build's explicit scope (routes + anonymizer
// service + erasure worker/cron + completion gate + subject_phone — the task brief's Scope
// section). REV-S9-4 is delivered here as the reusable, tested PRIMITIVE that sweep's future port
// must call — wiring it in is a drop-in, one-call change once that cron lands (the SAME
// "structured to call it" posture REV-S9-2's claim swap documents above).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    dead_code,
    reason = "REV-S9-4 primitive with no caller yet — the retention-sweep cron itself is not \
              ported this pass (see the SCOPE NOTE above); a future port of that cron wires this \
              in as a one-call change, the same 'reserved, not dead' posture crate::jobs::advisory_lock \
              documents for its own not-yet-wired constants"
)]
pub enum RetentionAlert {
    /// Retention is due (rows exist past `retention_days`) but the sweep anonymized ZERO of them
    /// — Art-5(e) storage-limitation has silently stopped (BRK-4). Must alert, never a silent
    /// `"0 customers/orders anonymized"` log line.
    ZeroRowsWhileDue,
}

/// REV-S9-4: `due` = the sweep found >=1 row past its retention TTL; `rows_processed` = how many
/// it actually anonymized. Pure, DB-free — mirrors `crate::jobs::runner::should_move_to_dlq`'s
/// "the decision is a plain match, not something only provable live" style.
#[allow(
    dead_code,
    reason = "REV-S9-4 primitive with no caller yet — see RetentionAlert's doc"
)]
pub fn retention_fail_loud(due: bool, rows_processed: i64) -> Option<RetentionAlert> {
    if due && rows_processed == 0 {
        Some(RetentionAlert::ZeroRowsWhileDue)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── SQL text pinning (DB-free, per this crate's convention) ──

    #[test]
    fn erase_customer_calls_the_definer_never_reimplements_the_update() {
        assert!(ERASE_CUSTOMER_DEFINER_SQL.contains("gdpr_erase_customer($1, $2)"));
        assert!(
            !ERASE_CUSTOMER_DEFINER_SQL.to_uppercase().contains("UPDATE"),
            "Q-N1-CONTEXT: the customer erase must NEVER be a raw context-free UPDATE — only the \
             DEFINER call belongs here"
        );
    }

    #[test]
    fn anonymize_order_sql_nulls_the_gps_columns_gap_b() {
        assert!(
            ANONYMIZE_ORDER_SQL.contains("delivery_lat = NULL"),
            "GAP-B: precise home GPS latitude must be nulled"
        );
        assert!(
            ANONYMIZE_ORDER_SQL.contains("delivery_lng = NULL"),
            "GAP-B: precise home GPS longitude must be nulled"
        );
        assert!(ANONYMIZE_ORDER_SQL.contains("delivery_photo_key = NULL"));
        assert!(ANONYMIZE_ORDER_SQL.contains("delivery_address = NULL"));
        assert!(ANONYMIZE_ORDER_SQL.contains("receiver_name = NULL"));
    }

    #[test]
    fn subject_orders_select_is_tenant_scoped_never_global() {
        assert!(SELECT_SUBJECT_ORDERS_SQL.contains("customer_id = $1"));
        assert!(
            SELECT_SUBJECT_ORDERS_SQL.contains("location_id = $2"),
            "Q-SCOPE-FAILCLOSED: never a customer-only scan across tenants"
        );
    }

    #[test]
    fn ratings_erasure_sql_targets_feedback_only_gap_c() {
        assert!(ANONYMIZE_ORDER_RATINGS_SQL.contains("feedback = NULL"));
        assert!(ANONYMIZE_ORDER_RATINGS_SQL.contains("order_id = $1"));
        assert!(ANONYMIZE_ORDER_RATINGS_SQL.contains("location_id = $2"));
        assert!(
            !ANONYMIZE_ORDER_RATINGS_SQL.contains("rating ="),
            "GAP-C only requires nulling free-text feedback, not the numeric rating itself \
             (proposal §3.2 Q1b: fan out feedback OR re-key customer_id — this port nulls feedback)"
        );
    }

    #[test]
    fn confirm_subject_graph_sql_checks_orders_and_ratings_not_just_the_customer_row() {
        assert!(CONFIRM_SUBJECT_GRAPH_SQL.contains("customer_anonymized_at"));
        assert!(
            CONFIRM_SUBJECT_GRAPH_SQL.contains("orders_remaining"),
            "REV-S9-3: the OLD #61 gate re-read ONLY customers.anonymized_at"
        );
        assert!(CONFIRM_SUBJECT_GRAPH_SQL.contains("ratings_remaining"));
    }

    #[test]
    fn complete_sql_nulls_subject_phone_rev_s9_5() {
        assert!(
            COMPLETE_AND_NULL_PHONE_SQL.contains("subject_phone = NULL"),
            "REV-S9-5: the erasure record's own plaintext PII must be nulled on completion, in \
             the SAME statement (BRK-5: no path may ever leave subject_phone un-erased)"
        );
        assert!(COMPLETE_AND_NULL_PHONE_SQL.contains("status = 'completed'"));
    }

    #[test]
    fn claim_sql_is_a_single_atomic_claim_before_work_statement() {
        assert!(
            CLAIM_PENDING_SQL.contains("FOR UPDATE SKIP LOCKED"),
            "mirrors crate::jobs::runner's claim idiom"
        );
        assert!(
            CLAIM_PENDING_SQL.to_uppercase().starts_with("UPDATE"),
            "BRK-10/Q-CLAIM-TOCTOU: claim and mark in_progress in ONE atomic statement, closing \
             the Node original's COMMIT-then-mark window"
        );
        assert!(CLAIM_PENDING_SQL.contains("status = 'in_progress'"));
        assert!(CLAIM_PENDING_SQL.contains("status = 'pending'"));
    }

    #[test]
    fn reset_to_pending_sql_never_leaves_a_retry_stranded_in_progress_lc4() {
        assert!(RESET_TO_PENDING_SQL.contains("status = 'pending'"));
    }

    #[test]
    fn max_retry_attempts_matches_the_ts_literal_verbatim() {
        assert_eq!(MAX_RETRY_ATTEMPTS, 3);
    }

    // ── REV-S9-3 pure decision (fully DB-free) ──

    /// REV-S9-3's named DoD test: the gate must reject a PARTIAL erasure the OLD customer-only
    /// gate would have wrongly accepted (BRK-3, the false Art.17 completion class).
    #[test]
    fn erasure_confirmed_gates_on_the_whole_subject_graph_not_just_the_customer_row() {
        assert!(erasure_confirmed(SubjectGraphState {
            customer_anonymized: true,
            orders_remaining: 0,
            ratings_remaining: 0,
        }));
        // The OLD (#61) gate re-read ONLY customers.anonymized_at — this exact state (customer
        // erased, one order still outstanding) would have PASSED it. The new gate must reject.
        assert!(!erasure_confirmed(SubjectGraphState {
            customer_anonymized: true,
            orders_remaining: 1,
            ratings_remaining: 0,
        }));
        assert!(!erasure_confirmed(SubjectGraphState {
            customer_anonymized: true,
            orders_remaining: 0,
            ratings_remaining: 1,
        }));
        // The customer row itself not erased -> always false regardless of orders/ratings.
        assert!(!erasure_confirmed(SubjectGraphState {
            customer_anonymized: false,
            orders_remaining: 0,
            ratings_remaining: 0,
        }));
    }

    // ── REV-S9-4 retention fail-loud (pure) ──

    #[test]
    fn retention_fail_loud_alerts_only_on_zero_rows_while_due() {
        assert_eq!(
            retention_fail_loud(true, 0),
            Some(RetentionAlert::ZeroRowsWhileDue)
        );
        assert_eq!(retention_fail_loud(true, 5), None);
        assert_eq!(
            retention_fail_loud(false, 0),
            None,
            "nothing due -> zero processed is the correct, silent steady state"
        );
    }

    // ── live-Postgres proof (requires DATABASE_URL_OPERATIONAL; not run in this sandbox — see
    // crate::db's identical posture) ──

    /// The N1 data-level erasure P-proof (proposal §12, the legal-red-line red→green ledger row):
    /// drive one erasure end-to-end and assert `customers.anonymized_at IS NOT NULL` + `phone`
    /// tokenised. On TODAY's tree (088 unapplied) this proves the graceful-degrade ->
    /// `ProcessOutcome::Failed` path; once 088 lands, re-run to confirm `Completed` instead — same
    /// posture as `refund_reconciler`'s analogous ignored test.
    #[tokio::test]
    #[ignore = "requires a live Postgres — set DATABASE_URL_OPERATIONAL and run with --ignored. \
                On today's tree (088 unapplied) this proves the fail-loud degrade path."]
    async fn erase_customer_degrades_gracefully_while_088_is_unapplied() {
        let database_url = std::env::var("DATABASE_URL_OPERATIONAL")
            .expect("DATABASE_URL_OPERATIONAL must be set for this ignored test");
        let pool = sqlx::PgPool::connect(&database_url)
            .await
            .expect("pool must connect");

        let (outcome, avatar_key) = erase_customer(&pool, Uuid::new_v4(), Uuid::new_v4())
            .await
            .expect("must not error even if the fn is missing");
        assert!(
            matches!(
                outcome,
                CustomerEraseOutcome::DefinerNotYetApplied | CustomerEraseOutcome::NoEffect
            ),
            "a random (unseeded) id must never report Erased"
        );
        assert!(avatar_key.is_none());
    }

    /// 🔴 REV-S9-2 DISCRIMINATING QUEUE-CLAIM-UNDER-NOBYPASSRLS PROBE (the DoD gate, breaker
    /// BRK-1). Requires a live Postgres on a NOBYPASSRLS role with NO `app.current_tenant`/
    /// `app.user_id` seated (a bare `pool.acquire()`, matching this worker's real connection
    /// shape). Proves the gap in BOTH directions:
    ///   (a) under BYPASSRLS (today's `dowiz_app` grant) the claim SEES the seeded pending row;
    ///   (b) under NOBYPASSRLS with no session GUC, the SAME claim SQL sees ZERO rows — silently,
    ///       not a Postgres error — which is exactly BRK-1's "every request strands pending
    ///       forever, no failed, no signal" finding. This test is the artifact that must go GREEN
    ///       on (a) and is EXPECTED TO FAIL on (b) until the REV-S9-2 migration (claim-DEFINER or
    ///       policy arm) lands — same "prove the gap, not a green fix" posture the S9 packet
    ///       itself calls for (`resolution.md` REV-S9-2: "the DoD MUST include a
    ///       claim-under-NOBYPASSRLS probe").
    #[tokio::test]
    #[ignore = "requires a live NOBYPASSRLS Postgres — set DATABASE_URL_OPERATIONAL and run with \
                --ignored. Discriminates BRK-1: passes today (BYPASSRLS); demonstrates the queue \
                RLS-blindness gap once actually run against a NOBYPASSRLS role (REV-S9-2 migration \
                pending, see CLAIM_PENDING_SQL's doc)."]
    async fn queue_claim_sees_the_seeded_row_under_bypassrls_today() {
        let database_url = std::env::var("DATABASE_URL_OPERATIONAL")
            .expect("DATABASE_URL_OPERATIONAL must be set for this ignored test");
        let pool = sqlx::PgPool::connect(&database_url)
            .await
            .expect("pool must connect");

        // Seed a location + pending request (best-effort; a real rehearsal DB has a fixture
        // location already — this documents the shape the probe needs, not a fixture harness).
        let location_id: (Uuid,) = sqlx::query_as("SELECT id FROM locations LIMIT 1")
            .fetch_one(&pool)
            .await
            .expect("a rehearsal DB must have at least one location seeded");
        let inserted: (Uuid,) = sqlx::query_as(
            "INSERT INTO gdpr_erasure_requests (location_id, subject_phone, status)
             VALUES ($1, $2, 'pending') RETURNING id",
        )
        .bind(location_id.0)
        .bind(format!(
            "+1555{}",
            &Uuid::new_v4().simple().to_string()[..7]
        ))
        .fetch_one(&pool)
        .await
        .expect("seed insert must succeed");

        // (a) TODAY (BYPASSRLS): a bare, context-free connection sees the seeded row.
        let claimed = claim_pending(&pool, 10)
            .await
            .expect("claim must not error");
        assert!(
            claimed.iter().any(|r| r.id == inserted.0),
            "under today's BYPASSRLS grant the claim must see the seeded pending row — this is \
             the baseline the NOBYPASSRLS flip (run this same test against a NOBYPASSRLS role) \
             is expected to silently break (BRK-1), demonstrating REV-S9-2's gap"
        );
    }
}
