// @ts-nocheck
import { Pool } from 'pg';
import type Boss from 'pg-boss';
import type { MessageBus } from '@deliveryos/platform';
import { BUS_CHANNELS, QUEUE_NAMES, orderChannel, dashboardChannel, courierChannel, shiftChannel } from '../lib/registry.js';
import { loadEnv } from '@deliveryos/config';
import { COURIER_POSITION_RETENTION_INTERVAL } from '../lib/courier-gps.js';

const env = loadEnv();

export class CourierCronWorker {
  constructor(
    private pool: Pool,
    private boss: Boss,
    private messageBus: MessageBus
  ) {}

  async start() {
    // 1. gps.purge (daily at 3 AM) — singletonKey prevents double-execution on N=2
    await this.boss.work(QUEUE_NAMES.GPS_PURGE, { singletonKey: QUEUE_NAMES.GPS_PURGE }, async () => this.handleGpsPurge());
    await this.boss.createQueue(QUEUE_NAMES.GPS_PURGE);
    await this.boss.schedule(QUEUE_NAMES.GPS_PURGE, '0 3 * * *', null, { singletonKey: QUEUE_NAMES.GPS_PURGE });

    // 2. courier.stale_check (every 2 minutes) — singletonKey prevents double-execution on N=2
    await this.boss.work(QUEUE_NAMES.COURIER_STALE_CHECK, { singletonKey: QUEUE_NAMES.COURIER_STALE_CHECK }, async () => this.handleStaleCheck());
    await this.boss.createQueue(QUEUE_NAMES.COURIER_STALE_CHECK);
    await this.boss.schedule(QUEUE_NAMES.COURIER_STALE_CHECK, '*/2 * * * *', null, { singletonKey: QUEUE_NAMES.COURIER_STALE_CHECK });
  }

  async handleGpsPurge() {
    const client = await this.pool.connect();
    try {
      // P0-1: retention window is a named constant (COURIER_POSITION_RETENTION_INTERVAL).
      // B3: this is a cross-tenant sweep — it deletes stale positions across ALL tenants in
      // one pass, so there is no single app.current_tenant to set. Encapsulated in a
      // SECURITY DEFINER fn (app_sweep_gps_purge) that mirrors the original DELETE and runs
      // above RLS; worker just invokes it. Identical behavior under today's BYPASSRLS.
      await client.query(`SELECT app_sweep_gps_purge($1::interval)`, [COURIER_POSITION_RETENTION_INTERVAL]);
    } catch (err) {
      console.error('Failed to purge GPS:', err);
      throw err;
    } finally {
      client.release();
    }
  }

  async handleStaleCheck() {
    const staleMs = parseInt(env.COURIER_STALE_HEARTBEAT_MS || '300000', 10); // 5 minutes default
    
    const client = await this.pool.connect();
    try {
      // B3: cross-tenant sweep — scans on_delivery shifts across ALL tenants AND writes
      // location_alerts, which is membership-keyed (app_member_location_ids()) and so cannot
      // be satisfied by app.current_tenant from a system worker (no membership). Both the
      // cross-tenant scan (FOR UPDATE SKIP LOCKED) and the alert INSERT (ON CONFLICT DO NOTHING)
      // are encapsulated in a single SECURITY DEFINER fn (app_sweep_stale_couriers) that mirrors
      // the original SQL and runs above RLS. The fn returns every stale row so the worker still
      // publishes one COURIER_STALE_HEARTBEAT per row (identical to the prior loop). Atomic single
      // statement → locks held across the insert, same as the old BEGIN/COMMIT shape.
      const res = await client.query(`SELECT shift_id, courier_id, order_id, location_id FROM app_sweep_stale_couriers($1::interval)`, [`${staleMs} milliseconds`]);

      for (const row of res.rows) {
        // Publish event to trigger notification workflow
        await this.messageBus.publish(BUS_CHANNELS.COURIER_STALE_HEARTBEAT, {
          orderId: row.order_id,
          locationId: row.location_id,
          courierId: row.courier_id
        });
      }
    } catch (err) {
      console.error('Failed to check stale couriers:', err);
      throw err;
    } finally {
      client.release();
    }
  }
}
