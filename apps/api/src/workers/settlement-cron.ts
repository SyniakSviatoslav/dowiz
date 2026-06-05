import type { Pool } from 'pg';
import type Boss from 'pg-boss';
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
    await this.boss.work('settlement.generate', async (job) => {
      const data = job.data as any;
      await this.handleGenerate(data?.referenceDate ? new Date(data.referenceDate) : new Date());
    });
    
    // Register cron
    const cronExpr = env.SETTLEMENT_CRON || '0 2 * * *';
    await this.boss.createQueue('settlement.generate');
    await this.boss.schedule('settlement.generate', cronExpr, null, { singletonKey: 'settlement.generate' });
  }

  async handleGenerate(referenceDate: Date) {
    const periodType = (env.SETTLEMENT_PERIOD || 'daily') as 'daily' | 'weekly';
    const { periodStart, periodEnd } = getSettlementPeriodBoundaries(referenceDate, periodType);
    
    const client = await this.pool.connect();
    try {
      await client.query('BEGIN');

      // 1. Find all active (courier, location) pairs that had delivered assignments in this period
      const pairsRes = await client.query(`
        SELECT DISTINCT courier_id, location_id
        FROM courier_assignments
        WHERE status = 'delivered'
          AND cash_collected = true
          AND delivered_at >= $1
          AND delivered_at < $2
      `, [periodStart, periodEnd]);

      for (const pair of pairsRes.rows) {
        // 2. Upsert payout
        const payoutRes = await client.query(`
          INSERT INTO courier_payouts (courier_id, location_id, period_start, period_end, status)
          VALUES ($1, $2, $3, $4, 'pending')
          ON CONFLICT (courier_id, location_id, period_start, period_end) DO UPDATE
            SET status = courier_payouts.status -- no-op but returns the row
          RETURNING id, status
        `, [pair.courier_id, pair.location_id, periodStart, periodEnd]);

        const payout = payoutRes.rows[0];

        // 3. Find missing assignments (using FOR UPDATE SKIP LOCKED to prevent races)
        const assignmentsRes = await client.query(`
          SELECT ca.id, ca.cash_amount, loc.currency_code
          FROM courier_assignments ca
          JOIN locations loc ON loc.id = ca.location_id
          WHERE ca.courier_id = $1
            AND ca.location_id = $2
            AND ca.status = 'delivered'
            AND ca.cash_collected = true
            AND ca.delivered_at >= $3
            AND ca.delivered_at < $4
            AND NOT EXISTS (
              SELECT 1 FROM settlement_items si WHERE si.assignment_id = ca.id
            )
          FOR UPDATE OF ca SKIP LOCKED
        `, [pair.courier_id, pair.location_id, periodStart, periodEnd]);

        let addedItems = 0;
        let addedTotal = 0;

        for (const ca of assignmentsRes.rows) {
          // 4. Insert settlement item
          await client.query(`
            INSERT INTO settlement_items (payout_id, assignment_id, location_id, amount, currency_code)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (assignment_id) DO NOTHING
          `, [payout.id, ca.id, pair.location_id, ca.cash_amount, ca.currency_code]);

          // Update the courier_assignment with the linked item id
          await client.query(`
            UPDATE courier_assignments 
            SET settlement_item_id = (SELECT id FROM settlement_items WHERE assignment_id = $1)
            WHERE id = $1
          `, [ca.id]);

          addedItems++;
          addedTotal += ca.cash_amount;
        }

        if (addedItems > 0) {
          // 5. Update payout totals
          await client.query(`
            UPDATE courier_payouts
            SET deliveries_count = deliveries_count + $1,
                total_earned = total_earned + $2
            WHERE id = $3
          `, [addedItems, addedTotal, payout.id]);

          // 6. Audit log
          await client.query(`
            INSERT INTO settlement_audit_log (payout_id, location_id, action, actor_kind, metadata)
            VALUES ($1, $2, 'generated', 'system', $3)
          `, [
            payout.id,
            pair.location_id,
            JSON.stringify({ added_items: addedItems, added_total: addedTotal })
          ]);
        }
      }

      await client.query('COMMIT');
    } catch (err) {
      await client.query('ROLLBACK');
      console.error('Failed to generate settlements:', err);
      throw err;
    } finally {
      client.release();
    }
  }
}
