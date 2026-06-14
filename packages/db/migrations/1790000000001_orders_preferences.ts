import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE orders ADD COLUMN IF NOT EXISTS preferences jsonb NOT NULL DEFAULT '{}';
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE orders DROP COLUMN IF EXISTS preferences;
  `);
}
