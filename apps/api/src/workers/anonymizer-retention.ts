// @ts-nocheck
import type { Pool } from 'pg';
import type Boss from 'pg-boss';
import type { MessageBus } from '@deliveryos/platform';
import { loadEnv } from '@deliveryos/config';
import { AnonymizerService } from '../lib/anonymizer/index.js';

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
    await this.boss.work('anonymizer.retention', { singletonKey: 'anonymizer.retention' }, async () => {
      await this.run();
    });
    const cron = env.ANONYMIZER_RETENTION_CRON || '0 3 * * *';
    await this.boss.createQueue('anonymizer.retention');
    await this.boss.schedule('anonymizer.retention', cron, null, { singletonKey: 'anonymizer.retention' });
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
      await this.messageBus.publish('worker.failed', { error: String(err), time: new Date().toISOString() });
    } finally {
      client.release();
    }
  }
}
