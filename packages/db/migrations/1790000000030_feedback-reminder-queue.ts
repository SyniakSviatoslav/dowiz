import type { MigrationBuilder } from 'node-pg-migrate';

// Provision the order.feedback_reminder pg-boss queue with the privileged migration
// role (postgres). The runtime operational role lacks pgboss DDL (migration 009), so
// it can only send() to a pre-existing queue — creating it here avoids a runtime
// "permission denied for schema pgboss" failure.

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DO $$
    BEGIN
      IF NOT EXISTS (SELECT 1 FROM pgboss.queue WHERE name = 'order.feedback_reminder') THEN
        PERFORM pgboss.create_queue('order.feedback_reminder', '{}'::json);
      END IF;
    END
    $$;
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DO $$
    BEGIN
      PERFORM pgboss.delete_queue('order.feedback_reminder');
    EXCEPTION WHEN OTHERS THEN NULL;
    END
    $$;
  `);
}
