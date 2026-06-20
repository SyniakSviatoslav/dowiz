import type { MigrationBuilder } from 'node-pg-migrate';

// UX-3 entry-anchor photo. Optional photo of the building entrance, attached by
// the customer at checkout and shown to the assigned courier. Distinct from any
// proof-of-delivery photo (different actor + lifecycle). Nullable; inherits the
// orders tenant_isolation RLS. The R2 key is revealed only to the active
// courier (gated in the assignment response); bytes are served via /images/*.
export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`ALTER TABLE orders ADD COLUMN IF NOT EXISTS delivery_photo_key text;`);
}

export async function down(): Promise<void> {
  // Forward-only per migration discipline.
}
