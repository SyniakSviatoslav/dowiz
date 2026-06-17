import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE theme_versions FORCE ROW LEVEL SECURITY;
    ALTER TABLE telegram_connect_tokens FORCE ROW LEVEL SECURITY;
    ALTER TABLE owner_notification_targets FORCE ROW LEVEL SECURITY;
    ALTER TABLE backup_metadata FORCE ROW LEVEL SECURITY;
    ALTER TABLE backup_audit_log FORCE ROW LEVEL SECURITY;
    ALTER TABLE order_ratings FORCE ROW LEVEL SECURITY;
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE theme_versions NO FORCE ROW LEVEL SECURITY;
    ALTER TABLE telegram_connect_tokens NO FORCE ROW LEVEL SECURITY;
    ALTER TABLE owner_notification_targets NO FORCE ROW LEVEL SECURITY;
    ALTER TABLE backup_metadata NO FORCE ROW LEVEL SECURITY;
    ALTER TABLE backup_audit_log NO FORCE ROW LEVEL SECURITY;
    ALTER TABLE order_ratings NO FORCE ROW LEVEL SECURITY;
  `);
}
