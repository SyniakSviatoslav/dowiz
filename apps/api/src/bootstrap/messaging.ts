import type { MessageBus } from '@deliveryos/platform';
import type { PgBoss } from 'pg-boss';
import { BUS_CHANNELS, QUEUE_NAMES, CUSTOMER_PUSH_EVENTS } from '../lib/registry.js';

export function registerNotifySubscriptions(messageBus: MessageBus, queueBoss: PgBoss): void {
  // Backup failure → Telegram alert to location owners
  messageBus.subscribe(BUS_CHANNELS.BACKUP_FAILED, async (payload: any) => {
    try {
      await queueBoss.send(QUEUE_NAMES.NOTIFY_TELEGRAM_SEND, {
        event: 'backup.failed',
        location_id: payload.locationId || 'system'
      });
    } catch (err) {
      console.error('[Notify] Failed to send backup.failed telegram job', err);
    }
  });

  // Settlement disputed → notify courier via Telegram
  messageBus.subscribe(BUS_CHANNELS.SETTLEMENT_DISPUTED, async (payload: any) => {
    try {
      await queueBoss.send(QUEUE_NAMES.NOTIFY_TELEGRAM_SEND, {
        event: 'settlement.disputed',
        location_id: payload.locationId
      });
    } catch (err) {
      console.error('[Notify] Failed to send settlement.disputed telegram job', err);
    }
  });

  messageBus.subscribe(BUS_CHANNELS.COURIER_STALE_HEARTBEAT, async (payload: any) => {
    try {
      await queueBoss.send(QUEUE_NAMES.NOTIFY_TELEGRAM_SEND, {
        event: 'order.pending_aging',
        entity_id: payload.orderId,
        location_id: payload.locationId
      });
    } catch (err) {
      console.error('[Notify] Failed to send courier.stale_heartbeat telegram job', err);
    }
  });

  const tgSend = (event: string, entity_id: string | undefined, location_id: string) => {
    const dedupKey = `${event}:${entity_id || ''}:${location_id}`;
    return queueBoss.send(QUEUE_NAMES.NOTIFY_TELEGRAM_SEND, {
      event,
      entity_id: entity_id || '',
      location_id,
      dedupKey,
    }, { singletonKey: dedupKey });
  };

  // Register subscriptions synchronously to ensure LISTEN is called before any events
  console.log('[Notify] Registering MessageBus subscriptions...');

  messageBus.subscribe(BUS_CHANNELS.ORDER_DELIVERED, async (payload: any) => {
    console.log(`[Notify] order.delivered received: orderId=${payload.orderId}, locationId=${payload.locationId}`);
    try {
      await tgSend('order.delivered', payload.orderId, payload.locationId);
      console.log(`[Notify] order.delivered job queued: orderId=${payload.orderId}`);
    } catch (err) {
      console.error('[Notify] Failed to send order.delivered telegram job', err);
    }
  });

  messageBus.subscribe(BUS_CHANNELS.ORDER_ASSIGNMENT_CREATED, async (payload: any) => {
    try {
      await tgSend('courier.assigned', payload.orderId, payload.locationId);
    } catch (err) {
      console.error('[Notify] Failed to send courier.assigned telegram job', err);
    }
  });

  messageBus.subscribe(BUS_CHANNELS.ORDER_CONFIRMED, async (payload: any) => {
    try {
      await tgSend('order.confirmed', payload.orderId, payload.locationId);
    } catch (err) {
      console.error('[Notify] Failed to send order.confirmed telegram job', err);
    }
  });

  messageBus.subscribe(BUS_CHANNELS.ORDER_REJECTED, async (payload: any) => {
    try {
      await tgSend('order.rejected', payload.orderId, payload.locationId);
    } catch (err) {
      console.error('[Notify] Failed to send order.rejected telegram job', err);
    }
  });

  messageBus.subscribe(BUS_CHANNELS.SHIFT_STARTED, async (payload: any) => {
    try {
      await tgSend('shift.started', payload.shiftId, payload.locationId);
    } catch (err) {
      console.error('[Notify] Failed to send shift.started telegram job', err);
    }
  });

  messageBus.subscribe(BUS_CHANNELS.SHIFT_CLOSED, async (payload: any) => {
    try {
      await tgSend('shift.closed', payload.shiftId, payload.locationId);
    } catch (err) {
      console.error('[Notify] Failed to send shift.closed telegram job', err);
    }
  });

  messageBus.subscribe(BUS_CHANNELS.ORDER_STATUS, async (payload: any) => {
    const eventKey = `order.${(payload.status || '').toLowerCase()}`;
    if (!CUSTOMER_PUSH_EVENTS.has(eventKey)) return;
    try {
      await queueBoss.send(QUEUE_NAMES.NOTIFY_CUSTOMER_STATUS, {
        orderId: payload.orderId,
        locationId: payload.locationId || payload.data?.locationId,
        event: payload.status,
      });
    } catch (err) {
      console.error('[Notify] Failed to enqueue customer status push', err);
    }
  });
}
