import type { Pool } from 'pg';
import type Boss from 'pg-boss';
import type { MessageBus } from '@deliveryos/platform';
import { AnonymizerService } from '../lib/anonymizer/index.js';

export class GdprErasureWorker {
  constructor(
    private pool: Pool,
    private boss: Boss,
    private messageBus: MessageBus,
    private anonymizerService: AnonymizerService,
  ) {}

  async start() {
    await this.boss.work('anonymizer.gdpr', { singletonKey: 'anonymizer.gdpr' }, async (job: any) => {
      await this.run(job);
    });
  }

  private async run(job?: any) {
    const client = await this.pool.connect();
    try {
      await client.query('BEGIN');
      const res = await client.query(`
        SELECT id, location_id, customer_id, subject_phone, metadata
        FROM gdpr_erasure_requests
        WHERE status = 'pending'
        ORDER BY requested_at ASC
        LIMIT 10
        FOR UPDATE SKIP LOCKED
      `);
      await client.query('COMMIT');

      for (const row of res.rows) {
        try {
          await client.query(
            `UPDATE gdpr_erasure_requests SET status = 'in_progress' WHERE id = $1`,
            [row.id],
          );

          let customerId: string | null = row.customer_id;
          if (!customerId && row.subject_phone) {
            const custRes = await this.pool.query(
              `SELECT id FROM customers WHERE location_id = $1 AND phone = $2`,
              [row.location_id, row.subject_phone],
            );
            if (custRes.rows.length > 0) {
              customerId = custRes.rows[0].id;
            }
          }

          if (!customerId) {
            await client.query(
              `UPDATE gdpr_erasure_requests SET status = 'failed', error_message = 'Customer not found' WHERE id = $1`,
              [row.id],
            );
            continue;
          }

          const result = await this.anonymizerService.anonymize({
            scope: 'gdpr',
            subject: { customerId, locationId: row.location_id },
          });

          await client.query(
            `UPDATE gdpr_erasure_requests
             SET status = 'completed', completed_at = now(), metadata = $1
             WHERE id = $2`,
            [JSON.stringify(result), row.id],
          );

          await client.query(
            `INSERT INTO anonymization_audit_log (scope, subject_kind, subject_id, location_id, actor_kind, actor_id, metadata)
             VALUES ($1, $2, $3, $4, $5, $6, $7)`,
            ['gdpr', 'customer', customerId, row.location_id, 'system', null, JSON.stringify({ requestId: row.id, ...result })],
          );

          await this.messageBus.publish(`location:${row.location_id}:dashboard`, {
            type: 'gdpr.erasure_completed',
            data: { requestId: row.id, customerId, ...result },
          });
        } catch (err: any) {
          console.error(`[GdprErasureWorker] Request ${row.id} failed:`, err);

          const meta = typeof row.metadata === 'string' ? JSON.parse(row.metadata) : (row.metadata || {});
          const retryCount = (meta.retryCount || 0) + 1;

          if (retryCount < 3) {
            const backoff = Math.pow(2, retryCount) * 60;
            await client.query(
              `UPDATE gdpr_erasure_requests
               SET metadata = $1
               WHERE id = $2`,
              [JSON.stringify({ ...meta, retryCount, lastError: 'Processing error' }), row.id],
            );
            await this.boss.send('anonymizer.gdpr', { requestId: row.id, retryCount }, { startAfter: backoff });
          } else {
            await client.query(
              `UPDATE gdpr_erasure_requests
               SET status = 'failed', error_message = 'Max retries exceeded'
               WHERE id = $1`,
              [row.id],
            );
          }
        }
      }
    } catch (err) {
      console.error('[GdprErasureWorker] Error:', err);
      await this.messageBus.publish('anonymizer.gdpr.failed', { error: String(err), time: new Date().toISOString() });
    } finally {
      client.release();
    }
  }
}
