// @ts-nocheck
import type { Pool } from 'pg';
import type Boss from 'pg-boss';
import type { MessageBus } from '@deliveryos/platform';
import { NotificationDispatcher } from '../notifications/provider.js';
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
    await this.boss.work('dwell.escalate', async (job: any) => {
      await this.handleEscalation(job.data);
    });
  }

  private async handleEscalation(data: { alertId: string; orderId: string; locationId: string; kind: string; tier: number }) {
    const { alertId, orderId, locationId, kind, tier } = data;

    const client = await this.pool.connect();
    try {
      // Check if alert is still active and not acknowledged
      const alertRes = await client.query(
        `SELECT status, acknowledged_at, escalation_level FROM location_alerts WHERE id = $1`,
        [alertId],
      );
      if (alertRes.rowCount === 0) return;
      const alert = alertRes.rows[0];
      if (alert.status !== 'active') return;
      if (alert.acknowledged_at) return;
      if ((alert.escalation_level || 0) >= tier) return;

      // Update escalation level
      await client.query(
        `UPDATE location_alerts SET escalation_level = GREATEST(escalation_level, $1) WHERE id = $2`,
        [tier, alertId],
      );

      const batchThreshold = parseInt(env.DWELL_BATCH_THRESHOLD || '10', 10);

      if (tier === 1) {
        // Native push (scaffold) — send to all active push targets
        const targetsRes = await client.query(
          `SELECT id, address FROM owner_notification_targets
           WHERE location_id = $1 AND channel = 'push' AND status = 'active'`,
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
          `SELECT id, address FROM owner_notification_targets
           WHERE location_id = $1 AND channel = 'telegram' AND status = 'active'`,
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
    await this.boss.send('notify.dispatch', {
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
      `SELECT COUNT(*)::int AS cnt FROM location_alerts
       WHERE location_id = $1 AND kind LIKE 'dwell_%' AND status = 'active' AND resolved_at IS NULL`,
      [locationId],
    );
    return res.rows[0].cnt;
  }

  private async wasTierDelivered(client: any, alertId: string, tier: number): Promise<boolean> {
    const res = await client.query(
      `SELECT id FROM location_alerts WHERE id = $1 AND escalation_level >= $2`,
      [alertId, tier],
    );
    return res.rowCount > 0;
  }

  private async logAttempt(client: any, alertId: string, tier: number, status: string | null, error: string | null) {
    const entry = { tier, time: new Date().toISOString(), status, error };
    await client.query(
      `UPDATE location_alerts SET
        attempts = COALESCE(attempts, '[]'::jsonb) || $1::jsonb,
        last_error = $2
       WHERE id = $3`,
      [JSON.stringify([entry]), error, alertId],
    );
  }

  private async publishEvent(locationId: string, type: string, data: any) {
    try {
      await this.messageBus.publish(`location:${locationId}:dashboard`, { type, data });
    } catch {
      // ignore publish errors — dashboard event is non-critical
      console.debug('[dwell-escalation] failed to publish event', type);
    }
  }
}
