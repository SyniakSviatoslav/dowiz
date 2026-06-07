import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE orders ADD COLUMN IF NOT EXISTS ready_at timestamptz;
    ALTER TABLE orders ADD COLUMN IF NOT EXISTS delivered_at timestamptz;
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE orders DROP COLUMN IF EXISTS ready_at;
    ALTER TABLE orders DROP COLUMN IF EXISTS delivered_at;
  `);
}
