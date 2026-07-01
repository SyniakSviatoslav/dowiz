import type { MigrationBuilder } from 'node-pg-migrate';

// Per-tenant storefront fonts (see docs/design/storefront-fonts/DESIGN.md).
// Two nullable text columns holding an allowlist FONT id (never a URL or raw family) — heading + body.
// Written at provision time from the cuisine default (demo-builder theme seam) and overridable by the
// owner via PUT /owner/brand. Null → the client resolves to the DEFAULT_FONT_PAIRING (Playfair/Inter),
// which matches the historical hardcoded storefront heading. Additive & reversible; RLS/FORCE on
// location_themes is unchanged (no new policy needed — same row, same tenant scope).
export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE location_themes
      ADD COLUMN IF NOT EXISTS heading_font text,
      ADD COLUMN IF NOT EXISTS body_font text;
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE location_themes
      DROP COLUMN IF EXISTS heading_font,
      DROP COLUMN IF EXISTS body_font;
  `);
}
