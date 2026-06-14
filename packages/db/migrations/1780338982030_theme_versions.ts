import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  // Add columns to location_themes if they don't exist
  pgm.sql(`
    ALTER TABLE location_themes
      ADD COLUMN IF NOT EXISTS bg_color text,
      ADD COLUMN IF NOT EXISTS text_color text,
      ADD COLUMN IF NOT EXISTS frame_ancestors text[] NOT NULL DEFAULT ARRAY['self']::text[],
      ADD COLUMN IF NOT EXISTS font_url text;
  `);

  // Create theme_versions table
  pgm.sql(`
    CREATE TABLE IF NOT EXISTS theme_versions (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      location_id uuid NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
      css_hash text NOT NULL,
      css_body text NOT NULL,
      version bigint NOT NULL,
      created_at timestamptz NOT NULL DEFAULT now()
    );
  `);

  // Indexes
  pgm.sql(`CREATE UNIQUE INDEX IF NOT EXISTS theme_versions_loc_hash_uniq ON theme_versions(location_id, css_hash);`);
  pgm.sql(`CREATE UNIQUE INDEX IF NOT EXISTS theme_versions_loc_version_uniq ON theme_versions(location_id, version);`);

  // RLS for theme_versions
  pgm.sql(`ALTER TABLE theme_versions ENABLE ROW LEVEL SECURITY;`);

  pgm.sql(`
    CREATE POLICY theme_versions_owner_write ON theme_versions 
    FOR ALL
    TO authenticated
    USING (
      location_id IN (
        SELECT location_id FROM memberships WHERE user_id = (current_setting('request.jwt.claim.sub', true))::uuid
      )
    );
  `);

  pgm.sql(`
    CREATE POLICY theme_versions_public_read ON theme_versions 
    FOR SELECT 
    USING (
      location_id IN (
        SELECT id FROM locations WHERE status = 'active'
      )
    );
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`DROP TABLE IF EXISTS theme_versions CASCADE;`);
  
  pgm.sql(`
    ALTER TABLE location_themes 
      DROP COLUMN IF EXISTS bg_color,
      DROP COLUMN IF EXISTS text_color,
      DROP COLUMN IF EXISTS frame_ancestors,
      DROP COLUMN IF EXISTS font_url;
  `);
}
