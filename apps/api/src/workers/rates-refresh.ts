import type { Pool } from 'pg';
import type { PgBoss } from 'pg-boss';
import { loadEnv } from '@deliveryos/config';
import { QUEUE_NAMES } from '../lib/registry.js';

const env = loadEnv();
const RATES_API_URL = 'https://cdn.jsdelivr.net/npm/@fawazahmed0/currency-api@latest/v1/currencies/all.min.json';

export class RatesRefreshWorker {
  constructor(
    private pool: Pool,
    private boss: PgBoss,
  ) {}

  async start() {
    const cron = env.RATES_CRON || '0 * * * *';

    await this.boss.work(QUEUE_NAMES.RATES_REFRESH, async () => {
      await this.run();
    });

    await this.boss.send(QUEUE_NAMES.RATES_REFRESH, null, { startAfter: 5 });
    await this.boss.schedule(QUEUE_NAMES.RATES_REFRESH, cron, null);
  }

  private async run() {
    const client = await this.pool.connect();
    try {
      const lock = await client.query("SELECT pg_try_advisory_lock(8192) AS locked");
      if (!lock.rows[0]?.locked) {
        console.log('[RatesRefresh] Skipped — advisory lock held by another instance');
        return;
      }

      try {
        const res = await fetch(RATES_API_URL);
        if (!res.ok) {
          console.error(`[RatesRefresh] API returned ${res.status}`);
          return;
        }

        const data: Record<string, Record<string, number>> = await res.json();
        const allEur = data['eur'];
        if (!allEur) {
          console.error('[RatesRefresh] No EUR rates found in API response');
          return;
        }

        const allPerEur = allEur['all'];
        if (!allPerEur || allPerEur <= 0) {
          console.error('[RatesRefresh] Invalid ALL rate from API:', allPerEur);
          return;
        }

        const rate = 1 / allPerEur;

        await client.query(
          `INSERT INTO exchange_rates (base_currency, target_currency, rate, source, fetched_at)
           VALUES ('ALL', 'EUR', $1, 'fawazahmed0', now())
           ON CONFLICT (base_currency, target_currency) DO UPDATE
           SET rate = EXCLUDED.rate, source = EXCLUDED.source, fetched_at = EXCLUDED.fetched_at`,
          [rate.toFixed(8)],
        );

        console.log(`[RatesRefresh] ALL→EUR rate updated: ${rate.toFixed(8)}`);
      } finally {
        await client.query("SELECT pg_advisory_unlock(8192)");
      }
    } catch (err) {
      console.error('[RatesRefresh] Error:', err);
    } finally {
      client.release();
    }
  }
}
