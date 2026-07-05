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

          // N1 fail-loud backstop (resolution-r2.md §1 N1.2 / §2): NEVER write `completed` without a
          // data-level confirmation that the erasure actually took effect. Under NOBYPASSRLS+MIG-2 a
          // context-free erasure silently sees ∅ and no-ops; keying the terminal write off a re-read
          // converts that silent false Art.17 completion into a loud `failed` + FAILED signal.
          // (Structural post-flip *success* — the DEFINER gdpr_erase_customer — rides LC4-MIG /
          // GATE-FLIP-E2E; this backstop makes the interim safe.) The re-read also credits the
          // idempotent already-anonymized case (skipped, but goal-state reached → success).
          //
          // REV-S9-3 (S9 council, resolution.md): this gate used to re-read ONLY
          // customers.anonymized_at — the GAP-A order fan-out (AnonymizerService.anonymize,
          // REV-S9-1) can under-erase (a swallowed per-order failure, or — post-flip — an RLS
          // no-op) yet the worker would still write `completed`. Extend the gate to the WHOLE
          // subject-graph: the customer row itself, every one of the subject's orders, and every
          // rating tied to those orders. `completed` fires ONLY when all three are erased;
          // otherwise `failed`, never a false Art.17 completion.
          //
          // S9 REV-S9-3: post-flip needs an orders erasure RLS arm (migration) — under
          // NOBYPASSRLS this re-read itself needs a context that can see orders/order_ratings
          // cross-subject; out of scope here (see resolution.md REV-S9-2/3).
          const confirm = await client.query(
            `SELECT
               c.anonymized_at AS customer_anonymized_at,
               (SELECT count(*)::int FROM orders o
                 WHERE o.customer_id = c.id AND o.location_id = c.location_id AND o.anonymized_at IS NULL) AS orders_remaining,
               (SELECT count(*)::int FROM order_ratings r
                 JOIN orders o2 ON o2.id = r.order_id
                 WHERE o2.customer_id = c.id AND o2.location_id = c.location_id AND r.feedback IS NOT NULL) AS ratings_remaining
             FROM customers c
             WHERE c.id = $1 AND c.location_id = $2`,
            [customerId, row.location_id],
          );
          const confirmRow = confirm.rows[0];
          const erasureConfirmed = !!confirmRow
            && confirmRow.customer_anonymized_at != null
            && Number(confirmRow.orders_remaining) === 0
            && Number(confirmRow.ratings_remaining) === 0;
          if (!erasureConfirmed) {
            await client.query(
              `UPDATE gdpr_erasure_requests
               SET status = 'failed', error_message = 'erasure incomplete (subject-graph not fully anonymized: customer/orders/ratings)'
               WHERE id = $1`,
              [row.id],
            );
            await this.messageBus.publish(BUS_CHANNELS.ANONYMIZER_GDPR_FAILED, {
              requestId: row.id,
              customerId,
              locationId: row.location_id,
              reason: 'no-effect',
              time: new Date().toISOString(),
            });
            continue;
          }

          // REV-S9-5 (S9 council, resolution.md): gdpr_erasure_requests.subject_phone is
          // plaintext and otherwise erased by NO path — the erasure record would itself become
          // a permanent PII record. Null it in the SAME UPDATE that marks completion.
          await client.query(
            `UPDATE gdpr_erasure_requests
             SET status = 'completed', completed_at = now(), metadata = $1, subject_phone = NULL
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
            // LC4 (reliability C4): reset to `pending`, NOT left `in_progress`. The retry scan
            // (run(): SELECT ... WHERE status='pending') only re-selects pending rows; leaving the
            // row `in_progress` strands the legally-mandated erasure forever — it never re-runs and
            // never reaches `failed`. Resetting to pending guarantees the next scan re-selects it.
            await client.query(
              `UPDATE gdpr_erasure_requests
               SET status = 'pending', metadata = $1
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
