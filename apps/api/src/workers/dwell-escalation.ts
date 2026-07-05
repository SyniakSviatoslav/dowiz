// @ts-nocheck
import type { Pool } from 'pg';
import type Boss from 'pg-boss';
import type { MessageBus } from '@deliveryos/platform';
import { NotificationDispatcher } from '../notifications/provider.js';
import { BUS_CHANNELS, QUEUE_NAMES, orderChannel, dashboardChannel, courierChannel, shiftChannel } from '../lib/registry.js';
import { loadEnv } from '@deliveryos/config';

const env = loadEnv();

export class DwellEscalationWorker {
  constructor(
    private pool: Pool,
    private boss: Boss,
    private messageBus: MessageBus,
    private notifyDispatcher: NotificationDispatcher,
  ) {}

  async start() {
    await this.boss.work(QUEUE_NAMES.DWELL_ESCALATE, async (job: any) => {
      await this.handleEscalation(job.data);
    });
  }

  private async handleEscalation(data: { alertId: string; orderId: string; locationId: string; kind: string; tier: number }) {
    const { alertId, orderId, locationId, kind, tier } = data;

    const client = await this.pool.connect();
    try {
      // Check if alert is still active and not acknowledged.
      // B3 (NOBYPASSRLS): location_alerts keys on app_member_location_ids() (member
      // identity); this worker is a system actor, so all location_alerts reads/writes
      // here run through app_* SECURITY DEFINER fns that mirror the original SQL exactly.
      const alertRes = await client.query(
        `SELECT * FROM app_alert_state($1)`,
        [alertId],
      );
      if (alertRes.rowCount === 0) return;
      const alert = alertRes.rows[0];
      if (alert.status !== 'active') return;
      if (alert.acknowledged_at) return;
      if ((alert.escalation_level || 0) >= tier) return;

      // Update escalation level
      await client.query(
        `SELECT app_bump_alert_escalation($1, $2)`,
        [alertId, tier],
      );

      const batchThreshold = parseInt(env.DWELL_BATCH_THRESHOLD || '10', 10);

      if (tier === 1) {
        // Native push (scaffold) — send to all active push targets
        const targetsRes = await client.query(
          `SELECT id, address FROM app_active_notification_targets($1, 'push')`,
          [locationId],
        );

        if (targetsRes.rows.length > 0) {
          // Check if we should batch
          const alertCount = await this.countActiveAlerts(client, locationId, kind);
          const shouldBatch = alertCount >= batchThreshold;

          for (const target of targetsRes.rows) {
            await this.sendWithBatchCheck(
              target, 'push', alertId, orderId, locationId, kind, tier,
              alertCount, shouldBatch, 0,
            );
          }
        }

        await this.logAttempt(client, alertId, tier, 'scheduled', null);
        await this.publishEvent(locationId, 'dwell.escalation_tier_changed', { alertId, orderId, kind, tier });

      } else if (tier === 2) {
        // Telegram — only fire if tier 1 did not deliver
        const tier1Delivered = await this.wasTierDelivered(client, alertId, 1);
        if (tier1Delivered) return;

        const targetsRes = await client.query(
          `SELECT id, address FROM app_active_notification_targets($1, 'telegram')`,
          [locationId],
        );

        if (targetsRes.rows.length > 0) {
          const alertCount = await this.countActiveAlerts(client, locationId, kind);
          const shouldBatch = alertCount >= batchThreshold;

          for (const target of targetsRes.rows) {
            await this.sendWithBatchCheck(
              target, 'telegram', alertId, orderId, locationId, kind, tier,
              alertCount, shouldBatch, 0,
            );
          }
        }

        await this.logAttempt(client, alertId, tier, 'sent', null);
        await this.publishEvent(locationId, 'dwell.escalation_tier_changed', { alertId, orderId, kind, tier });

      } else if (tier === 3) {
        // SMS scaffold — log only
        console.log(`[DwellEscalation] Tier 3 scaffold: alert=${alertId} order=${orderId} location=${locationId}`);
        await this.logAttempt(client, alertId, tier, 'scaffold_not_implemented', null);
      }
    } catch (err) {
      console.error(`[DwellEscalation] Error tier ${tier}:`, err);
      await this.logAttempt(client, alertId, tier, 'error', String(err));
    } finally {
      client.release();
    }
  }

  private async sendWithBatchCheck(
    target: any, channel: string, alertId: string, orderId: string,
    locationId: string, kind: string, tier: number,
    alertCount: number, shouldBatch: boolean, attempt: number,
  ) {
    const targetPayload: any = {
      id: target.id,
      channel,
      address: target.address,
      locationId,
    };

    const eventPayload: any = {
      type: 'dwell.alert',
      alertId,
      orderId,
      kind,
      tier,
      batchCount: shouldBatch ? alertCount : undefined,
    };

    // Use existing notify.dispatch queue for actual delivery
    await this.boss.send(QUEUE_NAMES.NOTIFY_DISPATCH, {
      targetId: target.id,
      eventType: 'dwell.alert',
      alertId,
      orderId,
      locationId,
      kind,
      tier,
      attempt,
      batchCount: shouldBatch ? alertCount : undefined,
      message: this.buildMessage(kind, orderId),
    }, { startAfter: 0 });
  }

  private buildMessage(kind: string, orderId: string): string {
    const shortId = orderId.substring(0, 8);
    const label = kind.replace('dwell_', '').replace('_', ' ').toUpperCase();
    return `⚠ Order #${shortId} — ${label} threshold exceeded`;
  }

  private async countActiveAlerts(client: any, locationId: string, kind: string): Promise<number> {
    const res = await client.query(
      `SELECT app_count_active_dwell_alerts($1) AS cnt`,
      [locationId],
    );
    return res.rows[0].cnt;
  }

  private async wasTierDelivered(client: any, alertId: string, tier: number): Promise<boolean> {
    const res = await client.query(
      `SELECT app_alert_tier_reached($1, $2) AS reached`,
      [alertId, tier],
    );
    return res.rows[0]?.reached === true;
  }

  private async logAttempt(client: any, alertId: string, tier: number, status: string | null, error: string | null) {
    const entry = { tier, time: new Date().toISOString(), status, error };
    await client.query(
      `SELECT app_alert_log_attempt($1, $2::jsonb, $3)`,
      [alertId, JSON.stringify([entry]), error],
    );
  }

  private async publishEvent(locationId: string, type: string, data: any) {
    try {
      await this.messageBus.publish(dashboardChannel(locationId), { type, data });
    } catch (err: any) {
      console.debug('[dwell-escalation] failed to publish event', type, err?.message);
    }
  }
}
