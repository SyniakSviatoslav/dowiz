import type { MigrationBuilder } from 'node-pg-migrate';

// Set the demo storefront (Dubin & Sushi) to the brand palette extracted from
// its real website (sushi-durres-menu.netlify.app): dark teal background + gold
// accent + cream text. These are SEED colours — the client derives the full
// coherent palette (surfaces/borders/muted text) from them at render time.
const PRIMARY = '#d69a3d';
const BG = '#061b1a';
const TEXT = '#f5efe5';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    INSERT INTO location_themes (location_id, primary_color, bg_color, text_color)
    SELECT id, '${PRIMARY}', '${BG}', '${TEXT}' FROM locations WHERE slug = 'sushi-durres'
    ON CONFLICT (location_id) DO UPDATE SET
      primary_color = EXCLUDED.primary_color,
      bg_color      = EXCLUDED.bg_color,
      text_color    = EXCLUDED.text_color;
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    UPDATE location_themes lt SET
      primary_color = '#E53935', bg_color = '#FFF8F0', text_color = '#1A1A1A'
    FROM locations l WHERE l.id = lt.location_id AND l.slug = 'sushi-durres';
  `);
}
