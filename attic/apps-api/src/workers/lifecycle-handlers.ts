// @ts-nocheck
import type { Pool } from 'pg';
import type Boss from 'pg-boss';
import type { MessageBus } from '@deliveryos/platform';
import { BUS_CHANNELS, QUEUE_NAMES, orderChannel, dashboardChannel, courierChannel, shiftChannel } from '../lib/registry.js';
import { STATUS_KIND_MAP } from '../lib/dwell-thresholds.js';

const TRANSITION_RESOLVE_MAP: Record<string, string[]> = {
  CONFIRMED: ['dwell_pending'],
  CANCELLED: ['dwell_pending', 'dwell_confirmed', 'dwell_preparing', 'dwell_en_route'],
  REJECTED: ['dwell_pending', 'dwell_confirmed', 'dwell_preparing'],
  PREPARING: ['dwell_confirmed'],
  IN_DELIVERY: ['dwell_preparing'],
  DELIVERED: ['dwell_en_route', 'dwell_preparing', 'dwell_confirmed'],
  EN_ROUTE: ['dwell_preparing'],
};

export class LifecycleHandlers {
  constructor(
    private pool: Pool,
    private boss: Boss,
    private messageBus: MessageBus,
  ) {}

  async start() {
    this.messageBus.subscribe(BUS_CHANNELS.ORDER_CONFIRMED, async (msg: any) => this.handleTransition(msg, 'CONFIRMED'));
    this.messageBus.subscribe(BUS_CHANNELS.ORDER_CANCELLED, async (msg: any) => this.handleTransition(msg, 'CANCELLED'));
    this.messageBus.subscribe(BUS_CHANNELS.ORDER_REJECTED, async (msg: any) => this.handleTransition(msg, 'REJECTED'));

    // Listen for order.status events that contain the new status
    this.messageBus.subscribe(BUS_CHANNELS.ORDER_STATUS, async (msg: any) => {
      const newStatus = msg.status;
      if (TRANSITION_RESOLVE_MAP[newStatus]) {
        await this.handleTransition({ orderId: msg.orderId }, newStatus);
      }
    });
  }

  private async handleTransition(msg: any, newStatus: string) {
    const orderId = msg.orderId || (msg.data?.orderId) || (msg.payload?.orderId);
    if (!orderId) return;

    const kindsToResolve = TRANSITION_RESOLVE_MAP[newStatus];
    if (!kindsToResolve || kindsToResolve.length === 0) return;

    const client = await this.pool.connect();
    try {
      for (const kind of kindsToResolve) {
        const res = await client.query(`
          UPDATE location_alerts
          SET status = 'resolved',
              resolved_at = now(),
              resolution_reason = $1
          WHERE order_id = $2
            AND kind = $3
            AND resolved_at IS NULL
          RETURNING id
        `, [`lifecycle_${newStatus.toLowerCase()}`, orderId, kind]);

        for (const row of res.rows) {
          // Cancel pending escalation jobs for this alert
          await this.boss.cancel(`notify.dispatch.${row.id}`);

          await this.messageBus.publish(BUS_CHANNELS.DWELL_ALERT_RESOLVED, {
            alertId: row.id,
            orderId,
            kind,
            resolvedBy: 'lifecycle',
            resolution: newStatus,
          });
        }
      }
    } catch (err) {
      console.error('[LifecycleHandlers] Error resolving alerts:', err);
    } finally {
      client.release();
    }
  }
}
