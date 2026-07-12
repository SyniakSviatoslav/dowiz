import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE locations
      ADD COLUMN IF NOT EXISTS address text,
      ADD COLUMN IF NOT EXISTS public_phone text,
      ADD COLUMN IF NOT EXISTS hours_json jsonb DEFAULT '{}'::jsonb,
      ADD COLUMN IF NOT EXISTS geo jsonb DEFAULT '{}'::jsonb;
  `);

  pgm.sql(`UPDATE locations SET hours_json = '{}'::jsonb WHERE hours_json IS NULL;`);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE locations
      DROP COLUMN IF EXISTS address,
      DROP COLUMN IF EXISTS public_phone,
      DROP COLUMN IF EXISTS hours_json,
      DROP COLUMN IF EXISTS geo;
  `);
}
