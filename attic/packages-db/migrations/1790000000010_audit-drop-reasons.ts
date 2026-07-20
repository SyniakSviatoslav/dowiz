import { MigrationBuilder } from 'node-pg-migrate';

/**
 * H-3 · Extend notification_outbox_audit status enum with drop-reason values.
 *
 * New statuses: no_target, unknown_event, quiet_hours, dedup
 * These cover every code path where a notification is suppressed.
 * Existing statuses (queued, sending, delivered, failed, archived) remain.
 */

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE notification_outbox_audit
    DROP CONSTRAINT IF EXISTS notification_outbox_audit_status_check;
  `);
  pgm.sql(`
    ALTER TABLE notification_outbox_audit
    ADD CONSTRAINT notification_outbox_audit_status_check
    CHECK (status IN (
      'queued', 'sending', 'delivered', 'failed', 'archived',
      'no_target', 'unknown_event', 'quiet_hours', 'dedup',
      'target_inactive', 'prefs_disabled', 'order_not_found',
      'circuit_open', 'rate_limited'
    ));
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
    CHECK (status IN ('queued','sending','delivered','failed','archived'));
  `);
}
