import type { Pool } from 'pg';
import type Boss from 'pg-boss';
import type { MessageBus } from '@deliveryos/platform';
import { loadEnv } from '@deliveryos/config';
import { computeSignals } from '../lib/signals/compute.js';

const env = loadEnv();

export class SignalRaiserWorker {
  constructor(
    private pool: Pool,
    private boss: Boss,
    private messageBus: MessageBus,
  ) {}

  async start() {
    await this.boss.work('signal.raiser', { singletonKey: 'signal.raiser' }, async () => {
      await this.run();
    });
    const cron = env.SIGNAL_RAISE_CRON || '*/5 * * * *';
    await this.boss.createQueue('signal.raiser');
    await this.boss.schedule('signal.raiser', cron, null, { singletonKey: 'signal.raiser' });
  }

  private async run() {
    const client = await this.pool.connect();
    try {
      const lock = await client.query("SELECT pg_try_advisory_lock(3) AS locked");
      if (!lock.rows[0]?.locked) {
        console.log('[SignalRaiser] Skipped — advisory lock held');
        return;
      }

      try {
        // Find locations with activity in last 24h
        const locsRes = await client.query(`
          SELECT DISTINCT v.location_id,
                 v.phone_hash, v.client_ip_hash,
                 ve.customer_id
          FROM velocity_events v
          LEFT JOIN LATERAL (
            SELECT customer_id FROM orders
            WHERE location_id = v.location_id
              AND created_at > now() - interval '24 hours'
            LIMIT 1
          ) ve ON true
          WHERE v.window_started_at > now() - interval '24 hours'
        `);

        // Group by location_id for per-location signal computation
        const locationMap = new Map<string, { phoneHashes: Set<string>; ipHashes: Set<string>; customerIds: Set<string> }>();
        for (const row of locsRes.rows) {
          let entry = locationMap.get(row.location_id);
          if (!entry) {
            entry = { phoneHashes: new Set(), ipHashes: new Set(), customerIds: new Set() };
            locationMap.set(row.location_id, entry);
          }
          if (row.phone_hash) entry.phoneHashes.add(row.phone_hash);
          if (row.client_ip_hash) entry.ipHashes.add(row.client_ip_hash);
          if (row.customer_id) entry.customerIds.add(row.customer_id);
        }

        for (const [locationId, entry] of locationMap) {
          for (const phoneHash of entry.phoneHashes) {
            const signals = await computeSignals(this.pool, {
              locationId,
              phoneHash,
              ...(entry.customerIds.size > 0 ? { customerId: entry.customerIds.values().next().value } : {}),
            });
            await this.persistSignals(client, locationId, null, signals);
          }

          for (const ipHash of entry.ipHashes) {
            const signals = await computeSignals(this.pool, {
              locationId,
              clientIpHash: ipHash,
              ...(entry.customerIds.size > 0 ? { customerId: entry.customerIds.values().next().value } : {}),
            });
            await this.persistSignals(client, locationId, null, signals);
          }

          // Compute no-show signals for known customers
          if (entry.customerIds.size > 0) {
            for (const customerId of entry.customerIds) {
              const signals = await computeSignals(this.pool, {
                locationId,
                customerId,
              });
              await this.persistSignals(client, locationId, customerId, signals);
            }
          }
        }
      } finally {
        await client.query("SELECT pg_advisory_unlock(3)");
      }
    } catch (err) {
      console.error('[SignalRaiser] Error:', err);
    } finally {
      client.release();
    }
  }

  private async persistSignals(client: any, locationId: string, customerId: string | null, signals: any[]) {
    for (const sig of signals) {
      if (!customerId) continue; // need customer_id to persist

      // De-dup: skip if same kind raised within 1h
      const existingRes = await client.query(
        `SELECT id FROM customer_signals
         WHERE customer_id = $1 AND kind = $2
           AND raised_at > now() - interval '1 hour'`,
        [customerId, sig.kind],
      );
      if (existingRes.rowCount > 0) continue;

      const res = await client.query(
        `INSERT INTO customer_signals (customer_id, location_id, kind, severity, evidence)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id`,
        [customerId, locationId, sig.kind, sig.severity, JSON.stringify(sig.evidence)],
      );
      const signalId = res.rows[0]?.id;
      if (signalId) {
        await this.messageBus.publish(`location:${locationId}:dashboard`, {
          type: 'preflight.signal_raised',
          data: { signalId, customerId, kind: sig.kind, severity: sig.severity },
        });
      }
    }
  }
}
