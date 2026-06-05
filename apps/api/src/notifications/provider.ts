export interface NotificationTarget {
  id: string; // the target id from owner_notification_targets
  channel: 'telegram' | 'push';
  address: string;
  locationId: string;
}

export type NotificationEventType = 'order.created' | 'order.pending_aging' | 'backup.failed' | 'settlement.disputed' | 'test';

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
