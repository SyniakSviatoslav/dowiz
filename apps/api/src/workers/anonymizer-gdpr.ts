// @ts-nocheck
import type { Pool } from 'pg';
import type Boss from 'pg-boss';
import type { MessageBus } from '@deliveryos/platform';
import { AnonymizerService } from '../lib/anonymizer/index.js';
import { BUS_CHANNELS, QUEUE_NAMES, dashboardChannel } from '../lib/registry.js';

export class GdprErasureWorker {
  constructor(
    private pool: Pool,
    private boss: Boss,
    private messageBus: MessageBus,
    private anonymizerService: AnonymizerService,
  ) {}

  async start() {
    await this.boss.work(QUEUE_NAMES.ANONYMIZER_GDPR, { singletonKey: QUEUE_NAMES.ANONYMIZER_GDPR }, async (job: any) => {
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

          // R2-5: stamp the SAME subject-true-tenant + provenance fields as the anonymizer's own
          // audit insert (lib/anonymizer/index.ts insertAuditLog) so the two audit rows for one
          // erasure never disagree on tenant. Read the customer's actual location_id back from the
          // DB rather than trusting the request row's location_id for the audit stamp — the request
          // row's tenant is the ACTOR's (gdpr.ts's entry gate already proved actor==subject before
          // this row could exist, but the audit trail should assert it independently, not inherit it).
          const subjectLocRes = await client.query(
            `SELECT location_id FROM customers WHERE id = $1`,
            [customerId],
          );
          const subjectLocationId = subjectLocRes.rows[0]?.location_id ?? row.location_id;

          await client.query(
            `INSERT INTO anonymization_audit_log (scope, subject_kind, subject_id, location_id, actor_kind, actor_id, metadata)
             VALUES ($1, $2, $3, $4, $5, $6, $7)`,
            [
              'gdpr',
              'customer',
              customerId,
              subjectLocationId,
              'system',
              null,
              JSON.stringify({
                actor_location_id: row.location_id,
                subject_location_id: subjectLocationId,
                request_id: row.id,
                ...result,
              }),
            ],
          );

          await this.messageBus.publish(dashboardChannel(row.location_id), {
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
            await this.boss.send(QUEUE_NAMES.ANONYMIZER_GDPR, { requestId: row.id, retryCount }, { startAfter: backoff });
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
      await this.messageBus.publish(BUS_CHANNELS.ANONYMIZER_GDPR_FAILED, { error: String(err), time: new Date().toISOString() });
    } finally {
      client.release();
    }
  }
}
