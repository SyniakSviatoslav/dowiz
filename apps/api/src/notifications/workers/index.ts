// @ts-nocheck
import type { Job } from 'pg-boss';
import type { NotificationDispatcher, NotificationEventType, NotificationTarget } from '../provider.js';
import { RetryPolicy } from '../retry.js';
import { WebPushAdapter } from '../adapters/webpush.js';
import type { MemoryService } from '../../lib/memory.js';
import webpush from 'web-push';

export interface NotifyDispatchJob {
  targetId: string;
  eventType: NotificationEventType;
  orderId?: string;
  locationId: string;
  attempt: number;
  testMessage?: string;
}

export interface CustomerStatusJob {
  orderId: string;
  locationId: string;
  event: 'CONFIRMED' | 'IN_DELIVERY' | 'DELIVERED';
}

const CUSTOMER_STATUS_EVENTS = ['CONFIRMED', 'IN_DELIVERY', 'DELIVERED'] as const;

export class NotificationWorker {
  private db: any;
  private dispatcher: NotificationDispatcher;
  private retryPolicy: RetryPolicy;
  private boss: any;
  private webPushAdapter: WebPushAdapter | null = null;
  private memory: MemoryService | null;

  constructor(
    db: any,
    boss: any,
    dispatcher: NotificationDispatcher,
    retryPolicy: RetryPolicy,
    memory?: MemoryService,
  ) {
    this.db = db;
    this.boss = boss;
    this.dispatcher = dispatcher;
    this.retryPolicy = retryPolicy;
    this.memory = memory ?? null;
  }

  // Lazy-init web push adapter from env (not needed for all workers)
  private getWebPushAdapter(): WebPushAdapter {
    if (!this.webPushAdapter) {
      const publicKey = process.env.VAPID_PUBLIC_KEY || '';
      const privateKey = process.env.VAPID_PRIVATE_KEY || '';
      const subject = process.env.VAPID_SUBJECT || 'push@deliveryos.app';
      this.webPushAdapter = new WebPushAdapter(publicKey, privateKey, subject);
    }
    return this.webPushAdapter;
  }

  // ─── Customer status push (opt-in) ───────────────────────────────
  async handleCustomerStatus(job: Job<CustomerStatusJob>) {
    const { orderId, locationId, event } = job.data;
    if (!orderId || !locationId || !event) return;

    if (!CUSTOMER_STATUS_EVENTS.includes(event as any)) return;

    const client = await this.db.connect();
    try {
      // 1. Fetch order with tenant isolation + get restaurant name
      const orderRes = await client.query(
        `SELECT o.id, o.short_id, o.total, o.currency, o.status, o.customer_id,
                l.name AS location_name
         FROM orders o
         JOIN locations l ON l.id = o.location_id
         WHERE o.id = $1 AND o.location_id = $2`,
        [orderId, locationId],
      );
      if (orderRes.rows.length === 0) return;
      const order = orderRes.rows[0];
      if (!order.customer_id) return;

      // 2. Fetch customer's opted-in web push subscriptions (with RLS context)
      if (order.customer_id) {
        await client.query("SELECT set_config('app.user_id', $1, true)", [order.customer_id]);
      }
      const devicesRes = await client.query(
        `SELECT vapid_endpoint, keys_p256dh, keys_auth, push_subscription
         FROM customer_devices
         WHERE customer_id = $1 AND platform = 'webpush' AND opted_in = true
           AND vapid_endpoint IS NOT NULL`,
        [order.customer_id],
      );

      if (devicesRes.rows.length === 0) return;

      // 3. Build minimal payload (no PII)
      const statusLabels: Record<string, string> = {
        CONFIRMED: 'Order confirmed',
        IN_DELIVERY: 'On the way',
        DELIVERED: 'Delivered',
      };
      const title = order.short_id
        ? `Order #${order.short_id} ${statusLabels[event] || event}`
        : `${order.location_name || 'Your order'} — ${statusLabels[event] || event}`;
      const body = order.total != null
        ? `${(order.total / 100).toFixed(2)} ${order.currency || 'ALL'}`
        : '';

      const payload = JSON.stringify({
        title,
        body,
        tag: `order-${orderId}`,
        data: {
          orderId,
          locationId,
          url: `/order/${orderId}`,
        },
      });

      // 4. Send to each device; prune on 410/404
      const adapter = this.getWebPushAdapter();
      for (const device of devicesRes.rows) {
        let subscription: PushSubscriptionJSON;
        try {
          subscription = device.push_subscription
            ? (typeof device.push_subscription === 'string' ? JSON.parse(device.push_subscription) : device.push_subscription)
            : { endpoint: device.vapid_endpoint, keys: { p256dh: device.keys_p256dh, auth: device.keys_auth } };
        } catch {
          continue;
        }

        try {
          await webpush.sendNotification(
            subscription as webpush.PushSubscription,
            payload,
            { TTL: 86400, urgency: 'high' },
          );
        } catch (err: any) {
          if (err.statusCode === 410 || err.statusCode === 404) {
            // Prune stale subscription
            await client.query(
              `UPDATE customer_devices
               SET opted_in = false, push_subscription = NULL, vapid_endpoint = NULL,
                   keys_p256dh = NULL, keys_auth = NULL
               WHERE vapid_endpoint = $1`,
              [device.vapid_endpoint],
            ).catch(() => {});
          } else {
            console.error(`[Notify] Customer push failed for ${orderId}:`, err.message);
          }
        }
      }
    } finally {
      client.release();
    }
  }

  async handleDispatch(job: Job<NotifyDispatchJob>) {
    const { targetId, eventType, orderId, locationId, attempt, testMessage } = job.data;
    const client = await this.db.connect();

    try {
      // 1. Re-fetch target and verify tenant isolation + status
      const targetRes = await client.query(
        `SELECT id, channel, address, status, prefs 
         FROM owner_notification_targets 
         WHERE id = $1 AND location_id = $2`,
        [targetId, locationId]
      );
      
      if (targetRes.rows.length === 0) return; // Target not found or location mismatch
      const targetRow = targetRes.rows[0];

      if (targetRow.status !== 'active') return; // Do not notify disabled/disconnected channels
      
      // 2. Check prefs
      if (eventType !== 'test') {
        const prefs = targetRow.prefs || {};
        if (prefs[eventType] === false) {
          return; // Owner disabled this specific event
        }
      }

      // 3. Quiet hours logic (simplified for P16, assume UTC 22:00-08:00)
      const hour = new Date().getUTCHours();
      const isQuietHours = hour >= 22 || hour < 8;
      if (isQuietHours && eventType !== 'order.pending_aging' && eventType !== 'test') {
        return; // Suppress non-critical during quiet hours
      }

      // 4. Re-fetch order strictly under location_id (Tenant Isolation + 0 PII payload check)
      let orderData: any = {};
      if (orderId) {
        const orderRes = await client.query(
          `SELECT id, short_id, total, currency, created_at, status
           FROM orders 
           WHERE id = $1 AND location_id = $2`,
          [orderId, locationId]
        );
        if (orderRes.rows.length === 0) return; // Order not found
        const row = orderRes.rows[0];
        orderData = {
          orderId: row.id,
          shortOrderId: row.short_id,
          total: row.total,
          currency: row.currency,
          createdAtLocal: row.created_at.toISOString(),
          ageMinutes: Math.floor((Date.now() - new Date(row.created_at).getTime()) / 60000)
        };
      }

      const target: NotificationTarget = {
        id: targetRow.id,
        channel: targetRow.channel as any,
        address: targetRow.address,
        locationId
      };

      // 5. Dispatch
      const result = await this.dispatcher.dispatch(target, { type: eventType }, { locationId, ...orderData, message: testMessage });

      // 6. Handle success/failure
      if (result.delivered) {
        await this.memory?.recordWorkerAction('notification', `Dispatched ${eventType} to ${targetRow.channel}`, {
          locationId,
          orderId,
          eventType,
          channel: targetRow.channel,
          success: true,
        });

        // Resolve location_alerts if it's an escalation
        if (eventType === 'order.pending_aging' && orderId) {
          await client.query(`UPDATE location_alerts SET resolved_at = now() WHERE order_id = $1 AND kind = 'pending_aging' AND resolved_at IS NULL`, [orderId]);
        }
      } else {
        // Retry or Disable logic
        if (result.reason?.startsWith('AUTH_OR_BLOCKED')) {
          await client.query(`UPDATE owner_notification_targets SET status = 'disabled', disabled_at = now(), last_error = $2 WHERE id = $1`, [targetId, result.reason]);
        } else {
          // Add to attempts in location_alerts if this is pending_aging
          if (eventType === 'order.pending_aging' && orderId) {
             await client.query(`
               UPDATE location_alerts 
               SET attempts = attempts || $1::jsonb, last_error = $2 
               WHERE order_id = $3 AND kind = 'pending_aging' AND resolved_at IS NULL
             `, [JSON.stringify([{ attempt, time: new Date(), error: result.reason }]), result.reason, orderId]);
          }

          const nextDelay = this.retryPolicy.getDelay(attempt);
          if (nextDelay === -1) {
             // Max attempts reached
             await client.query(`UPDATE owner_notification_targets SET status = 'disabled', disabled_at = now(), last_error = 'MAX_RETRIES_EXCEEDED' WHERE id = $1`, [targetId]);
          } else {
            // Respect retry-after if provided
            const delay = result.retryAfter ? Math.max(result.retryAfter, nextDelay) : nextDelay;
            // Enqueue retry
            await this.boss.send('notify.dispatch', { ...job.data, attempt: attempt + 1 }, { startAfter: Math.floor(delay / 1000) });
          }
        }
      }

    } finally {
      client.release();
    }
  }

  async escalatePendingAging() {
    const client = await this.db.connect();
    try {
      const thresholdMs = process.env.PENDING_AGING_THRESHOLD_MS || 5 * 60 * 1000;
      
      const res = await client.query(`
        SELECT o.id, o.location_id
        FROM orders o
        WHERE o.status = 'PENDING'
          AND o.created_at < now() - ($1 || ' milliseconds')::interval
          AND NOT EXISTS (
            SELECT 1 FROM location_alerts la
            WHERE la.order_id = o.id 
              AND la.kind = 'pending_aging' 
              AND la.resolved_at IS NULL
          )
        LIMIT 100
      `, [thresholdMs]);

      for (const row of res.rows) {
        // Insert alert and enqueue for targets
        await client.query(`INSERT INTO location_alerts (location_id, order_id, kind, status) VALUES ($1, $2, 'pending_aging', 'pending')`, [row.location_id, row.id]);
        
        const targetsRes = await client.query(`SELECT id FROM owner_notification_targets WHERE location_id = $1 AND status = 'active'`, [row.location_id]);
        for (const target of targetsRes.rows) {
          await this.boss.send('notify.dispatch', {
            targetId: target.id,
            eventType: 'order.pending_aging',
            orderId: row.id,
            locationId: row.location_id,
            attempt: 0
          });
        }
      }
    } finally {
      client.release();
    }
  }
}
