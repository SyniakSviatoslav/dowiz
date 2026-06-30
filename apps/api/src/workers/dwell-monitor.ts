// @ts-nocheck
import type { Pool } from 'pg';
import type Boss from 'pg-boss';
import type { MessageBus } from '@deliveryos/platform';
import { loadEnv } from '@deliveryos/config';
import { BUS_CHANNELS, QUEUE_NAMES, orderChannel, dashboardChannel, courierChannel, shiftChannel } from '../lib/registry.js';
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
    await this.boss.work(QUEUE_NAMES.DWELL_MONITOR, { singletonKey: QUEUE_NAMES.DWELL_MONITOR }, async () => {
      await this.run();
    });
    const cron = env.DWELL_CRON || '* * * * *';
    await this.boss.createQueue(QUEUE_NAMES.DWELL_MONITOR);
    await this.boss.schedule(QUEUE_NAMES.DWELL_MONITOR, cron, null, { singletonKey: QUEUE_NAMES.DWELL_MONITOR });
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
      await this.messageBus.publish(BUS_CHANNELS.DWELL_MONITOR_FAILED, { error: String(err), time: new Date().toISOString() });
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

      // B3 (NOBYPASSRLS): orders + location_alerts are FORCE-RLS; location_alerts keys
      // on app_member_location_ids() (member identity, which a system worker lacks), so
      // set_config('app.current_tenant') cannot admit it. The exact dwell-detection query
      // (same WHERE guards, same 50-row cap) runs inside app_dwell_due_orders() DEFINER fn.
      const ordersRes = await client.query(
        `SELECT * FROM app_dwell_due_orders($1, $2, $3, $4)`,
        [locationId, status, String(thresholdSec), kind]
      );

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
      // B3 (NOBYPASSRLS): location_alerts INSERT WITH CHECK keys on app_member_location_ids()
      // → wrapped in app_dwell_create_alert() DEFINER fn (same ON CONFLICT DO NOTHING RETURNING id).
      const res = await client.query(
        `SELECT * FROM app_dwell_create_alert($1, $2, $3)`,
        [locationId, orderId, kind]
      );
      if (res.rowCount === 0) return null;

      const alertId = res.rows[0].id;

      await this.messageBus.publish(dashboardChannel(locationId), {
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

    // Resolve active Telegram targets for this location.
    // B3 (NOBYPASSRLS): owner_notification_targets keys on app_member_location_ids()
    // → read via app_active_notification_targets() DEFINER fn (same active+channel filter).
    const targetsRes = await client.query(
      `SELECT id FROM app_active_notification_targets($1, 'telegram')`,
      [locationId]
    );

    if (targetsRes.rows.length === 0) return;

    // Send notification to each active target via notify.dispatch
    for (const target of targetsRes.rows) {
      // Immediate notification
      await this.boss.send(QUEUE_NAMES.NOTIFY_DISPATCH, {
        targetId: target.id,
        eventType: 'order.dwell_escalation',
        orderId,
        locationId,
        attempt: 0,
      }, { startAfter: 0 });

      // Tier 2: delayed notification (if the alert hasn't been resolved)
      await this.boss.send(QUEUE_NAMES.NOTIFY_DISPATCH, {
        targetId: target.id,
        eventType: 'order.dwell_escalation',
        orderId,
        locationId,
        attempt: 0,
      }, { startAfter: Math.floor(tier2Delay / 1000) });
    }
  }
}
