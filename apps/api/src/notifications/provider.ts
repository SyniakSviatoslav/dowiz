export interface NotificationTarget {
  id: string; // the target id from owner_notification_targets
  channel: 'telegram' | 'push';
  address: string;
  locationId: string;
  locale?: string;
}

export type NotificationEventType = 
  | 'order.created'
  | 'order.confirmed'
  | 'order.rejected'
  | 'order.delivered'
  | 'order.substitution_needs_human'
  | 'order.dwell_escalation'
  | 'order.timeout_cancelled'
  | 'order.dispatch_failed'
  | 'cash.reconcile_discrepancy'
  | 'delivery.flag_raised'
  | 'rating.low_received'
  | 'ops.worker_liveness'
  | 'ops.backup_failed'
  | 'ops.degradation_changed'
  | 'courier.assigned'
  | 'order.pending_aging'
  | 'order.ready_for_pickup'
  | 'shift.started'
  | 'shift.closed'
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
  subtotal?: number;
  deliveryFee?: number;
  discountTotal?: number;
  taxTotal?: number;
  currency?: string;
  createdAtLocal?: string;
  ageMinutes?: number;
  quantity?: number;
  orderType?: string;
  deliveryAddress?: string;
  deliveryInstructions?: string;
  cashPayWith?: number;
  preferences?: Record<string, any>;
  customerName?: string;
  customerPhone?: string;
  items?: Array<{ name: string; price: number; quantity: number }>;
  courierName?: string;
  shiftStartTime?: string;
  shiftDuration?: string;
  discrepancy?: number;
  rating?: number;
  message?: string; // for 'test'
  locale?: string;
  // P0-4: per-location Telegram body detail. Render defaults to 'area' when unset.
  alertDetail?: 'full' | 'area' | 'minimal';
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
