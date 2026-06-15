import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE location_themes
      ADD COLUMN IF NOT EXISTS google_rating NUMERIC(2,1),
      ADD COLUMN IF NOT EXISTS google_review_count INT,
      ADD COLUMN IF NOT EXISTS google_maps_url TEXT;
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE location_themes
      DROP COLUMN IF EXISTS google_rating,
      DROP COLUMN IF EXISTS google_review_count,
      DROP COLUMN IF EXISTS google_maps_url;
  `);
}
