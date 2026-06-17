import type { NotificationData } from './provider.js';
import { renderTelegramMessage } from './render.js';

export type QuietHoursPolicy = 'always' | 'during_business' | 'never';
export type RenderGroup = 'confirm_reject' | 'open_in_app' | 'aging_confirm' | 'track' | 'close_shift' | 'no_buttons';
export type TargetScope = 'order' | 'shift' | 'system' | 'test';

/** UI-visible category shown to the owner. 'orders' = critical, cannot be disabled. */
export type NotificationCategory = 'orders' | 'operations' | 'analytics';

export interface EventEntry {
  type: string;
  description: string;
  quietHours: QuietHoursPolicy;
  renderGroup: RenderGroup;
  targetScope: TargetScope;
  category: NotificationCategory;
}

export const EVENT_REGISTRY: Record<string, EventEntry> = {
  'order.created': {
    type: 'order.created',
    description: 'New order placed — owner must confirm or reject',
    quietHours: 'always',
    renderGroup: 'confirm_reject',
    targetScope: 'order',
    category: 'orders',
  },
  'order.confirmed': {
    type: 'order.confirmed',
    description: 'Order confirmed by owner',
    quietHours: 'always',
    renderGroup: 'open_in_app',
    targetScope: 'order',
    category: 'orders',
  },
  'order.rejected': {
    type: 'order.rejected',
    description: 'Order rejected by owner',
    quietHours: 'always',
    renderGroup: 'open_in_app',
    targetScope: 'order',
    category: 'orders',
  },
  'order.delivered': {
    type: 'order.delivered',
    description: 'Order delivered',
    quietHours: 'always',
    renderGroup: 'open_in_app',
    targetScope: 'order',
    category: 'orders',
  },
  'order.substitution_needs_human': {
    type: 'order.substitution_needs_human',
    description: 'Menu item substitution requires manual decision',
    quietHours: 'always',
    renderGroup: 'confirm_reject',
    targetScope: 'order',
    category: 'orders',
  },
  'order.dwell_escalation': {
    type: 'order.dwell_escalation',
    description: 'Order waiting too long without action',
    quietHours: 'always',
    renderGroup: 'open_in_app',
    targetScope: 'order',
    category: 'orders',
  },
  'order.timeout_cancelled': {
    type: 'order.timeout_cancelled',
    description: 'Order auto-cancelled due to confirmation timeout',
    quietHours: 'always',
    renderGroup: 'open_in_app',
    targetScope: 'order',
    category: 'operations',
  },
  'cash.reconcile_discrepancy': {
    type: 'cash.reconcile_discrepancy',
    description: 'Cash settlement has discrepancy',
    quietHours: 'always',
    renderGroup: 'open_in_app',
    targetScope: 'order',
    category: 'operations',
  },
  'delivery.flag_raised': {
    type: 'delivery.flag_raised',
    description: 'Delivery issue flagged',
    quietHours: 'always',
    renderGroup: 'open_in_app',
    targetScope: 'order',
    category: 'operations',
  },
  'rating.low_received': {
    type: 'rating.low_received',
    description: 'Low rating received',
    quietHours: 'during_business',
    renderGroup: 'open_in_app',
    targetScope: 'order',
    category: 'analytics',
  },
  'ops.worker_liveness': {
    type: 'ops.worker_liveness',
    description: 'Worker process stopped responding',
    quietHours: 'always',
    renderGroup: 'no_buttons',
    targetScope: 'system',
    category: 'operations',
  },
  'ops.backup_failed': {
    type: 'ops.backup_failed',
    description: 'Database backup failed',
    quietHours: 'always',
    renderGroup: 'no_buttons',
    targetScope: 'system',
    category: 'operations',
  },
  'ops.degradation_changed': {
    type: 'ops.degradation_changed',
    description: 'System degradation status changed',
    quietHours: 'always',
    renderGroup: 'no_buttons',
    targetScope: 'system',
    category: 'operations',
  },
  'courier.assigned': {
    type: 'courier.assigned',
    description: 'Courier assigned to order',
    quietHours: 'always',
    renderGroup: 'track',
    targetScope: 'order',
    category: 'operations',
  },
  'order.pending_aging': {
    type: 'order.pending_aging',
    description: 'Order pending longer than threshold',
    quietHours: 'always',
    renderGroup: 'aging_confirm',
    targetScope: 'order',
    category: 'orders',
  },
  'order.ready_for_pickup': {
    type: 'order.ready_for_pickup',
    description: 'Order ready for courier pickup',
    quietHours: 'always',
    renderGroup: 'open_in_app',
    targetScope: 'order',
    category: 'orders',
  },
  'shift.started': {
    type: 'shift.started',
    description: 'Courier shift started',
    quietHours: 'always',
    renderGroup: 'close_shift',
    targetScope: 'shift',
    category: 'operations',
  },
  'shift.closed': {
    type: 'shift.closed',
    description: 'Courier shift closed',
    quietHours: 'always',
    renderGroup: 'close_shift',
    targetScope: 'shift',
    category: 'operations',
  },
  'shift.close_reminder': {
    type: 'shift.close_reminder',
    description: 'Reminder to close open shift',
    quietHours: 'during_business',
    renderGroup: 'close_shift',
    targetScope: 'shift',
    category: 'operations',
  },
  'test': {
    type: 'test',
    description: 'Test notification',
    quietHours: 'never',
    renderGroup: 'no_buttons',
    targetScope: 'test',
    category: 'orders',
  },
};

export function isEventAllowedDuringQuietHours(event: string): boolean {
  const entry = EVENT_REGISTRY[event];
  if (!entry) return false;
  return entry.quietHours === 'always';
}

export function getEventCategory(event: string): NotificationCategory | null {
  return EVENT_REGISTRY[event]?.category ?? null;
}

/**
 * Check if an event should be sent given target prefs.
 * - 'orders' category is always allowed (critical, cannot be disabled via category toggle).
 * - 'operations' category: allowed unless prefs.category_operations === false.
 * - 'analytics' category: allowed only if prefs.category_analytics === true.
 * Legacy event-level prefs (e.g. `prefs["order.created"] = false`) are still honoured
 * if present, taking precedence over category-level.
 */
export function isEventAllowedByPrefs(event: string, prefs: Record<string, any>): boolean {
  if (event === 'test') return true;

  // Legacy event-level override (explicit false = suppressed, explicit true = allowed)
  if (typeof prefs[event] === 'boolean') return prefs[event];

  const category = getEventCategory(event);
  if (!category) return true; // unknown event → allow

  if (category === 'orders') return true; // critical, cannot be disabled
  if (category === 'operations') return prefs.category_operations !== false;
  if (category === 'analytics') return prefs.category_analytics === true;
  return true;
}
