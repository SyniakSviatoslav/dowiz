import type { Pool } from 'pg';
import type Boss from 'pg-boss';
import type { MessageBus } from '@deliveryos/platform';
import { loadEnv } from '@deliveryos/config';
import { STATUS_KIND_MAP, KIND_TO_THRESHOLD_KEY, DEFAULT_DWELL_THRESHOLDS } from '../lib/dwell-thresholds.js';

const env = loadEnv();

const MONITORED_STATUSES = ['PENDING', 'CONFIRMED', 'PREPARING', 'IN_DELIVERY'];

export class DwellMonitorWorker {
  constructor(
    private pool: Pool,
    private boss: Boss,
    private messageBus: MessageBus,
  ) {}

  async start() {
    await this.boss.work('dwell.monitor', { singletonKey: 'dwell.monitor' }, async () => {
      await this.run();
    });
    const cron = env.DWELL_CRON || '* * * * *';
    await this.boss.createQueue('dwell.monitor');
    await this.boss.schedule('dwell.monitor', cron, null, { singletonKey: 'dwell.monitor' });
  }

  private async run() {
    const client = await this.pool.connect();
    try {
      // Acquire advisory lock for N=2 safety (backup to singletonKey)
      const lock = await client.query("SELECT pg_try_advisory_lock(2) AS locked");
      if (!lock.rows[0]?.locked) {
        console.log('[DwellMonitor] Skipped — advisory lock held by another instance');
        return;
      }

      try {
        const locationsRes = await client.query(`
          SELECT id, dwell_thresholds FROM locations
        `);

        for (const loc of locationsRes.rows) {
          await this.checkLocation(client, loc.id, loc.dwell_thresholds);
        }
      } finally {
        await client.query("SELECT pg_advisory_unlock(2)");
      }
    } catch (err) {
      console.error('[DwellMonitor] Error:', err);
      await this.messageBus.publish('dwell.monitor.failed', { error: String(err), time: new Date().toISOString() });
    } finally {
      client.release();
    }
  }

  private async checkLocation(client: any, locationId: string, thresholdsJson: any) {
    const thresholds = typeof thresholdsJson === 'string' ? JSON.parse(thresholdsJson) : (thresholdsJson || DEFAULT_DWELL_THRESHOLDS);

    for (const status of MONITORED_STATUSES) {
      const kind = STATUS_KIND_MAP[status];
      const thresholdKey = KIND_TO_THRESHOLD_KEY[kind];
      const thresholdSec = thresholds[thresholdKey] ?? DEFAULT_DWELL_THRESHOLDS[thresholdKey as keyof typeof DEFAULT_DWELL_THRESHOLDS];

      if (!thresholdSec) continue;

      const ordersRes = await client.query(`
        SELECT o.id, o.status,
               COALESCE(o.confirmed_at, o.created_at) AS status_updated_at
        FROM orders o
        WHERE o.location_id = $1
          AND o.status = $2
          AND COALESCE(o.confirmed_at, o.created_at) < now() - ($3 || ' seconds')::interval
          AND NOT EXISTS (
            SELECT 1 FROM location_alerts la
            WHERE la.order_id = o.id
              AND la.kind = $4
              AND la.resolved_at IS NULL
          )
        LIMIT 50
      `, [locationId, status, String(thresholdSec), kind]);

      for (const order of ordersRes.rows) {
        const dwellSec = Math.floor((Date.now() - new Date(order.status_updated_at).getTime()) / 1000);
        const alertId = await this.createAlert(client, locationId, order.id, kind, dwellSec);
        if (alertId) {
          await this.scheduleEscalation(client, alertId, order.id, locationId, kind);
        }
      }
    }
  }

  private async createAlert(client: any, locationId: string, orderId: string, kind: string, dwellSeconds: number): Promise<string | null> {
    try {
      const res = await client.query(`
        INSERT INTO location_alerts (location_id, order_id, kind, status, escalation_level)
        VALUES ($1, $2, $3, 'active', 0)
        ON CONFLICT DO NOTHING
        RETURNING id
      `, [locationId, orderId, kind]);
      if (res.rowCount === 0) return null;

      const alertId = res.rows[0].id;

      await this.messageBus.publish(`location:${locationId}:dashboard`, {
        type: 'dwell.alert_created',
        data: { alertId, orderId, kind, dwellSeconds, severity: dwellSeconds > 0 ? 'warning' : 'info' },
      });

      return alertId;
    } catch (err: any) {
      if (err.code === '23505') return null; // Duplicate
      throw err;
    }
  }

  private async scheduleEscalation(client: any, alertId: string, orderId: string, locationId: string, kind: string) {
    const tier2Delay = parseInt(env.DWELL_TIER2_DELAY_MS || '30000', 10);
    const tier3Delay = parseInt(env.DWELL_TIER3_DELAY_MS || '90000', 10);
    const tier3Enabled = env.DWELL_TIER3_ENABLED === 'true';

    // Tier 1: immediate native-push (scaffold)
    await this.boss.send('notify.dispatch', {
      targetId: null,
      eventType: 'dwell.alert',
      alertId,
      orderId,
      locationId,
      kind,
      tier: 1,
      attempt: 0,
    }, { startAfter: 0 });

    // Tier 2: Telegram (delayed)
    await this.boss.send('notify.dispatch', {
      targetId: null,
      eventType: 'dwell.alert',
      alertId,
      orderId,
      locationId,
      kind,
      tier: 2,
      attempt: 0,
    }, { startAfter: Math.floor(tier2Delay / 1000) });

    // Tier 3: SMS scaffold (optional, delayed)
    if (tier3Enabled) {
      await this.boss.send('notify.dispatch', {
        targetId: null,
        eventType: 'dwell.alert',
        alertId,
        orderId,
        locationId,
        kind,
        tier: 3,
        attempt: 0,
      }, { startAfter: Math.floor(tier3Delay / 1000) });
    }
  }
}
