import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE menu_versions (
      location_id uuid PRIMARY KEY REFERENCES locations(id) ON DELETE CASCADE,
      version bigint NOT NULL DEFAULT 1,
      updated_at timestamptz NOT NULL DEFAULT now()
    );

    ALTER TABLE menu_versions ENABLE ROW LEVEL SECURITY;
    ALTER TABLE menu_versions FORCE ROW LEVEL SECURITY;

    CREATE POLICY tenant_isolation ON menu_versions
      USING ( location_id IN (SELECT app_member_location_ids()) );
  `);
}

export async function down(): Promise<void> {}
