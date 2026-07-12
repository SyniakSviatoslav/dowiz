import { MigrationBuilder } from 'node-pg-migrate';

/**
 * NX-5 · Notification outbox audit log
 * 
 * Provides observability for notification delivery pipeline.
 * Each row represents a delivery attempt or outcome.
 * PII-free: stores event type, target ID, channel, status, attempt count.
 */

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.createTable('notification_outbox_audit', {
    id: { type: 'uuid', notNull: true, primaryKey: true, default: pgm.func('gen_random_uuid()') },
    event: { type: 'text', notNull: true },
    target_id: { type: 'uuid' },
    location_id: { type: 'uuid', notNull: true },
    channel: { type: 'text', notNull: true },
    status: {
      type: 'text',
      notNull: true,
      check: "status IN ('queued','sending','delivered','failed','archived')",
    },
    attempts: { type: 'integer', notNull: true, default: 0 },
    error_message: { type: 'text' },
    created_at: { type: 'timestamptz', notNull: true, default: pgm.func('now()') },
    updated_at: { type: 'timestamptz', notNull: true, default: pgm.func('now()') },
  });

  pgm.createIndex('notification_outbox_audit', 'location_id');
  pgm.createIndex('notification_outbox_audit', 'status');
  pgm.createIndex('notification_outbox_audit', ['event', 'target_id']);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.dropTable('notification_outbox_audit');
}