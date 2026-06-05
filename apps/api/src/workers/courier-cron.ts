// @ts-nocheck
import { Pool } from 'pg';
import type Boss from 'pg-boss';
import type { MessageBus } from '@deliveryos/platform';
import { loadEnv } from '@deliveryos/config';

const env = loadEnv();

export class CourierCronWorker {
  constructor(
    private pool: Pool,
    private boss: Boss,
    private messageBus: MessageBus
  ) {}

  async start() {
    // 1. gps.purge (daily at 3 AM) — singletonKey prevents double-execution on N=2
    await this.boss.work('gps.purge', { singletonKey: 'gps.purge' }, async () => this.handleGpsPurge());
    await this.boss.createQueue('gps.purge');
    await this.boss.schedule('gps.purge', '0 3 * * *', null, { singletonKey: 'gps.purge' });

    // 2. courier.stale_check (every 2 minutes) — singletonKey prevents double-execution on N=2
    await this.boss.work('courier.stale_check', { singletonKey: 'courier.stale_check' }, async () => this.handleStaleCheck());
    await this.boss.createQueue('courier.stale_check');
    await this.boss.schedule('courier.stale_check', '*/2 * * * *', null, { singletonKey: 'courier.stale_check' });
  }

  async handleGpsPurge() {
    const client = await this.pool.connect();
    try {
      await client.query(`DELETE FROM courier_positions WHERE recorded_at < now() - interval '24 hours'`);
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
      await client.query('BEGIN');
      
      const res = await client.query(`
        SELECT cs.id as shift_id, cs.courier_id, ca.order_id, ca.location_id
        FROM courier_shifts cs
        JOIN courier_assignments ca ON cs.id = ca.shift_id AND ca.status IN ('assigned', 'accepted', 'picked_up')
        WHERE cs.status = 'on_delivery' 
          AND cs.last_heartbeat_at < now() - $1::interval
        FOR UPDATE SKIP LOCKED
      `, [`${staleMs} milliseconds`]);

      for (const row of res.rows) {
        // Create an alert
        await client.query(`
          INSERT INTO location_alerts (location_id, kind, target_id, message)
          VALUES ($1, 'courier_offline', $2, 'Courier went offline during delivery.')
        `, [row.location_id, row.order_id]);

        // Publish event to trigger notification workflow
        await this.messageBus.publish('courier.stale_heartbeat', {
          orderId: row.order_id,
          locationId: row.location_id,
          courierId: row.courier_id
        });
      }

      await client.query('COMMIT');
    } catch (err) {
      await client.query('ROLLBACK');
      console.error('Failed to check stale couriers:', err);
      throw err;
    } finally {
      client.release();
    }
  }
}
