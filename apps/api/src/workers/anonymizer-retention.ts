// @ts-nocheck
import type { Pool } from 'pg';
import type Boss from 'pg-boss';
import type { MessageBus } from '@deliveryos/platform';
import { loadEnv } from '@deliveryos/config';
import { AnonymizerService } from '../lib/anonymizer/index.js';
import { BUS_CHANNELS, QUEUE_NAMES } from '../lib/registry.js';

const env = loadEnv();

const BATCH_SIZE = 100;

export class AnonymizerRetentionWorker {
  constructor(
    private pool: Pool,
    private boss: Boss,
    private messageBus: MessageBus,
    private anonymizerService: AnonymizerService,
  ) {}

  async start() {
    await this.boss.work(QUEUE_NAMES.ANONYMIZER_RETENTION, { singletonKey: QUEUE_NAMES.ANONYMIZER_RETENTION }, async () => {
      await this.run();
    });
    const cron = env.ANONYMIZER_RETENTION_CRON || '0 3 * * *';
    await this.boss.createQueue(QUEUE_NAMES.ANONYMIZER_RETENTION);
    await this.boss.schedule(QUEUE_NAMES.ANONYMIZER_RETENTION, cron, null, { singletonKey: QUEUE_NAMES.ANONYMIZER_RETENTION });
  }

  private async run() {
    const client = await this.pool.connect();
    try {
      const lock = await client.query("SELECT pg_try_advisory_lock(4) AS locked");
      if (!lock.rows[0]?.locked) {
        console.log('[AnonymizerRetention] Skipped — advisory lock held by another instance');
        return;
      }

      try {
        // Housekeeping: drop expired customer track grants (the ?t= tracking-link
        // codes). Expired grants are already inert at exchange time (WHERE
        // expires_at > now()); this just keeps the table small. Lives here rather
        // than its own pg-boss queue because the app role lacks runtime DDL on the
        // pgboss schema (migration 009), so it can't create a new queue.
        const grantPurge = await client.query(
          `DELETE FROM customer_track_grants WHERE expires_at < now()`,
        );
        console.log(`[AnonymizerRetention] Purged ${grantPurge.rowCount} expired track grants`);

        // SENSOR-BUS §1.3: funnel_events 90-day retention sweep (ADR-0009). The funnel is the
        // heaviest writer; bounded-batch DELETEs (LIMIT loop) keep it from lock-contending with live
        // ingest (Breaker H4). Anonymous + advisory, so a straight age-based purge — no per-tenant
        // policy. Best-effort: a sweep failure never blocks the rest of retention.
        try {
          let funnelPurged = 0;
          for (let i = 0; i < 100; i++) {
            const del = await client.query(
              `DELETE FROM funnel_events
                WHERE id IN (SELECT id FROM funnel_events WHERE created_at < now() - interval '90 days' LIMIT $1)`,
              [BATCH_SIZE],
            );
            funnelPurged += del.rowCount ?? 0;
            if ((del.rowCount ?? 0) < BATCH_SIZE) break;
          }
          if (funnelPurged > 0) console.log(`[AnonymizerRetention] Purged ${funnelPurged} funnel_events older than 90d`);
        } catch (err) {
          console.error('[AnonymizerRetention] funnel_events sweep failed (non-fatal):', err);
        }

        const locationsRes = await client.query(`
          SELECT id, name, retention_days FROM locations
        `);

        if (locationsRes.rows.length === 0) {
          console.log('[AnonymizerRetention] No locations found');
          return;
        }

        for (const loc of locationsRes.rows) {
          const result = await this.anonymizerService.anonymize({
            scope: 'retention',
            locationId: loc.id,
            batchSize: BATCH_SIZE,
            actorId: undefined,
          });
          console.log(
            `[AnonymizerRetention] Location ${loc.name} (${loc.id}): ` +
            `${result.customersAnonymized} customers, ${result.ordersAnonymized} orders anonymized, ` +
            `${result.storagePurged} storage purged, ${result.skipped} skipped`,
          );
        }
      } finally {
        await client.query("SELECT pg_advisory_unlock(4)");
      }
    } catch (err) {
      console.error('[AnonymizerRetention] Error:', err);
      await this.messageBus.publish(BUS_CHANNELS.WORKER_FAILED, { error: String(err), time: new Date().toISOString() });
    } finally {
      client.release();
    }
  }
}
