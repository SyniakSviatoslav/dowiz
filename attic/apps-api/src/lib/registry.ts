export const BUS_CHANNELS = {
  ORDER_CREATED: 'order.created',
  ORDER_CONFIRMED: 'order.confirmed',
  ORDER_REJECTED: 'order.rejected',
  ORDER_DELIVERED: 'order.delivered',
  ORDER_STATUS: 'order.status',
  ORDER_CANCELLED: 'order.cancelled',
  ORDER_COURIER_ACCEPTED: 'order.courier_accepted',
  ORDER_PICKED_UP: 'order.picked_up',
  ORDER_ASSIGNMENT_CREATED: 'order.assignment_created',
  ORDER_DISPATCH_FAILED: 'order.dispatch_failed',
  ORDER_CANCEL_AFTER_DISPATCH: 'order.cancelled.customer_after_dispatch',
  COURIER_POSITION_UPDATED: 'courier.position_updated',
  COURIER_STALE_HEARTBEAT: 'courier.stale_heartbeat',
  SHIFT_STARTED: 'shift.started',
  SHIFT_CLOSED: 'shift.closed',
  SETTLEMENT_APPROVED: 'settlement.approved',
  SETTLEMENT_DISPUTED: 'settlement.disputed',
  BACKUP_FAILED: 'backup.failed',
  BACKUP_COMPLETED: 'backup.completed',
  DWELL_MONITOR_FAILED: 'dwell.monitor.failed',
  DWELL_ALERT_RESOLVED: 'dwell.alert_resolved',
  MENU_IMPORT_PREVIEWED: 'menu.import.previewed',
  MENU_IMPORTED: 'menu.imported',
  MENU_TRANSLATED: 'menu.translated',
  OTP_SENT: 'otp.sent',
  OTP_VERIFIED: 'otp.verified',
  CUSTOMER_NO_SHOW: 'customer.no_show_incremented',
  CUSTOMER_ANONYMIZED: 'customer.anonymized',
  ORDER_ANONYMIZED: 'order.anonymized',
  WORKER_STALE: 'worker.stale',
  WORKER_BATCH_STALE: 'worker.batch_stale',
  WORKER_RECOVERED: 'worker.recovered',
  WORKER_FAILED: 'worker.failed',
  ALERT_WORKER_LIVENESS: 'alert.worker_liveness',
  ALERT_ACKNOWLEDGED: 'alert.acknowledged',
  ALERT_RESOLVED: 'alert.resolved_automatically',
  CONTACT_REVEALED: 'contact.revealed',
  SIGNAL_CREATED: 'signal.created',
  SIGNAL_ACKNOWLEDGED: 'signal.acknowledged',
  SIGNAL_DISMISSED: 'signal.dismissed',
  LIVENESS_CHECK_FAILED: 'liveness.check.failed',
  ANONYMIZER_GDPR_FAILED: 'anonymizer.gdpr.failed',
  GDPR_ERASURE_COMPLETED: 'gdpr.erasure_completed',
} as const;

export const orderChannel = (id: string) => `order:${id}` as const;
export const dashboardChannel = (id: string) => `location:${id}:dashboard` as const;
export const courierChannel = (id: string) => `location:${id}:couriers` as const;
export const shiftChannel = (id: string) => `courier:${id}:shift` as const;

import { QUEUE_NAMES } from '@deliveryos/shared-types';
import { EVENT_REGISTRY } from '../notifications/event-registry.js';
export { QUEUE_NAMES };

export const ALL_QUEUES: readonly string[] = Object.values(QUEUE_NAMES);

export const CUSTOMER_PUSH_EVENTS: ReadonlySet<string> = new Set([
  'order.confirmed',
  'order.in_delivery',
  'order.delivered',
]);

export type ChannelName = typeof BUS_CHANNELS[keyof typeof BUS_CHANNELS];
export type QueueName = typeof QUEUE_NAMES[keyof typeof QUEUE_NAMES];
