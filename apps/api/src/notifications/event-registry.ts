import type { NotificationData } from './provider.js';
import { renderTelegramMessage } from './render.js';

export type QuietHoursPolicy = 'always' | 'during_business' | 'never';
export type RenderGroup = 'confirm_reject' | 'open_in_app' | 'aging_confirm' | 'track' | 'close_shift' | 'no_buttons';
export type TargetScope = 'order' | 'shift' | 'system' | 'test';

export interface EventEntry {
  type: string;
  description: string;
  quietHours: QuietHoursPolicy;
  renderGroup: RenderGroup;
  targetScope: TargetScope;
}

export const EVENT_REGISTRY: Record<string, EventEntry> = {
  'order.created': {
    type: 'order.created',
    description: 'New order placed — owner must confirm or reject',
    quietHours: 'always',
    renderGroup: 'confirm_reject',
    targetScope: 'order',
  },
  'order.confirmed': {
    type: 'order.confirmed',
    description: 'Order confirmed by owner',
    quietHours: 'always',
    renderGroup: 'open_in_app',
    targetScope: 'order',
  },
  'order.rejected': {
    type: 'order.rejected',
    description: 'Order rejected by owner',
    quietHours: 'always',
    renderGroup: 'open_in_app',
    targetScope: 'order',
  },
  'order.delivered': {
    type: 'order.delivered',
    description: 'Order delivered',
    quietHours: 'always',
    renderGroup: 'open_in_app',
    targetScope: 'order',
  },
  'order.substitution_needs_human': {
    type: 'order.substitution_needs_human',
    description: 'Menu item substitution requires manual decision',
    quietHours: 'always',
    renderGroup: 'confirm_reject',
    targetScope: 'order',
  },
  'order.dwell_escalation': {
    type: 'order.dwell_escalation',
    description: 'Order waiting too long without action',
    quietHours: 'always',
    renderGroup: 'open_in_app',
    targetScope: 'order',
  },
  'order.timeout_cancelled': {
    type: 'order.timeout_cancelled',
    description: 'Order auto-cancelled due to confirmation timeout',
    quietHours: 'always',
    renderGroup: 'open_in_app',
    targetScope: 'order',
  },
  // ADR-dispatch-recovery (ETHICAL-STOP-1): dispatch exhaustion — the owner MUST act
  // (assign manually or cancel). Transactional category (unlisted → fail-safe default):
  // never suppressed by prefs or quiet hours.
  'order.dispatch_failed': {
    type: 'order.dispatch_failed',
    description: 'No courier found after max dispatch attempts — owner action needed',
    quietHours: 'always',
    renderGroup: 'open_in_app',
    targetScope: 'order',
  },
  'cash.reconcile_discrepancy': {
    type: 'cash.reconcile_discrepancy',
    description: 'Cash settlement has discrepancy',
    quietHours: 'always',
    renderGroup: 'open_in_app',
    targetScope: 'order',
  },
  'delivery.flag_raised': {
    type: 'delivery.flag_raised',
    description: 'Delivery issue flagged',
    quietHours: 'always',
    renderGroup: 'open_in_app',
    targetScope: 'order',
  },
  'rating.low_received': {
    type: 'rating.low_received',
    description: 'Low rating received',
    quietHours: 'during_business',
    renderGroup: 'open_in_app',
    targetScope: 'order',
  },
  'ops.worker_liveness': {
    type: 'ops.worker_liveness',
    description: 'Worker process stopped responding',
    quietHours: 'always',
    renderGroup: 'no_buttons',
    targetScope: 'system',
  },
  'ops.backup_failed': {
    type: 'ops.backup_failed',
    description: 'Database backup failed',
    quietHours: 'always',
    renderGroup: 'no_buttons',
    targetScope: 'system',
  },
  'ops.degradation_changed': {
    type: 'ops.degradation_changed',
    description: 'System degradation status changed',
    quietHours: 'always',
    renderGroup: 'no_buttons',
    targetScope: 'system',
  },
  'courier.assigned': {
    type: 'courier.assigned',
    description: 'Courier assigned to order',
    quietHours: 'always',
    renderGroup: 'track',
    targetScope: 'order',
  },
  'order.pending_aging': {
    type: 'order.pending_aging',
    description: 'Order pending longer than threshold',
    quietHours: 'always',
    renderGroup: 'aging_confirm',
    targetScope: 'order',
  },
  'order.ready_for_pickup': {
    type: 'order.ready_for_pickup',
    description: 'Order ready for courier pickup',
    quietHours: 'always',
    renderGroup: 'open_in_app',
    targetScope: 'order',
  },
  'shift.started': {
    type: 'shift.started',
    description: 'Courier shift started',
    quietHours: 'always',
    renderGroup: 'close_shift',
    targetScope: 'shift',
  },
  'shift.closed': {
    type: 'shift.closed',
    description: 'Courier shift closed',
    quietHours: 'always',
    renderGroup: 'close_shift',
    targetScope: 'shift',
  },
  'shift.close_reminder': {
    type: 'shift.close_reminder',
    description: 'Reminder to close open shift',
    quietHours: 'during_business',
    renderGroup: 'close_shift',
    targetScope: 'shift',
  },
  'test': {
    type: 'test',
    description: 'Test notification',
    quietHours: 'never',
    renderGroup: 'no_buttons',
    targetScope: 'test',
  },
};

export function isEventAllowedDuringQuietHours(event: string): boolean {
  const entry = EVENT_REGISTRY[event];
  if (!entry) return false;
  return entry.quietHours === 'always';
}

// ── Notification categories (TG_CATEGORY_GATING) ─────────────────────────────
// Maps each event to one of three categories. Per the recorded ETHICAL invariant
// (docs/design/telegram-notifications-actions/ethical-decisions.md):
//   category = REVERSIBILITY OF CONSEQUENCE, not loudness of the alert.
// Anything irreversible within a quiet window stays `transactional` — always sent,
// non-mutable, never gated by prefs or quiet-hours. `operational` / `quality` are the
// ONLY toggleable categories. Unlisted events default to `transactional` (fail-safe:
// when in doubt, the owner still gets it). order.timeout_cancelled, cash.reconcile_*,
// delivery.flag_raised, substitution_needs_human and order.pending_aging are therefore
// transactional even though an earlier draft grouped some under operational.
export type NotificationCategory = 'transactional' | 'operational' | 'quality';

const OPERATIONAL_EVENTS = new Set<string>(['shift.started', 'shift.closed', 'shift.close_reminder']);
const QUALITY_EVENTS = new Set<string>(['rating.low_received']);

export function getEventCategory(event: string): NotificationCategory {
  if (OPERATIONAL_EVENTS.has(event)) return 'operational';
  if (QUALITY_EVENTS.has(event)) return 'quality';
  return 'transactional';
}

// Category defaults mirror the prefs backfill (migration 1790000000052):
// operational ON, quality OFF. transactional is never read from prefs.
const CATEGORY_DEFAULT_ON: Record<NotificationCategory, boolean> = {
  transactional: true,
  operational: true,
  quality: false,
};

/**
 * TG_CATEGORY_GATING decision. Returns true to SUPPRESS the event for a target with
 * these prefs (caller then writes audit 'prefs_disabled'). Transactional → never
 * suppressed. Operational/quality → governed by prefs[category] with the defaults above.
 */
export function isSuppressedByCategory(event: string, prefs: Record<string, unknown> | null | undefined): boolean {
  const cat = getEventCategory(event);
  if (cat === 'transactional') return false;
  const val = prefs?.[cat];
  const enabled = val === undefined || val === null ? CATEGORY_DEFAULT_ON[cat] : val === true;
  return !enabled;
}
