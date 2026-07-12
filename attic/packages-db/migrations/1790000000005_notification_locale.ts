import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE owner_notification_targets
      ADD COLUMN IF NOT EXISTS locale text NOT NULL DEFAULT 'sq';
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE owner_notification_targets
      DROP COLUMN IF EXISTS locale;
  `);
}
