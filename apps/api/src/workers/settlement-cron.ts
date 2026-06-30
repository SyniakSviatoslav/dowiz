// @ts-nocheck
import type { Pool } from 'pg';
import type Boss from 'pg-boss';
import { BUS_CHANNELS, QUEUE_NAMES, orderChannel, dashboardChannel, courierChannel, shiftChannel } from '../lib/registry.js';
import { loadEnv } from '@deliveryos/config';
import { getSettlementPeriodBoundaries } from '../lib/settlement-period.js';

const env = loadEnv();

export class SettlementCronWorker {
  constructor(
    private pool: Pool,
    private boss: Boss
  ) {}

  async start() {
    // 1. settlement.generate (daily at 2 AM UTC)
    await this.boss.work(QUEUE_NAMES.SETTLEMENT_GENERATE, async (job) => {
      const data = job.data as any;
      await this.handleGenerate(data?.referenceDate ? new Date(data.referenceDate) : new Date());
    });
    
    // Register cron
    const cronExpr = env.SETTLEMENT_CRON || '0 2 * * *';
    await this.boss.createQueue(QUEUE_NAMES.SETTLEMENT_GENERATE);
    await this.boss.schedule(QUEUE_NAMES.SETTLEMENT_GENERATE, cronExpr, null, { singletonKey: QUEUE_NAMES.SETTLEMENT_GENERATE });
  }

  async handleGenerate(referenceDate: Date) {
    const periodType = (env.SETTLEMENT_PERIOD || 'daily') as 'daily' | 'weekly';
    const { periodStart, periodEnd } = getSettlementPeriodBoundaries(referenceDate, periodType);

    // B3/NOBYPASSRLS: this is a cross-tenant money/audit sweep (red-line). It reads/writes
    // courier_assignments, courier_payouts, settlement_items and settlement_audit_log — all FORCE-RLS
    // keyed on app.current_tenant. settlement_items/settlement_audit_log policies use bare
    // current_setting('app.current_tenant')::uuid (no missing-ok NULLIF), so a context-free write does not
    // just see 0 rows — it ERRORS once the role stops bypassing. The entire generation transaction is
    // therefore encapsulated in a single SECURITY DEFINER fn (owner BYPASSRLS), which mirrors the prior SQL
    // EXACTLY: identical pair discovery, payout upsert, FOR UPDATE SKIP LOCKED item selection, per-item
    // INSERT/UPDATE, payout-total bump and audit row. Amounts/logic are unchanged — only the RLS-context
    // mechanism moves. A function call is atomic (its own transaction), so an error aborts the whole sweep
    // = the previous BEGIN/ROLLBACK semantics. See app_generate_settlements().
    try {
      await this.pool.query(`SELECT app_generate_settlements($1, $2)`, [periodStart, periodEnd]);
    } catch (err) {
      console.error('Failed to generate settlements:', err);
      throw err;
    }
  }
}
