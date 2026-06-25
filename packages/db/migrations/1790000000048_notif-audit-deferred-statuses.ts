import type { MigrationBuilder } from 'node-pg-migrate';

/**
 * TG-NOTIF-1 · Extend notification_outbox_audit status enum with the two NEW
 * hold/drop reasons introduced by the extended-Telegram-notifications design
 * (docs/design/telegram-notifications-actions/).
 *
 * DESIGN-VS-REALITY CORRECTION:
 *   The council proposal flagged "BUG-A" (gating no-ops throwing 23514 because the
 *   CHECK only had queued/sending/delivered/failed/archived). That was a stale read
 *   of the ORIGINAL 1790000000007 CHECK. Migration 1790000000010 (H-3) already
 *   extended the enum to cover every existing suppression path the dispatcher writes
 *   (no_target, target_inactive, prefs_disabled, quiet_hours, order_not_found, dedup,
 *   circuit_open, rate_limited, unknown_event) — verified against
 *   apps/api/src/notifications/workers/index.ts. So "BUG-A" is NOT re-fixed here.
 *
 *   This migration adds ONLY the two statuses this feature introduces:
 *     - 'held'              : quiet-hours deferred-deliver re-enqueue (pg-boss startAfter).
 *     - 'quiet_tz_fallback' : quiet-hours gating fell back to default TZ because
 *                             locations.timezone was NULL (BR-14 audit trail).
 *   Additive, forward-only. Emitted only once the dispatcher paths land behind
 *   TG_CATEGORY_GATING (default off); adding them first prevents a future 23514.
 */

const PRIOR_STATUSES = [
  'queued', 'sending', 'delivered', 'failed', 'archived',
  'no_target', 'unknown_event', 'quiet_hours', 'dedup',
  'target_inactive', 'prefs_disabled', 'order_not_found',
  'circuit_open', 'rate_limited',
];

const NEW_STATUSES = ['held', 'quiet_tz_fallback'];

function checkClause(values: string[]): string {
  return `CHECK (status IN (${values.map((v) => `'${v}'`).join(', ')}))`;
}

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE notification_outbox_audit
    DROP CONSTRAINT IF EXISTS notification_outbox_audit_status_check;
  `);
  pgm.sql(`
    ALTER TABLE notification_outbox_audit
    ADD CONSTRAINT notification_outbox_audit_status_check
    ${checkClause([...PRIOR_STATUSES, ...NEW_STATUSES])};
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE notification_outbox_audit
    DROP CONSTRAINT IF EXISTS notification_outbox_audit_status_check;
  `);
  pgm.sql(`
    ALTER TABLE notification_outbox_audit
    ADD CONSTRAINT notification_outbox_audit_status_check
    ${checkClause(PRIOR_STATUSES)};
  `);
}
