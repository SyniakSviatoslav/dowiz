import type { MigrationBuilder } from 'node-pg-migrate';

// UX-1 storefront links. Reuse the existing location_themes marketing block
// (which already holds google_maps_url / google_rating / google_review_count):
// add the Google Place ID (for the post-delivery "leave a review" deep link) and
// social handles. Nullable; inherit location_themes' tenant_isolation RLS.
export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE location_themes
      ADD COLUMN IF NOT EXISTS google_place_id  text,
      ADD COLUMN IF NOT EXISTS social_instagram text,
      ADD COLUMN IF NOT EXISTS social_facebook  text;
  `);
}

export async function down(): Promise<void> {
  // Forward-only per migration discipline.
}
