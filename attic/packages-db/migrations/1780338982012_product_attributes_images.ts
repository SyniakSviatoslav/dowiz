import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE products
      ADD COLUMN attributes jsonb NOT NULL DEFAULT '{}'::jsonb,
      ADD COLUMN image_key text;
  `);
}

export async function down(): Promise<void> {}
