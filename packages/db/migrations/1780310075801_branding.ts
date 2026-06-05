import type { MigrationBuilder } from 'node-pg-migrate';



export async function up(pgm: MigrationBuilder): Promise<void> {
  // 1. Location Themes
  pgm.sql(`
    CREATE TABLE location_themes (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      location_id uuid NOT NULL UNIQUE REFERENCES locations(id),
      logo_url text,
      primary_color text,
      css_hash text,
      updated_at timestamptz NOT NULL DEFAULT now()
    );
  `);

  // 2. RLS
  pgm.sql(`
    ALTER TABLE location_themes ENABLE ROW LEVEL SECURITY;
    ALTER TABLE location_themes FORCE ROW LEVEL SECURITY;
    CREATE POLICY tenant_isolation ON location_themes
      USING ( location_id IN (SELECT app_member_location_ids()) );
  `);
}

export async function down(): Promise<void> {}
