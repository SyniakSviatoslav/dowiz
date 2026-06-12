/**
 * Nightly Reconciliation Worker — read-only data drift detection
 * 
 * Runs as pg-boss cron (nightly). Checks monetary invariants, orphan data,
 * notification delivery gaps, retention compliance, and trend anomalies.
 * Reports DRIFT via NotificationProvider (Telegram-ops). Zero mutations.
 * 
 * To register: add to server.ts startup:
 *   const reconWorker = new ReconciliationWorker(pool, queue.boss, messageBus);
 *   await reconWorker.start();
 */

import type { Pool } from 'pg';
import type { PgBoss } from 'pg-boss';
import type { MessageBus } from '@deliveryos/platform';
import { QUEUE_NAMES } from '../lib/registry.js';

export interface ReconCheckResult {
  checkId: string;
  status: 'PASS' | 'DRIFT' | 'INCONCLUSIVE';
  detail: string;
  driftCount: number;
  evidence?: any[];
}

export class ReconciliationWorker {
  constructor(
    private pool: Pool,
    private boss: PgBoss,
    private messageBus: MessageBus,
  ) {}

  async start() {
    // Queue must be pre-created in migrations (runtime role lacks DDL)
    await this.boss.createQueue(QUEUE_NAMES.RECONCILIATION_NIGHTLY).catch((err: any) => {
      console.warn(`[Recon] Queue creation failed (expected if not pre-created): ${err.message}`);
    });
    await this.boss.work(QUEUE_NAMES.RECONCILIATION_NIGHTLY, async () => {
      await this.run();
    });
    await this.boss.schedule(QUEUE_NAMES.RECONCILIATION_NIGHTLY, '0 3 * * *', null, { singletonKey: QUEUE_NAMES.RECONCILIATION_NIGHTLY }).catch((err: any) => {
      console.warn(`[Recon] Schedule failed: ${err.message} — queue may not exist`);
    });
    console.log('[Recon] Worker registered: nightly at 3AM UTC');
  }

  async run(): Promise<void> {
    console.log('[Recon] Starting nightly reconciliation...');
    const startAll = Date.now();
    const results: ReconCheckResult[] = [];

    const checks: { id: string; fn: () => Promise<ReconCheckResult> }[] = [
      { id: 'M1', fn: () => this.checkPricingIntegrity() },
      { id: 'M2', fn: () => this.checkNegativeValues() },
      { id: 'M3', fn: () => this.checkCashAmount() },
      { id: 'M4', fn: () => this.checkDeliveredCashMatch() },
      { id: 'O1', fn: () => this.checkOrphanOrders() },
      { id: 'O2', fn: () => this.checkOpenShifts() },
      { id: 'O3', fn: () => this.checkFailedJobs() },
      { id: 'N1', fn: () => this.checkUndeliveredNotifications() },
      { id: 'R1', fn: () => this.checkRetention() },
      { id: 'F1', fn: () => this.checkMissingFKs() },
      { id: 'T1', fn: () => this.checkTrendAnomaly() },
      { id: 'A6', fn: () => this.checkWorkerLiveness() },
    ];

    for (const check of checks) {
      const start = Date.now();
      try {
        const result = await check.fn();
        result.checkId = check.id;
        results.push(result);
        const icon = result.status === 'PASS' ? '✅' : result.status === 'DRIFT' ? '🔴' : '🔶';
        console.log(`[Recon] ${icon} ${check.id} — ${result.status} (${Date.now() - start}ms, ${result.driftCount} drift)`);
      } catch (err: any) {
        results.push({ checkId: check.id, status: 'INCONCLUSIVE', detail: err.message, driftCount: 0 });
        console.log(`[Recon] 🔶 ${check.id} — INCONCLUSIVE (${err.message})`);
      }
    }

    const totalMs = Date.now() - startAll;
    const driftCount = results.filter(r => r.status === 'DRIFT').length;
    const inconclusiveCount = results.filter(r => r.status === 'INCONCLUSIVE').length;

    // Log full report
    console.log(`\n[Recon] === RECON REPORT ===`);
    console.log(`[Recon] Duration: ${totalMs}ms`);
    console.log(`[Recon] Checks: ${results.length} total, ${results.length - driftCount - inconclusiveCount} PASS, ${driftCount} DRIFT, ${inconclusiveCount} INCONCLUSIVE`);
    for (const r of results) {
      if (r.status !== 'PASS') {
        console.log(`[Recon] ${r.checkId}: ${r.detail}`);
      }
    }

    // Alert on DRIFT via MessageBus (→ NotificationProvider → Telegram-ops)
    if (driftCount > 0) {
      const driftSummary = results
        .filter(r => r.status === 'DRIFT')
        .map(r => `${r.checkId}: ${r.driftCount}x — ${r.detail.substring(0, 100)}`)
        .join('\n');

      await this.messageBus.publish('ops.reconciliation_drift', {
        timestamp: new Date().toISOString(),
        totalChecks: results.length,
        driftCount,
        inconclusiveCount,
        durationMs: totalMs,
        driftSummary,
      }).catch(err => {
        console.error('[Recon] Failed to publish drift alert (best-effort):', err.message);
      });
    }
  }

  // ── M1: total = subtotal + delivery_fee + tax_total - discount_total ──
  private async checkPricingIntegrity(): Promise<ReconCheckResult> {
    const res = await this.pool.query(
      `SELECT id, location_id, total, subtotal, delivery_fee, tax_total, discount_total
       FROM orders
       WHERE created_at > now() - interval '7 days'
         AND total != subtotal + delivery_fee + tax_total - discount_total
       LIMIT 50`
    );
    return {
      checkId: 'M1', status: res.rows.length === 0 ? 'PASS' : 'DRIFT',
      detail: `${res.rows.length} orders with mismatched total vs components`,
      driftCount: res.rows.length, evidence: res.rows,
    };
  }

  // ── M2: no negative monetary fields ──
  private async checkNegativeValues(): Promise<ReconCheckResult> {
    const res = await this.pool.query(
      `SELECT id, total, subtotal
       FROM orders
       WHERE total < 0 OR subtotal < 0
       LIMIT 20`
    );
    return {
      checkId: 'M2', status: res.rows.length === 0 ? 'PASS' : 'DRIFT',
      detail: `${res.rows.length} orders with negative monetary values`,
      driftCount: res.rows.length,
    };
  }

  // ── M3: cash_pay_with >= total ──
  private async checkCashAmount(): Promise<ReconCheckResult> {
    const res = await this.pool.query(
      `SELECT id, total, cash_pay_with
       FROM orders
       WHERE payment_method = 'cash'
         AND cash_pay_with IS NOT NULL
         AND cash_pay_with < total
       LIMIT 20`
    );
    return {
      checkId: 'M3', status: res.rows.length === 0 ? 'PASS' : 'DRIFT',
      detail: `${res.rows.length} cash orders with pay_with < total`,
      driftCount: res.rows.length,
    };
  }

  // ── M4: delivered cash matches total ──
  private async checkDeliveredCashMatch(): Promise<ReconCheckResult> {
    const res = await this.pool.query(
      `SELECT a.id, a.order_id, a.cash_amount, o.total
       FROM courier_assignments a
       JOIN orders o ON o.id = a.order_id
       WHERE a.cash_collected = true
         AND a.cash_amount IS NOT NULL
         AND a.cash_amount != o.total
       LIMIT 20`
    );
    return {
      checkId: 'M4', status: res.rows.length === 0 ? 'PASS' : 'DRIFT',
      detail: `${res.rows.length} delivered cash orders where amount != total`,
      driftCount: res.rows.length,
    };
  }

  // ── O1: stale non-terminal orders ──
  private async checkOrphanOrders(): Promise<ReconCheckResult> {
    const res = await this.pool.query(
      `SELECT id, status, location_id, created_at
       FROM orders
       WHERE status = 'PENDING'
         AND created_at < now() - interval '1 hour'
       ORDER BY created_at DESC
       LIMIT 50`
    );
    return {
      checkId: 'O1', status: res.rows.length <= 5 ? 'PASS' : 'DRIFT',
      detail: `${res.rows.length} PENDING orders older than 1h (tolerance: 5)`,
      driftCount: res.rows.length,
    };
  }

  // ── O2: open shifts > 24h ──
  private async checkOpenShifts(): Promise<ReconCheckResult> {
    const res = await this.pool.query(
      `SELECT id, courier_id, location_id, started_at
       FROM courier_shifts
       WHERE status IN ('available', 'on_delivery')
         AND started_at < now() - interval '24 hours'
       ORDER BY started_at
       LIMIT 20`
    );
    return {
      checkId: 'O2', status: res.rows.length === 0 ? 'PASS' : 'DRIFT',
      detail: `${res.rows.length} shifts open > 24h`,
      driftCount: res.rows.length,
    };
  }

  // ── A6: worker liveness — dead or stale workers ──
  private async checkWorkerLiveness(): Promise<ReconCheckResult> {
    const EXPECTED_WORKERS = ['dispatcher','settlement-cron','dwell-monitor','anonymizer-retention',
                              'signal-raiser','liveness-checker','courier-stale_check','backup-hourly'];
    const res = await this.pool.query(`
      SELECT expected.worker_id, h.last_seen_at, h.status
      FROM (SELECT unnest($1::text[]) AS worker_id) expected
      LEFT JOIN ops_worker_heartbeat h ON h.worker_id = expected.worker_id
      WHERE h.last_seen_at IS NULL
         OR h.last_seen_at < now() - interval '1 hour'
         OR h.status != 'healthy'
    `, [EXPECTED_WORKERS]);
    const deadWorkers = res.rows.map((r: any) => r.worker_id);
    return {
      checkId: 'A6', status: deadWorkers.length === 0 ? 'PASS' : 'DRIFT',
      detail: deadWorkers.length === 0
        ? 'All expected workers have recent heartbeats'
        : `Dead/stale workers: ${deadWorkers.join(', ')}`,
      driftCount: deadWorkers.length,
      evidence: res.rows,
    };
  }
  private async checkFailedJobs(): Promise<ReconCheckResult> {
    const res = await this.pool.query(
      `SELECT name, count(*)::int AS cnt
       FROM pgboss.job
       WHERE state = 'failed'
         AND created_on > now() - interval '24 hours'
       GROUP BY name
       HAVING count(*) > 10
       ORDER BY cnt DESC`
    );
    return {
      checkId: 'O3', status: res.rows.length === 0 ? 'PASS' : 'DRIFT',
      detail: `${res.rows.length} queue(s) with >10 failed jobs in 24h`,
      driftCount: res.rows.length,
      evidence: res.rows,
    };
  }

  // ── N1: critical events without delivered audit ──
  private async checkUndeliveredNotifications(): Promise<ReconCheckResult> {
    const res = await this.pool.query(
      `SELECT event, location_id, count(*)::int AS cnt
       FROM notification_outbox_audit
       WHERE event IN ('order.created', 'order.confirmed', 'order.rejected')
         AND created_at > now() - interval '24 hours'
       GROUP BY event, location_id
       HAVING bool_and(status != 'delivered')
       LIMIT 20`
    );
    return {
      checkId: 'N1', status: res.rows.length === 0 ? 'PASS' : 'DRIFT',
      detail: `${res.rows.length} location-event pairs with no delivered audit in 24h`,
      driftCount: res.rows.length,
      evidence: res.rows,
    };
  }

  // ── R1: PII retention ──
  private async checkRetention(): Promise<ReconCheckResult> {
    const res = await this.pool.query(
      `SELECT c.id, c.location_id, c.created_at AS customer_since, l.retention_days
       FROM customers c
       JOIN locations l ON l.id = c.location_id
       WHERE c.anonymized_at IS NULL
         AND c.created_at < now() - (l.retention_days || ' days')::interval
       LIMIT 20`
    );
    return {
      checkId: 'R1', status: res.rows.length === 0 ? 'PASS' : 'DRIFT',
      detail: `${res.rows.length} customers past retention limit without anonymization`,
      driftCount: res.rows.length,
    };
  }

  // ── F1/F2: FK orphans ──
  private async checkMissingFKs(): Promise<ReconCheckResult> {
    const missingLocation = await this.pool.query(
      `SELECT o.id FROM orders o LEFT JOIN locations l ON l.id = o.location_id WHERE l.id IS NULL LIMIT 10`
    );
    const missingCourier = await this.pool.query(
      `SELECT a.id FROM courier_assignments a LEFT JOIN couriers c ON c.id = a.courier_id WHERE c.id IS NULL LIMIT 10`
    );
    const total = missingLocation.rows.length + missingCourier.rows.length;
    return {
      checkId: 'F1', status: total === 0 ? 'PASS' : 'DRIFT',
      detail: `Orders w/o location: ${missingLocation.rows.length}, Assignments w/o courier: ${missingCourier.rows.length}`,
      driftCount: total,
    };
  }

  // ── T1: cancellation rate anomaly ──
  private async checkTrendAnomaly(): Promise<ReconCheckResult> {
    const res = await this.pool.query(`
      WITH daily AS (
        SELECT date_trunc('day', created_at) AS day,
               count(*) FILTER (WHERE status = 'CANCELLED') AS cancelled,
               count(*) AS total
        FROM orders
        WHERE created_at > now() - interval '8 days'
        GROUP BY date_trunc('day', created_at)
        ORDER BY day DESC
      )
      SELECT
        (SELECT cancelled::float / NULLIF(total, 0) FROM daily WHERE day = date_trunc('day', now())) AS today_rate,
        (SELECT cancelled::float / NULLIF(total, 0) FROM daily WHERE day < date_trunc('day', now())) AS yesterday_rate,
        (SELECT cancelled::float / NULLIF(total, 0) FROM daily WHERE day >= date_trunc('day', now()) - interval '7 days') AS week_avg
    `);
    const row = res.rows[0];
    if (!row || row.today_rate == null || row.week_avg == null) {
      return { checkId: 'T1', status: 'INCONCLUSIVE', detail: 'Insufficient data for trend', driftCount: 0 };
    }
    const drift = row.today_rate > row.week_avg * 2 && row.today_rate > 0.05;
    return {
      checkId: 'T1', status: drift ? 'DRIFT' : 'PASS',
      detail: drift
        ? `Cancellation rate today ${(row.today_rate * 100).toFixed(1)}% vs week avg ${(row.week_avg * 100).toFixed(1)}% (threshold: 2×)`
        : `Cancellation rate ${(row.today_rate * 100).toFixed(1)}% (week avg ${(row.week_avg * 100).toFixed(1)}%)`,
      driftCount: drift ? 1 : 0,
    };
  }
}
