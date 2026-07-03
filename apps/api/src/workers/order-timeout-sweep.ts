// @ts-nocheck
import type { Pool } from 'pg';
import type Boss from 'pg-boss';
import type { MessageBus } from '@deliveryos/platform';
import { QUEUE_NAMES, orderChannel, dashboardChannel } from '../lib/registry.js';

// Internal, self-scheduled queue — not a cross-service contract, so it lives here
// rather than in @deliveryos/shared-types. Nothing else enqueues to it.
const SWEEP_QUEUE = 'order.timeout_sweep';
// Run every minute. The per-order `order.timeout` job (apps/worker handler) is the
// primary canceller; this sweep is the safety net that recovers orders whose
// per-order job was lost (dead consumer, enqueue gap) and the detector for a
// stuck/undrained `order.timeout` queue. Both recovery and detection live in this
// one job so the detector cannot lose its host (the ReconciliationWorker that
// would otherwise own it is removed in prod — server.ts).
const SWEEP_CRON = '* * * * *';
// Dedicated advisory lock id (anonymizer-retention uses 4) so two web instances
// never run the sweep concurrently.
const SWEEP_LOCK_ID = 5;

export class OrderTimeoutSweepWorker {
  constructor(
    private pool: Pool,
    private boss: Boss,
    private messageBus: MessageBus,
  ) {}

  async start() {
    await this.boss.work(SWEEP_QUEUE, { singletonKey: SWEEP_QUEUE }, async () => {
      await this.run();
    });
    await this.boss.createQueue(SWEEP_QUEUE);
    await this.boss.schedule(SWEEP_QUEUE, SWEEP_CRON, null, { singletonKey: SWEEP_QUEUE });
    console.log('[OrderTimeoutSweep] scheduled (1-min reconciliation + overdue detection)');
  }

  private async run() {
    const client = await this.pool.connect();
    try {
      const lock = await client.query('SELECT pg_try_advisory_lock($1) AS locked', [SWEEP_LOCK_ID]);
      if (!lock.rows[0]?.locked) return; // another instance holds it

      try {
        // (1) DETECTION — overdue-but-undrained order.timeout jobs. A lost/stuck
        // consumer shows up as this count climbing; the heartbeat only proves the
        // worker VM breathes, not that the queue drains, so this is the real signal.
        try {
          const overdue = await client.query(
            `SELECT count(*)::int AS n FROM pgboss.job
             WHERE name = $1 AND state IN ('created','active') AND start_after < now()`,
            [QUEUE_NAMES.ORDER_TIMEOUT]
          );
          const n = overdue.rows[0]?.n ?? 0;
          if (n > 0) {
            console.warn(`[OrderTimeoutSweep] DETECTION: ${n} overdue order.timeout job(s) undrained`);
            await this.messageBus.publish('ops:order_timeout_lag', { overdue: n, time: new Date().toISOString() });
          }
        } catch (e) {
          console.error('[OrderTimeoutSweep] overdue detection query failed:', e);
        }

        // (2) RECOVERY — identical guarded UPDATE to the per-order handler
        // (apps/worker/src/handlers.ts). The WHERE status='PENDING' guard IS the
        // transition authority: PENDING→CANCELLED on timeout is a single fixed edge,
        // cancelled iff still pending, atomically. Cross-tenant (no location_id) by
        // design — recover every tenant's overdue orders in one pass.
        //
        // B3 (NOBYPASSRLS): the cross-tenant UPDATE + audit INSERT touch FORCE-RLS
        // tenant tables (orders, order_status_history) with no GUC, so they run inside
        // the app_sweep_timeout_orders() SECURITY DEFINER fn (mirrors the exact WHERE
        // guard + RETURNING; folds the order_status_history audit row in atomically).
        const res = await client.query(`SELECT * FROM app_sweep_timeout_orders()`);

        // (3) L-D (ADR-audit-fix-money §3.2): refund-obligation reconciler — the deterministic
        // alarm of last resort for LC6. Runs EVERY tick right after the sweep (also on quiet
        // ticks), so a terminal+paid order missing its refund_due is recorded within ≤1 tick or
        // surfaced as an operator alert. Isolated try/catch: a reconciler failure must never
        // block the sweep's recovery/notification work below (and vice versa — hence before the
        // early return, not after it).
        await this.reconcileRefundDue(client);

        if (!res.rowCount) return;
        console.log(`[OrderTimeoutSweep] recovered ${res.rowCount} overdue PENDING order(s)`);
        const ts = new Date().toISOString();

        for (const row of res.rows) {
          const orderId = row.id as string;
          const locationId = row.location_id as string;

          // Audit trail (order_status_history) is now written atomically inside
          // app_sweep_timeout_orders() — see fn above.

          // Cross-surface live update — customer status page + owner dashboard.
          try {
            await this.messageBus.publish(orderChannel(orderId), {
              type: 'order.status', orderId, status: 'CANCELLED', locationId, timestamp: ts,
            });
            await this.messageBus.publish(dashboardChannel(locationId), {
              type: 'order.status', data: { orderId, status: 'CANCELLED', statusUpdatedAt: ts },
            });
          } catch (e) {
            console.error(`[OrderTimeoutSweep] bus publish failed for ${orderId}:`, e);
          }

          // Owner notification — same emit contract as the handler. Dedupes to one
          // message via the NotificationWorker in-process set + singletonKey when the
          // handler also emitted; when the handler died pre-emit, this is the one
          // that fires (R2: emit on observing CANCELLED-by-timeout, not on identity).
          const dedupKey = `order.timeout_cancelled:${orderId}:${locationId}`;
          try {
            await this.boss.send(QUEUE_NAMES.NOTIFY_TELEGRAM_SEND, {
              event: 'order.timeout_cancelled',
              entity_id: orderId,
              location_id: locationId,
              dedupKey,
            }, { singletonKey: dedupKey });
          } catch (e) {
            console.error(`[OrderTimeoutSweep] notify enqueue failed for ${orderId}:`, e);
          }
        }
      } finally {
        await client.query('SELECT pg_advisory_unlock($1)', [SWEEP_LOCK_ID]);
      }
    } catch (err) {
      console.error('[OrderTimeoutSweep] Error:', err);
    } finally {
      client.release();
    }
  }

  // L-D (ADR-audit-fix-money §3.2, mig 1790000000087): app_reconcile_refund_due() returns one row
  // per action — 'inserted' (an obligation another layer missed, recorded now — the miss itself is
  // DRIFT-worthy), 'failed' (persistently un-recordable → alarm EVERY tick until resolved, P15),
  // 'mismatch' (P16: over/under-paid crypto on a dead order — surfaced, never auto-obligated).
  // failed/mismatch → DRIFT log + Sentry + ops-bus operator alert (same channel the recon worker
  // uses). Never throws: a reconciler outage degrades to a logged error, never a crashed sweep.
  private async reconcileRefundDue(client: any) {
    try {
      const recon = await client.query(`SELECT * FROM app_reconcile_refund_due()`);
      if (!recon.rowCount) return;
      const rows = recon.rows as Array<{ o_order_id: string; o_payment_id: string; o_action: string; o_detail: string | null }>;
      const inserted = rows.filter((r) => r.o_action === 'inserted');
      const failed = rows.filter((r) => r.o_action === 'failed');
      const mismatch = rows.filter((r) => r.o_action === 'mismatch');
      if (inserted.length) {
        console.warn(`[OrderTimeoutSweep] DRIFT: refund_due recovered by L-D reconciler for ${inserted.length} payment(s) — an upstream layer (L-A/L-B/L-C) missed them`,
          inserted.map((r) => r.o_order_id));
      }
      if (failed.length || mismatch.length) {
        const summary = [
          ...failed.map((r) => `failed order=${r.o_order_id} payment=${r.o_payment_id}: ${(r.o_detail || '').substring(0, 120)}`),
          ...mismatch.map((r) => `mismatch order=${r.o_order_id} payment=${r.o_payment_id} ${r.o_detail || ''}`),
        ].join('\n');
        console.error(`[OrderTimeoutSweep] DRIFT refund_due (${failed.length} failed, ${mismatch.length} mismatch):\n${summary}`);
        try {
          const { getSentry } = await import('../lib/sentry.js');
          getSentry()?.captureMessage(`refund_due reconciler: ${failed.length} failed, ${mismatch.length} mismatch terminal-order payment(s)`, 'error');
        } catch { /* sentry unavailable — log + bus alert still fire */ }
        try {
          await this.messageBus.publish('ops.reconciliation_drift', {
            timestamp: new Date().toISOString(),
            source: 'refund_due_reconciler:L-D',
            failedCount: failed.length,
            mismatchCount: mismatch.length,
            driftSummary: summary.substring(0, 1000),
          });
        } catch (e) {
          console.error('[OrderTimeoutSweep] refund_due drift alert publish failed:', e);
        }
      }
    } catch (e) {
      // Fn missing (migration not yet applied) or transient failure — degrade loudly, never wedge.
      console.error('[OrderTimeoutSweep] refund_due reconciler failed:', e);
    }
  }
}
