import type { Job } from 'pg-boss';
import type { NotificationDispatcher, NotificationEvent as NotifEvent, NotificationEventType, NotificationTarget } from '../provider.js';
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

export interface TelegramSendJob {
  event: NotificationEventType;
  entity_id?: string;
  location_id: string;
  dedupKey?: string;
  attempt?: number;
}

const CUSTOMER_STATUS_EVENTS = ['CONFIRMED', 'IN_DELIVERY', 'DELIVERED'] as const;

export class NotificationWorker {
  private db: any;
  private dispatcher: NotificationDispatcher;
  private retryPolicy: RetryPolicy;
  private boss: any;
  private webPushAdapter: WebPushAdapter | null = null;
  private memory: MemoryService | null;

  // NX-5: Per-chat rate limiting (in-memory, reset on restart)
  private lastSendPerChat = new Map<string, number>();
  private readonly CHAT_RATE_LIMIT_MS = 1200; // ~1 msg/s/chat

  // NX-5: Circuit breaker state per chat
  private circuitState = new Map<string, { failures: number; lastFailure: number; tripped: boolean }>();
  private readonly CIRCUIT_FAILURE_THRESHOLD = 5;
  private readonly CIRCUIT_COOLDOWN_MS = 60_000; // 1 min cooldown
  private readonly MAX_RETRIES = 10;

  // NX-5: Dedup set for idempotency (in-memory LRU)
  private dedupCache = new Set<string>();
  private readonly DEDUP_CACHE_MAX = 1000;

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
        `SELECT o.id, o.total, o.currency_code AS currency, o.status, o.customer_id,
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
      const shortId = order.id?.substring(0, 8);
      const title = shortId
        ? `Order #${shortId} ${statusLabels[event] || event}`
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
        [targetId, locationId],
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
          `SELECT id, total, currency_code AS currency, created_at, status
           FROM orders 
           WHERE id = $1 AND location_id = $2`,
          [orderId, locationId],
        );
        if (orderRes.rows.length === 0) return; // Order not found
        const row = orderRes.rows[0];
        orderData = {
          orderId: row.id,
          shortOrderId: row.id?.substring(0, 8),
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

async handleTelegramSend(job: Job<TelegramSendJob>) {
      const { event, entity_id, location_id } = job.data;
      const dedupKey = `${event}:${entity_id || ''}:${location_id}`;

      // NX-5: Idempotency check — skip if already delivered in this process lifetime
      if (this.dedupCache.has(dedupKey)) {
        console.log(`[TelegramSend] NX-5: Skipping duplicate job: ${dedupKey}`);
        return;
      }

      const client = await this.db.connect();
      try {
        console.log(`[TelegramSend] Processing job: event=${event}, entity_id=${entity_id || 'none'}, location_id=${location_id}`);
        
        // 1. Find all active telegram targets for the location that have this event enabled
        const targetsRes = await client.query(
          `SELECT id, address, user_id, prefs, locale
           FROM owner_notification_targets 
           WHERE location_id = $1 AND channel = 'telegram' AND status = 'active'`,
          [location_id]
        );
        
        console.log(`[TelegramSend] Found ${targetsRes.rows.length} active targets`);
  
        for (const target of targetsRes.rows) {
          try {
            // Check prefs: if prefs[event] is false, skip
            const prefs = target.prefs || {};
            if (prefs[event] === false) {
              console.log(`[TelegramSend] Skipping target ${target.id}: prefs[${event}] = false`);
              continue;
            }
  
            // NX-5: Per-chat circuit breaker
            const chatState = this.circuitState.get(target.address) || { failures: 0, lastFailure: 0, tripped: false };
            if (chatState.tripped) {
              const cooldownElapsed = Date.now() - chatState.lastFailure;
              if (cooldownElapsed < this.CIRCUIT_COOLDOWN_MS) {
                console.log(`[TelegramSend] NX-5: Circuit open for chat ${target.address}, skipping (${Math.round((this.CIRCUIT_COOLDOWN_MS - cooldownElapsed) / 1000)}s remaining)`);
                continue;
              }
              // Reset circuit after cooldown
              chatState.tripped = false;
              chatState.failures = 0;
              this.circuitState.set(target.address, chatState);
              console.log(`[TelegramSend] NX-5: Circuit reset for chat ${target.address} after cooldown`);
            }

            // NX-5: Per-chat rate limiting
            const lastSend = this.lastSendPerChat.get(target.address) || 0;
            const elapsed = Date.now() - lastSend;
            if (elapsed < this.CHAT_RATE_LIMIT_MS) {
              console.log(`[TelegramSend] NX-5: Rate limited chat ${target.address}, delaying ${this.CHAT_RATE_LIMIT_MS - elapsed}ms`);
              await new Promise(resolve => setTimeout(resolve, this.CHAT_RATE_LIMIT_MS - elapsed));
            }
  
            // 2. Build the notification data
            let data: any = { location_id, locale: target.locale || 'sq' };
            if (entity_id) {
              console.log(`[TelegramSend] Building data for event=${event}, entity_id=${entity_id}`);
              data = await this.buildTelegramData(event, entity_id, location_id, client, data.locale);
              console.log(`[TelegramSend] Built data: ${JSON.stringify({shortOrderId: data.shortOrderId, total: data.total, currency: data.currency})}`);
            }
  
            const targetObj: NotificationTarget = {
              id: target.id,
              channel: target.channel as any,
              address: target.address,
              locationId: location_id,
              locale: data.locale,
            };
  
            const eventObj: NotifEvent = { type: event };
  
            // 3. Audit: log attempt
            await client.query(
              `INSERT INTO notification_outbox_audit (event, target_id, location_id, channel, status, attempts)
               VALUES ($1, $2, $3, 'telegram', 'sending', 1)
               ON CONFLICT DO NOTHING`,
              [event, target.id, location_id]
            );

            // 4. Dispatch
            console.log(`[TelegramSend] Dispatching to target ${target.id}, channel=${target.channel}, address=${target.address}`);
            const result = await this.dispatcher.dispatch(targetObj, eventObj, data);

            // NX-5: Update rate limit tracker
            this.lastSendPerChat.set(target.address, Date.now());
  
            // 5. Handle success/failure
            if (!result.delivered) {
              console.error(`[TelegramSend] Failed to send notification to target ${target.id}: ${result.reason}`);

              // NX-5: Update circuit breaker
              chatState.failures++;
              chatState.lastFailure = Date.now();
              if (chatState.failures >= this.CIRCUIT_FAILURE_THRESHOLD) {
                chatState.tripped = true;
                console.log(`[TelegramSend] NX-5: Circuit tripped for chat ${target.address} (${chatState.failures} failures)`);
              }
              this.circuitState.set(target.address, chatState);

              // NX-5: Audit failure
              await client.query(
                `INSERT INTO notification_outbox_audit (event, target_id, location_id, channel, status, attempts, error_message)
                 VALUES ($1, $2, $3, 'telegram', 'failed', 1, $4)
                 ON CONFLICT DO NOTHING`,
                [event, target.id, location_id, result.reason]
              );
            } else {
              console.log(`[TelegramSend] Successfully sent notification to target ${target.id}`);

              // NX-5: Reset circuit on success
              if (chatState.failures > 0) {
                chatState.failures = 0;
                chatState.tripped = false;
                this.circuitState.set(target.address, chatState);
              }

              // NX-5: Mark as delivered in dedup cache
              this.dedupCache.add(dedupKey);
              if (this.dedupCache.size > this.DEDUP_CACHE_MAX) {
                const first = this.dedupCache.values().next().value;
                if (first) this.dedupCache.delete(first);
              }

              // NX-5: Audit success
              await client.query(
                `INSERT INTO notification_outbox_audit (event, target_id, location_id, channel, status, attempts)
                 VALUES ($1, $2, $3, 'telegram', 'delivered', 1, $4)
                 ON CONFLICT DO NOTHING`,
                [event, target.id, location_id]
              );
            }
          } catch (targetErr: any) {
            console.error(`[TelegramSend] Error processing target ${target.id}: ${targetErr.message}`);

            // NX-5: Circuit breaker on error
            const errChatState = this.circuitState.get(target.address) || { failures: 0, lastFailure: 0, tripped: false };
            errChatState.failures++;
            errChatState.lastFailure = Date.now();
            if (errChatState.failures >= this.CIRCUIT_FAILURE_THRESHOLD) {
              errChatState.tripped = true;
              console.log(`[TelegramSend] NX-5: Circuit tripped for chat ${target.address} (${errChatState.failures} failures)`);
            }
            this.circuitState.set(target.address, errChatState);
          }
        }
      } catch (err: any) {
        console.error(`[TelegramSend] Critical error processing job: ${err.message}`);

        // NX-5: Check retry limit before rethrowing
        const attempts = job.data?.attempt || 0;
        if (attempts >= this.MAX_RETRIES) {
          console.error(`[TelegramSend] NX-5: Max retries (${this.MAX_RETRIES}) reached for ${dedupKey}, moving to dead-letter`);
          try {
            await client.query(
              `INSERT INTO notification_outbox_audit (event, location_id, channel, status, attempts, error_message)
               VALUES ($1, $2, 'telegram', 'archived', $3, $4)
               ON CONFLICT DO NOTHING`,
              [event, location_id, attempts, err.message]
            );
          } catch {}
          return; // Do not re-throw — archive and move on
        }

        // Re-throw with incremented attempt count for pg-boss retry
        throw Object.assign(err, { data: { ...job.data, attempt: attempts + 1 } });
      } finally {
        client.release();
      }
    }

  private async fetchOrderDetails(entity_id: string, location_id: string, client: any): Promise<any> {
    const orderRes = await client.query(
      `SELECT o.id, o.total, o.subtotal, o.delivery_fee, o.discount_total,
              o.tax_total, o.currency_code, o.created_at, o.status,
              o.type, o.delivery_address, o.delivery_instructions,
              o.cash_pay_with,
              c.name AS customer_name, c.phone AS customer_phone
       FROM orders o
       LEFT JOIN customers c ON c.id = o.customer_id
       WHERE o.id = $1 AND o.location_id = $2`,
      [entity_id, location_id]
    );
    if (orderRes.rows.length === 0) throw new Error(`Order not found: ${entity_id}`);
    const row = orderRes.rows[0];

    // Fetch order items
    const itemsRes = await client.query(
      `SELECT oi.name_snapshot, oi.price_snapshot, oi.quantity
       FROM order_items oi
       WHERE oi.order_id = $1
       ORDER BY oi.id`,
      [entity_id]
    );
    const items = itemsRes.rows.map((r: any) => ({
      name: r.name_snapshot,
      price: r.price_snapshot,
      quantity: r.quantity,
    }));

    return {
      orderId: row.id,
      shortOrderId: row.id?.substring(0, 8),
      total: row.total,
      subtotal: row.subtotal,
      deliveryFee: row.delivery_fee,
      discountTotal: row.discount_total,
      taxTotal: row.tax_total,
      currency: row.currency_code,
      createdAtLocal: row.created_at.toISOString(),
      ageMinutes: Math.floor((Date.now() - new Date(row.created_at).getTime()) / 60000),
      orderType: row.type,
      deliveryAddress: row.delivery_address,
      deliveryInstructions: row.delivery_instructions,
      cashPayWith: row.cash_pay_with,
      customerName: row.customer_name,
      customerPhone: row.customer_phone,
      items,
      quantity: items.reduce((sum: number, r: any) => sum + r.quantity, 0),
    };
  }

  // Helper function to build the NotificationData for a given event and entity_id
  private async buildTelegramData(event: NotificationEventType, entity_id: string, location_id: string, client: any, locale?: string): Promise<any> {
    const data: any = { location_id, locale };

    switch (event) {
      case 'order.created':
      case 'order.confirmed':
      case 'order.rejected':
      case 'order.delivered':
      case 'order.substitution_needs_human':
      case 'order.dwell_escalation':
      case 'order.timeout_cancelled':
      case 'order.pending_aging':
      case 'order.ready_for_pickup':
      case 'delivery.flag_raised':
      case 'rating.low_received': {
        Object.assign(data, await this.fetchOrderDetails(entity_id, location_id, client));
        break;
      }

      case 'courier.assigned': {
        Object.assign(data, await this.fetchOrderDetails(entity_id, location_id, client));
        // Fetch courier name
        const caRes = await client.query(
          `SELECT c.name AS courier_name
           FROM courier_assignments ca
           JOIN couriers c ON c.id = ca.courier_id
           WHERE ca.order_id = $1 AND ca.location_id = $2
           ORDER BY ca.created_at DESC LIMIT 1`,
          [entity_id, location_id]
        );
        if (caRes.rows.length > 0) {
          data.courierName = caRes.rows[0].courier_name;
        }
        break;
      }

      case 'cash.reconcile_discrepancy': {
        Object.assign(data, await this.fetchOrderDetails(entity_id, location_id, client));
        // Use total as discrepancy amount for this event
        data.discrepancy = data.total;
        break;
      }

      case 'shift.started':
      case 'shift.closed': {
        // entity_id is the shift_id
        const shiftRes = await client.query(
          `SELECT cs.id, cs.started_at, cs.ended_at,
                  c.name AS courier_name
           FROM courier_shifts cs
           JOIN couriers c ON c.id = cs.courier_id
           WHERE cs.id = $1 AND cs.location_id = $2`,
          [entity_id, location_id]
        );
        if (shiftRes.rows.length === 0) throw new Error(`Shift not found: ${entity_id}`);
        const row = shiftRes.rows[0];
        data.courierName = row.courier_name;
        data.shiftStartTime = row.started_at ? new Date(row.started_at).toLocaleString() : '';
        if (row.ended_at) {
          const durationMs = new Date(row.ended_at).getTime() - new Date(row.started_at).getTime();
          const hours = Math.floor(durationMs / 3600000);
          const mins = Math.floor((durationMs % 3600000) / 60000);
          data.shiftDuration = `${hours}h ${mins}m`;
        }
        break;
      }

      case 'shift.close_reminder':
        // No entity data needed; just reminder text
        break;

      case 'ops.worker_liveness':
      case 'ops.backup_failed':
      case 'ops.degradation_changed':
        // System events, no entity data needed
        break;

      case 'test':
        data.message = 'Test message';
        break;

      default:
        throw new Error(`Unsupported event type for Telegram notification: ${event}`);
    }

    return data;
  }
}