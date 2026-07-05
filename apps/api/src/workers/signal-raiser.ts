// @ts-nocheck
import type { Pool } from 'pg';
import type Boss from 'pg-boss';
import type { MessageBus } from '@deliveryos/platform';
import { loadEnv } from '@deliveryos/config';
import { BUS_CHANNELS, QUEUE_NAMES, orderChannel, dashboardChannel, courierChannel, shiftChannel } from '../lib/registry.js';
import { computeSignals } from '../lib/signals/compute.js';

const env = loadEnv();

export class SignalRaiserWorker {
  constructor(
    private pool: Pool,
    private boss: Boss,
    private messageBus: MessageBus,
  ) {}

  async start() {
    await this.boss.work(QUEUE_NAMES.SIGNAL_RAISER, { singletonKey: QUEUE_NAMES.SIGNAL_RAISER }, async () => {
      await this.run();
    });
    const cron = env.SIGNAL_RAISE_CRON || '*/5 * * * *';
    await this.boss.createQueue(QUEUE_NAMES.SIGNAL_RAISER);
    await this.boss.schedule(QUEUE_NAMES.SIGNAL_RAISER, cron, null, { singletonKey: QUEUE_NAMES.SIGNAL_RAISER });
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
        // Find locations with activity in last 24h.
        // B3/NOBYPASSRLS: velocity_events is FORCE-RLS keyed on app_member_location_ids() (member-scoped,
        // no anon-SELECT policy) — a context-free cross-tenant scan returns 0 rows after the flip. The
        // whole all-tenant read goes through a SECURITY DEFINER fn (owner BYPASSRLS) mirroring the SQL
        // exactly. See app_sweep_velocity_active_locations().
        const locsRes = await client.query(`SELECT * FROM app_sweep_velocity_active_locations()`);

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

      // B3/NOBYPASSRLS: customer_signals is FORCE-RLS keyed on app_member_location_ids() (member-scoped) —
      // this system worker has no member identity, so neither app.current_tenant nor app.user_id satisfies
      // the policy. The de-dup read + insert are folded into a single SECURITY DEFINER fn (owner BYPASSRLS)
      // that mirrors the SQL exactly: skip (return NULL) if the same kind was raised within 1h, else INSERT
      // RETURNING id. See app_raise_customer_signal().
      const res = await client.query(
        `SELECT app_raise_customer_signal($1, $2, $3, $4, $5::jsonb) AS id`,
        [customerId, locationId, sig.kind, sig.severity, JSON.stringify(sig.evidence)],
      );
      const signalId = res.rows[0]?.id;
      if (signalId) {
        await this.messageBus.publish(dashboardChannel(locationId), {
          type: 'preflight.signal_raised',
          data: { signalId, customerId, kind: sig.kind, severity: sig.severity },
        });
      }
    }
  }
}
