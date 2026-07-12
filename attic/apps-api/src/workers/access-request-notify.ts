// @ts-nocheck
import type { Pool } from 'pg';
import type Boss from 'pg-boss';
import type { MessageBus } from '@deliveryos/platform';
import { loadEnv } from '@deliveryos/config';
import { QUEUE_NAMES } from '../lib/registry.js';
import { EmailAdapter } from '../notifications/adapters/email.js';

const env = loadEnv();

/**
 * AccessRequestNotifyWorker (ADR-soft-access-gate, B6/B8) — drains
 * `access-request.notify` ({ requestId } only — zero PII in the queue) and sends ONE
 * best-effort operator email per new request via a direct EmailAdapter.sendOps (NOT the
 * tenant dispatcher: an access request has no locationId).
 *
 * Idempotency = claim-before-send CAS (B8): exactly one delivery wins the claim; the
 * loser sends nothing; an erased row (0 rows) is acked WITHOUT throwing. A failed send
 * rolls notified_at back to NULL, bumps notify_attempts (R2-9 bound), and throws so
 * pg-boss retries; the reconcile cron re-feeds while notify_attempts < cap.
 */
export class AccessRequestNotifyWorker {
  private email: EmailAdapter;

  constructor(
    private pool: Pool,
    private boss: Boss,
    private messageBus: MessageBus,
    email?: EmailAdapter, // injectable for tests; defaults to the env-keyed Resend adapter
  ) {
    this.email = email ?? new EmailAdapter(env.RESEND_API_KEY);
  }

  async start() {
    await this.boss.createQueue(QUEUE_NAMES.ACCESS_REQUEST_NOTIFY);
    await this.boss.work(QUEUE_NAMES.ACCESS_REQUEST_NOTIFY, async (job: any) => {
      // pg-boss v10 may hand a single job or a batch; normalize.
      const jobs = Array.isArray(job) ? job : [job];
      for (const j of jobs) {
        await this.handle(j?.data?.requestId);
      }
    });
  }

  private async handle(requestId?: string) {
    if (!requestId) return; // malformed payload — nothing to do, ack
    const to = env.WAITLIST_NOTIFY_EMAIL;
    const client = await this.pool.connect();
    try {
      // CLAIM: atomically flip notified_at; only the winner proceeds. The claim-check
      // re-fetch (email) happens HERE — email never travels in the queue payload.
      const claim = await client.query(
        `UPDATE access_requests
            SET notified_at = now()
          WHERE id = $1 AND notified_at IS NULL
        RETURNING email, locale`,
        [requestId],
      );

      // 0 rows → already claimed/sent by a prior delivery, OR the row was erased between
      // enqueue and pickup (B8.1). Either way: ack, send nothing, never throw.
      if (claim.rowCount === 0) return;

      const { email } = claim.rows[0];

      // No operator inbox configured → nothing to send, but the claim stands (the row is
      // marked notified to avoid a reconcile treadmill; status='new' is the ops fallback).
      if (!to) {
        console.warn('[AccessRequestNotify] WAITLIST_NOTIFY_EMAIL unset — skipping send');
        return;
      }

      const result = await this.email.sendOps({
        to,
        subject: 'New access request — dowiz',
        text: `A new access request was submitted: ${email}\n\nReview with: SELECT * FROM access_requests WHERE status='new' ORDER BY created_at DESC;`,
      });

      if (result.delivered) {
        console.log(`[AccessRequestNotify] Notified operator for request ${requestId}`);
        return; // notified_at stays set → idempotent on re-delivery
      }

      // Email disabled (no key) → leave it claimed; the row is visible via status='new'.
      if (result.reason === 'email-disabled') {
        console.warn('[AccessRequestNotify] Email disabled (no RESEND_API_KEY) — request persisted, no push');
        return;
      }

      // Genuine send failure → release the claim, bump the attempt counter, and throw so
      // pg-boss retries (the reconcile cron also re-feeds while notify_attempts < cap).
      await client.query(
        `UPDATE access_requests
            SET notified_at = NULL, notify_attempts = notify_attempts + 1
          WHERE id = $1`,
        [requestId],
      );
      throw new Error(`access-request notify send failed: ${result.reason}`);
    } finally {
      client.release();
    }
  }
}
