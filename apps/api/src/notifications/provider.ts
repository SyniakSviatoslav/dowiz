export interface NotificationTarget {
  id: string; // the target id from owner_notification_targets
  channel: 'telegram' | 'push';
  address: string;
  locationId: string;
}

export type NotificationEventType = 
  | 'order.created'
  | 'order.substitution_needs_human'
  | 'order.dwell_escalation'
  | 'order.timeout_cancelled'
  | 'cash.reconcile_discrepancy'
  | 'delivery.flag_raised'
  | 'rating.low_received'
  | 'ops.worker_liveness'
  | 'ops.backup_failed'
  | 'ops.degradation_changed'
  | 'courier.assigned'
  | 'order.pending_aging'
  | 'order.ready_for_pickup'
  | 'shift.close_reminder'
  | 'test';

export interface NotificationEvent {
  type: NotificationEventType;
}

export interface NotificationData {
  orderId?: string;
  locationId: string;
  shortOrderId?: string;
  total?: number;
  currency?: string;
  createdAtLocal?: string;
  ageMinutes?: number;
  quantity?: number;
  message?: string; // for 'test'
}

export interface NotifyResult {
  delivered: boolean;
  reason?: string;
  retryAfter?: number;
  providerMessageId?: string;
}

export interface NotificationProvider {
  readonly id: string;
  notify(target: NotificationTarget, event: NotificationEvent, data: NotificationData): Promise<NotifyResult>;
}

export class NotificationDispatcher {
  private adapters = new Map<string, NotificationProvider>();

  register(channel: string, provider: NotificationProvider) {
    this.adapters.set(channel, provider);
  }

  async dispatch(target: NotificationTarget, event: NotificationEvent, data: NotificationData): Promise<NotifyResult> {
    const provider = this.adapters.get(target.channel);
    if (!provider) {
      return { delivered: false, reason: `No provider for channel ${target.channel}` };
    }
    return provider.notify(target, event, data);
  }
}
